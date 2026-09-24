//! Lookahead Brickwall Limiter untuk perlindungan peak tanpa batas overshoot.

use super::envelope::{DetectionMode, EnvelopeFollower};
use crate::buffer::delay_line::{DelayLine, InterpolationType};
use crate::gain::Gain;

#[derive(Debug, Clone)]
pub struct BrickwallLimiter {
    ceiling_db: f32,
    ceiling_linear: f32,
    lookahead_delay: DelayLine,
    lookahead_samples: f32,
    envelope: EnvelopeFollower,
}

impl BrickwallLimiter {
    /// Membuat Limiter dengan lookahead time
    ///
    /// # Arguments
    /// * `sample_rate` - Frekuensi sampling (Hz)
    /// * `ceiling_db` - Batas maksimum keluaran sinyal (dBFS, misal: -0.1 atau -1.0)
    /// * `release_ms` - Waktu pemulihan limiter (ms)
    /// * `lookahead_ms` - Jeda intip buffer (ms, misal: 2.0 s/d 5.0 ms)
    pub fn new(sample_rate: f32, ceiling_db: f32, release_ms: f32, lookahead_ms: f32) -> Self {
        let ceiling_linear = Gain::db_to_linear(ceiling_db.min(0.0));
        let lookahead_samples = (lookahead_ms * 0.001 * sample_rate).round().max(1.0);
        let max_delay = (lookahead_samples as usize) + 8;

        // Limiter memakai fast-attack peak detection
        let envelope = EnvelopeFollower::new(sample_rate, 0.1, release_ms, DetectionMode::Peak);

        Self {
            ceiling_db,
            ceiling_linear,
            lookahead_delay: DelayLine::new(max_delay),
            lookahead_samples,
            envelope,
        }
    }

    pub fn ceiling_db(&self) -> f32 {
        self.ceiling_db
    }

    pub fn ceiling_linear(&self) -> f32 {
        self.ceiling_linear
    }

    /// Memproses satu sampel audio dengan jaminan proteksi ceiling
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // 1. Catat input ke lookahead buffer
        self.lookahead_delay.push(input);

        // 2. Evaluasi envelope puncak
        let env_linear = self.envelope.process_sample(input);

        // 3. Hitung rasio pelemahan
        let gain_reduction = if env_linear > self.ceiling_linear && self.ceiling_linear > 1e-6 {
            self.ceiling_linear / env_linear
        } else {
            1.0
        };

        // 4. Ambil audio dari lookahead buffer
        let delayed_input = self
            .lookahead_delay
            .read_delay(self.lookahead_samples, InterpolationType::None);

        let limited_output = delayed_input * gain_reduction;

        // Hard safety clamp akhir
        limited_output.clamp(-self.ceiling_linear, self.ceiling_linear)
    }

    /// Memproses seluruh buffer in-place
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn reset(&mut self) {
        self.lookahead_delay.clear();
        self.envelope.reset();
    }
}
