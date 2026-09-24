//! Dashboard TUI untuk memantau sinyal sintetis dan mastering chain REScale.

use rescale_core::{
    BrickwallLimiter, Complex, Compressor, CompressorConfig, DetectionMode, Fft, Gain, Normalizer,
    ParametricEqualizer3Band, TerminalCanvas, Window, WindowType,
};
use std::f32::consts::PI;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::{Duration, Instant};

const SAMPLE_RATE: f32 = 44_100.0;
const BLOCK_SIZE: usize = 512;
const TOTAL_FRAMES: usize = 180;
const SPECTRUM_BANDS: [(usize, usize, &str); 9] = [
    (1, 2, "63"),
    (3, 4, "125"),
    (5, 8, "250"),
    (9, 15, "500"),
    (16, 30, "1k"),
    (31, 60, "2k"),
    (61, 120, "4k"),
    (121, 220, "8k"),
    (221, 255, "16k"),
];

fn main() {
    let tui = TerminalCanvas::new(100, 32);
    tui.clear_screen();

    let mut eq_left = configured_eq();
    let mut eq_right = configured_eq();
    let compression = CompressorConfig {
        threshold_db: -14.0,
        ratio: 4.0,
        knee_db: 4.0,
        attack_ms: 5.0,
        release_ms: 80.0,
        makeup_gain_db: 1.5,
        mode: DetectionMode::Peak,
    };
    let mut compressor_left = Compressor::new(SAMPLE_RATE, compression.clone());
    let mut compressor_right = Compressor::new(SAMPLE_RATE, compression);
    let mut limiter_left = BrickwallLimiter::new(SAMPLE_RATE, -0.2, 20.0, 2.0);
    let mut limiter_right = BrickwallLimiter::new(SAMPLE_RATE, -0.2, 20.0, 2.0);
    let limiter_latency = (SAMPLE_RATE * 0.002).round() as usize;
    let mut phase = 0.0_f32;

    for frame in 0..TOTAL_FRAMES {
        let frame_started = Instant::now();
        let sweep_frequency = 180.0 + (frame as f32 * 0.08).sin().abs() * 3_200.0;
        let (mut left, mut right) = generate_block(&mut phase, sweep_frequency);

        let input_peak_left = Normalizer::find_peak(&left);
        let input_peak_right = Normalizer::find_peak(&right);
        let input_rms_left = Normalizer::calculate_rms(&left);
        let input_rms_right = Normalizer::calculate_rms(&right);

        eq_left.process_buffer(&mut left);
        eq_right.process_buffer(&mut right);
        compressor_left.process_buffer(&mut left);
        compressor_right.process_buffer(&mut right);
        limiter_left.process_buffer(&mut left);
        limiter_right.process_buffer(&mut right);

        let output_peak_left = Normalizer::find_peak(&left);
        let output_peak_right = Normalizer::find_peak(&right);
        let output_rms_left = Normalizer::calculate_rms(&left);
        let output_rms_right = Normalizer::calculate_rms(&right);
        let (spectrum, dominant_frequency) = analyze_spectrum(&left);

        tui.clear_region(1, 1, 98, 30);
        tui.draw_box(1, 1, 98, 30, " RESCALE DSP MONITOR ");
        tui.draw_stat(3, 4, "ENGINE", "offline synthetic monitor");
        tui.draw_stat(3, 39, "FORMAT", "44.1 kHz / stereo / f32");
        tui.draw_stat(3, 73, "FRAME", &format!("{:03}/{TOTAL_FRAMES}", frame + 1));

        tui.move_to(5, 4);
        print!("\x1b[1;36mINPUT ANALYSIS\x1b[0m");
        tui.draw_level_meter(6, 6, "L", input_peak_left, input_rms_left, 32);
        tui.draw_level_meter(7, 6, "R", input_peak_right, input_rms_right, 32);

        tui.move_to(9, 4);
        print!("\x1b[1;36mOUTPUT / MASTER BUS\x1b[0m");
        tui.draw_level_meter(10, 6, "L", output_peak_left, output_rms_left, 32);
        tui.draw_level_meter(11, 6, "R", output_peak_right, output_rms_right, 32);

        tui.move_to(13, 4);
        print!("\x1b[1;36mSPECTRUM ANALYZER  ·  POST-LIMITER  ·  FFT 512 / HANN\x1b[0m");
        tui.draw_fft_spectrum(15, 6, 7, &spectrum);

        tui.move_to(24, 4);
        print!("\x1b[1;36mPROCESSING STATUS\x1b[0m");
        tui.draw_stat(
            25,
            6,
            "CHAIN",
            "EQ 3-band > compressor > sample-peak limiter",
        );
        tui.draw_stat(
            26,
            6,
            "LIMITER",
            &format!("-0.2 dBFS ceiling / {limiter_latency} sample lookahead"),
        );
        tui.draw_stat(
            27,
            6,
            "DELTA RMS",
            &format!(
                "L {:+.1} dB / R {:+.1} dB",
                level_delta_db(input_rms_left, output_rms_left),
                level_delta_db(input_rms_right, output_rms_right)
            ),
        );
        tui.draw_stat(28, 6, "DOMINANT", &format!("{dominant_frequency:>6.1} Hz"));
        tui.draw_stat(
            28,
            42,
            "FRAME TIME",
            &format!(
                "{:>5.2} ms",
                frame_started.elapsed().as_secs_f64() * 1_000.0
            ),
        );
        tui.draw_stat(
            29,
            6,
            "METER MODE",
            "sample peak + RMS; not LUFS or true-peak",
        );

        let _ = stdout().flush();
        sleep(Duration::from_millis(40));
    }

    tui.move_to(32, 1);
    tui.restore_cursor();
    println!("RESCALE TUI monitor done.");
}

fn configured_eq() -> ParametricEqualizer3Band {
    let mut equalizer = ParametricEqualizer3Band::new(SAMPLE_RATE);
    equalizer.set_low_band(120.0, 4.0);
    equalizer.set_mid_band(1_000.0, 1.2, -2.0);
    equalizer.set_high_band(8_000.0, 2.0);
    equalizer
}

fn generate_block(phase: &mut f32, sweep_frequency: f32) -> (Vec<f32>, Vec<f32>) {
    let mut left = Vec::with_capacity(BLOCK_SIZE);
    let mut right = Vec::with_capacity(BLOCK_SIZE);

    for _ in 0..BLOCK_SIZE {
        *phase += 2.0 * PI * sweep_frequency / SAMPLE_RATE;
        let bass = (*phase * 0.2).sin() * 0.4;
        let mid = phase.sin() * 0.35;
        let texture = (*phase * 13.0).sin() * 0.1;
        let sample = bass + mid + texture;
        left.push(sample);
        right.push(sample * 0.85);
    }
    (left, right)
}

fn analyze_spectrum(samples: &[f32]) -> (Vec<(f32, &'static str)>, f32) {
    let mut windowed = samples.to_vec();
    Window::apply(&mut windowed, WindowType::Hann);
    let mut spectrum: Vec<Complex> = windowed
        .into_iter()
        .map(|sample| Complex::new(sample, 0.0))
        .collect();
    Fft::forward(&mut spectrum);
    let (dominant_frequency, _) = Fft::find_dominant_frequency(&spectrum, SAMPLE_RATE);

    let bands = SPECTRUM_BANDS
        .iter()
        .map(|&(start, end, label)| {
            let magnitude = spectrum[start..=end]
                .iter()
                .map(|bin| bin.magnitude())
                .sum::<f32>()
                / (end - start + 1) as f32;
            (magnitude, label)
        })
        .collect();
    (bands, dominant_frequency)
}

fn level_delta_db(input: f32, output: f32) -> f32 {
    Gain::linear_to_db(output) - Gain::linear_to_db(input)
}
