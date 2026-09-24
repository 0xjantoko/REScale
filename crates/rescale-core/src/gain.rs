//! Modul manipulasi gain dan konversi unit decibel.

pub struct Gain;

impl Gain {
    /// Mengonversi nilai Decibel (dB) ke faktor amplitudo linier.
    /// Rumus: G = 10^(dB / 20)
    #[inline(always)]
    pub fn db_to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    /// Mengonversi faktor amplitudo linier ke Decibel (dB).
    /// Rumus: dB = 20 * log10(amplitude)
    #[inline(always)]
    pub fn linear_to_db(linear: f32) -> f32 {
        if linear <= 1e-6 {
            -120.0 // Threshold batas noise floor
        } else {
            20.0 * linear.log10()
        }
    }

    /// Mengaplikasikan gain linier langsung ke buffer audio in-place.
    #[inline]
    pub fn apply_linear(buffer: &mut [f32], factor: f32) {
        for sample in buffer.iter_mut() {
            *sample *= factor;
        }
    }

    /// Mengaplikasikan perubahan gain dalam dB ke buffer in-place.
    #[inline]
    pub fn apply_db(buffer: &mut [f32], db: f32) {
        let factor = Self::db_to_linear(db);
        Self::apply_linear(buffer, factor);
    }
}
