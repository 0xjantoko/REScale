//! Fungsi-fungsi Windowing untuk mengurangi Spectral Leakage pada FFT.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Rectangular,
    Hann,
    Hamming,
    BlackmanHarris,
}

pub struct Window;

impl Window {
    /// Menghasilkan bobot windowing untuk buffer berukuran `size`
    pub fn generate(size: usize, window_type: WindowType) -> Vec<f32> {
        if size == 0 {
            return Vec::new();
        }
        if size == 1 {
            return vec![1.0];
        }

        let n_minus_1 = (size - 1) as f32;
        let mut weights = Vec::with_capacity(size);

        for n in 0..size {
            let n_f = n as f32;
            let w = match window_type {
                WindowType::Rectangular => 1.0,
                WindowType::Hann => 0.5 * (1.0 - (2.0 * PI * n_f / n_minus_1).cos()),
                WindowType::Hamming => 0.54 - 0.46 * (2.0 * PI * n_f / n_minus_1).cos(),
                WindowType::BlackmanHarris => {
                    let a0 = 0.35875;
                    let a1 = 0.48829;
                    let a2 = 0.14128;
                    let a3 = 0.01168;

                    a0 - a1 * (2.0 * PI * n_f / n_minus_1).cos()
                        + a2 * (4.0 * PI * n_f / n_minus_1).cos()
                        - a3 * (6.0 * PI * n_f / n_minus_1).cos()
                }
            };
            weights.push(w);
        }

        weights
    }

    /// Mengaplikasikan windowing in-place ke buffer sinyal audio
    pub fn apply(buffer: &mut [f32], window_type: WindowType) {
        let weights = Self::generate(buffer.len(), window_type);
        for (sample, &weight) in buffer.iter_mut().zip(weights.iter()) {
            *sample *= weight;
        }
    }
}
