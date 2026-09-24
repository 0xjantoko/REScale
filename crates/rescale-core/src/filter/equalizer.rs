//! Parametric Equalizer 3-Band (Low Shelf, Mid Peaking, High Shelf).

use super::biquad::BiquadFilter;
use super::coefficients::BiquadCoefficients;

#[derive(Debug, Clone)]
pub struct ParametricEqualizer3Band {
    sample_rate: f32,
    low_shelf: BiquadFilter,
    mid_peaking: BiquadFilter,
    high_shelf: BiquadFilter,
}

impl ParametricEqualizer3Band {
    pub fn new(sample_rate: f32) -> Self {
        let low_coeffs = BiquadCoefficients::low_shelf(sample_rate, 200.0, 0.0);
        let mid_coeffs = BiquadCoefficients::peaking_eq(sample_rate, 1000.0, 1.0, 0.0);
        let high_coeffs = BiquadCoefficients::high_shelf(sample_rate, 5000.0, 0.0);

        Self {
            sample_rate,
            low_shelf: BiquadFilter::new(low_coeffs),
            mid_peaking: BiquadFilter::new(mid_coeffs),
            high_shelf: BiquadFilter::new(high_coeffs),
        }
    }

    /// Mengatur parameter Low-Shelf (Bass)
    pub fn set_low_band(&mut self, cutoff_hz: f32, gain_db: f32) {
        let coeffs = BiquadCoefficients::low_shelf(self.sample_rate, cutoff_hz, gain_db);
        self.low_shelf.set_coefficients(coeffs);
    }

    /// Mengatur parameter Mid Peaking (Vocal / Mids)
    pub fn set_mid_band(&mut self, center_hz: f32, q: f32, gain_db: f32) {
        let coeffs = BiquadCoefficients::peaking_eq(self.sample_rate, center_hz, q, gain_db);
        self.mid_peaking.set_coefficients(coeffs);
    }

    /// Mengatur parameter High-Shelf (Treble)
    pub fn set_high_band(&mut self, cutoff_hz: f32, gain_db: f32) {
        let coeffs = BiquadCoefficients::high_shelf(self.sample_rate, cutoff_hz, gain_db);
        self.high_shelf.set_coefficients(coeffs);
    }

    /// Memproses single sample melewati ketiga band secara serial
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let s1 = self.low_shelf.process_sample(input);
        let s2 = self.mid_peaking.process_sample(s1);
        self.high_shelf.process_sample(s2)
    }

    /// Memproses seluruh buffer in-place
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn reset(&mut self) {
        self.low_shelf.reset();
        self.mid_peaking.reset();
        self.high_shelf.reset();
    }
}
