//! Terminal UI Renderer — **DAW Plugin Dark Theme** dengan zero dependency.
//! Komponen visual bergaya hardware audio: VU meter LED, smooth spectrum analyzer,
//! panel bergaya rack unit, dan processing status indicator.

#![allow(clippy::too_many_arguments)]

use std::io::{stdout, Write};

// ═══════════════════════════════════════════
//  PALET WARNA DAW (ANSI 256 warna)
// ═══════════════════════════════════════════

/// Warna dasar / reset
const COL_RESET: &str = "\x1b[0m";
const COL_DIM: &str = "\x1b[90m";

/// Metal gray untuk border/rack panel — kesan hardware outboard
const COL_METAL: &str = "\x1b[38;5;240m";
const COL_METAL_LIGHT: &str = "\x1b[38;5;245m";

/// Aksen neon cyan — judul, highlights
const COL_ACCENT_CYAN: &str = "\x1b[1;36m";
const COL_ACCENT_GREEN: &str = "\x1b[1;32m";
const COL_ACCENT_AMBER: &str = "\x1b[1;33m";
const COL_ACCENT_RED: &str = "\x1b[1;31m";

/// Teks putih terang
const COL_TEXT_BRIGHT: &str = "\x1b[1;37m";

/// Gradien hijau–kuning–merah untuk VU meter (mode 256-color)
const COL_VU_LOW: &str = "\x1b[38;5;46m"; // Hijau lembut
const COL_VU_MID: &str = "\x1b[38;5;226m"; // Kuning terang
const COL_VU_HIGH: &str = "\x1b[38;5;214m"; // Orange
const COL_VU_PEAK: &str = "\x1b[1;31m"; // Merah menyala

// Karakter visual utama
const CH_FULL_BLOCK: char = '█';
const CH_HALF_TOP: char = '▀';
const CH_SOLID_BAR: char = '▓';
const CH_DOT: char = '•';
const CH_CIRCLE_FILLED: &str = "⬤";
const CH_ARROW_UP: &str = "▲";

pub struct TerminalCanvas {
    pub width: usize,
    pub height: usize,
}

impl TerminalCanvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    // ────────────────────────────────────────
    //  UTILITAS LAYAR
    // ────────────────────────────────────────

    pub fn clear_screen(&self) {
        print!("\x1b[2J\x1b[H\x1b[?25l");
        let _ = stdout().flush();
    }

    pub fn restore_cursor(&self) {
        println!("\x1b[?25h\x1b[0m");
        let _ = stdout().flush();
    }

    pub fn show_cursor(&self) {
        print!("\x1b[?25h");
        let _ = stdout().flush();
    }

    pub fn move_to(&self, row: usize, col: usize) {
        print!("\x1b[{};{}H", row, col);
    }

    pub fn clear_region(&self, top: usize, left: usize, w: usize, h: usize) {
        let blank = " ".repeat(w);
        for r in top..top + h {
            self.move_to(r, left);
            print!("{blank}");
        }
    }

    // ═══════════════════════════════════════
    //  PANEL PLUGIN — Frame Bergaya Rack Unit
    // ═══════════════════════════════════════

    /// Gambar panel dengan border metal, title bar, dan optional status teks.
    /// Mirip tampilan hardware compressor/limiter di studio profesional.
    pub fn draw_panel(
        &self,
        top: usize,
        left: usize,
        w: usize,
        h: usize,
        title: &str,
        status: Option<&str>,
    ) {
        // Bilah judul
        self.move_to(top, left);
        print!("┌─ ");
        print!("{COL_ACCENT_CYAN}{title}{COL_RESET}");

        let title_width = title.chars().count() + 3;
        if w > title_width + 2 {
            print!("{COL_METAL}{}", "─".repeat(w - title_width));
        }
        print!("┐");

        // Status text di kanan atas panel
        if let Some(st) = status {
            let st_width = st.chars().count();
            if left + w > st_width + 3 {
                self.move_to(top, left + w.saturating_sub(st_width + 3));
                print!("{COL_DIM}{st}{COL_RESET}");
            }
        }

        // Border samping — garis metal
        for r in (top + 1)..(top + h - 1) {
            self.move_to(r, left);
            print!("{COL_METAL}│{COL_RESET}");
            self.move_to(r, left + w - 1);
            print!("{COL_METAL}│{COL_RESET}");
        }

        // Border bawah
        self.move_to(top + h - 1, left);
        print!("└─{COL_METAL}{}", "─".repeat(w - 2));
        print!("┘{COL_RESET}");
    }

    // ═══════════════════════════════════════
    //  VU METER LED — Gaya Hardware Outboard
    // ═══════════════════════════════════════

    /// Render VU Meter bergaya plugin DAW lengkap:
    /// - Bar gradien hijau → kuning → merah (segmen LED)
    /// - Peak hold indicator (⬤) yang persist antar-frame
    /// - Panah arah (▲/▼) menunjukkan level naik/turun
    /// - Nilai dBFS digital di samping
    pub fn draw_vu_meter(
        &self,
        row: usize,
        col: usize,
        label: &str,
        current_linear: f32,
        peak_linear: f32,
        db_value: f32,
        width: usize,
    ) {
        self.move_to(row, col);

        let clamped = current_linear.min(1.0_f32.max(current_linear));
        let filled = ((clamped) * (width as f32)).round() as usize;
        let empty = width.saturating_sub(filled);

        // Bangun bar VU gradien per segmen
        let mut bar = String::new();
        for i in 0..filled {
            let ratio = i as f32 / width as f32;
            bar.push_str(match ratio {
                0.0..=0.55 => COL_VU_LOW,
                0.55..=0.80 => COL_VU_MID,
                0.80..=0.95 => COL_VU_HIGH,
                _ => COL_VU_PEAK,
            });
            bar.push(CH_FULL_BLOCK);
            bar.push_str(COL_RESET);
        }
        // Bagian belakang kosong
        for _ in 0..empty {
            bar.push('░');
        }

        // Indikator peak hold
        let peak_indicator = if peak_linear > 1e-4 && peak_linear <= 1.0 {
            format!("{COL_ACCENT_RED}{CH_CIRCLE_FILLED}{COL_RESET}")
        } else {
            "".to_string()
        };

        // Panah arah
        let arrow_db = 20.0 * current_linear.log10();
        let arrow = if arrow_db > -1.0 {
            format!("{COL_ACCENT_RED}{CH_ARROW_UP}{COL_RESET}")
        } else if arrow_db < -40.0 {
            format!("{COL_DIM}▼{COL_RESET}")
        } else {
            format!("{COL_DIM}▲{COL_RESET}")
        };

        print!(
            "{COL_TEXT_BRIGHT}{label}{COL_RESET} [{bar}] {peak_indicator} {arrow} \x1b[1;37m{db_value:>6.1} dBFS\x1b[0m"
        );
    }

    /// Versi ringkas tanpa peak hold & arrow
    pub fn draw_vu_simple(
        &self,
        row: usize,
        col: usize,
        label: &str,
        level_linear: f32,
        width: usize,
    ) {
        let db = if level_linear < 1e-4 {
            -90.0
        } else {
            20.0 * level_linear.log10()
        };
        self.draw_vu_meter(row, col, label, level_linear, 0.0, db, width);
    }

    // ═══════════════════════════════════════
    //  ANALISIS SPEKTRUM MULUS — Spektrum Bergerak
    // ═══════════════════════════════════════

    /// Spectrum analyzer vertikal dengan tepi mulus (antialiasing).
    /// Setiap kolom merepresentasikan satu band frekuensi.
    /// Menggunakan █ penuh + ▀ setengah atas untuk kesan halus seperti
    /// hardware spectrum analyzer analog.
    pub fn draw_smooth_spectrum(
        &self,
        top: usize,
        left: usize,
        height: usize,
        bands: &[(f32, &str)],
    ) {
        let max_mag = bands.iter().map(|b| b.0).fold(1e-4_f32, |a, b| a.max(b));

        for row in 0..height {
            // y_frac = 1.0 di puncak, turun ke 0 di bawah
            let y_frac = 1.0 - (row as f32 / height as f32);
            self.move_to(top + row, left);

            for &(mag, _) in bands {
                let norm = mag / max_mag;
                if norm >= y_frac {
                    // Bar aktif — warna berdasarkan ketinggian
                    let color = match norm {
                        x if x <= 0.4 => COL_VU_LOW,
                        x if x <= 0.65 => COL_VU_MID,
                        x if x <= 0.85 => COL_VU_HIGH,
                        _ => COL_VU_PEAK,
                    };
                    print!("{color}{CH_FULL_BLOCK}{COL_RESET}");
                } else if (norm - y_frac).abs() < 0.18 && norm > 0.05 {
                    // Setengah blok di tepi atas = efek anti-aliasing
                    print!("{COL_METAL}{CH_HALF_TOP}{COL_RESET}");
                } else {
                    // Titik grid latar
                    print!("{COL_METAL}·{COL_RESET}");
                }
            }
        }

        // Label frekuensi
        self.move_to(top + height, left);
        for &(mag, lbl) in bands {
            let norm = mag / max_mag;
            let color = if norm > 0.85 {
                COL_ACCENT_RED
            } else if norm > 0.65 {
                COL_VU_HIGH
            } else if norm > 0.4 {
                COL_VU_MID
            } else {
                COL_DIM
            };
            print!("{color}{lbl:^5}{COL_RESET}");
        }
    }

    // ═══════════════════════════════════════
    //  TAMPILAN WAVEFORM — Mini Time-Domain
    // ═══════════════════════════════════════

    /// Render mini waveform horizontal dalam area tertentu.
    /// Setiap sample direpresentasikan sebagai titik • pada posisi Y sesuai amplitude.
    pub fn draw_waveform(
        &self,
        top: usize,
        left: usize,
        width: usize,
        height: usize,
        samples: &[f32],
    ) {
        if samples.is_empty() {
            return;
        }

        // Resample-down agar muat lebar
        let step = samples.len().max(1) / (width + 1);
        let mid_row = top + height / 2;

        for col in 0..width.min(samples.len()) {
            let si = col * step;
            if si >= samples.len() {
                break;
            }
            let val = samples[si].clamp(-1.0, 1.0).abs();
            let offset = (val * (height as f32 / 2.0)).round() as isize;
            let target_row = (mid_row as isize) - offset;

            if target_row >= top as isize && target_row < (top + height) as isize {
                self.move_to(target_row as usize, left + col);
                print!("{COL_ACCENT_CYAN}{CH_DOT}{COL_RESET}");
            }
        }
    }

    // ═══════════════════════════════════════
    //  METER ISIAN BERGULIR — Hardware Outboard
    // ═══════════════════════════════════════════

    /// Rolling/falling level meter bergaya hardware mixer.
    /// Level turun secara gradual memberi kesan "live".
    pub fn draw_rolling_meter(
        &self,
        top: usize,
        left: usize,
        width: usize,
        height: usize,
        value: f32,
        peak_value: f32,
        label: &str,
    ) {
        self.move_to(top, left);
        print!("{COL_TEXT_BRIGHT}{label}{COL_RESET}");

        let cur_filled = (value * height as f32).round() as usize;
        let pk_filled = (peak_value * height as f32).round() as usize;

        for row in 0..height {
            self.move_to(top + height - row, left + width.saturating_sub(12));
            let is_peak = row == pk_filled || row == pk_filled + 1;

            if is_peak {
                print!("{COL_ACCENT_RED}{CH_CIRCLE_FILLED}{COL_RESET}");
            } else if row <= cur_filled {
                let ratio = row as f32 / height as f32;
                let c = match ratio {
                    x if x <= 0.55 => "\x1b[38;5;46m",
                    x if x <= 0.80 => "\x1b[38;5;226m",
                    _ => "\x1b[1;31m",
                };
                print!("{c}{CH_SOLID_BAR}{COL_RESET}");
            } else if row <= cur_filled + 2 {
                print!("{COL_METAL}░{COL_RESET}");
            } else {
                print!("{COL_METAL}·{COL_RESET}");
            }
        }

        let db = if value < 1e-4 {
            "-inf".to_string()
        } else {
            format!("{:.1}", 20.0 * value.log10())
        };
        self.move_to(top + height + 1, left + width.saturating_sub(12));
        print!("{COL_TEXT_BRIGHT}{db:>5} dB{COL_RESET}");
    }

    // ═══════════════════════════════════════
    //  METER PAN STEREO
    // ═══════════════════════════════════════

    /// Indikator stereo pan/phase balance horizontal.
    pub fn draw_stereo_pan(
        &self,
        row: usize,
        col: usize,
        pan_left: f32,
        _pan_right: f32,
        width: usize,
    ) {
        self.move_to(row, col);
        let center = width / 2;

        for i in 0..width {
            if i == center {
                print!("{COL_TEXT_BRIGHT}┼{COL_RESET}");
            } else if i < center {
                // Sisi kiri
                let dist_from_center = center - i;
                let intensity = if pan_left > 0.0 {
                    (pan_left * dist_from_center as f32 / center as f32).min(1.0)
                } else {
                    0.0
                };
                if intensity > 0.3 {
                    print!("{COL_ACCENT_GREEN}{CH_SOLID_BAR}{COL_RESET}");
                } else {
                    print!("{COL_METAL}·{COL_RESET}");
                }
            } else {
                // Sisi kanan
                let dist_from_center = i - center;
                let intensity = if pan_left < 0.0 {
                    (-pan_left * dist_from_center as f32 / center as f32).min(1.0)
                } else {
                    0.0
                };
                if intensity > 0.3 {
                    print!("{COL_ACCENT_CYAN}{CH_SOLID_BAR}{COL_RESET}");
                } else {
                    print!("{COL_METAL}·{COL_RESET}");
                }
            }
        }
    }

    // ═══════════════════════════════════════
    //  VISUALISER SLIDER — Kontrol Parameter
    // ═══════════════════════════════════════

    /// Visualisasi slider/knob horizontal bergaya mixer untuk parameter EQ atau gain.
    pub fn draw_h_slider(
        &self,
        row: usize,
        col: usize,
        value_pct: f32, // [0..100]
        min_val: f32,
        max_val: f32,
        label: &str,
        width: usize,
    ) {
        self.move_to(row, col);
        let pct = value_pct.clamp(0.0, 100.0) / 100.0;
        let filled = (pct * (width - 2) as f32).round() as usize;

        // Kurung track
        print!("{COL_METAL}[{COL_RESET}");
        // Isian
        print!("{COL_ACCENT_CYAN}{}", "━".repeat(filled));
        // Kosong
        print!("{}", "·".repeat((width - 2).saturating_sub(filled)));
        // Kurung penutup
        print!("{COL_METAL}]{COL_RESET}");

        // Nilai numerik
        let val = min_val + (max_val - min_val) * pct;
        let db_or_val = if label.contains("dB") || label.contains("Gain") || label.contains("EQ") {
            format!("{val:+6.1} dB")
        } else {
            format!("{val:.0}")
        };
        self.move_to(row, col + width + 2);
        print!("{COL_TEXT_BRIGHT}{db_or_val}{COL_RESET}");

        // Label kiri
        self.move_to(row, col.saturating_sub(label.chars().count() + 2));
        print!("{COL_DIM}{label}{COL_RESET}");
    }

    // ═══════════════════════════════════════
    //  INDIKATOR STATUS PEMROSESAN
    // ═══════════════════════════════════════

    /// Badge status proses DSP aktif — tampil seperti indikator di hardware compressor.
    pub fn draw_processing_status(&self, row: usize, col: usize, active_tags: &[&str]) {
        self.move_to(row, col);
        for tag in active_tags {
            let (text, color) = match tag.to_ascii_lowercase().as_str() {
                "comp" | "compressor" => ("COMPRESS".to_string(), COL_ACCENT_AMBER),
                "limit" | "limiter" => ("LIMIT".to_string(), COL_ACCENT_RED),
                "eq" | "equalizer" => ("EQ".to_string(), COL_ACCENT_CYAN),
                _ => (tag.to_ascii_uppercase(), COL_METAL_LIGHT),
            };
            print!("{color}[{text}]{COL_RESET} ");
        }
    }

    // ═══════════════════════════════════════
    //  HEADER / BILAH JUDUL
    // ═══════════════════════════════════════

    /// Header bergaya judul plugin DAW dengan info teknis sistem.
    pub fn draw_header(
        &self,
        row: usize,
        col: usize,
        title: &str,
        sample_rate: u32,
        channels: usize,
    ) {
        // Icon box kiri
        self.move_to(row, col);
        print!("{COL_ACCENT_CYAN}╔═╗{COL_RESET}");

        // Cetak judul karakter per karakter (hindari masalah hitung multi-byte)
        for (i, ch) in title.chars().enumerate() {
            self.move_to(row, col + 5 + i);
            print!("{COL_TEXT_BRIGHT}{ch}{COL_RESET}");
        }

        // Garis pemisah
        let after_title = col + 5 + title.chars().count();
        self.move_to(row, after_title);
        print!("{COL_METAL}╚═══════════════════════════════════════════════════{COL_RESET}");

        // Bilah sub-info
        self.move_to(row + 1, col + 5);
        print!(
            "{COL_DIM}Sample Rate: {COL_TEXT_BRIGHT}{sample_rate} Hz{COL_RESET}  │  Channels: {COL_TEXT_BRIGHT}{channels}{COL_RESET}  │  Format: {COL_TEXT_BRIGHT}f32[-1..1]{COL_RESET}"
        );
    }

    // ═══════════════════════════════════════
    //  RENDERER UTILITAS
    // ═══════════════════════════════════════

    pub fn draw_stat(&self, row: usize, col: usize, label: &str, value: &str) {
        self.move_to(row, col);
        print!("{COL_DIM}{label}:{COL_RESET} {COL_TEXT_BRIGHT}{value}{COL_RESET}");
    }

    pub fn draw_separator(&self, row: usize, col: usize, width: usize) {
        self.move_to(row, col);
        print!("{COL_METAL}{}{COL_RESET}", "─".repeat(width));
    }

    // ────────────────────────────────────────
    //  KOMPATIBILITAS MUNDUR — API LEGACY
    // ────────────────────────────────────────
    // Method lama masih tersedia agar tidak pecah saat build.

    pub fn draw_box(&self, top: usize, left: usize, w: usize, h: usize, title: &str) {
        self.draw_panel(top, left, w, h, title, None);
    }

    pub fn draw_level_meter(
        &self,
        row: usize,
        col: usize,
        label: &str,
        peak: f32,
        rms: f32,
        width: usize,
    ) {
        self.draw_vu_simple(row, col, label, peak, width);
        let label_cols = label.chars().count();
        // Sisakan dua kolom pemisah dari bacaan peak dBFS di atasnya.
        self.move_to(row, col + label_cols + width + 20);
        let rms_db = if rms < 1e-4 {
            -90.0
        } else {
            20.0 * rms.log10()
        };
        print!("{COL_DIM}RMS {:>5.1} dBFS{COL_RESET}", rms_db);
    }

    pub fn draw_fft_spectrum(&self, top: usize, left: usize, height: usize, bands: &[(f32, &str)]) {
        self.draw_smooth_spectrum(top, left, height, bands);
    }
}
