# Quickstart Guide

Build, install, and run REScale from zero to mastered audio.

---

## Build

```bash
git clone https://github.com/0xjantoko/REScale
cd REScale

# Full release build + test suite
cargo build --release
cargo test --workspace

# Fully offline (no crates.io download)
cargo build --workspace --locked --offline
```

All three commands should succeed — CI enforces this on every push.

The release binary lands at `target/release/rescale_cli`. To call it as `rescale_cli` from anywhere, install it once:

```bash
cargo install --path crates/rescale-cli
```

---

## Basic Usage

```bash
# Interactive TUI menu
rescale_cli --menu

# Master a file to default -1 dBFS (24-bit PCM, TPDF dither)
rescale_cli input.wav -1

# Custom target, bit depth, dither mode
rescale_cli input.wav -14 --bit-depth 24 --dither tpdf

# Bypass all DSP — passthrough, untouched
rescale_cli input.wav --no-dsp
```

Output is written next to the input as `REScale-<name>.wav`. Existing output files are never overwritten silently.

---

## MP3 Support

**MP3 input/output requires `ffmpeg` on your `PATH`.** This is an external binary, not a Rust dependency — the crate graph stays empty. The pipeline converts MP3 → WAV for processing, then writes the master result.

Moving MP3 support behind an optional Cargo feature flag is planned.

---

## Library Integration

Add `rescale-core` to your dependencies:

```toml
[dependencies]
rescale-core = { path = "../crates/rescale-core" }
```

Basic chain setup:

```rust
use rescale_core::{
    AudioBlock, AudioProcessor, AudioSpec, ProcessorChain, GainProcessor, LimiterProcessor,
};

let spec = AudioSpec::new(44_100.0, 2)?;
let mut chain = ProcessorChain::new();
chain.push(GainProcessor::new(-3.0));
chain.push(LimiterProcessor::new(-1.0, 25.0, 2.0));
chain.prepare(spec)?;

// Push planar blocks through the chain...
// chain.process(&mut block)?;

// Drain lookahead tail at end of stream
// let flushed = chain.flush(&mut block)?;
```

See [Library API Reference](api.md) for full signature details.

---

## TUI Monitor

Visualize the engine with the synthetic sweep demo:

```bash
cargo run --example tui_monitor
```

Renders VU meters, peak hold, 9-band FFT spectrum, and chain status using ANSI escapes only — no TUI framework dependency. Driven by synthetic test data, not live audio.

---

## Next Steps

- **Full CLI options**: [CLI Reference](cli-reference.md)
- **Engine internals**: [Architecture](architecture.md)
- **Known issues**: [Limitations](limitations.md)
