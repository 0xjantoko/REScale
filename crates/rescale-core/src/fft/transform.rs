//! Algoritma Fast Fourier Transform (FFT) Cooley-Tukey Radix-2 (In-Place & Zero Allocation).

use super::complex::Complex;
use std::f32::consts::PI;

pub struct Fft;

impl Fft {
    /// Menghitung Forward FFT (Time domain -> Frequency domain) in-place.
    /// Catatan: Panjang buffer `buffer.len()` WAJIB berupa perpangkatan 2 (Power of Two: 64, 128, 256, 512, 1024, dst).
    ///
    /// # Panics
    ///
    /// Panik jika `buffer` kosong atau panjangnya bukan perpangkatan dua.
    pub fn forward(buffer: &mut [Complex]) {
        let n = buffer.len();
        assert!(
            n > 0 && (n & (n - 1)) == 0,
            "FFT size must be a power of two"
        );

        // 1. Permutasi bit-reversal
        Self::bit_reverse_permutation(buffer);

        // 2. Komputasi Butterfly Cooley-Tukey
        let mut len = 2;
        while len <= n {
            let half_len = len / 2;
            let angle_step = -2.0 * PI / (len as f32);
            let w_step = Complex::from_polar(1.0, angle_step);

            let mut i = 0;
            while i < n {
                let mut w = Complex::new(1.0, 0.0);
                for j in 0..half_len {
                    let u = buffer[i + j];
                    let v = buffer[i + j + half_len] * w;

                    buffer[i + j] = u + v;
                    buffer[i + j + half_len] = u - v;

                    w = w * w_step;
                }
                i += len;
            }
            len *= 2;
        }
    }

    /// Menghitung Inverse FFT (Frequency domain -> Time domain) in-place.
    ///
    /// # Panics
    ///
    /// Panik jika `buffer` kosong atau panjangnya bukan perpangkatan dua.
    pub fn inverse(buffer: &mut [Complex]) {
        let n = buffer.len();
        assert!(
            n > 0 && (n & (n - 1)) == 0,
            "IFFT size must be a power of two"
        );

        // 1. Konjugat kompleks
        for item in buffer.iter_mut() {
            *item = item.conjugate();
        }

        // 2. Forward FFT
        Self::forward(buffer);

        // 3. Konjugat kompleks & skala 1/N
        let inv_n = 1.0 / (n as f32);
        for item in buffer.iter_mut() {
            *item = item.conjugate() * inv_n;
        }
    }

    /// Menghitung Power Spectrum Magnitude dari hasil FFT.
    ///
    /// Mengembalikan vektor kosong bila `buffer` kosong.
    pub fn compute_magnitude_spectrum(buffer: &[Complex]) -> Vec<f32> {
        if buffer.is_empty() {
            return Vec::new();
        }
        let half = buffer.len() / 2 + 1; // Nyquist limit
        let mut mags = Vec::with_capacity(half);
        for item in &buffer[0..half] {
            mags.push(item.magnitude());
        }
        mags
    }

    /// Menghitung frekuensi dominan (Peak Frequency Bin).
    ///
    /// Mengembalikan `(0.0, 0.0)` bila `buffer` kosong (tidak ada bin yang dominan).
    pub fn find_dominant_frequency(buffer: &[Complex], sample_rate: f32) -> (f32, f32) {
        if buffer.is_empty() {
            return (0.0, 0.0);
        }
        let half = buffer.len() / 2 + 1;
        let mut max_mag = 0.0_f32;
        let mut max_bin = 0;

        for (i, item) in buffer[0..half].iter().enumerate() {
            let mag = item.magnitude();
            if mag > max_mag {
                max_mag = mag;
                max_bin = i;
            }
        }

        let freq_hz = (max_bin as f32) * sample_rate / (buffer.len() as f32);
        (freq_hz, max_mag)
    }

    /// Penyusunan ulang indeks bit-reversal
    fn bit_reverse_permutation(buffer: &mut [Complex]) {
        let n = buffer.len();
        let mut j = 0;
        for i in 0..n {
            if i < j {
                buffer.swap(i, j);
            }
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
        }
    }
}
