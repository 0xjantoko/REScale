//! DelayLine dengan interpolasi pecahan (Fractional Delay).
//!
//! Semantik utama: [`DelayLine::read_delay`] selalu diukur terhadap sampel
//! **terakhir yang di-push** (delay `0.0` = sampel terbaru, `1.0` = satu sampel
//! sebelumnya). Pembaca yang harus mengambil nilai sebelum `push` memakai
//! [`DelayLine::read_delay_before_push`].

use super::ring_buffer::RingBuffer;

/// Cadangan slot di ring buffer untuk interpolasi cubic (membutuhkan `past + 2`).
const INTERPOLATION_HEADROOM: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationType {
    /// Bulatkan ke delay integer terdekat.
    None,
    /// Interpolasi linear (efisien, mulus).
    Linear,
    /// Interpolasi cubic Hermite 4 titik (kualitas tertinggi).
    Cubic,
}

#[derive(Debug, Clone)]
pub struct DelayLine {
    ring: RingBuffer<f32>,
    max_delay: usize,
}

impl DelayLine {
    /// Membuat DelayLine baru dengan delay maksimum dalam sample.
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            ring: RingBuffer::new(max_delay_samples + INTERPOLATION_HEADROOM),
            max_delay: max_delay_samples,
        }
    }

    /// Memasukkan sample audio baru ke delay line.
    #[inline(always)]
    pub fn push(&mut self, sample: f32) {
        self.ring.push(sample);
    }

    /// Membaca sinyal tertunda relatif terhadap sampel terakhir yang di-push.
    ///
    /// `delay_samples = 1.0` berarti `x[n - 1]` bila `x[n]` adalah sampel yang
    /// terakhir di-push.
    pub fn read_delay(&self, delay_samples: f32, interp: InterpolationType) -> f32 {
        self.read_past(self.clamp_delay(delay_samples), interp)
    }

    /// Varian untuk pembaca yang membaca **sebelum** `push` (mis. efek feedback
    /// yang perlu nilai tertunda untuk menghitung input berikutnya).
    ///
    /// Hasilnya setara dengan `read_delay(delay_samples)` **setelah** sampel baru
    /// di-push; pembaca menghitung satu langkah lebih muda, bukan menebak posisi.
    pub fn read_delay_before_push(&self, delay_samples: f32, interp: InterpolationType) -> f32 {
        let one_sample_younger = self.clamp_delay(delay_samples) - 1.0;
        self.read_past(one_sample_younger.max(0.0), interp)
    }

    /// Memproses satu sample: push `input` lalu kembalikan output delay.
    #[inline]
    pub fn process_sample(
        &mut self,
        input: f32,
        delay_samples: f32,
        interp: InterpolationType,
    ) -> f32 {
        self.push(input);
        self.read_delay(delay_samples, interp)
    }

    /// Reset internal delay memory.
    pub fn clear(&mut self) {
        self.ring.clear();
    }

    /// Batasi delay ke rentang yang valid agar indeks pembacaan aman.
    #[inline(always)]
    fn clamp_delay(&self, delay_samples: f32) -> f32 {
        delay_samples.clamp(0.0, self.max_delay as f32)
    }

    /// Membaca sampel sejauh `past` langkah ke belakang (0 = sampel terbaru).
    #[inline(always)]
    fn read_past(&self, past: f32, interp: InterpolationType) -> f32 {
        match interp {
            InterpolationType::None => self.ring.read_past(past.round() as usize),
            InterpolationType::Linear => self.interpolate_linear(past),
            InterpolationType::Cubic => self.interpolate_cubic(past),
        }
    }

    #[inline(always)]
    fn interpolate_linear(&self, past: f32) -> f32 {
        let index = past.floor() as usize;
        let fraction = past - index as f32;
        let older = self.ring.read_past(index);
        let newer = self.ring.read_past(index + 1);
        older + fraction * (newer - older)
    }

    #[inline(always)]
    fn interpolate_cubic(&self, past: f32) -> f32 {
        let index = past.floor() as usize;
        let fraction = past - index as f32;

        let ym1 = if index > 0 {
            self.ring.read_past(index - 1)
        } else {
            self.ring.read_past(0)
        };
        let y0 = self.ring.read_past(index);
        let y1 = self.ring.read_past(index + 1);
        let y2 = self.ring.read_past(index + 2);

        let c0 = y0;
        let c1 = 0.5 * (y1 - ym1);
        let c2 = ym1 - 2.5 * y0 + 2.0 * y1 - 0.5 * y2;
        let c3 = 0.5 * (y2 - ym1) + 1.5 * (y0 - y1);

        ((c3 * fraction + c2) * fraction + c1) * fraction + c0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Delay line dengan tiga sampel berurutan: 0.0, 10.0, 20.0.
    fn delay_line_with_ramp() -> DelayLine {
        let mut line = DelayLine::new(16);
        line.push(0.0);
        line.push(10.0);
        line.push(20.0);
        line
    }

    #[test]
    fn integer_delay_is_relative_to_the_last_pushed_sample() {
        let line = delay_line_with_ramp();

        assert!((line.read_delay(0.0, InterpolationType::None) - 20.0).abs() < 1e-4);
        assert!((line.read_delay(1.0, InterpolationType::None) - 10.0).abs() < 1e-4);
        assert!((line.read_delay(2.0, InterpolationType::None) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn fractional_delay_interpolates_between_adjacent_samples() {
        let line = delay_line_with_ramp();

        // delay 1.5 = rata-rata x[n-1] dan x[n-2] = (10 + 0) / 2.
        let value = line.read_delay(1.5, InterpolationType::Linear);
        assert!((value - 5.0).abs() < 1e-4, "expected 5.0, got {value}");
    }

    #[test]
    fn before_push_variant_matches_post_push_delay() {
        let line = delay_line_with_ramp();

        // Pembaca pre-push melihat satu sampel lebih muda: delay 1 = x[n-1] = 20.0.
        assert!((line.read_delay_before_push(1.0, InterpolationType::None) - 20.0).abs() < 1e-4);
        assert!((line.read_delay_before_push(2.0, InterpolationType::None) - 10.0).abs() < 1e-4);
    }

    #[test]
    fn process_sample_outputs_input_delayed_by_requested_amount() {
        let mut line = DelayLine::new(8);

        let first_output = line.process_sample(1.0, 2.0, InterpolationType::None);
        let second_output = line.process_sample(0.0, 2.0, InterpolationType::None);
        let third_output = line.process_sample(0.0, 2.0, InterpolationType::None);

        assert!((first_output - 0.0).abs() < 1e-6);
        assert!((second_output - 0.0).abs() < 1e-6);
        assert!(
            (third_output - 1.0).abs() < 1e-6,
            "impulse harus muncul setelah tepat 2 sampel"
        );
    }
}
