# REScale DSP Engine Usage Guide

> Implementation status as of September 2026: the full engine block API, streaming I/O, and CLI are available.

**REScale** is a pure-Rust DSP audio engine with no external dependencies. The library provides a modular API for audio manipulation, filtering, dynamic range control, delay effects, Fourier transforms (FFT), and binary I/O for the RIFF WAV format.

---

## 1. Adding REScale to Your Rust Project

### As a Library Dependency
```toml
# Cargo.toml project kamu (sesuaikan path ke lokasi clone REScale kamu)
[dependencies]
rescale-core = { path = "../REScale/crates/rescale-core" }
```

```rust
use rescale_core::{Gain, Normalizer, Compressor, BrickwallLimiter};
```

### As a Binary (CLI Tool)
```bash
cargo run --bin rescale_cli -- --help
```

---

## 2. Usage Patterns per Component

### A. Reading and Writing `.wav` Files
```rust
use rescale_core::{WavReader, WavWriter, WavSampleFormat};

// Baca batch (memuat seluruh file ke RAM)
let (header, channels) = WavReader::read_from_file("input.wav")?;
println!("Channels: {}, Rate: {} Hz, Format: {:?}",
    header.num_channels, header.sample_rate, header.format);
// channels = Vec<Vec<f32>> — channels[0] = Left, channels[1] = Right

// Tulis batch (TPDF dither default untuk PCM)
WavWriter::write_to_file("output.wav", &channels, header.sample_rate, WavSampleFormat::Pcm24Bit)?;
```

### B. Streaming Block I/O (Real-Time Friendly)

> **Note (v0.1.0):** this is a library-level API. `rescale-cli` still reads a whole
> file and processes it as a single block, and does not call `flush()`. See
> *Known limitations* in the README.
```rust
use rescale_core::{WavStreamReader, WavStreamWriter, WavWriteOptions, WavSampleFormat, DitherMode};
use rescale_core::{AudioBlock};

// Writer streaming
let options = WavWriteOptions::new(WavSampleFormat::Pcm24Bit, DitherMode::Tpdf);
let mut writer = WavStreamWriter::new(&mut cursor, 2, 44100, options)?;

// Tulis per block 256 frame
for start in (0..total_frames).step_by(256) {
    let end = (start + 256).min(total_frames);
    let block = AudioBlock::new(&mut slices)?;
    writer.write_block(&block)?;
}
writer.finalize()?; // Perbarui header RIFF

// Reader streaming
let mut reader = WavStreamReader::new(cursor)?;
while !reader.is_finished() {
    let frames = reader.read_block(&mut block)?;
    if frames == 0 { break; }
}
```

---

### C. Gain, Decibels & Headroom Normalization
```rust
use rescale_core::{Gain, Normalizer};

// Naikkan volume +3 dB (≈ 2x amplitude)
Gain::apply_db(&mut buffer, 3.0);

// Hitung Peak dan RMS
let peak = Normalizer::find_peak(&buffer);
let rms = Normalizer::calculate_rms(&buffer);

// Normalisasi peak ke -0.5 dBFS (~0.944 linear)
Normalizer::normalize_to_peak(&mut buffer, 0.944);

// Aman untuk stereo: gain seragam di semua channel
let mut channels = vec![left, right];
Normalizer::normalize_channels_to_peak(&mut channels, 0.9);
```

---

### D. 3-Band Parametric Equalizer (EQ)
```rust
use rescale_core::ParametricEqualizer3Band;

let mut eq = ParametricEqualizer3Band::new(44100.0);

// Bass (Low Shelf), Mid (Peaking), Treble (High Shelf)
eq.set_low_band(150.0, 3.0);        // Cutoff 150 Hz, Boost +3 dB
eq.set_mid_band(1000.0, 1.2, -2.0); // Center 1 kHz, Q=1.2, Cut -2 dB
eq.set_high_band(6000.0, 1.5);      // Cutoff 6 kHz, Boost +1.5 dB

eq.process_buffer(&mut buffer);
```

---

### E. Dynamic Compressor & Lookahead Brickwall Limiter
```rust
use rescale_core::{Compressor, CompressorConfig, DetectionMode, BrickwallLimiter};

// Compressor (soft-knee, deteksi RMS)
let config = CompressorConfig {
    threshold_db: -12.0,      // Mulai kompresi di atas -12 dBFS
    ratio: 4.0,               // Ratio 4:1
    knee_db: 4.0,             // Soft knee 4 dB
    attack_ms: 5.0,           // Fast response 5 ms
    release_ms: 100.0,        // Recovery 100 ms
    makeup_gain_db: 2.0,      // Compensation +2 dB
    mode: DetectionMode::Rms,
};
let mut compressor = Compressor::new(44100.0, config);
compressor.process_buffer(&mut buffer);

// Lookahead Brickwall Limiter
let mut limiter = BrickwallLimiter::new(44100.0, -0.3, 20.0, 2.0);
//                     (sample_rate, ceiling, release_ms, lookahead_ms)
limiter.process_buffer(&mut buffer);
```

---

### F. Fourier Transform (FFT) & Spectrum Analysis
```rust
use rescale_core::{Complex, Fft, Window, WindowType};

let fft_size = 1024; // Harus Power-of-Two

// 1. Siapkan buffer complex
let mut fft_buffer: Vec<Complex> = buffer[..fft_size]
    .iter().map(|&s| Complex::new(s, 0.0)).collect();

// 2. Windowing (Hann) untuk mengurangi kebocoran spektral
let mut time_weights: Vec<f32> = fft_buffer.iter().map(|c| c.re).collect();
Window::apply(&mut time_weights, WindowType::Hann);
for (c, &w) in fft_buffer.iter_mut().zip(time_weights.iter()) { c.re = w; }

// 3. Forward FFT
Fft::forward(&mut fft_buffer);

// 4. Deteksi frekuensi dominan
let (dominant_freq, magnitude) = Fft::find_dominant_frequency(&fft_buffer, 44100.0);

// 5. Inverse FFT (kembali ke time domain)
Fft::inverse(&mut fft_buffer);
```

---

### G. Engine Block & ProcessorChain
```rust
use rescale_core::{
    AudioSpec, AudioBlock, ProcessorChain,
    GainProcessor, EqualizerProcessor, CompressorProcessor, LimiterProcessor,
};

let spec = AudioSpec::new(44100.0, 2).unwrap(); // Stereo 44.1 kHz

// Bangun rantai processor
let mut chain = ProcessorChain::new();
chain.push(GainProcessor::new(3.0));
chain.push(EqualizerProcessor::new(150.0, 2.0, 1000.0, 1.0, 0.0, 6000.0, -3.0));
chain.push(CompressorProcessor::new(config));
chain.push(LimiterProcessor::new(-1.0, 20.0, 2.0));
chain.prepare(spec).unwrap();

// Proses block audio
chain.process(&mut block).unwrap();

// Flush sisa tail (latensi lookahead limiter)
let flushed = chain.flush(&mut flush_block).unwrap();

// Total latensi
let latency = chain.latency_samples();
```

---

## 3. Running the CLI

### Interactive TUI Menu
```bash
cargo run --bin rescale_cli
# atau
cargo run --bin rescale_cli -- --menu
```

### Command-Line Mastering
```bash
# Mastering WAV ke -1.0 dBFS (default 24-bit + TPDF dither)
cargo run --bin rescale_cli -- input.wav -1.0

# Output 16-bit
cargo run --bin rescale_cli -- input.wav -1.0 --bit-depth 16

# Mode pristine (lewati DSP, MP3→WAV transparan)
cargo run --bin rescale_cli -- input.mp3 --no-dsp

# Output float 32-bit
cargo run --bin rescale_cli -- input.wav -1.0 --bit-depth 32f
```

### Standalone Examples
```bash
# Demo pipeline lengkap
cargo run --example full_pipeline

# Monitor spektrum TUI
cargo run --example tui_monitor
```
