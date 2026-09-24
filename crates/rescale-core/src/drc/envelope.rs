//! Envelope Follower dengan dukungan mode deteksi Peak & RMS, serta parameter Attack/Release.

/// Mode deteksi level sinyal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMode {
    Peak,
    Rms,
}

#[derive(Debug, Clone)]
pub struct EnvelopeFollower {
    attack_coeff: f32,
    release_coeff: f32,
    current_level: f32,
    mode: DetectionMode,
}

impl EnvelopeFollower {
    /// Membuat Envelope Follower baru
    ///
    /// # Arguments
    /// * `sample_rate` - Frekuensi sampling sinyal (Hz)
    /// * `attack_ms` - Waktu respon attack (ms)
    /// * `release_ms` - Waktu pemulihan release (ms)
    /// * `mode` - Mode estimasi (`Peak` atau `Rms`)
    pub fn new(sample_rate: f32, attack_ms: f32, release_ms: f32, mode: DetectionMode) -> Self {
        let mut follower = Self {
            attack_coeff: 0.0,
            release_coeff: 0.0,
            current_level: 0.0,
            mode,
        };
        follower.set_time_constants(sample_rate, attack_ms, release_ms);
        follower
    }

    /// Memperbarui konstanta waktu attack dan release
    pub fn set_time_constants(&mut self, sample_rate: f32, attack_ms: f32, release_ms: f32) {
        let attack_sec = (attack_ms * 0.001).max(1e-5);
        let release_sec = (release_ms * 0.001).max(1e-5);

        // Faktor smoothing eksponensial: alpha = exp(-1 / (time_sec * sample_rate))
        self.attack_coeff = (-1.0 / (attack_sec * sample_rate)).exp();
        self.release_coeff = (-1.0 / (release_sec * sample_rate)).exp();
    }

    pub fn set_mode(&mut self, mode: DetectionMode) {
        self.mode = mode;
    }

    /// Memproses satu sampel audio dan mengembalikan envelope linear
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let target = match self.mode {
            DetectionMode::Peak => input.abs(),
            DetectionMode::Rms => input * input, // Domain power
        };

        if target > self.current_level {
            // Fase Attack
            self.current_level =
                self.attack_coeff * self.current_level + (1.0 - self.attack_coeff) * target;
        } else {
            // Fase Release
            self.current_level =
                self.release_coeff * self.current_level + (1.0 - self.release_coeff) * target;
        }

        match self.mode {
            DetectionMode::Peak => self.current_level,
            DetectionMode::Rms => self.current_level.sqrt(),
        }
    }

    #[inline(always)]
    pub fn current_level(&self) -> f32 {
        match self.mode {
            DetectionMode::Peak => self.current_level,
            DetectionMode::Rms => self.current_level.sqrt(),
        }
    }

    pub fn reset(&mut self) {
        self.current_level = 0.0;
    }
}
