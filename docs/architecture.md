# Architecture & Components

Block-based DSP pipeline with composable processor chain. Zero external dependencies.

---

## Design Principles

1. **Zero dependencies** — No crates.io deps in `rescale-core` or `rescale-cli`. Verified by CI every push.
2. **Unsafe-free core** — `rescale-core` contains zero `unsafe` blocks. Only one exists: Windows console FFI (UTF-8 + ANSI escapes) in the CLI.
3. **Pure DSP separation** — Engine has no I/O policy; codecs and terminal output live elsewhere.
4. **Planar block model** — Process per-channel planar buffers; chain any number of processors.
5. **Streaming I/O** — Block-by-block WAV read/write, never load entire file into RAM.

---

## Engine Overview

```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│  WavStreamReader │───▶│ ProcessorChain│───▶│ WavStreamWriter │
│  (PCM 16/24/32f) │    │  [composable] │    │   (PCM 16/24/32f)│
└─────────────────┘    └──────────────┘    └─────────────────┘
                          │
            ┌────────────┼────────────┐
            ▼            ▼            ▼
         GainProcessor  EQ3Band     Compressor
            │            │            │
            └────────────┬────────────┘
                         ▼
                 LimiterProcessor
                         │
                    Flush() → drain tail
```

Each processor implements `AudioProcessor` trait:

```rust
trait AudioProcessor {
    fn process(&mut self, block: &mut AudioBlock) -> Result<(), Error>;
    fn latency_samples(&self) -> usize;
    fn flush(&mut self, block: &mut AudioBlock) -> Result<usize, Error>;
}
```

---

## Component Reference

### GainProcessor

Simple amplitude scaling. Unit gain = passthrough.

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `gain_db` | `-60.0` … `∞` | `0.0` | Amplitude in decibels |

### Parametric EQ (3-Band)

Full RBJ biquad filter bank with 7 filter types per band.

**Filter Types:**
- Low-pass (LP)
- High-pass (HP)
- Band-pass (BP)
- Notch
- Peaking
- Low-shelf
- High-shelf

Per-band parameters:

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `frequency` | `20.0` … `fs/2` | `1000.0` Hz | Center frequency |
| `q_factor` | `0.1` … `∞` | `0.707` | Quality factor |
| `gain_db` | `-36.0` … `+36.0` | `0.0` | Boost/cut amount |

### Compressor

Dynamic range compression with configurable detection mode.

**Detection Modes:**
- `Rms` — RMS envelope tracking
- `Peak` — Sample peak detection

Configuration:

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `threshold_db` | `-60.0` … `0.0` | `-12.0` dB | Compression threshold |
| `ratio` | `1.0` … `∞` | `4.0` | Compression ratio |
| `knee_db` | `0.0` … `24.0` | `4.0` | Knee softness |
| `attack_ms` | `0.1` … `100.0` | `5.0` ms | Attack time |
| `release_ms` | `10.0` … `1000.0` | `80.0` ms | Release time |
| `makeup_gain_db` | `0.0` … `36.0` | `0.0` | Post-compression gain |

**⚠️ Limitation:** Release is ~2× slower than configured due to power-domain smoothing. No stereo linking — independent per-channel state.

### Lookahead Limiter

Brickwall ceiling protection with configurable lookahead window.

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| `ceiling_db` | `-60.0` … `0.0` | `-0.2` dB | Maximum output level |
| `lookahead_ms` | `0.1` … `50.0` | `2.0` ms | Lookahead window |
| `attack_ms` | `0.01` … `50.0` | `0.1` ms | Envelope attack |

**⚠️ Limitation:** Uses hard clamp for ceiling protection. Gain reduction from exponential envelope, not true sliding max-hold. Transients may attenuate insufficiently then clip. Latency compensation not wired into CLI yet.

---

## Buffer Model

### AudioBlock

Planar (non-interleaved) buffer:

```rust
struct AudioBlock {
    channels: Vec<Vec<f32>>,  // One vec per channel
    sample_rate: f64,
}
```

Processors receive mutable per-channel views. Block sizes are flexible — engine tested at 64, 256, 1024 frames.

### AudioSpec

Stream configuration passed during `prepare()`:

```rust
struct AudioSpec {
    sample_rate: f64,    // e.g., 44100.0
    num_channels: usize, // e.g., 2 for stereo
}
```

---

## FFT Toolkit

Window functions for spectral analysis:

| Window | Formula | Use Case |
|--------|---------|----------|
| Hann | `(1 - cos(2πn/N)) / 2` | General purpose spectrum |
| Hamming | `0.54 - 0.46·cos(2πn/N)` | Reduced spectral leakage |
| Blackman | `0.42 - 0.5·cos(...) + 0.08·cos(...)` | High dynamic range |
| Rectangular | `1` (no windowing) | Raw spectral view |

**⚠️ Limitations:**
- Windows use symmetric `N-1` denominator (not COLA-compatible periodic `N`)
- Coherent gain not compensated
- `Window::apply` allocates a `Vec` per call
- Not STFT-ready (no overlap-add support)

---

## Streaming WAV I/O

### WavStreamReader

Block-by-block PCM decoder:

- Reads 16-bit, 24-bit, 32-bit float
- Validates chunk size alignment
- Yields `AudioBlock` per internal buffer fill
- Supports TPDF dither on write

### WavStreamWriter

Block-by-block PCM encoder:

- Writes target bit depth
- Applies TPDF dither (auto-disabled for 32f)
- Finalizes header after last block

Both operate on streaming data — files can be larger than available RAM.

---

## Terminal UI Monitor

Built directly into `rescale-core` using raw ANSI escapes:

- **VU Meters**: Peak hold + RMS per channel
- **Spectrum Analyzer**: 9-band (63Hz – 16kHz), FFT-512, Hann window
- **Chain Status**: Current processors, limiter config, delta-RMS, dominant frequency
- **Frame Time**: Processing overhead measurement

Draws synthetic sweep data when no audio device is available. Not a live audio host.

---

## Future Architecture Evolution

See [Roadmap](roadmap.md) for planned crate splits (`rescale-formats`, `rescale-tui`).
