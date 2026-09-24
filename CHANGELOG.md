# Changelog

All notable changes to REScale are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Until `1.0.0`, minor releases may contain breaking changes to the public API.

## [Unreleased]

### Fixed
- `Fft::compute_magnitude_spectrum` and `Fft::find_dominant_frequency` no longer
  panic on an empty input buffer (slice out of range, division by zero); they
  return an empty spectrum and `(0.0, 0.0)` respectively.
- `# Panics` sections now document the power-of-two requirement of
  `Fft::forward`/`Fft::inverse` and the capacity invariants of `RingBuffer`.
- Rustdoc intra-doc link warnings in the biquad difference-equation formulas;
  CI additionally gates on `cargo doc` with `RUSTDOCFLAGS=-D warnings`.

### Planned
- Rewrite the brickwall limiter: sliding maximum-hold gain reduction over the
  lookahead window, min-hold gain smoothing, stereo-linked detection; the hard
  clamp becomes a last-resort safety net instead of part of the gain law.
- Route `rescale-cli` through `WavStreamReader`/`WavStreamWriter`, call
  `flush()`, and compensate `latency_samples()` so output is not time-shifted.
- Fix `ProcessorChain::flush` so the drained tail passes through every processor.
- Fix the `DelayLine` off-by-one (`read_delay(n)` currently returns the sample
  from `n - 1`) and make limiter latency a single source of truth.
- Move MP3 support behind an optional Cargo feature so the default build needs
  neither crates nor an external `ffmpeg` binary.
- Regression tests at 64, 256, and 1024 frames; first automated tests for
  `rescale-cli`.
- Validation and sanitisation: filter parameter checks, NaN/Inf guards, denormal
  handling.

## [0.1.0] - 2026-09-24

First public release. Early and API-unstable; see *Known limitations* in
[README.md](README.md).

### Added
- `rescale-core`: zero-dependency DSP engine — `AudioSpec`, planar `AudioBlock`,
  the `AudioProcessor` trait (prepare/process/reset/latency/flush), and a
  composable `ProcessorChain` with per-channel processor wrappers.
- DSP modules: gain and dB conversion, peak/RMS normalisation, sample format
  conversion, `RingBuffer`/`DelayLine`/`FeedbackDelay`, RBJ biquad coefficients
  with a 3-band parametric EQ, envelope follower with peak/RMS detection,
  soft-knee compressor, lookahead brickwall limiter, radix-2 FFT with four
  window functions and a dominant-frequency detector.
- WAV I/O: RIFF PCM 16/24-bit and 32-bit float reader/writer, TPDF dither, and a
  block-based `WavStreamReader`/`WavStreamWriter` that finalises the header at
  the end of the stream.
- `rescale-cli`: offline mastering (peak normalise → RMS compressor → brickwall
  limiter), `--no-dsp`/`--pristine` passthrough, selectable bit depth and dither,
  and a `ffmpeg`-based MP3 import/export path.
- ANSI terminal UI: master dashboard, VU meters with peak hold, smooth spectrum
  analyser, waveform and stereo-pan views (no TUI framework).
- 21 unit tests covering streaming round-trips, the limiter flush tail, chain
  contracts, error handling, filters, EQ, FFT, DRC, and a full mastering chain.
- CI quality gates on Linux and Windows: `fmt`, `clippy -D warnings`, build,
  tests, a locked offline build, and enforcement of the zero-dependency policy.

### Known limitations
Documented in detail in [README.md](README.md#known-limitations):
- Limiter ceiling is guaranteed by a final hard clamp; isolated transients can
  still distort.
- The CLI processes a whole file as one block: no lookahead latency compensation,
  no `flush()` call, and the streaming API is unused.
- No stereo linking, no sample-rate conversion, no true-peak or LUFS metering,
  RMS release ~2× slower than configured, FFT windows not STCA/COLA-calibrated,
  no denormal/NaN guards, and the terminal UI lives inside `rescale-core`.

[Unreleased]: https://github.com/0xjantoko/REScale/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/0xjantoko/REScale/releases/tag/v0.1.0
