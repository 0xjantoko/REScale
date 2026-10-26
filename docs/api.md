# Library API Reference

Public API for `rescale-core` crate. Use as a dependency in your Rust project.

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
rescale-core = { path = "../path/to/rescale-core" }

# Or from crates.io when published:
# rescale-core = "0.1.0"
```

**MSRV:** Rust 1.78+

---

## Core Types

### AudioSpec

Stream configuration passed to every processor.

```rust
pub struct AudioSpec {
    pub sample_rate: f64,   // e.g., 44100.0
    pub num_channels: usize, // e.g., 2 for stereo
}

impl AudioSpec {
    pub fn new(sample_rate: f64, channels: usize) -> Result<Self, Error>;
}
```

Must be prepared before processing begins. Sample rate should match input file metadata.

---

### AudioBlock

Planar (non-interleaved) audio buffer. Mutable views per channel.

```rust
pub struct AudioBlock {
    // Internals hidden; use constructors below
}

impl AudioBlock {
    pub fn new(num_channels: usize, num_frames: usize) -> Self;
    pub fn num_channels(&self) -> usize;
    pub fn num_frames(&self) -> usize;
    pub fn sample_rate(&self) -> f64;
    
    // Per-channel mutable access
    pub fn channel_mut(&mut self, ch: usize) -> &mut Vec<f32>;
    
    // Iterate over all channels
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Vec<f32>>;
}
```

Processors receive this by mutable reference. Block size is flexible — engine tested at 64, 256, 1024 frames.

---

## Processor Trait

All DSP units implement this trait. Chain composition happens through `ProcessorChain`.

```rust
pub trait AudioProcessor: Send {
    /// Process one block through this processor.
    fn process(&mut self, block: &mut AudioBlock) -> Result<(), Error>;
    
    /// Latency introduced by this processor (samples).
    fn latency_samples(&self) -> usize;
    
    /// Flush internal state/tail after end-of-stream.
    fn flush(&mut self, block: &mut AudioBlock) -> Result<usize, Error>;
}
```

The `flush()` hook drains lookahead tails and delay lines. Call at end of stream for processors with latency.

---

## ProcessorChain

Composable pipeline that chains any number of processors together.

```rust
pub struct ProcessorChain {
    // Internals hidden
}

impl ProcessorChain {
    pub fn new() -> Self;
    
    /// Add a processor to the chain.
    pub fn push(&mut self, processor: impl AudioProcessor + 'static);
    
    /// Apply stream spec to all processors. Must call before processing.
    pub fn prepare(&mut self, spec: AudioSpec) -> Result<(), Error>;
    
    /// Process one block through entire chain.
    pub fn process(&mut self, block: &mut AudioBlock) -> Result<(), Error>;
    
    /// Drain lookahead tail from all processors.
    pub fn flush(&mut self, block: &mut AudioBlock) -> Result<usize, Error>;
    
    /// Total latency across all processors (samples).
    pub fn latency_samples(&self) -> usize;
    
    /// Number of processors currently in chain.
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

Latency is **summed** across all processors. Compensate output frame offset accordingly.

---

## Gain Processor

Simple amplitude scaling.

```rust
pub struct GainProcessor {
    gain_db: f64,
}

impl GainProcessor {
    pub fn new(gain_db: f64) -> Self;  // gain_db: -60.0 to ∞
}
```

Zero latency. Unit gain (= 0 dB) = passthrough.

**Example:**
```rust
// Reduce by 6 dB
chain.push(GainProcessor::new(-6.0));
```

---

## Parametric EQ

Full RBJ biquad filter bank (see [architecture.md](architecture.md) for full parameter table).

```rust
pub struct Eq3BandConfig {
    pub bands: [EqBandConfig; 3],
}

pub struct EqBandConfig {
    pub frequency: f64,   // 20.0 .. fs/2 Hz
    pub q_factor: f64,    // 0.1 .. ∞
    pub gain_db: f64,     // -36.0 .. +36.0 dB
    pub filter_type: FilterType,
}

pub enum FilterType {
    LowPass, HighPass, BandPass, Notch, Peaking, LowShelf, HighShelf,
}

pub struct Eq3BandProcessor;

impl Eq3BandProcessor {
    pub fn new(config: Eq3BandConfig) -> Self;
}
```

Minimal latency (~1 sample). Cascades three independent biquad filters.

---

## Compressor

Dynamic range compression with configurable detection mode.

```rust
pub enum DetectionMode {
    Rms,  // RMS envelope tracking
    Peak, // Sample peak detection
}

pub struct CompressorConfig {
    pub threshold_db: f64,    // -60.0 .. 0.0 dB
    pub ratio: f64,           // 1.0 .. ∞ (∞ = hard knee)
    pub knee_db: f64,         // 0.0 .. 24.0 dB
    pub attack_ms: f64,       // 0.1 .. 100.0 ms
    pub release_ms: f64,      // 10.0 .. 1000.0 ms
    pub makeup_gain_db: f64,  // 0.0 .. 36.0 dB
    pub mode: DetectionMode,
}

pub struct CompressorProcessor;

impl CompressorProcessor {
    pub fn new(config: CompressorConfig) -> Self;
}
```

Per-channel state only — no stereo linking. Detects whether signal exceeds threshold, applies gain reduction per configured ratio.

---

## Lookahead Limiter

Brickwall ceiling protection with configurable window.

```rust
pub struct LimiterConfig {
    pub ceiling_db: f64,     // -60.0 .. 0.0 dB
    pub lookahead_ms: f64,   // 0.1 .. 50.0 ms
    pub attack_ms: f64,      // 0.01 .. 50.0 ms
}

pub struct LimiterProcessor;

impl LimiterProcessor {
    pub fn new(ceiling_db: f64, lookahead_ms: f64, attack_ms: f64) -> Self;
    pub fn new_with_config(config: LimiterConfig) -> Self;
}
```

Lookahead introduces ~2ms latency on default config. Requires `flush()` call to drain tail. Hard clamp protects ceiling.

---

## WAV Streaming I/O

### Reader

```rust
pub struct WavStreamReader {
    // Internals hidden
}

impl WavStreamReader {
    pub fn new(path: &Path) -> Result<Self, Error>;
    pub fn read_block(&mut self, block: &mut AudioBlock) -> Result<bool, Error>;
    /// true = more blocks available, false = EOF
    pub fn finished(&self) -> bool;
    pub fn spec(&self) -> &AudioSpec;
}
```

Processes files block-by-block. Never loads entire file into RAM. Supports PCM 16/24/32-bit float.

### Writer

```rust
pub struct WavStreamWriter {
    // Internals hidden  
}

impl WavStreamWriter {
    pub fn new(
        path: &Path, 
        spec: AudioSpec, 
        bit_depth: BitDepth
    ) -> Result<Self, Error>;
    
    pub fn write_block(&mut self, block: &AudioBlock) -> Result<(), Error>;
    pub fn finalize(&mut self) -> Result<(), Error>;
    /// Header written after last block processed
}

pub enum BitDepth {
    Bits16,
    Bits24,
    Bits32Float,
}
```

Applies TPDF dither when reducing from 32f → 16/24-bit. Auto-disables dither for 32f output.

---

## FFT Toolkit

```rust
pub enum WindowType {
    Hann, Hamming, Blackman, Rectangular,
}

pub struct Window;

impl Window {
    pub fn apply(window_type: WindowType, data: &mut [f32]);
    /// Applies window function in-place. Allocates internal buffer.
    
    pub fn fft_size(window_type: WindowType, frames: usize) -> usize;
    /// Returns optimal FFT size for given frame count
}
```

Four standard window functions for spectral analysis. Not STFT-ready — windows use symmetric N-1 denominator, not COLA-compatible periodic N. No overlap-add support.

---

## Error Types

```rust
pub enum Error {
    Io(std::io::Error),           // File operation failed
    InvalidSpec(AudioSpec),        // Invalid sample rate/channel count
    BufferTooSmall(usize, usize),  // Required > provided frames
    FilterError(String),           // Unstable coefficients / bad params
    DspError(String),              // General DSP failure
}

impl std::fmt::Display for Error { ... }
impl std::error::Error for Error { ... }
```

All methods return `Result<T, Error>` for fallible operations.

---

## Complete Example

See [`examples/master_chain.rs`](../crates/rescale-cli/examples/master_chain.rs) in repository for full working example.

Basic pattern:

```rust
use rescale_core::*;

fn master_file(input: &Path, output: &Path) -> Result<()> {
    // 1. Open reader
    let mut reader = WavStreamReader::new(input)?;
    let spec = *reader.spec();
    
    // 2. Build chain
    let mut chain = ProcessorChain::new();
    chain.push(GainProcessor::new(-3.0));
    chain.push(Eq3BandProcessor::new(default_eq_config()));
    chain.push(CompressorProcessor::new(compressor_config()));
    chain.push(LimiterProcessor::new(-1.0, 2.0, 0.1));
    chain.prepare(spec)?;
    
    // 3. Open writer
    let mut writer = WavStreamWriter::new(output, spec, BitDepth::Bits24)?;
    
    // 4. Process blocks
    while !reader.finished() {
        let mut block = AudioBlock::new(spec.num_channels, 1024);
        if reader.read_block(&mut block)? {
            chain.process(&mut block)?;
            writer.write_block(&block)?;
        }
    }
    
    // 5. Flush lookahead tail
    let mut tail = AudioBlock::new(spec.num_channels, 1024);
    chain.flush(&mut tail)?;
    
    writer.finalize()?;
    Ok(())
}
```

---

## See Also

- [Quickstart](quickstart.md) — Build & install
- [Architecture](architecture.md) — Design principles & component details
- [Known Limitations](limitations.md) — Current issues
