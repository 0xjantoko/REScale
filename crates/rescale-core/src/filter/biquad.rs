//! Implementasi Biquad Filter Direct Form I & Transposed Direct Form II.

use super::coefficients::BiquadCoefficients;

/// Biquad Filter menggunakan arsitektur Transposed Direct Form II
/// (Memiliki keunggulan stabilitas numerik terbaik untuk floating-point DSP)
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    coeffs: BiquadCoefficients,
    // State memory (z^-1, z^-2)
    s1: f32,
    s2: f32,
}

impl BiquadFilter {
    pub fn new(coeffs: BiquadCoefficients) -> Self {
        Self {
            coeffs,
            s1: 0.0,
            s2: 0.0,
        }
    }

    pub fn set_coefficients(&mut self, coeffs: BiquadCoefficients) {
        self.coeffs = coeffs;
    }

    pub fn coefficients(&self) -> &BiquadCoefficients {
        &self.coeffs
    }

    /// Memproses single sample audio secara real-time (Transposed Direct Form II)
    ///
    /// Persamaan (difference equations):
    ///
    /// ```text
    /// y[n]  = b0 * x[n] + s1
    /// s1    = b1 * x[n] - a1 * y[n] + s2
    /// s2    = b2 * x[n] - a2 * y[n]
    /// ```
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let output = self.coeffs.b0 * input + self.s1;
        self.s1 = self.coeffs.b1 * input - self.coeffs.a1 * output + self.s2;
        self.s2 = self.coeffs.b2 * input - self.coeffs.a2 * output;
        output
    }

    /// Memproses seluruh buffer sample in-place
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    /// Reset internal delay memory
    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}
