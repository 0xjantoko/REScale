//! Sanitasi sinyal audio: guard NaN/Inf dan flush nilai denormal.

/// Nilai absolut di bawah ambang ini dianggap denormal dan dibulatkan ke nol.
pub const DENORMAL_THRESHOLD: f32 = 1e-30;

/// Mensterilkan satu sample: non-finite menjadi 0.0, denormal menjadi 0.0.
///
/// Nilai NaN/Inf dapat menyebar ke seluruh buffer dengan cepat (umpan balik
/// filter/kompresor), dan denormal membuat operasi floating-point melambat
/// drastis di banyak CPU. Sanitasi di boundary engine mencegah keduanya.
#[inline(always)]
pub fn sanitize_sample(sample: f32) -> f32 {
    if !sample.is_finite() {
        return 0.0;
    }
    if sample.abs() < DENORMAL_THRESHOLD {
        return 0.0;
    }
    sample
}

/// Mensterilkan seluruh sample dalam buffer in-place.
pub fn sanitize_buffer(buffer: &mut [f32]) {
    for sample in buffer.iter_mut() {
        *sample = sanitize_sample(*sample);
    }
}

/// Mensterilkan seluruh channel pada sebuah audio block.
pub fn sanitize_block(block: &mut crate::engine::AudioBlock<'_>) {
    for channel_index in 0..block.channels() {
        if let Some(channel) = block.channel_mut(channel_index) {
            sanitize_buffer(channel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_values_become_silence() {
        assert_eq!(sanitize_sample(f32::NAN), 0.0);
        assert_eq!(sanitize_sample(f32::INFINITY), 0.0);
        assert_eq!(sanitize_sample(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn denormals_and_finite_values() {
        assert_eq!(sanitize_sample(1e-32), 0.0);
        assert!((sanitize_sample(0.5) - 0.5).abs() < 1e-6);
        assert!((sanitize_sample(-0.25) + 0.25).abs() < 1e-6);
    }

    #[test]
    fn buffer_is_sanitized_in_place() {
        let mut buffer = [f32::NAN, 0.5, f32::INFINITY, 1e-35];
        sanitize_buffer(&mut buffer);
        assert_eq!(buffer, [0.0, 0.5, 0.0, 0.0]);
    }
}
