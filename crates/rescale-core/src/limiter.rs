//! Modul saturasi non-linier dan pelindung clipping.

pub struct Limiter;

impl Limiter {
    /// Saturasi Hyperbolic Tangent (Tanh): y = tanh(x)
    #[inline(always)]
    pub fn tanh_soft_clip(sample: f32) -> f32 {
        sample.tanh()
    }

    /// Polynomial Soft Clipper (Cubic Saturation)
    #[inline(always)]
    pub fn cubic_soft_clip(sample: f32) -> f32 {
        if sample > 1.0 {
            2.0 / 3.0
        } else if sample < -1.0 {
            -2.0 / 3.0
        } else {
            sample - (sample.powi(3) / 3.0)
        }
    }

    /// Hard Clamp pengaman [-1.0, 1.0]
    #[inline(always)]
    pub fn hard_clamp(sample: f32) -> f32 {
        sample.clamp(-1.0, 1.0)
    }

    /// Memproses buffer dengan Soft-Clip Tanh in-place
    pub fn process_tanh_buffer(buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = Self::tanh_soft_clip(*sample);
        }
    }
}
