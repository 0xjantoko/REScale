# Roadmap

Planned improvements, feature additions, and architectural evolution for REScale. Priority ordered by impact vs effort.

---

## Phase 1: Core Stability (Next Release)

### Fix Limiter Algorithm
- [ ] Replace hard clamp with sliding max-hold gain reduction
- [ ] True maximum detection across lookahead window
- [ ] Eliminate transient clipping on percussive material
- **Impact:** Critical — removes primary audio quality bottleneck
- **Effort:** High (requires algorithmic redesign + testing)

### Stereo Linking
- [ ] Compressor: linked detection (gain reduction follows louder channel)
- [ ] Limiter: independent per-channel processing with linked ceiling protection
- [ ] Mid/side processing support for stereo-aware mastering
- **Impact:** Medium — fixes panning drift during heavy compression
- **Effort:** Medium

### Streaming I/O Integration
- [ ] Route CLI through `WavStreamReader`/`WavStreamWriter` block loop
- [ ] Implement `flush()` call at end-of-stream
- [ ] Compensate output frame offset by `chain.latency_samples()`
- **Impact:** High — eliminates ~2ms time shift, restores lookahead tail
- **Effort:** Medium

### ProcessorChain::flush Bug Fix
- [ ] Correct drain sequence when non-latency processors precede latency ones
- [ ] Regression tests at 64, 256, 1024-frame block sizes
- **Impact:** Medium — prevents sample loss in common chain configurations
- **Effort:** Low-Medium

---

## Phase 2: Quality of Life (Following Release)

### Sample-Rate Conversion
- [ ] Multi-stage polyphase filterbank resampler
- [ ] Support target rates: 44.1kHz, 48kHz, 88.2kHz, 96kHz, 192kHz
- [ ] Configurable oversampling ratio (up to 8×)
- **Impact:** High — finally delivers "REscale" name promise
- **Effort:** High (mathematical complexity + validation)

### LUFS Loudness Metering
- [ ] Integrated loudness (EBU R128)
- [ ] Short-term loudness measurement
- [ ] Loudness range (LRA)
- [ ] ReplayGain-style normalization preset
- **Impact:** Medium — required for broadcast/streaming compatibility
- **Effort:** Medium (standard algorithm implementation)

### True-Peak Limiting
- [ ] 2× oversampling mode for inter-sample peak detection
- [ ] 4× oversampling mode for strict ITU-R BS-1770 compliance
- [ ] Optional downsample post-limiting
- **Impact:** Low-Medium — prevents streaming platform normalization artifacts
- **Effort:** Medium (requires integration with existing limiter)

### Parameter Validation & Safety
- [ ] Denormal flusher on DSP buffers
- [ ] NaN / Inf guards throughout audio path
- [ ] Filter parameter clamping (f0 < fs/2, q > 0)
- [ ] Configurable panic/handle mode for invalid inputs
- **Impact:** Medium — prevents instability from untrusted parameters
- **Effort:** Low-Medium

---

## Phase 3: Performance & Format Expansion

### SIMD Vectorization
- [ ] AVX2 paths via `std::arch` intrinsics (x86_64)
- [ ] NEON paths via `std::arch` intrinsics (ARM64)
- [ ] Fallback to scalar for unsupported architectures
- [ ] Benchmarks tracking perf regression/addition
- **Impact:** High — 2-4× speedup on CPU-heavy operations
- **Effort:** High (code duplication + test matrix expansion)

### Optional Format Crate
- [ ] `rescale-formats` crate for extra codecs
- [ ] FLAC decode (lossless archival source)
- [ ] OGG/Vorbis decode
- [ ] Native MP3 bitstream decoder (no ffmpeg dependency)
- [ ] All optional features gated behind Cargo feature flags
- **Impact:** Medium — expands source format compatibility
- **Effort:** Medium-High (each codec = significant implementation)

### TUI Extraction
- [ ] Move terminal UI code to separate `rescale-tui` crate
- [ ] `rescale-core` becomes fully headless (no std threading dep)
- [ ] Clean separation of DSP logic from presentation layer
- **Impact:** Low-Medium — cleaner architecture, better testability
- **Effort:** Low

---

## Long-Term Vision

### Real-Time Engine
- [ ] ASIO/CoreAudio/WASAPI backend for live monitoring
- [ ] Plugin wrapper (VST3, AU) — future, requires major refactor
- [ ] Networked multi-client low-latency audio routing

### Advanced Processing
- [ ] De-essing module (frequency-split single-band compressor)
- [ ] Multiband compression (crossover + multiple bands)
- [ ] Dynamic EQ (EQ curves that respond to frequency-specific dynamics)
- [ ] Harmonic excitator / saturation modules
- [ ] Stereo imaging tools (M/S width control, phase correlation)

### Tooling
- [ ] CI performance benchmarks (automated regression detection)
- [ ] WASM port for browser-based preview tool
- [ ] VS Code extension for quick mastering presets
- [ ] Web dashboard for remote batch processing

---

## Contributing

This is a solo project — pull requests are not accepted except for Dependabot CI version bumps. However:

- **Issue reports** welcome (see [limitations.md](limitations.md) for reporting format)
- **Feature ideas** welcome as GitHub issues with motivation and use case
- **Documentation feedback** welcome as PRs to `docs/` directory only

The fastest path to a fix is a minimal, reproducible bug report with input file, exact command, expected vs observed result.

---

## Maintenance Notes

Source comments in Indonesian by convention; user-facing content in English. All roadmap items tracked as GitHub Issues with milestone labels. Priorities may shift based on discovered blocker severity or user adoption patterns.

Last updated: v0.1.0
