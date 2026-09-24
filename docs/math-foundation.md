# DSP Math Foundations & Implementation: ReScale Audio

> Implementation status as of September 2026: every formula in this document has been implemented in `rescale-core`. The code samples below use the actual API (`Gain`, `Normalizer`, `Limiter`, `BrickwallLimiter`, `SampleConvert`).

## Current Quality Boundaries

Peak normalization and BrickwallLimiter operate on sample peaks. LUFS metering and oversampled true-peak detection are not yet available (see the [engine roadmap](engine-roadmap.md), Phase 10+).

This document contains the mathematical foundation and a from-scratch implementation (*pure from scratch*, zero external dependency) of the **ReScale Audio** operation in Digital Signal Processing (DSP).

---

## 1. Digital Audio Signal Representation

Digital audio data is commonly stored in the following formats:
- **PCM Integer (`i16`)**: value range $-32768$ to $+32767$ (the standard 16-bit CD audio format).
- **Normalized Floating Point (`f32`)**: standard amplitude range $[-1.0, +1.0]$. Internal DSP processing is almost always performed in `f32` or `f64` to avoid losing computational precision.

---

## 2. Mathematical Foundations

### A. Linear Mapping (Min-Max Rescaling)
Maps a signal from input range $[A, B]$ to target range $[C, D]$:

$$y = C + \frac{(x - A) \cdot (D - C)}{B - A}$$

**Special Case: `i16` to `f32` $[-1.0, +1.0]$ Conversion**
$$y = \frac{x}{32768.0}$$

---

### B. Gain Scaling (Linear Amplitude vs Decibels)
Multiply the signal amplitude linearly by a gain factor $G$:

$$y = x \cdot G$$

Relation between the linear factor $G$ and **Decibels (dB)**:

$$G = 10^{\frac{\text{dB}}{20}}$$

$$y = x \cdot 10^{\frac{\text{dB}}{20}}$$

> **Rule of Thumb:**
> - $+6\text{ dB} \approx 2\times$ amplitude
> - $-6\text{ dB} \approx 0.5\times$ amplitude
> - $0\text{ dB} = 1.0\times$ (unity gain)

---

### C. Peak Normalization
Find the largest absolute amplitude in the buffer $|x|_{\max}$, then scale all samples so the peak touches the target value exactly (e.g. $1.0$ or $-0.1\text{ dBFS}$):

$$|x|_{\max} = \max(|x_0|, |x_1|, \dots, |x_{N-1}|)$$

$$G = \frac{\text{TargetPeak}}{|x|_{\max}}$$

$$y_n = x_n \cdot G \quad \text{for all } n \in [0, N-1]$$

---

### D. Soft-Clipping (Non-Linear Rescaling / Saturation)
To prevent *hard digital clipping*, which produces harsh harmonics when the signal exceeds the $[-1.0, +1.0]$ bounds, a continuous non-linear function such as the *hyperbolic tangent* ($\tanh$) is used:

$$y = \tanh(x)$$

---

## 3. Pure Rust Implementation (Zero Dependency)

The following example uses the actual `rescale-core` API (September 2026):

```rust
use rescale_core::{Gain, Normalizer, Limiter, SampleConvert};
use rescale_core::{BiquadCoefficients, BiquadFilter};
use rescale_core::{Compressor, CompressorConfig, DetectionMode, BrickwallLimiter};

fn main() {
    let mut buffer: Vec<f32> = vec![0.1, -0.4, 0.5, -0.2];

    // 1. Konversi format: i16 -> f32 [-1.0, 1.0]
    let raw_i16: Vec<i16> = vec![0, 16384, -16384, 32767, -32768];
    let float_buffer: Vec<f32> = raw_i16.iter().map(|&s| s.to_f32()).collect();

    // 2. Penskalaan gain via Decibel (dB)
    //    Rumus: G = 10^(dB/20)
    Gain::apply_db(&mut buffer, 6.0);  // +6 dB ≈ 2x amplitude

    // 3. Hitung Peak dan RMS
    let peak = Normalizer::find_peak(&buffer);
    let rms = Normalizer::calculate_rms(&buffer);

    // 4. Normalisasi peak ke target -0.5 dBFS (~0.944 linear)
    Normalizer::normalize_to_peak(&mut buffer, 0.944);

    // 5. Normalisasi aman untuk stereo (gain seragam di semua channel)
    let mut left = vec![0.3; 1024];
    let mut right = vec![0.5; 1024];
    let mut channels = vec![left.clone(), right.clone()];
    Normalizer::normalize_channels_to_peak(&mut channels, 0.9);

    // 6. Soft-Clipping (mencegah distorsi digital tajam)
    //    Tanh: y = tanh(x), cubic: y = x - x³/3
    Limiter::tanh_soft_clip(1.5);    // → ~0.905
    Limiter::cubic_soft_clip(1.5);   // → 2/3 ≈ 0.667
    Limiter::hard_clamp(1.5);        // → 1.0

    // 7. Filter Biquad IIR Low-Pass (RBJ Cookbook)
    let coeffs = BiquadCoefficients::low_pass(44100.0, 1000.0, 0.7071);
    let mut filter = BiquadFilter::new(coeffs);
    filter.process_buffer(&mut buffer);

    // 8. Compressor (Soft-Knee)
    let config = CompressorConfig {
        threshold_db: -12.0,
        ratio: 4.0,
        knee_db: 4.0,
        attack_ms: 5.0,
        release_ms: 100.0,
        makeup_gain_db: 2.0,
        mode: DetectionMode::Rms,
    };
    let mut compressor = Compressor::new(44100.0, config);
    compressor.process_buffer(&mut buffer);

    // 9. Lookahead Brickwall Limiter
    let mut limiter = BrickwallLimiter::new(44100.0, -0.3, 20.0, 2.0);
    limiter.process_buffer(&mut buffer);

    println!("Buffer processed: {:?}", &buffer[..4]);
}
```
