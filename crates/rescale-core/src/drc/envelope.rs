//! Envelope Follower dengan dukungan mode deteksi Peak & RMS, serta parameter Attack/Release.

/// Batas bawah time constant (detik) agar perhitungan koefisien tetap stabil.
const MIN_TIME_SECONDS: f32 = 1e-5;

/// Mode deteksi level sinyal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMode {
    Peak,
    Rms,
}

#[derive(Debug, Clone)]
pub struct EnvelopeFollower {
    /// Koefisien smoothing pada domain amplitudo (bukan domain internal).
    attack_coeff: f32,
    release_coeff: f32,
    /// Level internal: amplitudo untuk `Peak`, power untuk `Rms`.
    current_level: f32,
    mode: DetectionMode,
}

impl EnvelopeFollower {
    /// Membuat Envelope Follower baru.
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

    /// Memperbarui konstanta waktu attack dan release.
    ///
    /// Koefisien yang disimpan selalu berada di domain amplitudo; koreksi domain
    /// power diterapkan saat pemrosesan (lihat `effective_coefficients`).
    pub fn set_time_constants(&mut self, sample_rate: f32, attack_ms: f32, release_ms: f32) {
        let attack_seconds = (attack_ms * 0.001).max(MIN_TIME_SECONDS);
        let release_seconds = (release_ms * 0.001).max(MIN_TIME_SECONDS);

        self.attack_coeff = Self::smoothing_coefficient(attack_seconds, sample_rate);
        self.release_coeff = Self::smoothing_coefficient(release_seconds, sample_rate);
    }

    pub fn set_mode(&mut self, mode: DetectionMode) {
        self.mode = mode;
    }

    /// Memproses satu sampel audio dan mengembalikan envelope linear.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let (attack_coeff, release_coeff) = self.effective_coefficients();
        let target = match self.mode {
            DetectionMode::Peak => input.abs(),
            DetectionMode::Rms => input * input, // Domain power
        };

        let coefficient = if target > self.current_level {
            attack_coeff
        } else {
            release_coeff
        };
        self.current_level = coefficient * self.current_level + (1.0 - coefficient) * target;

        self.to_linear()
    }

    #[inline(always)]
    pub fn current_level(&self) -> f32 {
        self.to_linear()
    }

    pub fn reset(&mut self) {
        self.current_level = 0.0;
    }

    /// Koefisien smoothing satu kutub: `alpha = exp(-1 / (tau * fs))`.
    #[inline]
    fn smoothing_coefficient(time_seconds: f32, sample_rate: f32) -> f32 {
        (-1.0 / (time_seconds * sample_rate)).exp()
    }

    /// Koefisien efektif untuk domain internal saat ini.
    ///
    /// Mode RMS menyimpan level sebagai power = amplitudo². Karena itu koefisien
    /// amplitudo `r` harus dipakai sebagai `r²` pada domain power — tanpa koreksi
    /// ini, release RMS berjalan ~2× lebih lambat dari nilai yang dikonfigurasi
    /// (power meluruh dua kali lebih cepat daripada amplitudo).
    #[inline(always)]
    fn effective_coefficients(&self) -> (f32, f32) {
        match self.mode {
            DetectionMode::Peak => (self.attack_coeff, self.release_coeff),
            DetectionMode::Rms => (
                self.attack_coeff * self.attack_coeff,
                self.release_coeff * self.release_coeff,
            ),
        }
    }

    /// Konversi level internal ke amplitudo linear.
    #[inline(always)]
    fn to_linear(&self) -> f32 {
        match self.mode {
            DetectionMode::Peak => self.current_level,
            DetectionMode::Rms => self.current_level.sqrt(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 1_000.0;
    const RELEASE_MS: f32 = 100.0;
    const TARGET_TIME_CONSTANT_SAMPLES: f32 = RELEASE_MS; // 100 ms @ 1 kHz = 100 sampel

    /// Mengukur waktu (dalam sampel) sampai envelope turun ke `1/e` dari nilai awal
    /// setelah sinyal input berhenti.
    fn measured_release_samples(mode: DetectionMode) -> f32 {
        let mut envelope = EnvelopeFollower::new(SAMPLE_RATE, 1.0, RELEASE_MS, mode);

        // Settle: dorong envelope ke level stabil 1.0.
        for _ in 0..500 {
            envelope.process_sample(1.0);
        }

        let settled_level = envelope.process_sample(0.0);
        let threshold = settled_level / std::f32::consts::E;

        let mut samples = 0_u32;
        loop {
            let level = envelope.process_sample(0.0);
            samples += 1;
            if level <= threshold || samples > 10 * TARGET_TIME_CONSTANT_SAMPLES as u32 {
                break;
            }
        }
        samples as f32
    }

    #[test]
    fn rms_release_time_matches_configured_value() {
        let measured = measured_release_samples(DetectionMode::Rms);
        assert!(
            (measured - TARGET_TIME_CONSTANT_SAMPLES).abs() <= 5.0,
            "release RMS {measured:.1} sampel, seharusnya ~{TARGET_TIME_CONSTANT_SAMPLES:.1}"
        );
    }

    #[test]
    fn peak_release_time_matches_configured_value() {
        let measured = measured_release_samples(DetectionMode::Peak);
        assert!(
            (measured - TARGET_TIME_CONSTANT_SAMPLES).abs() <= 5.0,
            "release Peak {measured:.1} sampel, seharusnya ~{TARGET_TIME_CONSTANT_SAMPLES:.1}"
        );
    }

    #[test]
    fn rms_level_of_full_scale_sine_stays_near_unity() {
        let mut envelope = EnvelopeFollower::new(SAMPLE_RATE, 1.0, RELEASE_MS, DetectionMode::Rms);

        // RMS dari sine amplitudo 1.0 adalah 1/sqrt(2) ~= 0.707; envelope RMS
        // yang di-smoothing menuju nilai absolut rata-rata yang sama.
        for _ in 0..2_000 {
            envelope.process_sample(1.0);
        }

        assert!(
            envelope.current_level() > 0.95,
            "RMS envelope untuk input DC 1.0 seharusnya mendekati 1.0, dapat {}",
            envelope.current_level()
        );
    }
}
