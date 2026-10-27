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
- `ProcessorChain::flush` now drains the tail of every latency-bearing stage
  (chain tail = sum of stage latencies) for any processor order; previously only
  the first stage that reported flushed frames was drained, which truncated the
  lookahead tail when other processors were part of the chain.
- `DelayLine` off-by-one: `read_delay(n)` now returns the signal delayed by
  exactly `n` samples relative to the last pushed sample, so the limiter's
  reported latency matches its physical delay. Readers that fetch a delayed
  value before pushing (e.g. `FeedbackDelay`) use the new
  `read_delay_before_push`, which keeps their behaviour unchanged.
- RMS `EnvelopeFollower` time constants: smoothing now compensates for the power
  domain (`r²` for RMS mode), so release matches the configured value instead of
  running roughly 2× slower.
- CLI streaming: `rescale-cli` now reads and writes block by block through
  `WavStreamReader`/`WavStreamWriter`, calls `flush()` at end-of-stream and
  discards the first `latency_samples` output frames. The master therefore has
  the same frame count as the input and is no longer time-shifted; peak
  normalisation uses a read-only first pass so behaviour stays global.
- CLI parser: unrecognised long flags are reported as unknown arguments instead
  of being swallowed as the positional target dBFS.
- Limiter rewritten: gain reduction now comes from a sliding maximum-hold over
  the full lookahead window (`SlidingMax`, monotonic deque, zero allocation).
  Attack is instantaneous, release is smoothed with a min-hold, and the hard
  clamp is only a floating-point safety net — percussive transients no longer
  clip.
- Stereo-linked limiting: `LimiterProcessor` shares one `LookaheadDetector`
  driven by the loudest channel peak and applies the same gain reduction to
  every channel, so the stereo image no longer drifts under heavy limiting.
- Filter parameters are validated: frequency clamped into `(0, Nyquist)`, `q`
  forced positive, EQ gain clamped to ±40 dB, and invalid sample rates fall back
  to 48 kHz.
- Engine output is sanitised (`safety::sanitize_block`): NaN/Inf become silence
  and denormals are flushed to zero.

### Added
- Regression tests for block sizes 64, 256 and 1024 frames proving block-wise
  rendering equals single-block rendering (including the flushed tail), plus
  unit tests for delay-line semantics, envelope release timing and multi-stage
  chain flush.
- CLI integration suite (`crates/rescale-cli/tests/cli.rs`, 9 tests) running the
  real binary: latency-compensated mastering, pristine sample accuracy, bit
  depth, missing/unsupported input, out-of-range target, overwrite refusal,
  unknown argument and `--help`.
- Public exports: `LookaheadDetector`, `SlidingMax`, and safety helpers
  (`sanitize_sample`, `sanitize_buffer`, `sanitize_block`).
- Limiter tests: sliding-window maximum tracker (brute-force cross-check),
  detector gain law, percussive ceiling, look-ahead attack timing, stereo-linked
  gain, and parameter-validation tests. Test suite: 58 passing (49 core + 9 CLI).

### Planned
- Compressor: linked detection (gain reduction follows the louder channel) and
  mid/side processing support.
- True-peak limiting with 2×/4× oversampling (depends on the new limiter).
- Sample-rate conversion and LUFS (EBU R128) measurement.
- Move MP3 support behind an optional Cargo feature so the default build needs
  neither crates nor an external `ffmpeg` binary.

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
