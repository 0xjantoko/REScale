//! Modul normalisasi audio berdasarkan Peak dan RMS.

use crate::gain::Gain;

pub struct Normalizer;

impl Normalizer {
    /// Mencari nilai absolut amplitude terbesar dalam buffer.
    #[inline]
    pub fn find_peak(buffer: &[f32]) -> f32 {
        buffer
            .iter()
            .map(|&x| x.abs())
            .fold(0.0_f32, |acc, x| acc.max(x))
    }

    /// Menghitung Root Mean Square (RMS) dari buffer.
    #[inline]
    pub fn calculate_rms(buffer: &[f32]) -> f32 {
        if buffer.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = buffer.iter().map(|&x| x * x).sum();
        (sum_squares / buffer.len() as f32).sqrt()
    }

    /// Menskalakan buffer agar amplitude peak mencapai `target_peak`.
    /// Mengembalikan rasio gain yang diterapkan.
    pub fn normalize_to_peak(buffer: &mut [f32], target_peak: f32) -> f32 {
        let current_peak = Self::find_peak(buffer);
        if current_peak > 1e-6 {
            let gain = target_peak / current_peak;
            for sample in buffer.iter_mut() {
                *sample *= gain;
            }
            gain
        } else {
            1.0
        }
    }

    /// Menormalkan seluruh channel dengan gain yang sama agar balance stereo tetap terjaga.
    pub fn normalize_channels_to_peak(channels: &mut [Vec<f32>], target_peak: f32) -> f32 {
        let current_peak = channels
            .iter()
            .map(|channel| Self::find_peak(channel))
            .fold(0.0_f32, f32::max);

        if current_peak <= 1e-6 {
            return 1.0;
        }

        let gain = target_peak / current_peak;
        for channel in channels {
            Gain::apply_linear(channel, gain);
        }
        gain
    }
}
