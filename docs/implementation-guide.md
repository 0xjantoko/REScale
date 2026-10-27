# Full DSP ReScale Audio Implementation in Rust (Pure / From Scratch)

> Implementation status as of September 2026: all DSP modules, WAV I/O (batch & streaming), the engine block, and the CLI are implemented. This document summarizes the architecture and actual usage.

## Project Architecture (Multi-Crate Cargo Workspace)

```text
REScale/
├── Cargo.toml                      ← Workspace root (resolver = "2")
├── crates/
│   ├── rescale-core/               ← 🧠 Library crate (zero dependency)
│   │   ├── Cargo.toml              ← 0 dependencies
│   │   └── src/
│   │       ├── lib.rs              ← Public API + 21 unit tests
│   │       ├── gain.rs             ← dB ↔ linear conversion
│   │       ├── normalize.rs        ← Peak & RMS normalization
│   │       ├── convert.rs          ← Sample format (i16/i32/f32)
│   │       ├── limiter.rs          ← Soft-clip (tanh/cubic/hard)
│   │       ├── engine.rs           ← AudioSpec, AudioBlock, ProcessorChain
│   │       ├── buffer/             ← RingBuffer, DelayLine, FeedbackDelay
│   │       ├── filter/             ← Biquad, RBJ coefficients, 3-Band EQ
│   │       ├── drc/                ← Envelope, Compressor, Limiter
│   │       ├── fft/                ← Complex, Window, Cooley-Tukey FFT
│   │       ├── wav/                ← Header, Reader, Writer, Stream I/O
│   │       └── tui/                ← ANSI canvas, Interactive menu
│   │
│   └── rescale-cli/                ← 🖥️ Binary crate
│       ├── Cargo.toml              ← depends on rescale-core
│       ├── src/bin/rescale_cli.rs  ← CLI entry point
│       └── examples/               ← full_pipeline, tui_monitor
```

---

## Public API Reference

### 1. `SampleConvert` — Signal Format Conversion

```rust
use rescale_core::SampleConvert;

// i16 → f32 (rentang asimetris: neg/32768, pos/32767)
let f: f32 = 16384_i16.to_f32();     // ≈ 0.5
let i: i16 = i16::from_f32(0.5);     // → 16384

// i32 → f32 (simetris: /2^31)
let f: f32 = 2147483647_i32.to_f32(); // ≈ 1.0

// f32 → f32 (identitas)
let f: f32 = 0.5_f32.to_f32();       // → 0.5
```

### 2. `Gain` — Decibel Manipulation

```rust
use rescale_core::Gain;

// dB → linear: G = 10^(dB/20)
let linear: f32 = Gain::db_to_linear(6.0);  // ≈ 1.995 (≈ 2x)

// Linear → dB: dB = 20·log₁₀(x)
let db: f32 = Gain::linear_to_db(0.5);      // ≈ -6.02 dB

// Terapkan gain ke buffer
let mut buffer = vec![0.1, -0.2, 0.3];
Gain::apply_db(&mut buffer, 6.0);            // +6 dB
Gain::apply_linear(&mut buffer, 0.5);        // × 0.5
```

### 3. `Normalizer` — Peak & RMS

```rust
use rescale_core::Normalizer;

let buffer = vec![0.1, -0.4, 0.5, -0.2];

// Cari peak
let peak = Normalizer::find_peak(&buffer);   // → 0.5

// RMS
let rms = Normalizer::calculate_rms(&buffer); // → sqrt(mean(x²))

// Normalisasi buffer tunggal
Normalizer::normalize_to_peak(&mut buffer, 0.9);

// Aman untuk stereo (gain seragam di semua channel)
let mut channels = vec![vec![0.3; 100], vec![0.5; 100]];
Normalizer::normalize_channels_to_peak(&mut channels, 0.8);
```

### 4. `Limiter` — Soft-Clipping

```rust
use rescale_core::Limiter;

let clipped = Limiter::tanh_soft_clip(1.5);    // → ≈ 0.905
let cubic   = Limiter::cubic_soft_clip(1.5);   // → 2/3 ≈ 0.667
let safe    = Limiter::hard_clamp(1.5);         // → 1.0
```

### 5. Biquad IIR Filter & 3-Band EQ

```rust
use rescale_core::{BiquadCoefficients, BiquadFilter, ParametricEqualizer3Band};

// Filter Low-Pass (RBJ Cookbook)
let coeffs = BiquadCoefficients::low_pass(44100.0, 1000.0, 0.7071);
let mut filter = BiquadFilter::new(coeffs);
filter.process_buffer(&mut buffer);

// EQ Parametrik 3-Band
let mut eq = ParametricEqualizer3Band::new(44100.0);
eq.set_low_band(150.0, 3.0);       // Bass: cutoff 150 Hz, +3 dB
eq.set_mid_band(1000.0, 1.2, -2.0); // Mid: center 1 kHz, Q=1.2, -2 dB
eq.set_high_band(6000.0, 1.5);     // Treble: cutoff 6 kHz, +1.5 dB
eq.process_buffer(&mut buffer);
```

### 6. Dynamic Range Control

```rust
use rescale_core::{Compressor, CompressorConfig, DetectionMode, BrickwallLimiter};

// Compressor (soft-knee)
let config = CompressorConfig {
    threshold_db: -12.0, ratio: 4.0, knee_db: 4.0,
    attack_ms: 5.0, release_ms: 100.0, makeup_gain_db: 2.0,
    mode: DetectionMode::Rms,
};
let mut comp = Compressor::new(44100.0, config);
comp.process_buffer(&mut buffer);

// Lookahead Brickwall Limiter (ceiling -0.3 dBFS, 2ms lookahead)
let mut lim = BrickwallLimiter::new(44100.0, -0.3, 20.0, 2.0);
lim.process_buffer(&mut buffer);
```

### 7. FFT & Frequency Analysis

```rust
use rescale_core::{Complex, Fft, Window, WindowType};

let fft_size = 1024;
let mut fft_buf: Vec<Complex> = buffer[..fft_size].iter()
    .map(|&s| Complex::new(s, 0.0)).collect();

// Windowing
let mut win: Vec<f32> = fft_buf.iter().map(|c| c.re).collect();
Window::apply(&mut win, WindowType::Hann);
for (c, &w) in fft_buf.iter_mut().zip(win.iter()) { c.re = w; }

// Forward FFT → Domain Frekuensi
Fft::forward(&mut fft_buf);

// Deteksi frekuensi dominan
let (freq, mag) = Fft::find_dominant_frequency(&fft_buf, 44100.0);

// Inverse FFT → Domain Waktu
Fft::inverse(&mut fft_buf);
```

### 8. WAV File I/O

```rust
use rescale_core::{WavReader, WavWriter, WavSampleFormat};

// Baca batch
let (header, channels) = WavReader::read_from_file("input.wav")?;

// Tulis batch (TPDF dither default untuk PCM)
WavWriter::write_to_file("output.wav", &channels, 44100, WavSampleFormat::Pcm24Bit)?;
```

### 9. Engine Block & ProcessorChain

```rust
use rescale_core::{
    AudioSpec, AudioBlock, ProcessorChain,
    GainProcessor, EqualizerProcessor, CompressorProcessor, LimiterProcessor,
};

let spec = AudioSpec::new(44100.0, 2).unwrap();
let mut chain = ProcessorChain::new();
chain.push(GainProcessor::new(3.0));           // +3 dB
chain.push(CompressorProcessor::new(config));  // DRC
chain.push(LimiterProcessor::new(-1.0, 20.0, 2.0));
chain.prepare(spec).unwrap();

// Proses block
chain.process(&mut block).unwrap();

// Flush sisa tail (latensi limiter)
chain.flush(&mut flush_block).unwrap();
```

---

## Complete Pipeline Example

```rust
use rescale_core::*;
use std::io::Cursor;

fn main() {
    let sample_rate = 44100.0;
    let num_samples = 4410;

    // 1. Buat gelombang sinus
    let channel: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
        })
        .collect();

    // 2. EQ
    let mut eq = ParametricEqualizer3Band::new(sample_rate);
    eq.set_low_band(150.0, 2.0);
    eq.set_high_band(6000.0, -3.0);

    let mut audio = channel.clone();
    eq.process_buffer(&mut audio);

    // 3. Compressor
    let config = CompressorConfig {
        threshold_db: -10.0, ratio: 3.0, knee_db: 4.0,
        attack_ms: 5.0, release_ms: 80.0, makeup_gain_db: 2.0,
        mode: DetectionMode::Rms,
    };
    let mut comp = Compressor::new(sample_rate, config);
    comp.process_buffer(&mut audio);

    // 4. Limiter
    let mut limiter = BrickwallLimiter::new(sample_rate, -0.5, 30.0, 2.0);
    limiter.process_buffer(&mut audio);

    // 5. Verifikasi ceiling
    let peak = Normalizer::find_peak(&audio);
    assert!(peak <= Gain::db_to_linear(-0.5) + 1e-4);

    // 6. Tulis WAV
    let stereo = vec![audio.clone(), audio];
    WavWriter::write_to_stream(&mut Cursor::new(Vec::new()), &stereo, 44100, WavSampleFormat::Pcm24Bit).unwrap();
}
```

---

## Test Suite (58/58 Passing — 49 core + 9 CLI integration)

| Test | Verification |
|---|---|
| `test_gain_and_normalize` | dB→linear→apply→normalize peak |
| `test_audio_block_and_processor_chain` | AudioBlock + ProcessorChain execution |
| `test_wav_write_and_read_roundtrip` | Write stereo 16-bit → read, error < 1e-4 |
| `test_wav_stream_reader_and_writer_block_roundtrip` | Block streaming 24-bit roundtrip |
| `test_limiter_processor_flush_tail` | Flush returns the lookahead tail |
| `test_engine_error_validation_and_channel_mismatch` | Invalid spec, channel mismatch |
| `test_equalizer_and_gain_processors_in_chain` | EQ + Gain in the chain |
| `test_float_wav_lossless` | 32-bit float WAV lossless roundtrip |
| `test_ring_buffer` | Circular buffer read_past |
| `test_fractional_delay_interpolation` | Accurate linear interpolation |
| `test_feedback_delay_echo` | 2-sample delay, 50% feedback |
| `test_biquad_low_pass_attenuation` | 500 Hz LPF attenuates 10 kHz |
| `test_parametric_equalizer_boost` | Mid-band boost ~6 dB |
| `test_dynamic_compressor_reduction` | Compressor reduction matches ratio |
| `test_brickwall_lookahead_limiter_ceiling` | Output peak ≤ ceiling |
| `test_fft_forward_and_inverse_roundtrip` | FFT→IFFT reconstruction |
| `test_fft_dominant_frequency_detection` | Frequency detected within 1 bin |
| `test_fft_helpers_accept_empty_input` | Empty input → empty spectrum, `(0.0, 0.0)` |
| `test_fft_magnitude_spectrum_has_nyquist_bin` | `len / 2 + 1` bins (DC … Nyquist) |
| `test_fft_forward_rejects_non_power_of_two` | Panics on a 100-sample FFT |
| `test_full_mastering_chain_pipeline` | Full EQ→Comp→Limit→WAV pipeline |
| `engine::flush_*` (3 tests) | Chain tail drained for any stage order and multiple latency stages |
| `engine::block_size_does_not_change_output` | 64/256/1024-frame blocks equal single-block render |
| `delay_line::*` (4 tests) | Post-push delay semantics, fractional interpolation, pre-push variant |
| `envelope::*` (3 tests) | Peak/RMS release time constants match the configured value |
| `cli.rs` (9 integration tests) | Real binary: latency compensation, pristine accuracy, bit depth, error paths |
| `max_hold::*` (3 tests) | Sliding maximum: window tracking, brute-force cross-check, reset |
| `limiter::*` (4 tests) | Detector gain law, percussive ceiling, pre-output attack, reset |
| `engine::limiter_*` (2 tests) | Stereo-linked gain and percussive ceiling on the processor |
| `coefficients::*` (5 tests) | Frequency/Q/gain validation, no NaN from invalid input |
| `safety::*` (3 tests) | NaN/Inf guard and denormal flush |
