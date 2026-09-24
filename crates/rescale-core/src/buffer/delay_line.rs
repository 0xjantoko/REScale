//! DelayLine dengan interpolasi pecahan (Fractional Delay).

use super::ring_buffer::RingBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationType {
    None,   // Round ke integer terdekat
    Linear, // Linear interpolation (efisien, mulus)
    Cubic,  // Hermite 4-point cubic interpolation (kualitas tinggi)
}

#[derive(Debug, Clone)]
pub struct DelayLine {
    ring: RingBuffer<f32>,
    max_delay: usize,
}

impl DelayLine {
    /// Membuat DelayLine baru dengan delay maksimum dalam sample
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            ring: RingBuffer::new(max_delay_samples + 4), // Headroom untuk cubic interpolation
            max_delay: max_delay_samples,
        }
    }

    /// Memasukkan sample audio baru ke delay line
    #[inline(always)]
    pub fn push(&mut self, sample: f32) {
        self.ring.push(sample);
    }

    /// Membaca sinyal tertunda dengan fractional delay (misal 12.45 samples)
    pub fn read_delay(&self, delay_samples: f32, interp: InterpolationType) -> f32 {
        if delay_samples < 1.0 {
            return self.ring.read_past(0);
        }
        let delay_clamped = delay_samples.min(self.max_delay as f32);
        let d_past = delay_clamped - 1.0; // Karena sample 0 adalah item yang baru saja di-push

        match interp {
            InterpolationType::None => {
                let d = d_past.round() as usize;
                self.ring.read_past(d)
            }
            InterpolationType::Linear => {
                let d_int = d_past.floor() as usize;
                let frac = d_past - (d_int as f32);

                let y0 = self.ring.read_past(d_int);
                let y1 = self.ring.read_past(d_int + 1);

                // y = y0 + frac * (y1 - y0)
                y0 + frac * (y1 - y0)
            }
            InterpolationType::Cubic => {
                let d_int = d_past.floor() as usize;
                let frac = d_past - (d_int as f32);

                let ym1 = if d_int > 0 {
                    self.ring.read_past(d_int - 1)
                } else {
                    self.ring.read_past(0)
                };
                let y0 = self.ring.read_past(d_int);
                let y1 = self.ring.read_past(d_int + 1);
                let y2 = self.ring.read_past(d_int + 2);

                let c0 = y0;
                let c1 = 0.5 * (y1 - ym1);
                let c2 = ym1 - 2.5 * y0 + 2.0 * y1 - 0.5 * y2;
                let c3 = 0.5 * (y2 - ym1) + 1.5 * (y0 - y1);

                ((c3 * frac + c2) * frac + c1) * frac + c0
            }
        }
    }

    /// Memproses single sample: push sample dan kembalikan output delay
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

    /// Reset internal delay memory
    pub fn clear(&mut self) {
        self.ring.clear();
    }
}
