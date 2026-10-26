# REScale Terminal UI (TUI) Guide

> Implementation status as of September 2026: the TUI is an ANSI visualization demo driven by a synthetic signal. It is not an audio device host and does not yet guarantee an end-to-end real-time audio path.

The monitor view shows input/output analysis per channel, peak and RMS in dBFS, the 9-band post-limiter spectrum, dominant frequency, lookahead-limiter latency, RMS change, and frame render time. Meters are based on sample peak and RMS; LUFS and true-peak are not yet available (see [roadmap.md](roadmap.md), Phase 2). Fixes: cursor visible during input, clean exit on EOF, spectrogram alignment, multi-byte offset.

## Visual Theme: DAW Plugin Dark Theme (Stage 10)

The TUI has been upgraded from a plain-text display to a **DAW plugin / hardware outboard-style dashboard**:

| Component | Description | Hardware Analogy |
|---|---|---|
| `draw_panel` | Panel with metal border + title bar + status text | Rack unit compressor |
| `draw_vu_meter` | Gradient LED VU meter (green→yellow→orange→red) + peak hold + arrow | SSL G-Series meter |
| `draw_smooth_spectrum` | Spectrum analyzer with edge antialiasing (full blocks + upper halves) | TDR Nova / SPAN |
| `draw_waveform` | Per-sample time-domain preview | DAW waveform editor |
| `draw_rolling_meter` | Rolling/falling meter with blur tail | Mixer channel strip |
| `draw_stereo_pan` | Horizontal pan/phase indicator | Channel strip pan knob |
| `draw_h_slider` | Horizontal slider for EQ/gain parameters | Plugin UI fader |
| `draw_processing_status` | LED badges `[EQ] [COMPRESS] [LIMIT]` | Chain status indicator |
| `draw_header` | Header with technical info (rate, channels, format) | Plugin window header |

**Color palette (ANSI 256-color, zero dependency):**
- Metal gray (240/245) — rack panel borders
- VU gradient: green (46) → yellow (226) → orange (214) → red (1;31)
- Cyan accent — titles & highlights
- Bright white — numeric values

The old API (`draw_box`, `draw_level_meter`, `draw_fft_spectrum`) remains available as compatibility wrappers.

The Terminal UI module in **REScale** is built with **Pure ANSI Escape Codes** (*zero dependency*). The module renders a **Box Frame**, **VU Meter**, and **FFT Spectrum Analyzer** live in the terminal.

---

## 1. Running the Interactive TUI Menu

Open the interactive visual menu directly from the terminal by running:

```bash
cargo run --bin rescale_cli
```
or:
```bash
cargo run --bin rescale_cli -- --menu
```

### Features Available in the TUI Menu:
- `[1] 🎵 Master / ReScale Audio File`: Load an audio file (.wav / .mp3), set the target dBFS, bit depth format (16/24/32f), and dither.
- `[2] 📊 Realtime DSP & Spectrum Monitor`: Live 9-band FFT spectrogram, stereo VU level meters, and a DSP frequency sweep.
- `[3] 🔍 Inspect Audio File`: Inspect RIFF metadata, sample rate, peak/RMS dBFS detection, and clipping status.
- `[4] 🎛️ 3-Band Parametric EQ & Gain Tool`: Set Bass, Mid, and Treble frequencies plus overall master gain.
- `[5] ❌ Exit`

---

## 2. Running the Live FFT Monitor Demo

Just run the standalone example:

```bash
cargo run --example tui_monitor
```

---

## 3. Using `TerminalCanvas` in Your Own Code

### A. Canvas Initialization
```rust
use rescale_core::TerminalCanvas;

let tui = TerminalCanvas::new(80, 24); // Width: 80 columns, Height: 24 rows

// Bersihkan terminal & sembunyikan cursor saat mulai
tui.clear_screen();
```

---

### B. Drawing a Box Frame (Border Box)
```rust
// draw_box(row, col, width, height, title)
tui.draw_box(1, 1, 78, 22, "My Audio DSP Monitor");
```

---

### C. Drawing the VU Level Meter (Audio Bar)
Display the amplitude level (linear `0.0` to `1.0+`):
```rust
use rescale_core::Normalizer;

let peak_left = Normalizer::find_peak(&left_channel_buffer);
let peak_right = Normalizer::find_peak(&right_channel_buffer);

// draw_vu_meter(row, col, label, level_linear, bar_width)
tui.draw_vu_meter(17, 6, "Ch L", peak_left, 30);
tui.draw_vu_meter(18, 6, "Ch R", peak_right, 30);
```
> **Automatic color coding:**
> - Level $\le 0.75$ (below $-2.5\text{ dBFS}$) $\rightarrow$ **Green** (Safe)
> - Level $0.75 - 0.99$ (between $-2.5$ and $0\text{ dBFS}$) $\rightarrow$ **Yellow** (Optimal)
> - Level $> 0.99$ $\rightarrow$ **Red** (Clipping Warning)

---

### D. Drawing the Frequency Spectrogram (FFT Spectrum Analyzer)
```rust
use rescale_core::{Complex, Fft, Window, WindowType};

// 1. Siapkan buffer FFT & terapkan Window
let mut fft_buffer: Vec<Complex> = audio_buffer[0..512]
    .iter()
    .map(|&s| Complex::new(s, 0.0))
    .collect();

let mut win_samples: Vec<f32> = fft_buffer.iter().map(|c| c.re).collect();
Window::apply(&mut win_samples, WindowType::Hann);
for (c, &w) in fft_buffer.iter_mut().zip(win_samples.iter()) {
    c.re = w;
}

// 2. Jalankan FFT
Fft::forward(&mut fft_buffer);

// 3. Kelompokkan ke band frekuensi (magnitude, label)
let spectrum_bands = vec![
    (fft_buffer[1..4].iter().map(|c| c.magnitude()).sum::<f32>(), "60"),
    (fft_buffer[5..10].iter().map(|c| c.magnitude()).sum::<f32>(), "250"),
    (fft_buffer[11..30].iter().map(|c| c.magnitude()).sum::<f32>(), "1k"),
    (fft_buffer[31..80].iter().map(|c| c.magnitude()).sum::<f32>(), "4k"),
    (fft_buffer[81..180].iter().map(|c| c.magnitude()).sum::<f32>(), "10k"),
];

// 4. Render spektrogram
// draw_fft_spectrum(row, col, height, bands)
tui.draw_fft_spectrum(7, 6, 6, &spectrum_bands);
```

---

### E. Cleanup & Restoring the Terminal Cursor
Before your program exits:
```rust
tui.move_to(23, 1);
tui.restore_cursor(); // Kembalikan cursor terminal normal
```

---

## 4. Minimal TUI Program Template

```rust
use rescale_core::{Normalizer, TerminalCanvas};
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let tui = TerminalCanvas::new(80, 24);
    tui.clear_screen();

    for frame in 0..100 {
        let fake_audio_peak = ((frame as f32 * 0.1).sin().abs()) * 0.95;

        // Render UI
        tui.draw_box(1, 1, 70, 10, "Live Level Meter");
        tui.draw_vu_meter(4, 5, "Master Out", fake_audio_peak, 35);

        let _ = stdout().flush();
        sleep(Duration::from_millis(50));
    }

    tui.move_to(11, 1);
    tui.restore_cursor();
}
```
