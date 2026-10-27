# CLI Reference

Complete command-line interface for the REScale mastering engine.

---

## Command Syntax

```bash
rescale_cli [FLAGS] [target_dbfs] input.wav
```

| Position | Name | Required | Description |
|----------|------|----------|-------------|
| 1 | `[target_dbfs]` | No | Target peak level in dBFS |
| — | `input.wav` | Yes | Input WAV file path |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (invalid args, file not found, DSP failure) |
| 2 | Usage error (wrong arguments or flags) |

---

## Positional Arguments

### `[target_dbfs]`

Target peak normalization level applied before limiting.

| Range | Value | Default | Behavior |
|-------|-------|---------|----------|
| `-60.0` … `0.0` | Any real number | `-1.0` | Normalize audio so peak approaches target |

**Usage examples:**

```bash
rescale_cli input.wav -1      # Standard mastering (~-1 dBFS)
resscale_cli input.wav -6     # Moderate loudness (~-6 dBFS headroom)
rescale_cli input.wav -14 --bit-depth 24   # Broadcast / podcast style
rescale_cli input.wav -3 --no-dsp           # Silent flag ignored with --no-dsp
```

---

## Flag Options

| Short | Long | Type | Values | Default | Description |
|-------|------|------|--------|---------|-------------|
| | `--bit-depth` | Int | `16`, `24`, `32f` | `24` | Output bit depth / precision |
| | `--dither` | Str | `none`, `tpdf` | `tpdf` | Dithering mode (auto-disabled for `32f`) |
| `-n` | `--no-dsp` | Flag | — | `off` | Bypass all DSP processing (passthrough) |
| `-p` | `--pristine` | Flag | — | `off` | Alias for `--no-dsp` |
| `-m` | `--menu` | Flag | — | — | Launch interactive TUI menu |
| `-i` | `--interactive` | Flag | — | — | Alias for `--menu` |
| `-h` | `--help` | Flag | — | — | Show help message |

---

### `--bit-depth`

Output encoding format:

| Value | Format | Bits/Channel | Use Case |
|-------|--------|--------------|----------|
| `16` | PCM signed integer | 16-bit | CD quality, maximum compatibility |
| `24` | PCM signed integer | 24-bit | Professional archival, editing headroom |
| `32f` | IEEE 754 float | 32-bit float | Maximum dynamic range, no dither needed |

**⚠️ Note:** `32f` auto-disables dither regardless of `--dither` flag value. Dither adds noise floor; float already has sufficient headroom.

---

### `--dither`

Noise shaping strategy when reducing bit depth below 32-bit:

| Mode | When Used | Effect |
|------|-----------|--------|
| `tpdf` | Reducing from 32f to 16/24-bit | Triangular PDF dither masks quantization noise |
| `none` | Always (when source already ≤ target bits) | No noise added; quantization error may be audible |

TPDF produces statistically unbiased quantization noise — less coloration than rectangular PDF dither.

---

### `--no-dsp` / `--pristine`

Bypass entire processor chain:

- Gain → **skipped**
- EQ → **skipped**  
- Compressor → **skipped**
- Limiter → **skipped**

Output is exact copy of input, only WAV format conversion occurs. File size may change if bit depth differs.

**When to use:** Archival copies, non-destructive workflow preservation, testing pipeline integrity.

---

### `--menu` / `-m` / `-i`

Launches interactive terminal menu system. Navigate DSP parameters via keyboard:

1. Select input file
2. Configure target level, bit depth, dither
3. Preview chain settings
4. Execute and monitor real-time progress

Terminal UI renders VU meters, spectrum analyzer, and chain status using ANSI escape sequences. Requires terminal with 256-color support and UTF-8 encoding.

---

## Output Behavior

- **Filename:** `REScale-<original_name>.wav` written adjacent to input
- **Overwrite protection:** Existing output files are never silently overwritten
- **Location:** Same directory as input file
- **Streaming:** Input is processed block by block (constant memory) and the
  lookahead latency is compensated, so the output has exactly the same frame
  count as the input — no time shift and no truncated tail

Example:
```
Input:  music/audio/mix.wav
Output: music/audio/REScale-mix.wav
```

---

## Platform Notes

### Windows

- Console FFI enables UTF-8 code page + ANSI escape sequences
- ffmpeg must be on `PATH` for MP3 support
- Native `.exe` built with Rust MSVC toolchain

### Linux / macOS

- ELF/Mach-O binary, no platform-specific FFI required
- ffmpeg available via package managers: `apt install ffmpeg`, `brew install ffmpeg`

---

## Examples

### Standard Mastering Chain

```bash
# Full chain: normalize → EQ → compressor → limiter at -1 dBFS
# Output: 24-bit PCM with TPDF dither
rescale_cli ./podcast.wav -1
```

### Loud Broadcast Master

```bash
# Aggressive targeting: -14 dBFS, 16-bit for streaming
rescale_cli ./music.wav -14 --bit-depth 16 --dither tpdf
```

### Non-Destructive Pass-Through

```bash
# Zero DSP — preserve original content exactly
rescale_cli ./original.wav --no-dsp --bit-depth 32f
```

### Interactive Configuration

```bash
# Launch menu, configure interactively
rescale_cli --menu
```

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `command not found` | Not in PATH | Run `cargo install --path crates/rescale-cli` or use `cargo run --` |
| `FileNotFoundError` | Relative path issue | Use absolute path or verify cwd |
| `ffmpeg not found` | MP3 mode without encoder | Install ffmpeg and add to system PATH |
| Output louder than expected | Target > limiter ceiling | Ensure `target_dbfs` < limiter ceiling (-1 dB default) |

---

## See Also

- [Quickstart Guide](quickstart.md) — Build & first run
- [Architecture](architecture.md) — How processors work
- [Known Limitations](limitations.md) — What's broken now
