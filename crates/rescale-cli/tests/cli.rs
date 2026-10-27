//! Integration test: menjalankan binary `rescale_cli` sungguhan lalu memeriksa
//! file output yang dihasilkan — streaming I/O, kompensasi latency, dan opsi CLI.
//!
//! Zero dependency: hanya `std::process::Command` dan `rescale_core` yang sudah
//! menjadi dependency crate ini (tanpa `assert_cmd`, `tempfile`, dsb.).

use rescale_core::{
    DitherMode, Gain, Normalizer, WavReader, WavSampleFormat, WavWriteOptions, WavWriter,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SAMPLE_RATE: u32 = 44_100;
const CHANNELS: usize = 2;
const TOTAL_FRAMES: usize = 22_050; // 0,5 detik @ 44,1 kHz
const SINE_HZ: f32 = 440.0;
const INPUT_AMPLITUDE: f32 = 0.5;

/// Folder sementara per test; dibersihkan otomatis saat test selesai.
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(test_name: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "rescale_cli_test_{}_{}",
            std::process::id(),
            test_name
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("buat folder test");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_cli(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rescale_cli"))
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .expect("jalankan rescale_cli")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// WAV float32 stereo berisi sine wave (satu channel dibalik fasenya).
fn write_test_wav(path: &Path) {
    let channels: Vec<Vec<f32>> = (0..CHANNELS)
        .map(|channel_index| {
            (0..TOTAL_FRAMES)
                .map(|frame| {
                    let time = frame as f32 / SAMPLE_RATE as f32;
                    let sample =
                        (2.0 * std::f32::consts::PI * SINE_HZ * time).sin() * INPUT_AMPLITUDE;
                    if channel_index == 0 {
                        sample
                    } else {
                        -sample
                    }
                })
                .collect()
        })
        .collect();

    WavWriter::write_to_file_with_options(
        path,
        &channels,
        SAMPLE_RATE,
        WavWriteOptions::new(WavSampleFormat::Pcm32BitFloat, DitherMode::None),
    )
    .expect("tulis WAV uji");
}

fn peak_of(channels: &[Vec<f32>]) -> f32 {
    channels
        .iter()
        .map(|channel| Normalizer::find_peak(channel))
        .fold(0.0_f32, f32::max)
}

#[test]
fn mastering_compensates_latency_and_keeps_frame_count() {
    let dir = TestDir::new("latency");
    let input = dir.path().join("song.wav");
    write_test_wav(&input);

    let output = run_cli(&[input.to_str().unwrap(), "-1.0"]);
    assert!(output.status.success(), "CLI gagal: {}", stderr_of(&output));

    let output_path = dir.path().join("REScale-song.wav");
    assert!(output_path.exists(), "file output tidak dibuat");

    let (header, channels) = WavReader::read_from_file(&output_path).expect("baca output");
    assert_eq!(
        header.num_samples_per_channel, TOTAL_FRAMES,
        "jumlah frame output harus sama dengan input (latency terkompensasi)"
    );
    assert_eq!(channels.len(), CHANNELS);

    let peak = peak_of(&channels);
    let ceiling = Gain::db_to_linear(-1.0) + 1e-3;
    assert!(
        peak <= ceiling,
        "peak output {peak} melampaui ceiling {ceiling}"
    );
}

#[test]
fn pristine_mode_is_sample_accurate() {
    let dir = TestDir::new("pristine");
    let input = dir.path().join("song.wav");
    write_test_wav(&input);

    let output = run_cli(&[input.to_str().unwrap(), "--no-dsp", "--bit-depth", "32f"]);
    assert!(output.status.success(), "CLI gagal: {}", stderr_of(&output));

    let output_path = dir.path().join("REScale-song.wav");
    let (output_header, output_channels) =
        WavReader::read_from_file(&output_path).expect("baca output");
    let (input_header, input_channels) = WavReader::read_from_file(&input).expect("baca input");

    assert_eq!(output_header.format, WavSampleFormat::Pcm32BitFloat);
    assert_eq!(
        output_header.num_samples_per_channel, input_header.num_samples_per_channel,
        "mode pristine tidak boleh mengubah jumlah frame"
    );

    for (channel_index, (input_channel, output_channel)) in input_channels
        .iter()
        .zip(output_channels.iter())
        .enumerate()
    {
        for (frame, (input_sample, output_sample)) in
            input_channel.iter().zip(output_channel.iter()).enumerate()
        {
            assert!(
                (input_sample - output_sample).abs() < 1e-6,
                "channel {channel_index} frame {frame}: {input_sample} vs {output_sample}"
            );
        }
    }
}

#[test]
fn bit_depth_option_sets_output_format() {
    let dir = TestDir::new("bit_depth");
    let input = dir.path().join("song.wav");
    write_test_wav(&input);

    let output = run_cli(&[
        input.to_str().unwrap(),
        "--bit-depth",
        "16",
        "--dither",
        "none",
    ]);
    assert!(output.status.success(), "CLI gagal: {}", stderr_of(&output));

    let output_path = dir.path().join("REScale-song.wav");
    let (header, channels) = WavReader::read_from_file(&output_path).expect("baca output");
    assert_eq!(header.format, WavSampleFormat::Pcm16Bit);
    assert_eq!(header.num_samples_per_channel, TOTAL_FRAMES);
    assert!(peak_of(&channels) > 0.1, "output tidak boleh senyap");
}

#[test]
fn missing_input_file_fails() {
    let dir = TestDir::new("missing");
    let missing = dir.path().join("nope.wav");

    let output = run_cli(&[missing.to_str().unwrap(), "-1.0"]);
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("File not found"),
        "stderr: {}",
        stderr_of(&output)
    );
}

#[test]
fn unsupported_extension_is_rejected() {
    let dir = TestDir::new("extension");
    let input = dir.path().join("song.flac");
    fs::write(&input, b"not audio").expect("tulis file uji");

    let output = run_cli(&[input.to_str().unwrap(), "-1.0"]);
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("not supported"),
        "stderr: {}",
        stderr_of(&output)
    );
}

#[test]
fn out_of_range_target_is_rejected() {
    let dir = TestDir::new("target_range");
    let input = dir.path().join("song.wav");
    write_test_wav(&input);

    let output = run_cli(&[input.to_str().unwrap(), "5.0"]);
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("between -60.0 and 0.0"),
        "stderr: {}",
        stderr_of(&output)
    );
}

#[test]
fn existing_output_is_not_overwritten_silently() {
    let dir = TestDir::new("overwrite");
    let input = dir.path().join("song.wav");
    write_test_wav(&input);
    let output_path = dir.path().join("REScale-song.wav");
    fs::write(&output_path, b"keep me").expect("tulis output awal");

    let output = run_cli(&[input.to_str().unwrap(), "-1.0"]);
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("Output already exists"),
        "stderr: {}",
        stderr_of(&output)
    );
    assert_eq!(
        fs::read(&output_path).expect("baca output lama"),
        b"keep me",
        "file output lama tidak boleh ditimpa"
    );
}

#[test]
fn unknown_argument_is_rejected() {
    let dir = TestDir::new("unknown_argument");
    let input = dir.path().join("song.wav");
    write_test_wav(&input);

    let output = run_cli(&[input.to_str().unwrap(), "--bogus"]);
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("Unknown argument"),
        "stderr: {}",
        stderr_of(&output)
    );
}

#[test]
fn help_exits_successfully() {
    let output = run_cli(&["--help"]);
    assert!(output.status.success());
    assert!(
        stdout_of(&output).contains("Usage:"),
        "stdout: {}",
        stdout_of(&output)
    );
}
