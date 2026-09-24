# Roadmap of Additional Components for a Complete DSP Audio Engine (From Scratch)

> Implementation status as of September 2026: Stages 1–9 complete. All core DSP components, WAV I/O (batch & streaming), the engine block, the TUI, and the pristine MP3→WAV mode are implemented. This document is a historical architecture map; the next development roadmap is in [engine-roadmap.md](engine-roadmap.md).

## Current Status (Stages 1–9 Complete)

- ✅ Done: WAV PCM 16/24/float32, compressor, sample-peak lookahead limiter, engine block API, streaming WAV (reader/writer), limiter flush tail, 3-band parametric EQ, FFT/IFFT, TUI visualizer, CLI mastering + pristine mode.
- ⬜ Not yet: stereo-linked detector, true-peak oversampling, LUFS measurement (EBU R128), mid/side processing, SIMD vectorization, native MP3 decoder, FLAC/OGG support.

Once the basic **ReScale & Gain Normalization** module was complete, these were the essential next components required when building an audio DSP system from scratch (*pure Rust / zero-dependency*):

---

## 1. Digital Filters (Frequency Shaping)
Used to cut or boost specific frequencies (e.g., removing low/high-frequency noise or building an EQ).

- **Biquad IIR Filter (Second-Order IIR)**:
  - *Low-pass*, *High-pass*, *Band-pass*, *Notch / Band-stop*
  - *Peaking EQ*, *Low-shelf*, *High-shelf*
- **Direct Form I / II equations**:
  $$y[n] = b_0 x[n] + b_1 x[n-1] + b_2 x[n-2] - a_1 y[n-1] - a_2 y[n-2]$$
- **Implementation**: coefficient computation based on the sampling frequency ($F_s$), cutoff frequency ($f_0$), and quality factor ($Q$).

---

## 2. Dynamic Range Control (DRC)
Controls the dynamics of the sound over time (rather than a static multiplier such as gain).

- **Compressor & Expander**:
  - Envelope detector (RMS / peak follower with *Attack* & *Release* parameters).
  - *Gain computer* with *Threshold*, *Ratio*, and *Knee* curves.
- **Lookahead Limiter**:
  - A small delay buffer (e.g., 1–5 ms) so the limiter knows about peaks before they are played, preventing overshoot without distortion.

---

## 3. Signal Generation & Modulation (Synthesizer & Test Tools)
For generating reference waveforms or modulating effects:
- **Basic oscillators**: sine, cosine, triangle, sawtooth, square wave (with *anti-aliased bandlimited synthesis*).
- **LFO (Low Frequency Oscillator)**: for parameter modulation (e.g., vibrato, tremolo, and auto-pan effects).

---

## 4. Delay-Based Effects (Time & Space)
Built on **Ring Buffer / Circular Buffer** data structures:
- **Delay line & feedback delay**
- **Chorus & flanger** (very short modulation delays: $0.1 - 20\text{ ms}$)
- **Reverberation (Schroeder / Freeverb)**: space algorithms combining *Comb Filters* and *All-Pass Filters*.

---

## 5. Frequency Domain (Spectral Processing)
For visual analysis (spectrogram) or precise frequency-domain filtering:
- **FFT (Fast Fourier Transform)**: *Cooley-Tukey Radix-2 FFT* implemented from scratch.
- **Window functions**: Hann, Hamming, Blackman-Harris (to reduce *spectral leakage*).
- **STFT (Short-Time Fourier Transform)** & overlap-add reconstruction.

---

## 6. Audio File I/O & Interfacing (From Scratch)
So the system can read and write real audio files without external libraries:
- **RIFF WAV parser & encoder**:
  - Manual parsing of the WAV PCM 16/24/32-bit float header from raw bytes (`std::io::Read` & `Write`).
- **Interleaved to de-interleaved buffers**: multi-channel conversion (stereo `L, R, L, R...` into `[L, L, L...]` and `[R, R, R...]`).

---

## Development Priority Summary (Historical — All Complete)

| Stage | Component | Status |
|---|---|---|
| **Stage 1** | ReScale, Gain, Peak Normalizer, Soft-Clip | ✅ Done |
| **Stage 2** | RIFF WAV File I/O (Parser & Encoder) | ✅ Done |
| **Stage 3** | Ring Buffer, Delay Line, Feedback Delay | ✅ Done |
| **Stage 4** | Biquad IIR Filter (RBJ), 3-Band EQ | ✅ Done |
| **Stage 5** | Envelope Follower, Compressor, Limiter | ✅ Done |
| **Stage 6** | Cooley-Tukey FFT, Windowing, Dominant Freq | ✅ Done |
| **Stage 7** | AudioProcessor trait, ProcessorChain, Streaming I/O | ✅ Done |
| **Stage 8** | Multi-crate workspace, Quality Gate | ✅ Done |
| **Stage 9** | Pristine MP3→WAV mode, TPDF dither default | ✅ Done |

## Architecture Decision: Multi-Crate Workspace (Stage 8)
- Structure: `crates/rescale-core` (domain-agnostic DSP engine, zero dependencies) + `crates/rescale-cli` (`rescale_cli` binary + examples; path-dependency on core).
- Rationale: separation of domain vs. presentation per workspace conventions; new domain crates can be added without forking core.
- Hot-path rule: zero allocation during `process()` — streaming per block, pre-sized buffers. Performance budget: full chain on 3.4 min of stereo audio < 0.2 s (release build). *(v0.1.0 caveat: `fft::Window::apply` still allocates a `Vec` per call, so this rule is not yet universal — see README *Known limitations*.)*
