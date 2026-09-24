# REScale

**A zero-dependency audio DSP engine and mastering CLI written in pure Rust.**

[![CI](https://github.com/0xjantoko/REScale/actions/workflows/ci.yml/badge.svg)](https://github.com/0xjantoko/REScale/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org)
[![MSRV](https://img.shields.io/badge/MSRV-1.78-blue)](#quality-gates)
[![docs.rs](https://img.shields.io/docsrs/rescale-core)](https://docs.rs/rescale-core)
[![Dependencies](https://img.shields.io/badge/crates.io%20dependencies-0-success)](#zero-dependency-policy)

REScale is a block-based DSP library with a composable processor chain (gain,
3-band parametric EQ, compressor, lookahead brickwall limiter), streaming WAV
I/O, an FFT toolkit, and a terminal UI monitor — all implemented from scratch on
top of the Rust standard library, with **no third-party crates**.

> **Status: v0.1.0 — early, API unstable.** This is the first public release of a
> solo project. The DSP core is tested and usable, but the public API will change.
> Read [Known limitations](#known-limitations) before relying on the limiter or on
> latency-sensitive workflows.

## Highlights

- **Zero dependencies** — `rescale-core` and `rescale-cli` pull in no external
  crates at all. Verified by `cargo tree` in CI on every push.
- **`rescale-core` is pure DSP** — WAV codec, filters, dynamics, FFT, buffers,
  and the block engine, with no I/O policy decisions baked in.
- **`unsafe`-free core** — `rescale-core` contains no `unsafe` code (enforced in
  CI). The workspace has exactly one `unsafe` block: a Windows-only `kernel32`
  FFI call in the CLI that enables UTF-8 code page and ANSI escape sequences.
- **Planar block model** — `AudioBlock` gives processors mutable per-channel
  views; `ProcessorChain` composes any number of `AudioProcessor`s, reports the
  summed latency, and exposes a `flush` hook for lookahead tails.
- **Streaming WAV I/O** — `WavStreamReader` / `WavStreamWriter` process files
  block by block instead of loading them into RAM, with a header finalized at the
  end of the stream.
- **Formats** — WAV PCM 16/24/32-bit float read + write, TPDF dither, 4 window
  functions, RBJ biquad coefficients (LP/HP/BP/notch/peaking/low-shelf/high-shelf).
- **Reference speed** — the maintainer's release-build measurement of the full
  mastering chain plus WAV I/O on 3.4 minutes of 44.1 kHz stereo audio is under
  0.2 s. This is not a CI benchmark; re-measure on your own machine.

## Terminal UI monitor

`rescale_cli --menu` opens an interactive menu, and
`cargo run --example tui_monitor` renders the master dashboard below — VU meters
with peak hold, a 9-band FFT spectrum, and the processing chain status. It is
drawn with raw ANSI escapes and no TUI framework:

```text
┌─  RESCALE DSP MONITOR ──────────────────────────────────────────────────────────────────────────┐
│                                                                                                │
│  ENGINE: offline synthetic monitor  FORMAT: 44.1 kHz / stereo / f32   FRAME: 180/180           │
│                                                                                                │
│  INPUT ANALYSIS                                                                                │
│    L [███████████████████████████░░░░░]  ▲   -1.4 dBFS  RMS  -8.1 dBFS                         │
│    R [███████████████████████░░░░░░░░░]  ▲   -2.8 dBFS  RMS  -9.6 dBFS                         │
│                                                                                                │
│  OUTPUT / MASTER BUS                                                                           │
│    L [██████████████░░░░░░░░░░░░░░░░░░]  ▲   -7.0 dBFS  RMS -13.7 dBFS                         │
│    R [██████████████░░░░░░░░░░░░░░░░░░]  ▲   -7.3 dBFS  RMS -14.1 dBFS                         │
│                                                                                                │
│  SPECTRUM ANALYZER  ·  POST-LIMITER  ·  FFT 512 / HANN                                         │
│                                                                                                │
│    ··█······                                                                                   │
│    ··█······                                                                                   │
│    ··█······                                                                                   │
│    ··█······                                                                                   │
│    ··█······                                                                                   │
│    ··█▀·▀···                                                                                   │
│    ··██·█···                                                                                   │
│     63   125  250  500  1k   2k   4k   8k   16k                                                │
│                                                                                                │
│  PROCESSING STATUS                                                                             │
│    CHAIN: EQ 3-band > compressor > sample-peak limiter                                         │
│    LIMITER: -0.2 dBFS ceiling / 88 sample lookahead                                            │
│    DELTA RMS: L -5.5 dB / R -4.5 dB                                                            │
│    DOMINANT:  430.7 Hz                                                                         │
│    FRAME TIME:  0.70 ms                                                                        │
│    METER MODE: sample peak + RMS; not LUFS or true-peak                                        │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

The dashboard is a visualisation demo driven by a synthetic sweep, not an audio
device host.

## Quick start

```bash
git clone https://github.com/0xjantoko/REScale
cd REScale
cargo build --release
cargo test --workspace
```

The build is fully offline — there is nothing to download:

```bash
cargo build --workspace --locked --offline
```

### CLI

```bash
# Interactive TUI menu
rescale_cli --menu

# Master a file to -1 dBFS (defaults: 24-bit PCM, TPDF dither)
rescale_cli input.wav -1

# Custom target, bit depth and dither
rescale_cli input.wav -14 --bit-depth 24 --dither tpdf

# Pristine passthrough (no normalize/compress/limit)
rescale_cli input.wav --no-dsp
```

| Flag | Values | Default |
|---|---|---|
| `[target_dbfs]` (positional) | `-60.0` … `0.0` | `-1.0` |
| `--bit-depth` | `16` \| `24` \| `32f` | `24` |
| `--dither` | `none` \| `tpdf` | `tpdf` (auto-disabled for `32f`) |
| `--no-dsp` / `--pristine` | — | off |
| `--menu` / `-m` / `-i` / `--interactive` | — | — |
| `--help` / `-h` | — | — |

Output is written next to the input as `REScale-<name>.wav`. An existing output
file is never overwritten silently.

**MP3 input/output requires `ffmpeg` on your `PATH`.** This is an external
*binary*, not a Rust dependency: the crate graph stays empty. Moving MP3 support
behind an optional Cargo feature is planned.

### Library

```rust
use rescale_core::{
    AudioBlock, AudioProcessor, AudioSpec, CompressorConfig, CompressorProcessor,
    DetectionMode, GainProcessor, LimiterProcessor, ProcessorChain,
};

let spec = AudioSpec::new(44_100.0, 2)?;

let mut chain = ProcessorChain::new();
chain.push(GainProcessor::new(-3.0));
chain.push(CompressorProcessor::new(CompressorConfig {
    threshold_db: -10.0,
    ratio: 3.0,
    knee_db: 4.0,
    attack_ms: 5.0,
    release_ms: 80.0,
    makeup_gain_db: 2.0,
    mode: DetectionMode::Rms,
}));
chain.push(LimiterProcessor::new(-1.0, 25.0, 2.0));
chain.prepare(spec)?;

// Push planar blocks of any size through the chain...
// chain.process(&mut block)?;

// ...then drain the lookahead tail at end of stream, and compensate
// `chain.latency_samples()` in your output.
// let flushed = chain.flush(&mut block)?;
```

See [`docs/usage-guide.md`](docs/usage-guide.md) for end-to-end workflows and
[`docs/implementation-guide.md`](docs/implementation-guide.md) for engine internals.

## Known limitations

Honest, current state — most of these are on the
[roadmap](#roadmap) or are being fixed before wider use:

- **The limiter protects the ceiling with a final hard clamp.** Gain reduction is
  derived from an exponential envelope (0.1 ms attack) rather than a true
  sliding maximum over the lookahead window, so isolated transients can be
  attenuated too little and then clipped. Sustained material is limited cleanly;
  percussive material can distort.
- **The CLI does not compensate lookahead latency.** The whole file is processed
  as a single block and `flush()` is never called, so output is time-shifted by
  the limiter lookahead (~2 ms by default) and the tail is truncated. The
  streaming reader/writer path also is not wired into the CLI yet.
- **`ProcessorChain::flush` drains the tail incorrectly** when processors without
  latency precede a latency-introducing processor.
- **The `DelayLine` is one sample short** of its nominal delay
  (`read_delay(n)` returns the sample from `n - 1`), so reported latency and the
  actual delay differ by one sample.
- **No stereo linking.** Compressor and limiter keep independent per-channel
  state, so heavy limiting can shift the stereo image.
- **No resampling.** Despite the name, the engine currently processes at the
  input sample rate; there is no sample-rate converter yet.
- **No loudness metering.** Peak and RMS only — no true-peak (inter-sample) or
  EBU R128 LUFS measurement.
- **RMS envelope release is ~2× slower than configured**, because attack/release
  smoothing runs in the power domain.
- **FFT windowing is not STFT-ready** — windows use the symmetric `N - 1`
  denominator (not COLA-compatible periodic `N`) and coherent gain is not
  compensated. `Window::apply` also allocates a `Vec` per call.
- **The terminal UI lives inside `rescale-core`** instead of a separate crate, so
  the DSP core currently depends on `std` threads and terminal output.
- **No denormal or NaN/Inf guards** on the audio path; unvalidated filter
  parameters (e.g. `f0 >= fs/2`, `q <= 0`) can produce unstable coefficients.
- **`rescale-cli` has no automated tests** and needs `ffmpeg` for MP3.

## Zero-dependency policy

`crates.io` dependencies are treated as a design constraint, not an accident:

- `cargo tree --workspace -e normal,build,dev` must list only `rescale-core` and
  `rescale-cli`. The CI job **Zero dependency** fails the build otherwise.
- `[dev-dependencies]` and `[build-dependencies]` must stay empty — no test or
  benchmark crates.
- `cargo build --workspace --locked --offline` must succeed.
- `rescale-core` must contain no `unsafe`; the only allowed `unsafe` is the
  Windows console FFI in the CLI.
- Optional, non-WAV features (extra codecs, native MP3, real-time device I/O)
  belong behind Cargo features or in separate opt-in crates — never in the
  default dependency graph.

## Quality gates

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
```

Current status: 21/21 tests pass, clippy clean with `-D warnings`, `fmt` clean,
build 0 warnings. Tests cover streaming WAV round-trips (16/24/32f), the limiter
flush tail, `ProcessorChain` contracts, channel-mismatch and error handling,
filter/EQ/FFT/DRC behaviour, and a full mastering-chain pipeline.

**MSRV: Rust 1.78**, verified by a dedicated CI job (`cargo check --locked`
against `1.78.0`); the committed `Cargo.lock` uses lockfile version 4.

## Documentation

| Document | Contents |
|---|---|
| [Usage guide](docs/usage-guide.md) | CLI workflows, mastering recipes |
| [MP3 rescale guide](docs/mp3-rescale-guide.md) | Pristine MP3 → WAV pipeline |
| [TUI guide](docs/tui-guide.md) | Terminal dashboard & monitor |
| [Implementation guide](docs/implementation-guide.md) | Engine internals, block model |
| [Architecture roadmap](docs/architecture-roadmap.md) | Crate layout & evolution plan |
| [Engine roadmap](docs/engine-roadmap.md) | Milestones & quality gates |
| [Math foundation](docs/math-foundation.md) | DSP math behind the modules |
| [Project analysis](docs/project-analysis.md) | Strengths, risks, positioning |
| [Changelog](CHANGELOG.md) | Release history and per-version changes |
| [Security policy](SECURITY.md) | How to report a vulnerability |

These documents were written alongside the implementation and still contain
optimistic wording that this README supersedes; the **Known limitations** section
above is the authoritative status.

## Repository layout

```
crates/
  rescale-core/   # DSP engine library (zero deps)
                  #   engine, buffer, filter, drc, fft, wav, gain, normalize,
                  #   convert, limiter, tui
  rescale-cli/    # mastering CLI binary + runnable examples
docs/             # English guides & roadmaps
```

## Roadmap

- Fix the limiter (sliding max-hold gain reduction + stereo-linked detection) and
  the latency/flush contract.
- Route the CLI through the streaming reader/writer and compensate latency.
- Add regression tests at 64, 256, and 1024-frame block sizes.
- Sample-rate conversion (the `REScale` in the name).
- True-peak limiting with 2×/4× oversampling; mid/side processing.
- LUFS measurement (EBU R128): integrated, short-term, loudness range.
- SIMD vectorization (AVX2/NEON) via `std::arch` intrinsics.
- Optional `rescale-formats` crate: FLAC/OGG, native MP3 bitstream decoder.

## Maintenance

This is a solo project. Pull requests are **not** accepted, but bug reports and
feature ideas are welcome as issues — the fastest path to a fix is a minimal,
reproducible report (input format, exact command, expected vs. observed result).
The one exception is automated Dependabot pull requests that bump the pinned
GitHub Actions versions; the maintainer reviews and merges those directly.
Security reports follow [SECURITY.md](SECURITY.md).

Source comments are written in Indonesian by convention; everything user-facing
(CLI and TUI strings, README, `docs/`) is English.

## License

Licensed under either of

- **MIT** license ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
