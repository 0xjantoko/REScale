//! Dynamic Range Audio Compressor dengan Soft-Knee dan Makeup Gain.

use super::envelope::{DetectionMode, EnvelopeFollower};
use crate::gain::Gain;

/// Konfigurasi parameter kompresor audio
#[derive(Debug, Clone)]
pub struct CompressorConfig {
    pub threshold_db: f32,   // Titik ambang aktivasi (dBFS, misal: -20.0)
    pub ratio: f32,          // Rasio kompresi (misal: 4.0 untuk 4:1)
    pub knee_db: f32,        // Lebar kurva transisi halus (dB)
    pub attack_ms: f32,      // Waktu respon attack (ms)
    pub release_ms: f32,     // Waktu pemulihan release (ms)
    pub makeup_gain_db: f32, // Kompensasi level output (dB)
    pub mode: DetectionMode, // Mode deteksi sinyal (Peak / RMS)
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            threshold_db: -20.0,
            ratio: 4.0,
            knee_db: 6.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 0.0,
            mode: DetectionMode::Peak,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Compressor {
    config: CompressorConfig,
    envelope: EnvelopeFollower,
}

impl Compressor {
    pub fn new(sample_rate: f32, config: CompressorConfig) -> Self {
        let envelope = EnvelopeFollower::new(
            sample_rate,
            config.attack_ms,
            config.release_ms,
            config.mode,
        );
        Self { config, envelope }
    }

    pub fn set_config(&mut self, sample_rate: f32, config: CompressorConfig) {
        self.envelope
            .set_time_constants(sample_rate, config.attack_ms, config.release_ms);
        self.envelope.set_mode(config.mode);
        self.config = config;
    }

    pub fn config(&self) -> &CompressorConfig {
        &self.config
    }

    /// Menghitung kurva penguatan (Gain Computer) dengan karakteristik Soft-Knee
    #[inline(always)]
    fn compute_gain_reduction_db(&self, env_db: f32) -> f32 {
        let t = self.config.threshold_db;
        let r = self.config.ratio.max(1.0);
        let w = self.config.knee_db;

        if w <= 0.0 {
            // Hard knee
            if env_db <= t {
                0.0
            } else {
                (t - env_db) * (1.0 - 1.0 / r)
            }
        } else if 2.0 * (env_db - t) < -w {
            // Sinyal di bawah knee: tidak ada kompresi
            0.0
        } else if 2.0 * (env_db - t).abs() <= w {
            // Sinyal berada dalam transisi kuadratik Soft-Knee
            let diff = env_db - t + w / 2.0;
            -(diff * diff) / (2.0 * w) * (1.0 - 1.0 / r)
        } else {
            // Sinyal di atas knee: kompresi penuh
            (t - env_db) * (1.0 - 1.0 / r)
        }
    }

    /// Memproses single sample audio
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let env_linear = self.envelope.process_sample(input);
        let env_db = Gain::linear_to_db(env_linear);

        let gain_reduction_db = self.compute_gain_reduction_db(env_db);
        let total_gain_db = gain_reduction_db + self.config.makeup_gain_db;
        let total_gain_linear = Gain::db_to_linear(total_gain_db);

        input * total_gain_linear
    }

    /// Memproses seluruh buffer sample in-place
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn reset(&mut self) {
        self.envelope.reset();
    }
}
