# REScale Engine Roadmap

> Validation status: cargo test 21/21 pass, cargo clippy 0 warnings, cargo fmt clean. Last validated: September 2026.

## Completed (Stages 1–10 Complete)

### Stages 1–6: Fundamental DSP Modules
- [x] SampleConvert trait (i16/i32/f32), Gain (dB↔linear), Peak/RMS Normalizer.
- [x] RIFF WAV I/O: Reader/Writer PCM 16/24/32f, stereo de-interleave/interleave.
- [x] Buffers: RingBuffer (generic, zero-alloc), DelayLine (fractional interpolation), FeedbackDelay.
- [x] Filters: Biquad TDF-II, RBJ coefficients (7 types), 3-Band Parametric EQ.
- [x] DRC: EnvelopeFollower (Peak/RMS), Compressor (soft-knee), BrickwallLimiter (lookahead).
- [x] FFT: Complex f32, Windowing (4 types), Cooley-Tukey Radix-2 FFT/IFFT, dominant frequency.

- [x] Added AudioSpec, AudioBlock, and the AudioProcessor trait (with the prepare, process, reset, latency_samples, and flush contract).
- [x] Added ProcessorChain with composable effect-chain processing & multi-processor latency drainage.
- [x] Per-channel state isolation for CompressorProcessor, LimiterProcessor, EqualizerProcessor, and GainProcessor.
- [x] Created WavStreamReader that emits planar blocks of flexible size (AudioBlock).
- [x] Created WavStreamWriter with a header that is finalized after all frames are written.
- [x] Implemented flush on AudioProcessor so the lookahead limiter tail is not truncated.
- [x] Comprehensive unit tests: streaming roundtrip, limiter flush tail, channel mismatch handling, and error validation.

## Stages 7–9: Engine, Workspace & Pristine Mode (Complete)

- [x] AudioSpec, AudioBlock, `AudioProcessor` trait (prepare, process, reset, latency_samples, flush).
- [x] Composable ProcessorChain with multi-processor latency drainage.
- [x] Per-channel state isolation: CompressorProcessor, LimiterProcessor, EqualizerProcessor, GainProcessor.
- [x] Block-based WavStreamReader/Writer (zero-allocation, streaming seekable) — library API only; `rescale-cli` still reads a whole file into one block.
- [x] Lookahead limiter flush tail.
- [x] Multi-crate Cargo workspace: `rescale-core` + `rescale-cli`.
- [x] Bug fix: `SampleConvert for i32`, clippy lint.
- [x] Pristine MP3→WAV mode: `--no-dsp` flag, pcm_f32le decode, default 24-bit + TPDF dither.
- [x] Performance: 3.4 min of stereo < 0.2 s (release build).

## Stage 10: TUI Redesign — DAW Plugin Dark Theme (Complete)

- [x] ANSI 256-color palette + centralized visual character constants.
- [x] `draw_panel` (metal border + title bar), `draw_vu_meter` (LED gradient + peak hold + arrow).
- [x] `draw_smooth_spectrum` (edge antialiasing), `draw_waveform`, `draw_rolling_meter`.
- [x] `draw_stereo_pan`, `draw_h_slider`, `draw_processing_status`, `draw_header`.
- [x] Multi-panel monitor dashboard layout (INPUT / OUTPUT / SPECTRUM), 300 frames.
- [x] Backward-compatibility wrapper for the old API.
- [x] Runtime smoke test without panics/errors.

## Quality Gate (Complete)

- [x] `cargo build --all-targets` — 0 errors.
- [x] `cargo clippy --all-targets -- -D warnings` — 0 issues.
- [x] `cargo test` — 21/21 pass.
- [x] `cargo fmt --check` — clean.
- [x] Unit tests for the AudioBlock contract, ProcessorChain, channel mismatch, and error handling.
- [x] Streaming block roundtrip tests (16, 24, 32f).
- [x] Lookahead limiter flush-tail test.
- [x] TUI runtime smoke test (300 frames rendered, 0 panics).

## Next Development Roadmap

### Mastering Quality
- [ ] Stereo-linked detector and mid/side mode.
- [ ] 2x/4x oversampling and true-peak limiter.
- [ ] LUFS integrated, short-term, and loudness range measurement (EBU R128).
- [ ] Preset delivery and serializable master WAV.

### Formats & Distribution
- [ ] Separate `rescale-formats` crate for optional codecs.
- [ ] Optional FFmpeg backend for MP3, FLAC, and Opus.
- [ ] Native WAV stays free of external Rust dependencies.

### Performance & Ecosystem
- [ ] SIMD vectorization (AVX-512 / NEON) for FFT and buffers.
- [ ] Real-time benchmarks on 64, 256, and 1024-frame blocks.
- [ ] Native MP3 bitstream decoder (Huffman + MDCT).
- [ ] FLAC / OGG format support (lossless entropy decoder from scratch).
