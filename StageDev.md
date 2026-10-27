# Stage Development — REScale

## Target: DSP audio engine + mastering CLI zero-dependency (pure Rust) | Architecture: Cargo workspace multi-crate (`rescale-core` library + `rescale-cli` binary)

### Overall Progress: ✅ Stage 1–14 selesai (core workspace, publikasi v0.1.0, stabilisasi DSP, streaming CLI, limiter baru) | ⬜ Stage 15+ mengikuti `docs/roadmap.md`

> Terakhir diperbarui: sesi Sprint 2B — rewrite limiter + stereo linking + validasi (September 2026). Status quality gate diverifikasi ulang: **58/58 test pass** (49 core + 9 CLI), clippy `-D warnings` 0 issue, fmt clean, build 0 warning.

---

## Stage 1: Setup Proyek & Fondasi ReScale/Gain (Selesai)
### Deliverables
- [x] Inisialisasi struktur modul Rust (`convert`, `gain`, `normalize`, `limiter`).
- [x] Trait `SampleConvert` (i16/i32 <-> f32).
- [x] Fungsi Gain (Linear & Decibel conversion).
- [x] Peak Normalizer & RMS calculation.
- [x] Soft-clipping protection (tanh & cubic saturation).

---

## Stage 2: RIFF WAV File I/O (Parser & Encoder) (Selesai)
### Deliverables
- [x] Implementasi RIFF Header parser untuk membaca file `.wav` (PCM 16-bit, 24-bit, 32-bit float).
- [x] Implementasi WAV encoder untuk menulis buffer audio ke file `.wav`.
- [x] Penanganan multi-channel (Stereo de-interleaving dan interleaving: `LRLR...` <-> `[L], [R]`).
- [x] Unit test: generate sine wave, simpan ke file `.wav`, baca kembali, verifikasi integritas data.

---

## Stage 3: Buffer Management & Delay Primitive (Selesai)
### Deliverables
- [x] Implementasi `RingBuffer` / `CircularBuffer` zero-allocation yang aman.
- [x] Implementasi `DelayLine` dengan fractional delay (Linear & Hermite/Catmull-Rom interpolation).
- [x] Implementasi `FeedbackDelay` effect (Wet/Dry mix, feedback loop control).
- [x] Unit test pemrosesan delay kontinu & interpolasi.

---

## Stage 4: Filter Digital (Biquad IIR Filter / EQ) (Selesai)
### Deliverables
- [x] Implementasi struktur Biquad Filter (Transposed Direct Form II).
- [x] Generator koefisien filter standar Robert Bristow-Johnson (RBJ Audio EQ Cookbook): Low-pass, High-pass, Band-pass, Notch, Peaking EQ, Low-shelf, High-shelf.
- [x] Implementasi Parametric 3-Band Equalizer (Bass, Mid, Treble).
- [x] Unit test atenuasi frekuensi dan boost gain EQ.

---

## Stage 5: Dynamic Range Control (Compressor & Limiter) (Selesai)
### Deliverables
- [x] Implementasi `EnvelopeFollower` (Peak & RMS detection modes, exponential attack/release smoothing).
- [x] Implementasi `Compressor` (Soft-Knee quadratic gain computer, threshold, ratio, makeup gain).
- [x] Implementasi `BrickwallLimiter` (Lookahead delay buffer).
- [x] Unit test kompresi sinyal, verifikasi batas ceiling limiter, dan Full Mastering Chain pipeline.
- [x] **Utang teknis:** gain reduction limiter diturunkan dari envelope eksponensial, bukan sliding maximum-hold — lihat `docs/limitations.md` #1. *(Diselesaikan di Stage 14.)*

---

## Stage 6: Analisis Domain Frekuensi (FFT & Spectrogram) (Selesai)
### Deliverables
- [x] Implementasi struktur bilangan `Complex` (Polar form, magnitude, power, phase, conjugate, vector arithmetic).
- [x] Windowing functions (Rectangular, Hann, Hamming, Blackman-Harris).
- [x] Implementasi Cooley-Tukey Radix-2 FFT & IFFT in-place (Bit-reversal permutation).
- [x] Magnitude Spectrum & Dominant Frequency Peak Detector.
- [x] Unit test verifikasi: IFFT(FFT(x)) == x lossless roundtrip & deteksi presisi frekuensi nada sinyal.
- [ ] **Utang teknis:** window memakai denominator simetris `N-1` (belum COLA-compatible) — lihat `docs/limitations.md` #10.

---

## Stage 7: Engine Architecture & Streaming Runtime (Selesai)
### Deliverables
- [x] Tambahkan AudioSpec, AudioBlock, AudioProcessor, dan ProcessorChain.
- [x] Tambahkan wrapper compressor, limiter, equalizer, dan gain dengan state independen per channel.
- [x] Integrasikan mastering chain CLI dengan API engine block.
- [x] WavStreamReader dan WavStreamWriter berbasis block (Zero-Allocation on process & streaming seekable).
- [x] Flush tail lookahead limiter dan unit test streaming block roundtrip.
- [ ] **Utang teknis:** `ProcessorChain::flush` belum meneruskan tail melewati seluruh processor — lihat `docs/limitations.md` #3.

---

## Stage 8: Konsolidasi Workspace & Quality Gate (Selesai)
### Tujuan
Menata struktur project sesuai konvensi Rust workspace multi-crate dan menutup quality gate penuh.
### Deliverables
- [x] Restrukturisasi ke Cargo workspace multi-crate:
  - `crates/rescale-core` — engine DSP murni (zero dependency, tidak tahu apa-apa tentang CLI).
  - `crates/rescale-cli` — binary `rescale_cli` + examples (`full_pipeline`, `tui_monitor`), path-dependency ke `rescale-core`.
- [x] Bug fix: `SampleConvert for i32` di `convert.rs` (cabang `if/else` identik — literal `2147483647.0` bulat ke `2147483648.0` dalam f32; plus lint `approx_constant` pada test LPF, `0.7071` → `FRAC_1_SQRT_2`).
- [x] Quality gate (saat stage ini ditutup): `cargo build --all-targets` 0 error · `cargo clippy --all-targets -- -D warnings` 0 issue · `cargo test` 18/18 pass · `cargo fmt --check` 0 diff.
- [x] Performance budget terukur: full chain (normalisasi → kompresor RMS → brickwall limiter) + WAV I/O untuk 3.4 menit audio stereo (9.05 jt sampel/ch, 44.1 kHz) = **0.170 detik** (build release). Hot path zero-allocation: streaming per-block, buffer pre-sized.
- [x] Perbaikan Terminal UI (commit `e91e6bd`): kursor terlihat saat input (`show_cursor` + restore di jalur keluar), exit bersih saat EOF (`read_line` → `Ok(0)`), alignment spektrogram (cell 3 kolom seragam), warna abu-abu VU meter, dan offset kolom multi-byte (`chars().count()` bukan `len()` byte).
### Komponen
- `crates/rescale-core/` — library engine (domain DSP)
- `crates/rescale-cli/` — binary CLI & examples (presentation)

---

## Stage 9: Mode Konversi Pristine MP3→WAV (Selesai)
### Tujuan
Konversi MP3→WAV transparan penuh: decode di float presisi penuh, tanpa DSP mastering yang tidak diminta, kuantisasi output berkualitas (default 24-bit + TPDF dither di semua jalur export).
### Latar (audit algoritma)
Audit menemukan 3 kebocoran transparansi kritis di jalur MP3→WAV: (1) decode ffmpeg default `pcm_s16le` → kuantisasi 16-bit sebelum DSP; (2) chain mastering (normalize + kompresor + limiter) selalu aktif — tidak ada mode passthrough; (3) default output 16-bit + jalur export tanpa dither (`WavWriter::write_to_file/write_to_stream` default `DitherMode::None`).
### Deliverables
- [x] Fix #1: decode ffmpeg `-c:a pcm_f32le` — seluruh chain jalan di float 32-bit, tanpa kuantisasi 16-bit di tengah (`rescale_cli.rs` Step 1).
- [x] Fix #2: flag CLI `--no-dsp` / `--pristine` (mode pristine): decode → tulis, tanpa normalize/kompresor/limiter; log peak + warning jika peak > 0 dBFS.
- [x] Fix #3: default bit depth output 24-bit (CLI & TUI) + TPDF dither default di semua jalur export writer (`write_to_file`/`write_to_stream` no-options; otomatis nonaktif untuk 32f).
- [x] Quality gate (saat stage ini ditutup): `cargo build --all-targets` 0 error · `cargo clippy --all-targets -- -D warnings` 0 issue · `cargo test` 18/18 pass · `cargo fmt --check` 0 diff.
- [x] Verifikasi fungsional (MP3 uji: sine 440 Hz, 5 dtk, stereo, 44.1 kHz, 192 kbps): pristine `--bit-depth 32f` menghasilkan output **bit-identical** (SHA-256 raw data) dengan decode langsung `ffmpeg -c:a pcm_f32le`; pristine default = `pcm_s24le` 44.1 kHz 2 ch; mode mastering lama tetap jalan (peak → −1.00 dBFS).
- [x] Update guide user: [`docs/mp3-rescale-guide.md`](docs/mp3-rescale-guide.md).
### Komponen
- `crates/rescale-cli/src/` — flag & alur pristine
- `crates/rescale-core/src/wav/writer.rs` — default dither export

---

## Stage 10: TUI Redesign — DAW Plugin Dark Theme (Selesai)
### Tujuan
Meningkatkan TUI dari tampilan teks sederhana menjadi dashboard visual bergaya plugin DAW / hardware outboard, dengan VU meter LED, smooth spectrum analyzer, dan panel rack unit.
### Deliverables
- [x] Palet warna ANSI 256-color (metal gray, neon cyan, VU gradient hijau→kuning→orange→merah) + konstanta karakter visual terpusat di `canvas.rs`.
- [x] `draw_panel` — border metal + title bar + status text (pengganti `draw_box`).
- [x] `draw_vu_meter` — gradient LED bar per-segmen + peak hold indicator + direction arrow + nilai dBFS digital.
- [x] `draw_smooth_spectrum` — edge antialiasing (blok penuh + setengah atas) + warna per ketinggian + label frekuensi berwarna.
- [x] Komponen baru: `draw_waveform`, `draw_rolling_meter`, `draw_stereo_pan`, `draw_h_slider`, `draw_processing_status`, `draw_header`.
- [x] Tampilan ulang main menu (track-selection cursor, panel header dengan info arsitektur & test suite).
- [x] Realtime monitor: layout multi-panel (INPUT / OUTPUT / SPECTRUM) + peak-hold decay + 300 frame.
- [x] Backward compatibility: `draw_box`, `draw_level_meter`, `draw_fft_spectrum` tetap tersedia sebagai wrapper.
- [x] Bug fix saat integrasi: `header` out-of-scope di monitor, type mismatch `&str`/`String` pada badge status, `i32`/`usize` mismatch pada bar inspector, 4 unused variable, dead-code const, clippy `too_many_arguments`.
- [x] Quality gate (saat stage ini ditutup): `cargo build --all-targets` 0 error · `cargo clippy --all-targets -- -D warnings` 0 issue · `cargo test` 18/18 pass · `cargo fmt --check` 0 diff.
- [x] Smoke test runtime: 300 frame render monitor tanpa panic/error (verifikasi dump output).
### Komponen
- `crates/rescale-core/src/tui/canvas.rs` — palet warna, panel, VU meter, spectrum, waveform, slider
- `crates/rescale-core/src/tui/menu.rs` — layout dashboard, monitor multi-panel, preview EQ

> Catatan sinkronisasi: sejak Stage 11 jumlah test menjadi **21** (3 test tambahan: panic-safe `Fft::compute_magnitude_spectrum`/`find_dominant_frequency` dan kontrak `RingBuffer`), sesuai status terkini di bagian bawah.

---

## Stage 11: Publikasi Open Source & Infrastruktur Kualitas (Selesai)
### Tujuan
Menerbitkan REScale sebagai proyek open source `v0.1.0` — publik, dapat diaudit, dengan quality gate otomatis, dokumentasi kanonik, dan tata kelola solo-maintainer (PR tidak diterima, issue terbuka).
### Deliverables
- [x] Repo publik [`github.com/0xjantoko/REScale`](https://github.com/0xjantoko/REScale) — commit awal, tag `v0.1.0`, **GitHub Release** dengan notes dari CHANGELOG.
- [x] Metadata crates.io lengkap di `rescale-core` (`repository`, `documentation`, `readme`, `rust-version`); `rescale-cli` menyatakan `publish = false`.
- [x] CI 3 job: **Quality gate** (Linux+Windows: fmt, clippy `-D warnings`, rustdoc `-D warnings`, build, test), **MSRV 1.78**, dan **Zero-dependency policy** (`cargo tree` gate, locked offline build, grep `unsafe` di core, larangan vendor/patch/git dependency).
- [x] Quality gate terverifikasi ulang (saat stage ini ditutup): **21/21 test pass**, clippy 0 issue, fmt clean, build 0 warning, `cargo tree` hanya berisi 2 crate workspace.
- [x] Berkas tata kelola: `CHANGELOG.md` (Keep a Changelog + SemVer), `SECURITY.md`, 3 issue template, `.github/dependabot.yml`, `.editorconfig`, `.gitattributes`, `LICENSE-MIT` + `LICENSE-APACHE` (dual license).
- [x] Dokumentasi kanonik di `docs/`: quickstart, cli-reference, api, architecture, limitations, roadmap, implementation-guide, math-foundation, mp3-rescale-guide, tui-guide, project-analysis; README ringkas + tabel navigasi.
- [x] Konsolidasi dokumentasi: arsipkan duplikat historis (`architecture-roadmap`, `engine-roadmap`, `usage-guide`) ke `private/archived/` (gitignored), perbaiki tautan rusak `quickstart → api.md`, dan tautkan seluruh dokumen kanonik dari README.
- [x] Koreksi akurasi README: perintah clippy lengkap (`--all-targets --locked -- -D warnings`) dan lokasi binary setelah build (`target/release/rescale_cli` / `cargo install --path`).
- [x] `StageDev.md` dikembalikan ke root proyek dan disinkronkan dengan status terkini.

### Ketergantungan (belum selesai)
- [ ] Publish `rescale-core` ke crates.io (badge docs.rs baru valid setelahnya — status terbit belum terverifikasi).
- [ ] Setting GitHub: topics, homepage, Discussions, social preview, branch protection.
- [ ] Benchmark zero-dep + integration test `rescale-cli` (menutup `docs/limitations.md` #12).
- [ ] Pindahkan TUI keluar dari `rescale-core` (limitations #11) dan integrasi streaming CLI + kompensasi latency (limitations #2).

### Komponen
- `.github/workflows/ci.yml`, `.github/ISSUE_TEMPLATE/`, `.github/dependabot.yml`
- `README.md`, `docs/`, `CHANGELOG.md`, `SECURITY.md`, `LICENSE-*`
- `private/` — arsip internal (gitignored, tidak ikut distribusi)

### Konsep
- **Solo-maintainer:** kualitas dijaga oleh otomasi (CI + gate), bukan review manusia; kebijakan "PR tidak diterima" tertulis di README.
- **Satu sumber kebenaran dokumentasi:** `docs/limitations.md` menang atas dokumen lain jika terjadi konflik; roadmap forward-looking hanya di `docs/roadmap.md`; catatan historis staged development di dokumen ini.
- **Zero-dependency sebagai kontrak:** setiap dependensi baru wajib lewat kebijakan di README dan dijaga job CI khusus.

---

## Stage 12: Stabilisasi DSP — Sprint 1 (Selesai)
### Tujuan
Menutup bug kebenaran di core DSP sebelum rilis berikutnya: kehilangan tail pada rantai processor, off-by-one pada delay line, dan time constant RMS yang meleset — semuanya tanpa menambah dependensi.
### Deliverables
- [x] `ProcessorChain::flush` ditulis ulang: tail = jumlah latency seluruh stage, dibangkitkan dengan mengalirkan input senyap melewati seluruh chain; benar untuk urutan stage apa pun (termasuk beberapa stage berlatency).
- [x] `DelayLine` off-by-one diperbaiki: `read_delay(n)` relatif terhadap sampel terakhir yang di-push; ditambah `read_delay_before_push` untuk pembaca pre-push (`FeedbackDelay` perilakunya tidak berubah).
- [x] Latency limiter konsisten: delay fisik = `lookahead_samples` = nilai yang dilaporkan `latency_samples()`.
- [x] RMS envelope: koefisien domain power dikoreksi (`r²`), release sesuai nilai konfigurasi (sebelumnya ~2× lebih lambat).
- [x] Regression test skala blok 64/256/1024 frame: output block-wise identik dengan single-block, termasuk tail hasil flush.
- [x] Unit test baru: semantik delay (integer/fractional/pre-push/process_sample), waktu release envelope (Peak & RMS), flush chain dengan prefix processor dan dua stage berlatency, flush chain tanpa latency.
- [x] Quality gate terverifikasi (saat stage ini ditutup): `cargo fmt --check` clean, `cargo clippy -D warnings` 0 issue, `cargo test` **32/32 pass**.
### Komponen
- `crates/rescale-core/src/engine.rs` — `ProcessorChain::flush`, penghitung tail, helper `process_stages`, modul test chain
- `crates/rescale-core/src/buffer/delay_line.rs` — semantik delay + interpolasi + test
- `crates/rescale-core/src/buffer/effects.rs` — `read_delay_before_push`
- `crates/rescale-core/src/drc/envelope.rs` — koreksi koefisien RMS + test
### Konsep
- **Tail chain = Σ latency stage**: satu-satunya cara benar untuk rantai arbitrer adalah memberi makan senyap dan membiarkan state internal setiap stage mengeluarkan sampel tertahan.
- **Satu kontrak delay**: `read_delay` selalu post-push; varian pre-push dibuat eksplisit agar tidak ada lagi off-by-one implisit di pemakai.
- **Zero-dependency tetap utuh**: semua perbaikan murni Rust std, tanpa crate baru; `cargo tree` tetap hanya 2 crate workspace.

---

## Stage 13: Sprint 2A — Streaming CLI & Kompensasi Latency (Selesai)
### Tujuan
Menghapus time-shift dan tail terpotong pada output CLI dengan memindahkan jalur pemrosesan ke streaming block, lalu mengunci perilakunya lewat integration test CLI.
### Deliverables
- [x] CLI membaca input via `WavStreamReader` per blok (pass 1: analisis header + level; pass 2: proses + tulis), tanpa memuat seluruh file ke RAM.
- [x] Pemrosesan per blok melalui `ProcessorChain` + gain normalisasi linier (presisi penuh), hasil ditulis via `WavStreamWriter`.
- [x] Kompensasi latency: `latency_samples` frame awal dibuang dan `flush()` dipanggil di EOF sehingga jumlah frame output = jumlah frame input.
- [x] Parser CLI: flag panjang tak dikenal dilaporkan "Unknown argument" (tidak lagi tertelan sebagai target dBFS).
- [x] `tests/cli.rs` — 9 integration test zero-dep (`std::process::Command`) untuk mastering, pristine, bit-depth, error path, overwrite, dan `--help`.
- [x] Quality gate: fmt clean, clippy `-D warnings` 0 issue, **41 test pass** (32 core + 9 CLI).
### Komponen
- `crates/rescale-cli/src/bin/rescale_cli.rs` — pipeline streaming, metrik output, kompensasi latency
- `crates/rescale-cli/tests/cli.rs` — integration test CLI
### Konsep
- **Dua pass tanpa menahan memori**: pass-1 mengukur, pass-2 memproses; normalisasi peak tetap global (identik perilaku lama) tanpa menyimpan audio.
- **Kompensasi latency eksplisit**: buang frame awal lalu drain tail = output sepanjang input.
- **Zero-dependency**: test memakai `std` + crate workspace, tanpa `assert_cmd`/`tempfile`.

---

## Stage 14: Sprint 2B — Rewrite Limiter, Stereo Linking & Validasi (Selesai)
### Tujuan
Mengganti limiter berbasis envelope dengan sliding max-hold sejati, membuat deteksinya ter-link antar channel, dan menutup celah keamanan parameter — semua zero-dependency.
### Deliverables
- [x] `SlidingMax` baru (`drc/max_hold.rs`): monotonic deque zero-alloc, O(1) amortisasi, diuji terhadap brute-force.
- [x] `LookaheadDetector` baru (`drc/limiter.rs`): gain dari sliding maximum window lookahead (L+1 sampel), attack instan, release eksponensial + min-hold.
- [x] `BrickwallLimiter` ditulis ulang memakai `LookaheadDetector` + `DelayLine`; hard clamp hanya jaring pengaman f32; latency fisik = `latency_samples()`.
- [x] `LimiterProcessor` memakai satu detektor bersama → **stereo-linked limiting** (gain dari channel terkeras, diterapkan ke semua channel).
- [x] Validasi parameter filter (`BiquadCoefficients`): frekuensi diklem `(0, Nyquist)`, `q > 0`, gain EQ ±40 dB, sampling rate invalid → 48 kHz.
- [x] Modul `safety.rs`: guard NaN/Inf + flush denormal pada boundary `ProcessorChain`.
- [x] Test baru: max-hold (3), detektor/limiter (4), linked stereo + ceiling percussive (2), validasi koefisien (5), safety (3).
- [x] Quality gate: fmt clean, clippy `-D warnings` 0 issue, **58 test pass** (49 core + 9 CLI).
### Komponen
- `crates/rescale-core/src/drc/max_hold.rs` — sliding maximum tracker
- `crates/rescale-core/src/drc/limiter.rs` — `LookaheadDetector` + `BrickwallLimiter`
- `crates/rescale-core/src/engine.rs` — `LimiterProcessor` linked + sanitasi chain
- `crates/rescale-core/src/filter/coefficients.rs` — validasi parameter
- `crates/rescale-core/src/safety.rs` — sanitasi NaN/Inf/denormal
### Konsep
- **Deteksi dipisah dari delay audio**: satu detektor bisa melayani banyak channel → linked limiting tanpa duplikasi state.
- **Sliding max instan**: begitu puncak masuk window, gain turun sebelum puncak sampai output; release min-hold mencegah pompa.
- **Pertahanan berlapis**: validasi di konstruktor + sanitasi di boundary engine + clamp f32 sebagai jaring terakhir.

---

## 🏆 Status Engine (verifikasi terakhir)

- **Arsitektur**: Cargo workspace multi-crate — `rescale-core` (engine DSP, zero dependency) + `rescale-cli` (binary & examples). Keputusan arsitektur tercatat di [`docs/architecture.md`](docs/architecture.md).
- **Modul DSP**: 14 modul di `rescale-core` — gain, normalize, convert, limiter, buffer (3), filter (3), drc (3), fft (3), wav (4), engine, tui (2).
- **Dependency**: 0 external crates (pure standard library); `Cargo.lock` di-commit; diverifikasi `cargo tree` + CI.
- **Test Suite**: **58 test** lulus (100% passing) — 49 unit test core (max-hold, limiter linked, validasi koefisien, safety, regression blok 64/256/1024, semantik delay, envelope, flush multi-stage) + 9 integration test CLI.
- **Quality Gate**: build 0 error, clippy `-D warnings` 0 issue, fmt clean, rustdoc `-D warnings` 0 issue.
- **Performance Budget**: full mastering chain + WAV I/O, 3.4 menit audio stereo (44.1 kHz) = 0.170 detik (release). Hot path zero-allocation (streaming per-block, buffer pre-sized).
- **Streaming Ready**: I/O WAV streaming per block tanpa memuat seluruh file ke RAM.
- **CLI**: WAV + MP3 (via ffmpeg), interactive TUI menu, command-line mastering, `--no-dsp` pristine mode.
- **Status publikasi**: `v0.1.0` early / API unstable; daftar bug & batasan terkini ada di [`docs/limitations.md`](docs/limitations.md).
