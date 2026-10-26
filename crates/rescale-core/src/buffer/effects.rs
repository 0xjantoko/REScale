//! Efek Audio Berbasis Waktu (Echo, Feedback Delay, Comb Filter).

use super::delay_line::{DelayLine, InterpolationType};

/// Feedback Delay Effect sederhana dengan parameter Wet/Dry mix dan Feedback
pub struct FeedbackDelay {
    delay_line: DelayLine,
    delay_samples: f32,
    feedback: f32,
    mix: f32, // 0.0 (hanya Dry) s/d 1.0 (hanya Wet)
    interp: InterpolationType,
}

impl FeedbackDelay {
    pub fn new(max_delay_samples: usize, initial_delay: f32, feedback: f32, mix: f32) -> Self {
        Self {
            delay_line: DelayLine::new(max_delay_samples),
            delay_samples: initial_delay,
            feedback: feedback.clamp(-0.99, 0.99), // Safety limiter untuk mencegah ledakan resonansi
            mix: mix.clamp(0.0, 1.0),
            interp: InterpolationType::Linear,
        }
    }

    pub fn set_delay_samples(&mut self, delay: f32) {
        self.delay_samples = delay;
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(-0.99, 0.99);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn set_interpolation(&mut self, interp: InterpolationType) {
        self.interp = interp;
    }

    /// Memproses 1 sample audio secara real-time.
    ///
    /// Efek ini membaca delay **sebelum** menulis sampel baru, sehingga memakai
    /// varian `read_delay_before_push` agar nilai yang diambil tetap `x[n - D]`.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let delayed = self
            .delay_line
            .read_delay_before_push(self.delay_samples, self.interp);
        let to_delay = input + delayed * self.feedback;
        self.delay_line.push(to_delay);

        // Dry/Wet mixing: y = (1 - mix) * dry + mix * wet
        (1.0 - self.mix) * input + self.mix * delayed
    }

    /// Memproses seluruh buffer in-place
    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn clear(&mut self) {
        self.delay_line.clear();
    }
}
