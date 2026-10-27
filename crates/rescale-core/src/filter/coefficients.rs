//! Perhitungan koefisien Biquad IIR Filter berdasarkan Robert Bristow-Johnson (RBJ) Audio EQ Cookbook.
//!
//! Semua konstruktor memvalidasi parameter input: frekuensi diklem ke rentang
//! `(0, Nyquist)`, Q diklem ke nilai positif minimum, dan gain EQ diklem ke
//! rentang aman. Tanpa ini, `f0 >= fs/2` atau `q <= 0` menghasilkan koefisien
//! tidak stabil (osilasi/amplifikasi liar).

use std::f32::consts::PI;

/// Frekuensi sampling cadangan bila input tidak valid.
const DEFAULT_SAMPLE_RATE: f32 = 48_000.0;
/// Q minimum agar `alpha` tetap hingga.
const MIN_Q: f32 = 1e-4;
/// Batas gain EQ (dB) untuk mencegah overflow koefisien.
const MAX_GAIN_DB: f32 = 40.0;
/// Frekuensi minimum absolut (Hz).
const MIN_FREQUENCY_HZ: f32 = 1.0;
/// Batas atas frekuensi relatif terhadap Nyquist.
const NYQUIST_MARGIN: f32 = 0.999;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoefficients {
    /// Membuat pass-through (unity gain) filter
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Low-Pass Filter (LPF)
    pub fn low_pass(sample_rate: f32, cutoff_hz: f32, q: f32) -> Self {
        let (_, w0, alpha) = Self::normalized_parameters(sample_rate, cutoff_hz, q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// High-Pass Filter (HPF)
    pub fn high_pass(sample_rate: f32, cutoff_hz: f32, q: f32) -> Self {
        let (_, w0, alpha) = Self::normalized_parameters(sample_rate, cutoff_hz, q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Band-Pass Filter (BPF) - constant skirt gain, peak gain = Q
    pub fn band_pass(sample_rate: f32, center_hz: f32, q: f32) -> Self {
        let (_, w0, alpha) = Self::normalized_parameters(sample_rate, center_hz, q);
        let cos_w0 = w0.cos();

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Notch / Band-Stop Filter
    pub fn notch(sample_rate: f32, center_hz: f32, q: f32) -> Self {
        let (_, w0, alpha) = Self::normalized_parameters(sample_rate, center_hz, q);
        let cos_w0 = w0.cos();

        let b0 = 1.0;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Peaking Equalizer Filter
    pub fn peaking_eq(sample_rate: f32, center_hz: f32, q: f32, gain_db: f32) -> Self {
        let (_, w0, alpha) = Self::normalized_parameters(sample_rate, center_hz, q);
        let cos_w0 = w0.cos();
        let a = 10.0_f32.powf(Self::sanitize_gain_db(gain_db) / 40.0); // sqrt(gain linear)

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Low Shelf Filter
    pub fn low_shelf(sample_rate: f32, cutoff_hz: f32, gain_db: f32) -> Self {
        let (sample_rate, frequency) = Self::sanitize_inputs(sample_rate, cutoff_hz);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let a = 10.0_f32.powf(Self::sanitize_gain_db(gain_db) / 40.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * (2.0_f32).sqrt(); // Q = 1/sqrt(2) slope butterworth standar

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// High Shelf Filter
    pub fn high_shelf(sample_rate: f32, cutoff_hz: f32, gain_db: f32) -> Self {
        let (sample_rate, frequency) = Self::sanitize_inputs(sample_rate, cutoff_hz);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let a = 10.0_f32.powf(Self::sanitize_gain_db(gain_db) / 40.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * (2.0_f32).sqrt();

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Normalisasi koefisien dengan membagi semua parameter dengan a0
    #[inline(always)]
    fn normalize(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv_a0 = 1.0 / a0;
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }

    /// Validasi sampling rate + frekuensi, lalu hasilkan `(sample_rate, w0, alpha)`.
    fn normalized_parameters(sample_rate: f32, frequency_hz: f32, q: f32) -> (f32, f32, f32) {
        let (sample_rate, frequency) = Self::sanitize_inputs(sample_rate, frequency_hz);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * Self::sanitize_q(q));
        (sample_rate, w0, alpha)
    }

    /// Sampling rate dan frekuensi yang aman dipakai di perhitungan koefisien.
    fn sanitize_inputs(sample_rate: f32, frequency_hz: f32) -> (f32, f32) {
        let safe_sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            DEFAULT_SAMPLE_RATE
        };
        let nyquist = 0.5 * safe_sample_rate;
        let minimum = MIN_FREQUENCY_HZ.min(0.5 * nyquist);
        let maximum = NYQUIST_MARGIN * nyquist;
        let safe_frequency = if frequency_hz.is_finite() {
            frequency_hz.clamp(minimum, maximum)
        } else {
            minimum
        };
        (safe_sample_rate, safe_frequency)
    }

    fn sanitize_q(q: f32) -> f32 {
        if q.is_finite() {
            q.max(MIN_Q)
        } else {
            MIN_Q
        }
    }

    fn sanitize_gain_db(gain_db: f32) -> f32 {
        if gain_db.is_finite() {
            gain_db.clamp(-MAX_GAIN_DB, MAX_GAIN_DB)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_finite(coefficients: &BiquadCoefficients) -> bool {
        [
            coefficients.b0,
            coefficients.b1,
            coefficients.b2,
            coefficients.a1,
            coefficients.a2,
        ]
        .iter()
        .all(|value| value.is_finite())
    }

    #[test]
    fn out_of_range_frequency_is_clamped_to_nyquist() {
        let coefficients = BiquadCoefficients::low_pass(48_000.0, 96_000.0, 0.707);
        assert!(is_finite(&coefficients), "koefisien harus finite");
    }

    #[test]
    fn zero_and_negative_q_do_not_produce_nan() {
        for q in [0.0, -1.0, f32::NAN] {
            let coefficients = BiquadCoefficients::high_pass(44_100.0, 1_000.0, q);
            assert!(
                is_finite(&coefficients),
                "Q {q} menghasilkan koefisien tidak finite"
            );
        }
    }

    #[test]
    fn invalid_sample_rate_falls_back_to_default() {
        let coefficients = BiquadCoefficients::band_pass(0.0, 1_000.0, 1.0);
        assert!(is_finite(&coefficients));
    }

    #[test]
    fn non_finite_and_extreme_gain_is_clamped() {
        for gain_db in [f32::NAN, f32::INFINITY, 500.0, -500.0] {
            let coefficients = BiquadCoefficients::peaking_eq(44_100.0, 1_000.0, 1.0, gain_db);
            assert!(is_finite(&coefficients), "gain {gain_db} tidak aman");
        }
    }

    #[test]
    fn normal_parameters_still_produce_expected_low_pass_shape() {
        // Cutoff jauh di bawah Nyquist: b0 harus positif dan < 1.
        let coefficients = BiquadCoefficients::low_pass(44_100.0, 500.0, 0.707);
        assert!(coefficients.b0 > 0.0 && coefficients.b0 < 1.0);
        assert!(is_finite(&coefficients));
    }
}
