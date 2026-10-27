//! Lookahead Brickwall Limiter dengan sliding maximum-hold gain reduction.
//!
//! Desain lama menurunkan gain reduction dari envelope eksponensial, sehingga
//! transient perkusif bisa lolos lalu dipotong hard clamp (distorsi). Desain ini
//! memakai nilai maksimum absolut di seluruh window lookahead: pelemahan sudah
//! aktif sebelum transient mencapai output, dan hard clamp hanya menjadi jaring
//! pengaman terakhir.

use super::max_hold::SlidingMax;
use crate::buffer::delay_line::{DelayLine, InterpolationType};
use crate::gain::Gain;

/// Ambang minimum ceiling linear agar pembagian tetap valid.
const MIN_CEILING_LINEAR: f32 = 1e-6;
/// Cadangan kapasitas DelayLine agar pembacaan interpolasi selalu aman.
const DELAY_CAPACITY_HEADROOM: usize = 8;
/// Waktu release minimum (ms) agar koefisien smoothing tetap stabil.
const MIN_RELEASE_MS: f32 = 0.1;

/// Detektor gain reduction berbasis sliding maximum pada window lookahead.
///
/// Detektor dipisahkan dari delay audio supaya beberapa channel bisa berbagi
/// satu detektor — dasar untuk stereo-linked limiting (`LimiterProcessor`).
pub struct LookaheadDetector {
    ceiling_linear: f32,
    lookahead_samples: usize,
    max_peak: SlidingMax,
    current_gain: f32,
    release_coeff: f32,
    hold_frames_remaining: usize,
}

impl LookaheadDetector {
    /// # Arguments
    /// * `sample_rate` - Frekuensi sampling (Hz)
    /// * `ceiling_db` - Batas keluaran (dBFS, mis. -1.0)
    /// * `release_ms` - Waktu pemulihan gain reduction (ms)
    /// * `lookahead_ms` - Panjang window lookahead (ms)
    pub fn new(sample_rate: f32, ceiling_db: f32, release_ms: f32, lookahead_ms: f32) -> Self {
        let ceiling_linear = Gain::db_to_linear(ceiling_db.min(0.0));
        let lookahead_samples = ((lookahead_ms * 0.001 * sample_rate).round() as usize).max(1);
        let release_seconds = (release_ms.max(MIN_RELEASE_MS) * 0.001).max(MIN_RELEASE_MS * 0.001);
        let release_coeff = (-1.0 / (release_seconds * sample_rate)).exp();

        Self {
            ceiling_linear,
            lookahead_samples,
            // Window mencakup sampel yang sedang keluar + seluruh lookahead ke depan.
            max_peak: SlidingMax::new(lookahead_samples + 1),
            current_gain: 1.0,
            release_coeff,
            hold_frames_remaining: 0,
        }
    }

    /// Jumlah sample delay audio yang menyertai detektor ini.
    pub fn latency_samples(&self) -> usize {
        self.lookahead_samples
    }

    pub fn current_gain(&self) -> f32 {
        self.current_gain
    }

    pub fn ceiling_linear(&self) -> f32 {
        self.ceiling_linear
    }

    /// Memasukkan puncak input satu frame dan mengembalikan gain yang harus dipakai.
    ///
    /// Attack bersifat instan: begitu puncak baru masuk window, gain langsung
    /// turun sehingga tidak ada sampel yang lolos di atas ceiling. Release
    /// dihaluskan eksponensial, dengan min-hold agar gain tidak naik sebelum
    /// transient melewati output.
    pub fn process_input_peak(&mut self, input_peak: f32) -> f32 {
        let peak = if input_peak.is_finite() {
            input_peak.abs()
        } else {
            0.0
        };
        self.max_peak.push(peak);

        let window_peak = self.max_peak.max();
        let target_gain =
            if window_peak > self.ceiling_linear && self.ceiling_linear > MIN_CEILING_LINEAR {
                self.ceiling_linear / window_peak
            } else {
                1.0
            };

        if target_gain < self.current_gain {
            self.current_gain = target_gain;
            self.hold_frames_remaining = self.lookahead_samples;
        } else if self.hold_frames_remaining > 0 {
            self.hold_frames_remaining -= 1;
        } else {
            self.current_gain =
                self.release_coeff * self.current_gain + (1.0 - self.release_coeff) * target_gain;
        }

        self.current_gain.clamp(0.0, 1.0)
    }

    pub fn reset(&mut self) {
        self.max_peak.reset();
        self.current_gain = 1.0;
        self.hold_frames_remaining = 0;
    }
}

/// Limiter single-channel berbasis lookahead delay + [`LookaheadDetector`].
pub struct BrickwallLimiter {
    ceiling_db: f32,
    ceiling_linear: f32,
    lookahead_delay: DelayLine,
    detector: LookaheadDetector,
}

impl BrickwallLimiter {
    /// Membuat Limiter dengan lookahead time.
    ///
    /// # Arguments
    /// * `sample_rate` - Frekuensi sampling (Hz)
    /// * `ceiling_db` - Batas maksimum keluaran sinyal (dBFS, misal: -0.1 atau -1.0)
    /// * `release_ms` - Waktu pemulihan limiter (ms)
    /// * `lookahead_ms` - Jeda intip buffer (ms, misal: 2.0 s/d 5.0 ms)
    pub fn new(sample_rate: f32, ceiling_db: f32, release_ms: f32, lookahead_ms: f32) -> Self {
        let detector = LookaheadDetector::new(sample_rate, ceiling_db, release_ms, lookahead_ms);
        Self {
            ceiling_db,
            ceiling_linear: detector.ceiling_linear(),
            lookahead_delay: DelayLine::new(detector.latency_samples() + DELAY_CAPACITY_HEADROOM),
            detector,
        }
    }

    pub fn ceiling_db(&self) -> f32 {
        self.ceiling_db
    }

    pub fn ceiling_linear(&self) -> f32 {
        self.ceiling_linear
    }

    pub fn latency_samples(&self) -> usize {
        self.detector.latency_samples()
    }

    /// Memproses satu sampel audio dengan jaminan proteksi ceiling.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let delay_samples = self.detector.latency_samples() as f32;
        self.lookahead_delay.push(input);
        let delayed_input = self
            .lookahead_delay
            .read_delay(delay_samples, InterpolationType::None);
        let gain = self.detector.process_input_peak(input);
        let limited_output = delayed_input * gain;

        // Jaring pengaman terakhir: pembulatan f32 tidak boleh melewati ceiling.
        limited_output.clamp(-self.ceiling_linear, self.ceiling_linear)
    }

    /// Memproses seluruh buffer in-place.
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn reset(&mut self) {
        self.lookahead_delay.clear();
        self.detector.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44_100.0;
    const CEILING_DB: f32 = -1.0;
    const LOOKAHEAD_MS: f32 = 2.0;

    fn ceiling_linear() -> f32 {
        Gain::db_to_linear(CEILING_DB)
    }

    fn lookahead_samples() -> usize {
        (LOOKAHEAD_MS * 0.001 * SAMPLE_RATE).round() as usize
    }

    #[test]
    fn lookahead_detector_gain_matches_window_peak() {
        // Window 4 sample: puncak 0.8 dengan ceiling 0.5 => gain 0.625.
        let mut detector = LookaheadDetector::new(1_000.0, Gain::linear_to_db(0.5), 50.0, 4.0);

        assert!((detector.process_input_peak(0.2) - 1.0).abs() < 1e-6);
        assert!((detector.process_input_peak(0.4) - 1.0).abs() < 1e-6);
        assert!((detector.process_input_peak(0.8) - 0.625).abs() < 1e-6);
    }

    #[test]
    fn percussive_transients_never_exceed_ceiling() {
        let mut limiter = BrickwallLimiter::new(SAMPLE_RATE, CEILING_DB, 50.0, LOOKAHEAD_MS);
        let mut signal = vec![0.0_f32; 2_000];
        for index in [100, 400, 401, 900, 1_500] {
            signal[index] = 0.95;
        }

        limiter.process_buffer(&mut signal);

        let peak = signal
            .iter()
            .fold(0.0_f32, |acc, sample| acc.max(sample.abs()));
        assert!(
            peak <= ceiling_linear() + 1e-6,
            "peak {peak} melebihi ceiling {}",
            ceiling_linear()
        );
    }

    #[test]
    fn gain_reduction_is_active_before_the_transient_reaches_output() {
        let mut limiter = BrickwallLimiter::new(SAMPLE_RATE, CEILING_DB, 50.0, LOOKAHEAD_MS);
        let latency = lookahead_samples();
        let quiet_level = 0.1_f32;
        let burst_level = 0.95_f32;
        let burst_index = 200;

        let mut signal = vec![quiet_level; 1_000];
        signal[burst_index] = burst_level;
        limiter.process_buffer(&mut signal);

        // Jauh sebelum burst masuk window: tidak ada pelemahan.
        assert!(
            (signal[100] - quiet_level).abs() < 1e-4,
            "sampel awal tidak boleh teredam, dapat {}",
            signal[100]
        );
        // Saat burst sudah masuk window tetapi belum sampai output: gain turun.
        let attenuated = quiet_level * (ceiling_linear() / burst_level);
        assert!(
            (signal[burst_index + 40] - attenuated).abs() < 1e-3,
            "gain reduction harus aktif sebelum transient mencapai output, dapat {}",
            signal[burst_index + 40]
        );
        // Sampel burst sendiri keluar tepat di ceiling.
        assert!(
            (signal[burst_index + latency] - ceiling_linear()).abs() < 1e-4,
            "transient harus keluar tepat di ceiling, dapat {}",
            signal[burst_index + latency]
        );
    }

    #[test]
    fn reset_restores_unity_gain() {
        let mut limiter = BrickwallLimiter::new(SAMPLE_RATE, CEILING_DB, 50.0, LOOKAHEAD_MS);
        let mut hot = vec![0.95_f32; 200];
        limiter.process_buffer(&mut hot);
        limiter.reset();

        let mut quiet = vec![0.0_f32; 200];
        limiter.process_buffer(&mut quiet);
        assert!(quiet.iter().all(|&sample| sample.abs() < 1e-6));
    }
}
