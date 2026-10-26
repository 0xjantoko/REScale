# ReScaling MP3 Files with the REScale CLI

> Implementation status as of September 2026: MP3 requires the `ffmpeg` executable on PATH. The Rust engine itself remains zero-external-crate, but the MP3 path is not free of system dependencies. The limiter currently protects sample peak (not true peak / inter-sample peak — planned in the [roadmap](roadmap.md)). Pristine mode (`--no-dsp`) is verified bit-identical to a direct ffmpeg decode.

> Output correction: the CLI writes a WAV named `REScale-<input_name>.wav` in the input folder. For MP3 input, the CLI also attempts to write `REScale-<input_name>.mp3` at 320 kbps via ffmpeg. The WAV bit depth can be 16-bit, 24-bit, or 32-bit float (default: 24-bit); TPDF dither is the default for PCM. MP3 decoding uses `pcm_f32le` (full float precision, no intermediate 16-bit quantization). The `--no-dsp` / `--pristine` flags bypass the entire mastering chain — 32f output in this mode is verified bit-identical to a direct ffmpeg decode.

The **`rescale_cli`** CLI tool processes and normalizes (**ReScale**) **MP3** and **WAV** audio files directly from the terminal.

---

## 1. How the MP3 ReScale Pipeline Works

Because **MP3** is psychoacoustic compression (*lossy encoded frames*), high-precision audio DSP is performed using the following industry-standard pipeline:

```text
[ Original .mp3 File ]
       │
       ▼  (1. Decode to lossless PCM Float32 stream)
[ Raw Audio Samples ]
       │
       ▼  (2. DSP Engine: Peak Normalizer -> RMS Compressor -> Brickwall Limiter)
[ Processed ReScaled Audio ]
       │
       ▼  (3. Encode back)
[ ReScaled .mp3 (320 kbps) & .wav Master Output ]
```

---

## 2. How to Run the Command

Open a terminal in the project root directory, then run:

### A. Standard ReScale (Target: `-1.0 dBFS` — Spotify/YouTube Streaming Standard)
```bash
cargo run --bin rescale_cli -- "path/to/your_song.mp3" -1.0
```

### B. Harder ReScale (Target: `-0.3 dBFS` — CD/Maximum Mastering Standard)
```bash
cargo run --bin rescale_cli -- "path/to/your_song.mp3" -0.3
```

### C. ReScale a WAV File
```bash
cargo run --bin rescale_cli -- "rekaman.wav" -1.0
```

---

## 3. What the Tool Does Automatically

1. **Automatic decoding**: reads the MP3 audio and unpacks it into a float (`f32`) sample stream.
2. **Acoustic analysis**: computes the **initial peak** and **RMS energy** in decibels (dBFS).
3. **ReScaling**:
   - **Peak normalization**: raises the signal level to the desired dBFS target.
   - **Soft-knee dynamic compressor**: evens out the dynamic range so quiet passages are lifted and loud passages are controlled without distortion.
   - **Lookahead brickwall limiter**: keeps the output under the ceiling, but in v0.1.0 the last part of that protection comes from a final hard clamp (residual transient overshoot) rather than from an exact sliding-maximum gain reduction; see *Known limitations* in the README.
4. **Output**:
   - Writes `REScale-<input_name>.wav` (24-bit default + TPDF dither; or via `--bit-depth 16/24/32f`).
   - Writes `REScale-<input_name>.mp3` (320 kbps via ffmpeg, when the input is MP3).
   - Displays a **VU meter** visualization of the processed result in the terminal.

---

## 4. CLI Options

```bash
# Opsi yang tersedia
--bit-depth 16|24|32f    # Kedalaman bit output (default: 24)
--dither none|tpdf       # Mode dither (default: tpdf untuk PCM)
--no-dsp / --pristine    # Mode transparan: decode → write, tanpa DSP mastering
--help                   # Tampilkan bantuan
```

**Notes:**
- The dBFS target must be in the range -60.0 to 0.0.
- Existing output files are never overwritten.
- Pristine mode defaults to 24-bit; use `--bit-depth 32f` for lossless output.
- MP3 decoding uses `pcm_f32le` (full float precision).

- Make sure `ffmpeg` is available on your system PATH to handle MP3 container decompression (if it is not, you can feed `.wav` files directly instead).
