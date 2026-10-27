# Known Limitations

Honest status of current issues, workarounds, and planned fixes. This document supersedes all other documentation when in conflict.

---

## Critical Issues

### 1. Limiter Hard Clamp ⚠️ HIGH — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)  
**Severity:** High (audio distortion)

~~Limiter protects ceiling with final hard clamp rather than proper sliding max-hold gain reduction. Gain reduction is derived from exponential envelope (0.1 ms attack) instead of true sliding maximum over lookahead window.~~

The limiter now derives gain reduction from the **sliding maximum absolute peak over the full lookahead window** (monotonic-deque `SlidingMax`, zero allocation). Attack is instantaneous so a new peak in the window is never allowed through, release is exponentially smoothed with a min-hold, and the hard clamp remains only as a floating-point safety net — it is no longer part of the gain law.

**Symptoms:** ~~Sustained material limited cleanly; percussive transients attenuated too little then clipped.~~ Resolved — the ceiling holds on percussive material (regression test).

**Workaround:** ~~Use lower target dBFS values (-2 or -3 instead of -0.5). Avoid mastering heavily compressed sources.~~ No longer needed.

**Fix path:** ~~Implement sliding max-hold algorithm tracking maximum gain reduction across entire lookahead window.~~ Done.

---

### 2. No Latency Compensation ⚠️ HIGH — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)  
**Severity:** High (timing shift)

~~CLI processes whole file as single block; `flush()` never called. Output time-shifted by limiter lookahead (~2 ms default) with lookahead tail truncated. Streaming reader/writer path not wired into CLI yet.~~

The CLI now streams through `WavStreamReader` → processor chain → `WavStreamWriter`: it calls `flush()` at EOF and discards the first `latency_samples` output frames, so the output frame count equals the input and the master is no longer time-shifted. Covered by an integration test that compares frame counts and measures latency compensation end-to-end.

**Symptoms:** ~~Output misaligned with input by ~2ms; transient onset slightly delayed.~~ Resolved.

**Workaround:** ~~Manual sample-level alignment using DAW transport sync markers. Future: CLI will compensate automatically via streaming I/O path.~~ No longer needed.

**Fix path:** ~~Route CLI through `WavStreamReader`/`WavStreamWriter` loop, call `flush()` at EOF, offset output frames by `chain.latency_samples()`.~~ Done.

---

### 3. ProcessorChain::flush Tail Drain Bug 🐛 BUG — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)  
**Severity:** Medium (data loss)

When processors without latency precede a latency-introducing processor, `flush()` drains tail incorrectly — samples lost from lookahead buffer.

**Reproduction:** `GainProcessor → Compressor → Limiter` chain produces truncated tail vs `Compressor → Limiter` alone.

**Fix path:** ~~Correct internal delay compensation calculation in drain sequence.~~ Done — `flush()` now feeds silence through the whole chain, draining the sum of every stage's latency regardless of processor order.

---

## Moderate Issues

### 4. DelayLine Off-by-One Sample — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)  
**Severity:** Low-Medium (precision error)

Delay line returns one sample short of nominal delay: `read_delay(n)` yields sample from position `n-1` instead of `n`. Reported latency differs from actual physical delay by one sample.

**Impact:** Subtle phase alignment errors in EQ filters and all-pass sections. Usually inaudible but breaks mathematical precision guarantees.

**Fix path:** ~~Adjust read index by +1 in delay line implementation.~~ Done — `read_delay(n)` is now relative to the last pushed sample; pre-push readers use `read_delay_before_push`. Limiter latency now equals its physical delay.

---

### 5. No Stereo Linking — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)  
**Severity:** Medium (panning shift)

~~Compressor and limiter maintain independent per-channel state. Heavy limiting compresses one channel more than the other, shifting stereo image toward less-compressed side.~~

The limiter now uses a single shared `LookaheadDetector` driven by the loudest channel peak, and applies the same gain reduction to every channel (stereo-linked limiting), so the stereo image no longer drifts. The compressor still keeps per-channel state; linked compressor detection and mid/side processing remain on the roadmap.

**Symptoms:** ~~Source panned center appears to drift during heavy limiting on asymmetric material.~~ Resolved for the limiter.

**Fix path:** ~~Implement linked detection where gain reduction for both channels derives from whichever has higher instantaneous compression ratio.~~ Done for the limiter.

---

### 6. RMS Envelope Release Too Slow — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)  
**Severity:** Low-Medium (dynamic response)

RMS envelope release time runs approximately 2× slower than configured value because attack/release smoothing operates in power domain rather than linear domain.

**Example:** Configured `release_ms = 80` ms → actual ~160 ms.

**Fix path:** ~~Convert RMS computation to linear amplitude domain before applying exponential smoothing.~~ Done — RMS mode applies the squared amplitude coefficient (`r²`) in the power domain, so the release matches the configured time constant.

---

### 7. Unvalidated Filter Parameters — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)  
**Severity:** Medium (potential instability)

~~No guards against denormal numbers, NaN, or Inf in audio path. Filter parameters like `f0 >= fs/2` or `q <= 0` produce unstable coefficients — oscillator behavior, runaway amplification.~~

Filter constructors now clamp frequency into `(0, Nyquist)`, force `q > 0`, clamp EQ gain to ±40 dB and fall back to 48 kHz for invalid sample rates. The engine additionally sanitises its output (`safety::sanitize_block`): NaN/Inf become silence and denormals are flushed to zero.

**Symptoms:** ~~Output contains DC offset, clipping artifacts, or silent periods interspersed with noise bursts.~~ Prevented at construction and at the engine boundary.

**Workaround:** ~~Validate inputs before constructing processors.~~ No longer required, but still recommended for custom coefficients.

**Fix path:** ~~Add parameter validation clamps in constructor functions; denormal flusher on DSP buffers.~~ Done.

---

## Minor Issues

### 8. No Sample-Rate Conversion

Despite the name "REscale," engine currently processes at input sample rate only. No resampling capability exists.

---

### 9. No Loudness Metering

Peak and RMS only — no true-peak (inter-sample peak) measurement, no EBU R128 LUFS loudness normalization. Modern broadcast/streaming platforms require integrated loudness matching.

---

### 10. FFT Windowing Not STFT-Ready

Windows use symmetric `N-1` denominator instead of COLA-compatible periodic `N`. Coherent gain not compensated. `Window::apply` allocates a new `Vec` per invocation.

**Impact:** Spectral analysis accuracy degraded; cannot build overlap-add reconstruction pipelines. Memory allocation overhead per frame.

---

### 11. TUI Inside rescale-core

Terminal UI code lives inside the DSP crate, forcing core dependency on `std` threads and terminal output. Violates separation-of-concerns principle.

**Planned fix:** Extract to separate `rescale-tui` crate depending only on `rescale-core`.

---

### 12. CLI Has No Automated Tests — ✅ FIXED (unreleased)

**Status:** ✅ Fixed (unreleased)

~~Only library components have tests. CLI binary relies on manual verification. Edge cases in argument parsing, file handling, and error reporting go undetected until integration testing.~~

The CLI now has an integration suite (`crates/rescale-cli/tests/cli.rs`, 9 tests) that runs the real binary and covers mastering latency compensation, pristine sample-accuracy, bit-depth handling, missing/unsupported input, out-of-range targets, unknown arguments, overwrite refusal and `--help`.

**Planned fix:** ~~Add CLI-specific test harness in CI that exercises all command combinations.~~ Done.

---

## Documentation Gaps

| Area | Issue | Priority |
|------|-------|----------|
| Examples directory | Minimal working examples missing for common use cases | Medium |
| Error messages | User-facing errors sometimes expose internal state | Low |
| Parameter ranges | Some filter/EQ configs lack documented valid ranges in source | Medium |
| Performance benchmarks | No automated performance regression tracking | Medium |

---

## Workload Assessment

| Severity | Count | Fix ETA |
|----------|-------|---------|
| Critical (blocks production use) | 0 | — (fixed in unreleased) |
| Bug (confirmed, needs fix) | 0 | — (fixed in unreleased) |
| Design limitation (on roadmap) | 2 | Phased rollout |
| Minor / cosmetic | 4 | Best-effort |

---

## Reporting Bugs

Provide minimal reproducible report:
1. **Input format** — sample rate, bit depth, channels, file size
2. **Exact command** — full CLI invocation with all flags
3. **Expected result** — what should happen
4. **Observed result** — what actually happens (error message, waveform screenshot, spectrogram diff)

Bug reports with this structure get fastest resolution. Solo project — limited triage capacity.

---

## See Also

- [Roadmap](roadmap.md) — Planned fixes & feature additions
- [Architecture](architecture.md) — How things work now
