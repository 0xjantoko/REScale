//! Contoh Program Lengkap Penggunaan Engine DSP REScale

use rescale_core::{
    BrickwallLimiter, Complex, Compressor, CompressorConfig, DetectionMode, FeedbackDelay, Fft,
    Gain, Normalizer, ParametricEqualizer3Band, WavReader, WavSampleFormat, WavWriter, Window,
    WindowType,
};
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 1. CREATING TEST AUDIO SIGNAL (SINE SYNTHESIZER) ===");
    let sample_rate = 44100;
    let duration_sec = 2.0;
    let total_samples = (sample_rate as f32 * duration_sec) as usize;

    // Menghasilkan akord audio: 220Hz (A3) + 440Hz (A4) + 880Hz (A5)
    let mut left_channel = Vec::with_capacity(total_samples);
    let mut right_channel = Vec::with_capacity(total_samples);

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let tone_bass = (2.0 * PI * 220.0 * t).sin() * 0.3;
        let tone_mid = (2.0 * PI * 440.0 * t).sin() * 0.4;
        let tone_high = (2.0 * PI * 880.0 * t).sin() * 0.3;

        let sample = tone_bass + tone_mid + tone_high;
        left_channel.push(sample);
        right_channel.push(sample * 0.9); // Channel kanan sedikit lebih pelan
    }

    let mut channels = vec![left_channel, right_channel];
    println!(
        "Total Samples: {} ({} seconds, Stereo)",
        total_samples, duration_sec
    );

    // -------------------------------------------------------------
    println!("\n=== 2. FREQUENCY SPECTRUM ANALYSIS (FFT) ===");
    let fft_size = 1024;
    let mut fft_buffer: Vec<Complex> = (0..fft_size)
        .map(|i| Complex::new(channels[0][i], 0.0))
        .collect();

    // Terapkan Hann Window
    let mut time_window: Vec<f32> = fft_buffer.iter().map(|c| c.re).collect();
    Window::apply(&mut time_window, WindowType::Hann);
    for (c, &w) in fft_buffer.iter_mut().zip(time_window.iter()) {
        c.re = w;
    }

    // Jalankan Forward FFT
    Fft::forward(&mut fft_buffer);
    let (dominant_freq, peak_mag) = Fft::find_dominant_frequency(&fft_buffer, sample_rate as f32);
    println!(
        "Detected Dominant Frequency: {:.1} Hz (Magnitude: {:.2})",
        dominant_freq, peak_mag
    );

    // -------------------------------------------------------------
    println!("\n=== 3. FREQUENCY SHAPING WITH 3-BAND EQUALIZER ===");
    let mut eq = ParametricEqualizer3Band::new(sample_rate as f32);
    eq.set_low_band(200.0, 3.0); // Boost Bass +3.0 dB
    eq.set_mid_band(1000.0, 1.2, -2.0); // Cut Mid -2.0 dB
    eq.set_high_band(6000.0, 1.5); // Boost Treble +1.5 dB

    for ch in channels.iter_mut() {
        eq.process_buffer(ch);
    }
    println!("3-Band EQ applied to both channels.");

    // -------------------------------------------------------------
    println!("\n=== 4. ADDING FEEDBACK ECHO / DELAY EFFECT ===");
    let delay_samples = (sample_rate as f32 * 0.25) as usize; // 250 ms delay
    let mut delay_effect = FeedbackDelay::new(delay_samples + 100, delay_samples as f32, 0.4, 0.3); // 40% feedback, 30% wet mix
    delay_effect.process_buffer(&mut channels[1]); // Berikan delay hanya pada channel kanan (efek pelebaran stereo)
    println!("Stereo Delay (250ms) applied to Right Channel.");

    // -------------------------------------------------------------
    println!("\n=== 5. DYNAMIC RANGE CONTROL (COMPRESSOR & BRICKWALL LIMITER) ===");
    let comp_config = CompressorConfig {
        threshold_db: -12.0,
        ratio: 4.0,
        knee_db: 4.0,
        attack_ms: 5.0,
        release_ms: 100.0,
        makeup_gain_db: 2.0,
        mode: DetectionMode::Rms,
    };
    let mut compressor = Compressor::new(sample_rate as f32, comp_config);
    for ch in channels.iter_mut() {
        compressor.process_buffer(ch);
    }

    // Brickwall Limiter pada -0.3 dBFS agar sinyal aman dari clipping
    let mut limiter = BrickwallLimiter::new(sample_rate as f32, -0.3, 20.0, 2.0);
    for ch in channels.iter_mut() {
        limiter.process_buffer(ch);
    }

    let max_peak = channels
        .iter()
        .map(|ch| Normalizer::find_peak(ch))
        .fold(0.0_f32, |a, b| a.max(b));
    println!(
        "Final Maximum Peak after Mastering Chain: {:.4} ({:.2} dBFS)",
        max_peak,
        Gain::linear_to_db(max_peak)
    );

    // -------------------------------------------------------------
    println!("\n=== 6. SAVING RESULT TO RIFF WAV FILE ===");
    // Relatif secara default agar contoh bisa jalan di mesin apa pun; timpa dengan
    // `cargo run --example full_pipeline -- <path>`.
    let output_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "output_mastered.wav".to_string());
    WavWriter::write_to_file(
        &output_path,
        &channels,
        sample_rate,
        WavSampleFormat::Pcm16Bit,
    )?;
    println!("WAV file saved successfully to: {}", output_path);

    // -------------------------------------------------------------
    println!("\n=== 7. READING BACK FROM WAV FILE ===");
    let (header, read_channels) = WavReader::read_from_file(&output_path)?;
    println!(
        "Successfully parsed WAV: {} Hz, {} Channels, {:?}, {} samples/channel",
        header.sample_rate,
        header.num_channels,
        header.format,
        read_channels[0].len()
    );

    println!("\n=== DSP PROCESS COMPLETED SUCCESSFULLY ===");
    Ok(())
}
