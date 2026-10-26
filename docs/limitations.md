# Known Limitations

Honest status of current issues, workarounds, and planned fixes. This document supersedes all other documentation when in conflict.

---

## Critical Issues

### 1. Limiter Hard Clamp ⚠️ HIGH

**Status:** Active — on roadmap  
**Severity:** High (audio distortion)

Limiter protects ceiling with final hard clamp rather than proper sliding max-hold gain reduction. Gain reduction is derived from exponential envelope (0.1 ms attack) instead of true sliding maximum over lookahead window.

**Symptoms:** Sustained material limited cleanly; percussive transients attenuated too little then clipped.

**Workaround:** Use lower target dBFS values (-2 or -3 instead of -0.5). Avoid mastering heavily compressed sources.

**Fix path:** Implement sliding max-hold algorithm tracking maximum gain reduction across entire lookahead window.

---

### 2. No Latency Compensation ⚠️ HIGH

**Status:** Active — on roadmap  
**Severity:** High (timing shift)

CLI processes whole file as single block; `flush()` never called. Output time-shifted by limiter lookahead (~2 ms default) with lookahead tail truncated. Streaming reader/writer path not wired into CLI yet.

**Symptoms:** Output misaligned with input by ~2ms; transient onset slightly delayed.

**Workaround:** Manual sample-level alignment using DAW transport sync markers. Future: CLI will compensate automatically via streaming I/O path.

**Fix path:** Route CLI through `WavStreamReader`/`WavStreamWriter` loop, call `flush()` at EOF, offset output frames by `chain.latency_samples()`.

---

### 3. ProcessorChain::flush Tail Drain Bug 🐛 BUG

**Status:** Bug confirmed  
**Severity:** Medium (data loss)

When processors without latency precede a latency-introducing processor, `flush()` drains tail incorrectly — samples lost from lookahead buffer.

**Reproduction:** `GainProcessor → Compressor → Limiter` chain produces truncated tail vs `Compressor → Limiter` alone.

**Fix path:** Correct internal delay compensation calculation in drain sequence.

---

## Moderate Issues

### 4. DelayLine Off-by-One Sample

**Status:** Bug confirmed  
**Severity:** Low-Medium (precision error)

Delay line returns one sample short of nominal delay: `read_delay(n)` yields sample from position `n-1` instead of `n`. Reported latency differs from actual physical delay by one sample.

**Impact:** Subtle phase alignment errors in EQ filters and all-pass sections. Usually inaudible but breaks mathematical precision guarantees.

**Fix path:** Adjust read index by +1 in delay line implementation.

---

### 5. No Stereo Linking

**Status:** Design limitation — on roadmap  
**Severity:** Medium (panning shift)

Compressor and limiter maintain independent per-channel state. Heavy limiting compresses one channel more than the other, shifting stereo image toward less-compressed side.

**Symptoms:** Source panned center appears to drift during heavy limiting on asymmetric material (e.g., mono vocals with stereo pad).

**Fix path:** Implement linked detection where gain reduction for both channels derives from whichever has higher instantaneous compression ratio.

---

### 6. RMS Envelope Release Too Slow

**Status:** Implementation artifact  
**Severity:** Low-Medium (dynamic response)

RMS envelope release time runs approximately 2× slower than configured value because attack/release smoothing operates in power domain rather than linear domain.

**Example:** Configured `release_ms = 80` ms → actual ~160 ms.

**Fix path:** Convert RMS computation to linear amplitude domain before applying exponential smoothing.

---

### 7. Unvalidated Filter Parameters

**Status:** Security/safety gap  
**Severity:** Medium (potential instability)

No guards against denormal numbers, NaN, or Inf in audio path. Filter parameters like `f0 >= fs/2` or `q <= 0` produce unstable coefficients — oscillator behavior, runaway amplification.

**Symptoms:** Output contains DC offset, clipping artifacts, or silent periods interspersed with noise bursts.

**Workaround:** Validate inputs before constructing processors. Never feed untrusted parameter values directly from user input.

**Fix path:** Add parameter validation clamps in constructor functions; denormal flusher on DSP buffers.

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

### 12. CLI Has No Automated Tests

Only library components have tests. CLI binary relies on manual verification. Edge cases in argument parsing, file handling, and error reporting go undetected until integration testing.

**Planned fix:** Add CLI-specific test harness in CI that exercises all command combinations.

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
| Critical (blocks production use) | 2 | Next minor release |
| Bug (confirmed, needs fix) | 2 | Backlog |
| Design limitation (on roadmap) | 3 | Phased rollout |
| Minor / cosmetic | 5 | Best-effort |

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
