//! Core DSP Engine & ReScaler in Pure Rust (Zero Dependency)

pub mod buffer;
pub mod convert;
pub mod drc;
pub mod engine;
pub mod fft;
pub mod filter;
pub mod gain;
pub mod limiter;
pub mod normalize;
pub mod tui;
pub mod wav;

pub use buffer::{DelayLine, FeedbackDelay, InterpolationType, RingBuffer};
pub use convert::SampleConvert;
pub use drc::{BrickwallLimiter, Compressor, CompressorConfig, DetectionMode, EnvelopeFollower};
pub use engine::{
    AudioBlock, AudioProcessor, AudioSpec, CompressorProcessor, EngineError, EqualizerProcessor,
    GainProcessor, LimiterProcessor, ProcessorChain,
};
pub use fft::{Complex, Fft, Window, WindowType};
pub use filter::{BiquadCoefficients, BiquadFilter, ParametricEqualizer3Band};
pub use gain::Gain;
pub use limiter::Limiter;
pub use normalize::Normalizer;
pub use tui::{MenuItem, TerminalCanvas, TuiMenu};
pub use wav::{
    DitherMode, WavHeader, WavReader, WavSampleFormat, WavStreamReader, WavStreamWriter,
    WavWriteOptions, WavWriter,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_gain_and_normalize() {
        let mut buffer = vec![0.1, -0.2, 0.4, -0.5];

        Gain::apply_db(&mut buffer, 6.0206);
        assert!((buffer[3] - (-1.0)).abs() < 1e-3);

        Normalizer::normalize_to_peak(&mut buffer, 0.8);
        let peak = Normalizer::find_peak(&buffer);
        assert!((peak - 0.8).abs() < 1e-4);
    }

    #[test]
    fn test_audio_block_and_processor_chain() {
        let spec = AudioSpec::new(48_000.0, 2).expect("valid audio specification");
        let mut left = vec![0.25, -0.25];
        let mut right = vec![0.5, -0.5];
        let mut channels: Vec<&mut [f32]> = vec![&mut left, &mut right];
        let mut block = AudioBlock::new(&mut channels).expect("matching channel lengths");

        let config = CompressorConfig {
            threshold_db: 0.0,
            ratio: 1.0,
            knee_db: 0.0,
            attack_ms: 1.0,
            release_ms: 10.0,
            makeup_gain_db: 0.0,
            mode: DetectionMode::Peak,
        };
        let mut chain = ProcessorChain::new();
        chain.push(CompressorProcessor::new(config));
        chain.prepare(spec).expect("processors prepare");
        chain.process(&mut block).expect("processors run");

        assert_eq!(block.frames(), 2);
        assert_eq!(block.channels(), 2);
        assert!((left[0] - 0.25).abs() < 1e-6);
        assert!((right[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_wav_write_and_read_roundtrip() {
        let sample_rate = 44100;
        let freq = 440.0;
        let num_samples = 4410;
        let mut channel_l = Vec::with_capacity(num_samples);
        let mut channel_r = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.75;
            channel_l.push(sample);
            channel_r.push(-sample);
        }

        let channels = vec![channel_l.clone(), channel_r.clone()];

        let mut wav_bytes = Vec::new();
        WavWriter::write_to_stream(
            &mut wav_bytes,
            &channels,
            sample_rate,
            WavSampleFormat::Pcm16Bit,
        )
        .expect("Failed to write WAV stream");

        let mut cursor = Cursor::new(wav_bytes);
        let (header, decoded_channels) =
            WavReader::read_from_stream(&mut cursor).expect("Failed to read WAV stream");

        assert_eq!(header.num_channels, 2);
        assert_eq!(header.sample_rate, 44100);
        assert_eq!(header.num_samples_per_channel, num_samples);
        assert_eq!(decoded_channels.len(), 2);

        for i in 0..num_samples {
            assert!((decoded_channels[0][i] - channels[0][i]).abs() < 1e-4);
            assert!((decoded_channels[1][i] - channels[1][i]).abs() < 1e-4);
        }
    }

    #[test]
    fn test_wav_stream_reader_and_writer_block_roundtrip() {
        let sample_rate = 48000;
        let num_channels = 2;
        let total_frames = 1024;

        // Siapkan buffer input stereo
        let mut in_l = Vec::with_capacity(total_frames);
        let mut in_r = Vec::with_capacity(total_frames);
        for i in 0..total_frames {
            let t = i as f32 / sample_rate as f32;
            in_l.push((2.0 * std::f32::consts::PI * 300.0 * t).sin() * 0.8);
            in_r.push((2.0 * std::f32::consts::PI * 600.0 * t).sin() * 0.5);
        }

        // 1. Tulis via WavStreamWriter per block 256 frame
        let mut cursor = Cursor::new(Vec::new());
        let options = WavWriteOptions::new(WavSampleFormat::Pcm24Bit, DitherMode::None);
        let mut stream_writer =
            WavStreamWriter::new(&mut cursor, num_channels, sample_rate, options)
                .expect("init stream writer");

        let chunk_size = 256;
        for start in (0..total_frames).step_by(chunk_size) {
            let end = (start + chunk_size).min(total_frames);
            let mut left_slice = in_l[start..end].to_vec();
            let mut right_slice = in_r[start..end].to_vec();
            let mut slices: Vec<&mut [f32]> = vec![&mut left_slice, &mut right_slice];
            let block = AudioBlock::new(&mut slices).expect("block create");
            stream_writer.write_block(&block).expect("write block");
        }
        let written_header = stream_writer.finalize().expect("finalize stream writer");
        assert_eq!(written_header.num_samples_per_channel, total_frames);

        // 2. Baca via WavStreamReader per block 128 frame
        cursor.set_position(0);
        let mut stream_reader = WavStreamReader::new(cursor).expect("init stream reader");
        assert_eq!(stream_reader.total_frames(), total_frames);
        assert_eq!(stream_reader.header().format, WavSampleFormat::Pcm24Bit);

        let read_chunk_size = 128;
        let mut out_l = Vec::new();
        let mut out_r = Vec::new();

        let mut buf_l = vec![0.0_f32; read_chunk_size];
        let mut buf_r = vec![0.0_f32; read_chunk_size];

        while !stream_reader.is_finished() {
            let mut slices: Vec<&mut [f32]> = vec![&mut buf_l, &mut buf_r];
            let mut block = AudioBlock::new(&mut slices).expect("block create");
            let frames_read = stream_reader.read_block(&mut block).expect("read block");
            if frames_read == 0 {
                break;
            }
            out_l.extend_from_slice(&buf_l[..frames_read]);
            out_r.extend_from_slice(&buf_r[..frames_read]);
        }

        assert_eq!(out_l.len(), total_frames);
        assert_eq!(out_r.len(), total_frames);
        for i in 0..total_frames {
            assert!((out_l[i] - in_l[i]).abs() < 1e-4);
            assert!((out_r[i] - in_r[i]).abs() < 1e-4);
        }
    }

    #[test]
    fn test_limiter_processor_flush_tail() {
        let sample_rate = 44100.0;
        let spec = AudioSpec::new(sample_rate, 1).expect("valid spec");
        let lookahead_ms = 2.0;

        let mut limiter_proc = LimiterProcessor::new(-1.0, 20.0, lookahead_ms);
        limiter_proc.prepare(spec).expect("limiter prepare");

        let latency = limiter_proc.latency_samples();
        assert!(latency > 0);

        // Sinyal pendek dengan impulse di awal
        let mut impulse = vec![0.8_f32; 100];
        let mut slices: Vec<&mut [f32]> = vec![&mut impulse];
        let mut block = AudioBlock::new(&mut slices).expect("block");
        limiter_proc.process(&mut block).expect("process");

        // Flush tail latency
        let mut flush_buf = vec![0.0_f32; latency + 10];
        let mut flush_slices: Vec<&mut [f32]> = vec![&mut flush_buf];
        let mut flush_block = AudioBlock::new(&mut flush_slices).expect("flush block");
        let flushed = limiter_proc.flush(&mut flush_block).expect("limiter flush");

        assert_eq!(flushed, latency);
        // Sisa delay output harus bernilai ~0.8 bukan 0
        assert!((flush_block.channel(0).unwrap()[0] - 0.8).abs() < 0.2);

        // Flush kedua harus mengembalikan 0 frame
        let flushed_again = limiter_proc.flush(&mut flush_block).expect("second flush");
        assert_eq!(flushed_again, 0);
    }

    #[test]
    fn test_engine_error_validation_and_channel_mismatch() {
        assert!(AudioSpec::new(0.0, 2).is_err());
        assert!(AudioSpec::new(44100.0, 0).is_err());

        let mut ch1 = vec![0.0; 10];
        let mut ch2 = vec![0.0; 12]; // panjang berbeda
        let mut channels = vec![ch1.as_mut_slice(), ch2.as_mut_slice()];
        assert_eq!(
            AudioBlock::new(&mut channels).err(),
            Some(EngineError::UnequalChannelLengths)
        );

        let mut empty_channels: Vec<&mut [f32]> = Vec::new();
        assert_eq!(
            AudioBlock::new(&mut empty_channels).err(),
            Some(EngineError::NoChannels)
        );

        // Test channel count mismatch pada processor
        let spec_stereo = AudioSpec::new(48000.0, 2).unwrap();
        let mut gain_proc = GainProcessor::new(0.0);
        gain_proc.prepare(spec_stereo).unwrap();

        let mut mono_buf = vec![0.5; 8];
        let mut mono_slices = vec![mono_buf.as_mut_slice()];
        let mut mono_block = AudioBlock::new(&mut mono_slices).unwrap();
        assert!(matches!(
            gain_proc.process(&mut mono_block),
            Err(EngineError::ChannelCountMismatch {
                expected: 2,
                actual: 1
            })
        ));
    }

    #[test]
    fn test_equalizer_and_gain_processors_in_chain() {
        let spec = AudioSpec::new(44100.0, 2).expect("spec");
        let mut chain = ProcessorChain::new();
        chain.push(GainProcessor::new(6.0206)); // ~2x linear gain (+6dB)
        chain.push(EqualizerProcessor::new(
            100.0, 0.0, 1000.0, 1.0, 0.0, 8000.0, 0.0,
        ));
        chain.prepare(spec).expect("prepare chain");

        let mut left = vec![0.25; 100];
        let mut right = vec![0.4; 100];
        let mut slices: Vec<&mut [f32]> = vec![&mut left, &mut right];
        let mut block = AudioBlock::new(&mut slices).expect("block");

        chain.process(&mut block).expect("process chain");
        assert!((left[50] - 0.5).abs() < 1e-3);
        assert!((right[50] - 0.8).abs() < 1e-3);
    }

    #[test]
    fn test_float_wav_lossless() {
        let sample_rate = 48000;
        let samples = vec![vec![0.0, 0.123456, -0.654321, 0.99999]];

        let mut wav_bytes = Vec::new();
        WavWriter::write_to_stream(
            &mut wav_bytes,
            &samples,
            sample_rate,
            WavSampleFormat::Pcm32BitFloat,
        )
        .expect("Failed to write 32-bit float WAV");

        let mut cursor = Cursor::new(wav_bytes);
        let (_, decoded) =
            WavReader::read_from_stream(&mut cursor).expect("Failed to read 32-bit float WAV");

        for i in 0..samples[0].len() {
            assert_eq!(samples[0][i], decoded[0][i]);
        }
    }

    #[test]
    fn test_ring_buffer() {
        let mut ring = RingBuffer::new(4);
        ring.push(10.0);
        ring.push(20.0);
        ring.push(30.0);

        assert_eq!(ring.read_past(0), 30.0);
        assert_eq!(ring.read_past(1), 20.0);
        assert_eq!(ring.read_past(2), 10.0);

        ring.push(40.0);
        ring.push(50.0);
        assert_eq!(ring.read_past(0), 50.0);
        assert_eq!(ring.read_past(1), 40.0);
        assert_eq!(ring.read_past(2), 30.0);
        assert_eq!(ring.read_past(3), 20.0);
    }

    #[test]
    fn test_fractional_delay_interpolation() {
        let mut delay = DelayLine::new(10);
        delay.push(0.0);
        delay.push(10.0);
        delay.push(20.0);

        // Semantik read_delay: relatif terhadap sampel terakhir yang di-push.
        // delay 1.5 = interpolasi x[n-1] dan x[n-2] = (10 + 0) / 2.
        let val_linear = delay.read_delay(1.5, InterpolationType::Linear);
        assert!((val_linear - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_feedback_delay_echo() {
        let mut echo = FeedbackDelay::new(100, 2.0, 0.5, 1.0);
        let mut impulse = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        echo.process_buffer(&mut impulse);

        assert_eq!(impulse[0], 0.0);
        assert_eq!(impulse[1], 0.0);
        assert_eq!(impulse[2], 1.0);
        assert_eq!(impulse[3], 0.0);
        assert_eq!(impulse[4], 0.5);
    }

    #[test]
    fn test_biquad_low_pass_attenuation() {
        let sample_rate = 44100.0;
        let lpf_coeffs =
            BiquadCoefficients::low_pass(sample_rate, 500.0, std::f32::consts::FRAC_1_SQRT_2);
        let mut filter = BiquadFilter::new(lpf_coeffs);

        let high_freq = 10000.0;
        let mut high_freq_signal: Vec<f32> = (0..500)
            .map(|i| (2.0 * std::f32::consts::PI * high_freq * (i as f32) / sample_rate).sin())
            .collect();

        filter.process_buffer(&mut high_freq_signal);

        let output_peak = Normalizer::find_peak(&high_freq_signal[100..]);
        assert!(
            output_peak < 0.1,
            "High frequency must be attenuated by LPF"
        );
    }

    #[test]
    fn test_parametric_equalizer_boost() {
        let sample_rate = 44100.0;
        let mut eq = ParametricEqualizer3Band::new(sample_rate);
        eq.set_mid_band(1000.0, 1.0, 6.0);

        let mid_freq = 1000.0;
        let mut signal: Vec<f32> = (0..1000)
            .map(|i| (2.0 * std::f32::consts::PI * mid_freq * (i as f32) / sample_rate).sin() * 0.5)
            .collect();

        eq.process_buffer(&mut signal);

        let output_peak = Normalizer::find_peak(&signal[200..]);
        assert!(
            (output_peak - 1.0).abs() < 0.15,
            "Mid band should be boosted by ~6dB"
        );
    }

    #[test]
    fn test_dynamic_compressor_reduction() {
        let sample_rate = 44100.0;
        let config = CompressorConfig {
            threshold_db: -12.0,
            ratio: 4.0,
            knee_db: 0.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            makeup_gain_db: 0.0,
            mode: DetectionMode::Peak,
        };
        let mut comp = Compressor::new(sample_rate, config);

        let mut hot_signal = vec![0.8_f32; 1000];
        comp.process_buffer(&mut hot_signal);

        let steady_output = hot_signal[500];
        assert!(steady_output < 0.45 && steady_output > 0.3);
    }

    #[test]
    fn test_brickwall_lookahead_limiter_ceiling() {
        let sample_rate = 44100.0;
        let ceiling_db = -1.0;
        let mut limiter = BrickwallLimiter::new(sample_rate, ceiling_db, 20.0, 3.0);

        let mut extreme_signal: Vec<f32> = (0..500)
            .map(|i| (2.0 * std::f32::consts::PI * 200.0 * (i as f32) / sample_rate).sin() * 2.5)
            .collect();

        limiter.process_buffer(&mut extreme_signal);

        let output_peak = Normalizer::find_peak(&extreme_signal);
        let max_allowed = Gain::db_to_linear(ceiling_db) + 1e-4;
        assert!(
            output_peak <= max_allowed,
            "Limiter ceiling breached: peak was {}",
            output_peak
        );
    }

    #[test]
    fn test_fft_forward_and_inverse_roundtrip() {
        let n = 256;
        let original: Vec<Complex> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                Complex::new((2.0 * std::f32::consts::PI * 5.0 * t).sin(), 0.0)
            })
            .collect();

        let mut buffer = original.clone();

        Fft::forward(&mut buffer);
        Fft::inverse(&mut buffer);

        for i in 0..n {
            assert!((buffer[i].re - original[i].re).abs() < 1e-5);
            assert!(buffer[i].im.abs() < 1e-5);
        }
    }

    #[test]
    fn test_fft_dominant_frequency_detection() {
        let sample_rate = 44100.0;
        let fft_size = 1024;
        let target_freq = 1720.0;

        let mut buffer: Vec<Complex> = (0..fft_size)
            .map(|i| {
                let t = (i as f32) / sample_rate;
                Complex::new((2.0 * std::f32::consts::PI * target_freq * t).sin(), 0.0)
            })
            .collect();

        let mut time_domain: Vec<f32> = buffer.iter().map(|c| c.re).collect();
        Window::apply(&mut time_domain, WindowType::Hann);
        for (c, &w) in buffer.iter_mut().zip(time_domain.iter()) {
            c.re = w;
        }

        Fft::forward(&mut buffer);

        let (detected_freq, _) = Fft::find_dominant_frequency(&buffer, sample_rate);
        let freq_resolution = sample_rate / (fft_size as f32);

        assert!(
            (detected_freq - target_freq).abs() <= freq_resolution,
            "Detected freq: {}, Target freq: {}",
            detected_freq,
            target_freq
        );
    }

    #[test]
    fn test_fft_helpers_accept_empty_input() {
        let empty: Vec<Complex> = Vec::new();
        assert!(Fft::compute_magnitude_spectrum(&empty).is_empty());
        assert_eq!(Fft::find_dominant_frequency(&empty, 44100.0), (0.0, 0.0));
    }

    #[test]
    fn test_fft_magnitude_spectrum_has_nyquist_bin() {
        let mut buffer = vec![Complex::new(0.0, 0.0); 8];
        Fft::forward(&mut buffer);
        // DC sampai Nyquist inklusif: len / 2 + 1 bin
        assert_eq!(Fft::compute_magnitude_spectrum(&buffer).len(), 5);
    }

    #[test]
    #[should_panic(expected = "FFT size must be a power of two")]
    fn test_fft_forward_rejects_non_power_of_two() {
        let mut buffer = vec![Complex::new(0.0, 0.0); 100];
        Fft::forward(&mut buffer);
    }

    #[test]
    fn test_full_mastering_chain_pipeline() {
        let sample_rate = 44100.0;
        let num_samples = 4410;

        let mut audio_channel: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                let f1 = (2.0 * std::f32::consts::PI * 100.0 * t).sin() * 0.4;
                let f2 = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.4;
                let f3 = (2.0 * std::f32::consts::PI * 8000.0 * t).sin() * 0.4;
                f1 + f2 + f3
            })
            .collect();

        let mut eq = ParametricEqualizer3Band::new(sample_rate);
        eq.set_low_band(150.0, 2.0);
        eq.set_high_band(6000.0, -3.0);
        eq.process_buffer(&mut audio_channel);

        let comp_config = CompressorConfig {
            threshold_db: -10.0,
            ratio: 3.0,
            knee_db: 4.0,
            attack_ms: 5.0,
            release_ms: 80.0,
            makeup_gain_db: 2.0,
            mode: DetectionMode::Rms,
        };
        let mut comp = Compressor::new(sample_rate, comp_config);
        comp.process_buffer(&mut audio_channel);

        let mut limiter = BrickwallLimiter::new(sample_rate, -0.5, 30.0, 2.0);
        limiter.process_buffer(&mut audio_channel);

        let final_peak = Normalizer::find_peak(&audio_channel);
        let max_ceiling = Gain::db_to_linear(-0.5) + 1e-4;
        assert!(final_peak <= max_ceiling);

        let stereo_channels = vec![audio_channel.clone(), audio_channel];
        let mut wav_bytes = Vec::new();
        WavWriter::write_to_stream(
            &mut wav_bytes,
            &stereo_channels,
            sample_rate as u32,
            WavSampleFormat::Pcm24Bit,
        )
        .expect("Failed to write mastering output to WAV");

        let mut cursor = Cursor::new(wav_bytes);
        let (header, decoded_channels) =
            WavReader::read_from_stream(&mut cursor).expect("Failed to read mastered WAV");

        assert_eq!(header.num_channels, 2);
        assert_eq!(header.format, WavSampleFormat::Pcm24Bit);
        assert_eq!(decoded_channels[0].len(), num_samples);
    }
}
