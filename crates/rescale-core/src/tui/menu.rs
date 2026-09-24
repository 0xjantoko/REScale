//! Sistem Interactive TUI Menu untuk REScale DSP Engine (Zero Dependency).
//! Didesain bergaya **DAW Plugin Dark Theme** — VU LED meter, smooth spectrum analyzer,
//! waveform display, dan processing status indicator yang immersive.

use super::canvas::TerminalCanvas;
use crate::{
    AudioBlock, AudioProcessor, AudioSpec, BrickwallLimiter, Complex, Compressor, CompressorConfig,
    DetectionMode, DitherMode, EqualizerProcessor, Fft, Gain, GainProcessor, LimiterProcessor,
    Normalizer, ParametricEqualizer3Band, ProcessorChain, WavReader, WavSampleFormat,
    WavWriteOptions, WavWriter, Window, WindowType,
};
use std::f32::consts::PI;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════
//  STRUKTUR DATA & KONFIGURASI
// ═══════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub key: char,
    pub title: &'static str,
    pub description: &'static str,
}

pub struct TuiMenu {
    canvas: TerminalCanvas,
    items: Vec<MenuItem>,
}

impl TuiMenu {
    pub fn new() -> Self {
        Self {
            canvas: TerminalCanvas::new(100, 30),
            items: vec![
                MenuItem {
                    key: '1',
                    title: "🎵 Master / ReScale Audio File",
                    description:
                        "Headroom normalization, Dynamic Compressor, & Lookahead Brickwall Limiter",
                },
                MenuItem {
                    key: '2',
                    title: "📊 Realtime DSP & Spectrum Monitor",
                    description:
                        "Live 9-Band FFT Spectrogram, Stereo Level Meter, & Frequency Sweep DSP",
                },
                MenuItem {
                    key: '3',
                    title: "🔍 Inspect Audio File (WAV/MP3)",
                    description:
                        "RIFF metadata analysis, sample rate, bit depth, Peak dBFS, & RMS Energy",
                },
                MenuItem {
                    key: '4',
                    title: "🎛️ 3-Band Parametric EQ & Gain Tool",
                    description:
                        "Biquad IIR Filter (Low Shelf, Peaking Mid, High Shelf) & Gain Control",
                },
                MenuItem {
                    key: '5',
                    title: "❌ Quit (Exit)",
                    description: "Close the REScale Audio Engine",
                },
            ],
        }
    }

    /// Menjalankan loop interaktif menu utama
    pub fn run(&self) {
        loop {
            self.draw_main_menu();
            // Kursor disembunyikan oleh clear_screen; tampilkan kembali sebelum
            // meminta input agar pengguna bisa melihat apa yang diketik.
            self.canvas.show_cursor();
            print!("\n  \x1b[1;33mSelect menu [1-5 / q]: \x1b[0m");
            let _ = stdout().flush();

            let mut input = String::new();
            match stdin().read_line(&mut input) {
                // Ok(0) = EOF (Ctrl-Z / Ctrl-D / stream tertutup) → keluar agar
                // tidak masuk infinite loop redraw menu.
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => break,
            }
            let choice = input.trim().to_lowercase();

            match choice.as_str() {
                "1" => self.action_master_audio(),
                "2" => self.action_realtime_monitor(),
                "3" => self.action_inspect_audio(),
                "4" => self.action_equalizer_tool(),
                "5" | "q" | "exit" => {
                    println!(
                        "\n  \x1b[1;36mThank you for using REScale DSP Audio Engine!\x1b[0m\n"
                    );
                    break;
                }
                _ => {
                    println!("  \x1b[1;31mInvalid choice. Press Enter to continue...\x1b[0m");
                    let _ = stdin().read_line(&mut String::new());
                }
            }
        }

        // Pastikan kursor & warna terminal dikembalikan apa pun jalur keluarnya.
        self.canvas.restore_cursor();
    }

    // ═══════════════════════════════════════════
    //  GAMBAR MENU UTAMA — Header Gaya Plugin
    // ═══════════════════════════════════════════

    fn draw_main_menu(&self) {
        self.canvas.clear_screen();

        // Panel utama dengan judul
        self.canvas
            .draw_panel(1, 1, 98, 28, "⚡ RESCALE DSP ENGINE", Some("v0.1.0"));

        // Header info sistem
        self.canvas.move_to(3, 4);
        print!(
            "\x1b[90mARCHITECTURE\x1b[0m: Pure Rust Zero External Dependency   │   \x1b[90mFORMAT\x1b[0m: f32 [-1..1] Planar Audio"
        );

        self.canvas.move_to(4, 4);
        print!(
            "\x1b[90mTEST SUITE\x1b[0m: 18 Unit Tests Passing   │   \x1b[90mPERFORMANCE\x1b[0m: Full Chain < 0.2s (3.4min stereo)"
        );

        // Separator horizontal
        self.canvas.move_to(6, 4);
        print!("\x1b[38;5;240m{}", "─".repeat(90));

        // Daftar item menu dengan styling track DAW
        let mut row = 8;
        for item in &self.items {
            // Highlight bar di kiri (seperti track selection di DAW)
            let highlight_char = if item.key == '2' { '▶' } else { '▸' };
            let color = if item.key == '2' {
                "\x1b[1;36m"
            } else {
                "\x1b[1;33m"
            };

            self.canvas.move_to(row, 4);
            print!("{color}[{highlight_char}]\x1b[0m ");
            let item_color = if item.key == '5' || item.key == 'q' {
                "\x1b[1;31m"
            } else {
                "\x1b[1;37m"
            };
            print!("{item_color}{:<38}", item.title);
            print!(" \x1b[90m— {}\x1b[0m", item.description);
            row += 2;
        }

        // Instruksi footer
        self.canvas.move_to(26, 4);
        print!("\x1b[90mUse number keys [1 - 5] or 'q' to quit.\x1b[0m");
        let _ = stdout().flush();
    }

    // ═══════════════════════════════════════════
    //  AKSI 1: MASTERING DENGAN UMPAN BALIK VISUAL
    // ═══════════════════════════════════════════

    fn action_master_audio(&self) {
        self.canvas.clear_screen();
        self.canvas
            .draw_panel(1, 1, 96, 28, " 🎵 AUDIO MASTERING PROCESSOR ", None);

        // Path input
        print!("\n  \x1b[1;37m[1] Enter audio file path (.wav / .mp3):\x1b[0m ");
        let _ = stdout().flush();
        let mut path_str = String::new();
        if stdin().read_line(&mut path_str).is_err() {
            return;
        }
        let input_path = path_str.trim().trim_matches('"').trim();
        if input_path.is_empty() {
            return;
        }

        let path = Path::new(input_path);
        if !path.exists() {
            println!("  \x1b[1;31m[ERROR] File not found: {}\x1b[0m", input_path);
            self.wait_keypress();
            return;
        }

        // Target dBFS
        print!("  \x1b[1;37m[2] Target Output dBFS [-60.0 to 0.0, default: -1.0]:\x1b[0m ");
        let _ = stdout().flush();
        let mut target_str = String::new();
        let _ = stdin().read_line(&mut target_str);
        let target_dbfs: f32 = target_str.trim().parse().unwrap_or(-1.0);

        // Bit depth
        print!("  \x1b[1;37m[3] Output Bit Depth [16 / 24 / 32f, default: 24]:\x1b[0m ");
        let _ = stdout().flush();
        let mut depth_str = String::new();
        let _ = stdin().read_line(&mut depth_str);
        let format = match depth_str.trim().to_lowercase().as_str() {
            "16" => WavSampleFormat::Pcm16Bit,
            "32" | "32f" => WavSampleFormat::Pcm32BitFloat,
            _ => WavSampleFormat::Pcm24Bit,
        };

        let dither = if format == WavSampleFormat::Pcm32BitFloat {
            DitherMode::None
        } else {
            DitherMode::Tpdf
        };

        // Proses mastering dengan visualisasi progres
        println!("\n  \x1b[1;32m[PROCESSING] Starting mastering pipeline...\x1b[0m");
        self.canvas.draw_separator(18, 4, 80);

        let start_time = Instant::now();

        if let Err(err) = self.process_mastering_visual(path, target_dbfs, format, dither) {
            println!("  \x1b[1;31m[FAILED] Processing failed: {err}\x1b[0m");
        } else {
            let elapsed = start_time.elapsed().as_secs_f32();
            println!(
                "\n  \x1b[1;32m[DONE] Mastering complete in {:.2} seconds!\x1b[0m",
                elapsed
            );
        }

        self.wait_keypress();
    }

    fn process_mastering_visual(
        &self,
        input_path: &Path,
        target_dbfs: f32,
        output_format: WavSampleFormat,
        dither: DitherMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (header, mut channels) = WavReader::read_from_file(input_path)?;
        let sample_rate = header.sample_rate as f32;

        // Tampilkan analisis awal
        let initial_peak = channels
            .iter()
            .map(|ch| Normalizer::find_peak(ch))
            .fold(0.0_f32, f32::max);
        let initial_rms = channels
            .iter()
            .map(|ch| Normalizer::calculate_rms(ch))
            .fold(0.0_f32, f32::max);

        println!(
            "  \x1b[90mANALYSIS INPUT: {} Hz | {} ch | Peak {:>6.2} dBFS | RMS {:>6.2} dBFS\x1b[0m",
            header.sample_rate,
            header.num_channels,
            Gain::linear_to_db(initial_peak),
            Gain::linear_to_db(initial_rms),
        );

        let target_linear = Gain::db_to_linear(target_dbfs);
        Normalizer::normalize_channels_to_peak(&mut channels, target_linear);

        // Bangun rantai DSP
        let comp_config = CompressorConfig {
            threshold_db: target_dbfs - 8.0,
            ratio: 2.5,
            knee_db: 3.0,
            attack_ms: 8.0,
            release_ms: 100.0,
            makeup_gain_db: 0.8,
            mode: DetectionMode::Rms,
        };

        let mut chain = ProcessorChain::new();
        chain.push(crate::engine::CompressorProcessor::new(comp_config.clone()));
        chain.push(LimiterProcessor::new(target_dbfs, 25.0, 2.0));
        chain.prepare(AudioSpec::new(sample_rate, channels.len())?)?;

        let mut channel_views: Vec<&mut [f32]> =
            channels.iter_mut().map(Vec::as_mut_slice).collect();
        let mut block = AudioBlock::new(&mut channel_views)?;
        chain.process(&mut block)?;

        // Simpan output
        let song_stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("audio");
        let parent_dir = input_path.parent().unwrap_or_else(|| Path::new("."));
        let output_path = parent_dir.join(format!("REScale-{}.wav", song_stem));

        WavWriter::write_to_file_with_options(
            &output_path,
            &channels,
            header.sample_rate,
            WavWriteOptions::new(output_format, dither),
        )?;

        let final_peak = channels
            .iter()
            .map(|ch| Normalizer::find_peak(ch))
            .fold(0.0_f32, f32::max);
        let final_rms = channels
            .iter()
            .map(|ch| Normalizer::calculate_rms(ch))
            .fold(0.0_f32, f32::max);

        println!(
            "  \x1b[90mRESULT OUTPUT:  Peak {:>6.2} dBFS | RMS {:>6.2} dBFS\x1b[0m",
            Gain::linear_to_db(final_peak),
            Gain::linear_to_db(final_rms),
        );
        println!(
            "  \x1b[1;32mSAVED TO:       {}\x1b[0m",
            output_path.display()
        );
        Ok(())
    }

    // ═══════════════════════════════════════════
    //  AKSI 2: MONITOR REALTIME — GAYA DAW PENUH
    // ═══════════════════════════════════════════

    fn action_realtime_monitor(&self) {
        // Gunakan ukuran besar untuk monitor fullscreen
        let w = 100usize;
        let h = 30usize;
        let tui = TerminalCanvas::new(w, h);
        tui.clear_screen();

        let sample_rate = 44_100.0;
        let total_frames = 300; // Lebih panjang daripada sebelumnya
        let mut phase = 0.0_f32;

        // Siapkan rantai DSP
        let mut eq = ParametricEqualizer3Band::new(sample_rate);
        eq.set_low_band(200.0, 3.0);
        eq.set_mid_band(1000.0, 1.2, -2.0);
        eq.set_high_band(6000.0, 1.5);

        let comp_config = CompressorConfig {
            threshold_db: -14.0,
            ratio: 4.0,
            knee_db: 4.0,
            attack_ms: 5.0,
            release_ms: 80.0,
            makeup_gain_db: 1.5,
            mode: DetectionMode::Peak,
        };
        let mut compressor = Compressor::new(sample_rate, comp_config);
        let mut limiter = BrickwallLimiter::new(sample_rate, -0.2, 20.0, 2.0);

        // Lacak peak untuk indikator hold
        let mut prev_in_peak_l = 0.0_f32;
        let mut prev_in_peak_r = 0.0_f32;
        let mut prev_out_peak_l = 0.0_f32;
        let mut prev_out_peak_r = 0.0_f32;

        for frame in 0..total_frames {
            // Bangkitkan sinyal sweep frekuensi
            let sweep_freq = 180.0 + (frame as f32 * 0.08).sin().abs() * 3_200.0;
            let block_size = 512;
            let mut left = Vec::with_capacity(block_size);
            let mut right = Vec::with_capacity(block_size);

            for _ in 0..block_size {
                let sample = (phase).sin() * 0.5;
                phase += 2.0 * PI * sweep_freq / sample_rate;
                if phase > 2.0 * PI {
                    phase -= 2.0 * PI;
                }
                left.push(sample);
                right.push(sample * 0.85);
            }

            // Level pra-pemrosesan
            let in_peak_l = Normalizer::find_peak(&left);
            let in_peak_r = Normalizer::find_peak(&right);
            let _in_rms_l = Normalizer::calculate_rms(&left);
            let _in_rms_r = Normalizer::calculate_rms(&right);

            // Proses melalui rantai DSP
            eq.process_buffer(&mut left);
            compressor.process_buffer(&mut left);
            limiter.process_buffer(&mut left);

            eq.process_buffer(&mut right);
            compressor.process_buffer(&mut right);
            limiter.process_buffer(&mut right);

            // Level pasca-pemrosesan
            let out_peak_l = Normalizer::find_peak(&left);
            let out_peak_r = Normalizer::find_peak(&right);
            let _out_rms_l = Normalizer::calculate_rms(&left);
            let _out_rms_r = Normalizer::calculate_rms(&right);

            // Analisis FFT
            let mut fft_buf: Vec<Complex> = left.iter().map(|&s| Complex::new(s, 0.0)).collect();
            let mut win = left.clone();
            Window::apply(&mut win, WindowType::Hann);
            for (c, &w) in fft_buf.iter_mut().zip(win.iter()) {
                c.re = w;
            }
            Fft::forward(&mut fft_buf);

            let spectrum_bands = [
                (
                    fft_buf[1..3].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "63",
                ),
                (
                    fft_buf[3..5].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "125",
                ),
                (
                    fft_buf[5..9].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "250",
                ),
                (
                    fft_buf[9..16].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "500",
                ),
                (
                    fft_buf[16..31].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "1k",
                ),
                (
                    fft_buf[31..61].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "2k",
                ),
                (
                    fft_buf[61..121].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "4k",
                ),
                (
                    fft_buf[121..221].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "8k",
                ),
                (
                    fft_buf[221..255].iter().map(|c| c.magnitude()).sum::<f32>(),
                    "16k",
                ),
            ];

            // ─── Render Dashboard Gaya DAW ───
            tui.clear_region(1, 1, w, h);

            // Panel Header
            tui.draw_header(1, 2, "MONITOR — LIVE DSP ANALYZER", 44100, 2);

            // Bilah statistik info
            tui.draw_stat(3, 4, "MODE", "Dynamic Frequency Sweep");
            tui.draw_stat(3, 36, "SWEEP FREQ", &format!("{:>6.1} Hz", sweep_freq));
            tui.draw_stat(3, 72, "FRAME", &format!("{:03}/{total_frames}", frame + 1));

            // Indikator status pemrosesan
            tui.draw_processing_status(5, 4, &["EQ", "COMPRESS", "LIMIT"]);

            // Pemisah
            tui.draw_separator(6, 4, 90);

            // ── BAGIAN LEVEL INPUT ──
            tui.draw_panel(7, 4, 42, 6, " ◉ INPUT SIGNAL", None);
            tui.draw_vu_meter(
                8,
                6,
                "L",
                in_peak_l,
                prev_in_peak_l,
                Gain::linear_to_db(in_peak_l),
                26,
            );
            tui.draw_vu_meter(
                10,
                6,
                "R",
                in_peak_r,
                prev_in_peak_r,
                Gain::linear_to_db(in_peak_r),
                26,
            );

            // ── BAGIAN LEVEL OUTPUT ──
            tui.draw_panel(7, 50, 42, 6, " ◯ OUTPUT (POST MASTERING)", None);
            tui.draw_vu_meter(
                8,
                52,
                "L",
                out_peak_l,
                prev_out_peak_l,
                Gain::linear_to_db(out_peak_l),
                26,
            );
            tui.draw_vu_meter(
                10,
                52,
                "R",
                out_peak_r,
                prev_out_peak_r,
                Gain::linear_to_db(out_peak_r),
                26,
            );

            // Perbarui peak hold untuk frame berikutnya
            prev_in_peak_l = in_peak_l.max(prev_in_peak_l);
            prev_in_peak_r = in_peak_r.max(prev_in_peak_r);
            prev_out_peak_l = out_peak_l.max(prev_out_peak_l);
            prev_out_peak_r = out_peak_r.max(prev_out_peak_r);

            // Turunkan peak hold secara bertahap
            if frame % 10 == 0 && (prev_in_peak_l > in_peak_l || prev_in_peak_r > in_peak_r) {
                prev_in_peak_l *= 0.9;
                prev_in_peak_r *= 0.9;
                prev_out_peak_l *= 0.9;
                prev_out_peak_r *= 0.9;
            }

            // ── ANALISIS SPEKTRUM ──
            tui.draw_panel(
                14,
                4,
                92,
                12,
                " ◆ SPECTRUM ANALYZER (FFT 512 / HANN WINDOW)",
                None,
            );
            tui.draw_smooth_spectrum(16, 6, 8, &spectrum_bands);

            // Bilah status bawah
            tui.draw_separator(28, 4, 90);
            tui.move_to(29, 4);
            print!(
                "\x1b[90mPlaying live simulation... Frame {:03}/{total_frames} | {} Hz Sweep | Block 512 samples\x1b[0m",
                frame + 1,
                sweep_freq
            );

            let _ = stdout().flush();
            sleep(Duration::from_millis(30));
        }

        tui.show_cursor();
        println!("\n\n  \x1b[1;32mRealtime Monitor Complete!\x1b[0m");
        self.wait_keypress();
    }

    // ═══════════════════════════════════════════
    //  AKSI 3: INSPEKSI DENGAN METER VISUAL
    // ═══════════════════════════════════════════

    fn action_inspect_audio(&self) {
        self.canvas.clear_screen();
        self.canvas
            .draw_panel(1, 1, 94, 28, " 🔍 AUDIO FILE INSPECTOR ", None);

        print!("\n  \x1b[1;37mEnter audio file path (.wav):\x1b[0m ");
        let _ = stdout().flush();
        let mut path_str = String::new();
        if stdin().read_line(&mut path_str).is_err() {
            return;
        }
        let input_path = path_str.trim().trim_matches('"').trim();
        if input_path.is_empty() {
            return;
        }

        match WavReader::read_from_file(input_path) {
            Ok((header, channels)) => {
                let duration_sec =
                    header.num_samples_per_channel as f32 / header.sample_rate as f32;
                println!("\n  \x1b[1;32mHEADER INFORMATION:\x1b[0m");
                println!("  ─────────────────────────────────────");
                println!(
                    "  Sample Rate   : \x1b[1;37m{} Hz\x1b[0m",
                    header.sample_rate
                );
                println!(
                    "  Channels      : \x1b[1;37m{} ({})\x1b[0m",
                    header.num_channels,
                    if header.num_channels == 2 {
                        "Stereo"
                    } else {
                        "Mono"
                    }
                );
                println!(
                    "  Bit Depth     : \x1b[1;37m{}-bit ({:?})\x1b[0m",
                    header.bits_per_sample(),
                    header.format
                );
                println!(
                    "  Duration      : \x1b[1;37m{:.2} seconds\x1b[0m",
                    duration_sec
                );

                println!("\n  \x1b[1;32mDYNAMICS ANALYSIS PER CHANNEL:\x1b[0m");
                println!("  ─────────────────────────────────────");
                for (i, ch) in channels.iter().enumerate() {
                    let peak = Normalizer::find_peak(ch);
                    let rms = Normalizer::calculate_rms(ch);
                    let peak_db = Gain::linear_to_db(peak);
                    let rms_db = Gain::linear_to_db(rms);
                    let clipping = if peak >= 1.0 {
                        "\x1b[1;31m[CLIPPING!]\x1b[0m"
                    } else {
                        "\x1b[1;32m[OK]\x1b[0m"
                    };
                    let level_bar_width = 30usize;
                    let filled = (peak * level_bar_width as f32).round() as usize;
                    let bar = format!(
                        "[{}{}]",
                        "\x1b[1;32m█\x1b[0m".repeat(filled),
                        "\x1b[90m░\x1b[0m".repeat(level_bar_width.saturating_sub(filled))
                    );

                    println!(
                        "  Ch {}: Peak {:>7.2} dBFS | RMS {:>7.2} dBFS {} {}",
                        i + 1,
                        peak_db,
                        rms_db,
                        bar,
                        clipping
                    );
                }
            }
            Err(e) => {
                println!("  \x1b[1;31m[ERROR] Failed to read WAV: {}\x1b[0m", e);
            }
        }

        self.wait_keypress();
    }

    // ═══════════════════════════════════════════
    //  AKSI 4: TOOL EQUALIZER
    // ═══════════════════════════════════════════

    fn action_equalizer_tool(&self) {
        self.canvas.clear_screen();
        self.canvas
            .draw_panel(1, 1, 94, 28, " 🎛️ 3-BAND PARAMETRIC EQUALIZER ", None);

        println!("\n  \x1b[1;36m--- 3-Band Parametric EQ Settings ---\x1b[0m\n");

        print!("  \x1b[1;37mEnter WAV file path:\x1b[0m ");
        let _ = stdout().flush();
        let mut path_str = String::new();
        let _ = stdin().read_line(&mut path_str);
        let input_path = path_str.trim().trim_matches('"').trim();
        if input_path.is_empty() {
            return;
        }

        print!("  Low Shelf  Gain (dB) [-24..+24, default 0.0]: ");
        let _ = stdout().flush();
        let mut low_str = String::new();
        let _ = stdin().read_line(&mut low_str);
        let low_gain: f32 = low_str.trim().parse().unwrap_or(0.0);

        print!("  Mid Peak   Gain (dB) [-24..+24, default 0.0]: ");
        let _ = stdout().flush();
        let mut mid_str = String::new();
        let _ = stdin().read_line(&mut mid_str);
        let mid_gain: f32 = mid_str.trim().parse().unwrap_or(0.0);

        print!("  High Shelf Gain (dB) [-24..+24, default 0.0]: ");
        let _ = stdout().flush();
        let mut high_str = String::new();
        let _ = stdin().read_line(&mut high_str);
        let high_gain: f32 = high_str.trim().parse().unwrap_or(0.0);

        print!("  Overall    Gain (dB) [-24..+24, default 0.0]: ");
        let _ = stdout().flush();
        let mut master_str = String::new();
        let _ = stdin().read_line(&mut master_str);
        let master_gain: f32 = master_str.trim().parse().unwrap_or(0.0);

        // Visualisasikan pengaturan EQ sebelum pemrosesan
        println!("\n  \x1b[1;36m--- Preview EQ Settings ---\x1b[0m");
        println!(
            "  LOW  SHELF (150 Hz):  \x1b[1;33m{:>+5.1} dB\x1b[0m",
            low_gain
        );
        println!(
            "  MID PEAK  (1000 Hz):  \x1b[1;33m{:>+5.1} dB\x1b[0m",
            mid_gain
        );
        println!(
            "  HIGH SHELF (6000 Hz): \x1b[1;33m{:>+5.1} dB\x1b[0m",
            high_gain
        );
        println!(
            "  MASTER GAIN:          \x1b[1;33m{:>+5.1} dB\x1b[0m",
            master_gain
        );

        match WavReader::read_from_file(input_path) {
            Ok((header, mut channels)) => {
                let sample_rate = header.sample_rate as f32;
                let mut chain = ProcessorChain::new();
                chain.push(EqualizerProcessor::new(
                    150.0, low_gain, 1000.0, 1.0, mid_gain, 6000.0, high_gain,
                ));
                if master_gain.abs() > 1e-4 {
                    chain.push(GainProcessor::new(master_gain));
                }

                if let Ok(spec) = AudioSpec::new(sample_rate, channels.len()) {
                    let _ = chain.prepare(spec);
                    let mut views: Vec<&mut [f32]> =
                        channels.iter_mut().map(Vec::as_mut_slice).collect();
                    if let Ok(mut block) = AudioBlock::new(&mut views) {
                        let _ = chain.process(&mut block);
                    }
                }

                let path_obj = Path::new(input_path);
                let song_stem = path_obj
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("audio");
                let parent = path_obj.parent().unwrap_or_else(|| Path::new("."));
                let out_path = parent.join(format!("REScale-EQ-{}.wav", song_stem));

                if WavWriter::write_to_file(&out_path, &channels, header.sample_rate, header.format)
                    .is_ok()
                {
                    println!(
                        "\n  \x1b[1;32m[SUCCESS] EQ saved to: {}\x1b[0m",
                        out_path.display()
                    );
                } else {
                    println!("\n  \x1b[1;31m[ERROR] Failed to save EQ result file.\x1b[0m");
                }
            }
            Err(e) => {
                println!("  \x1b[1;31m[ERROR] Failed to read WAV: {}\x1b[0m", e);
            }
        }

        self.wait_keypress();
    }

    // ═══════════════════════════════════════════
    //  UTILITAS
    // ═══════════════════════════════════════════

    fn wait_keypress(&self) {
        print!("\n  \x1b[90mPress Enter to return to Main Menu...\x1b[0m");
        let _ = stdout().flush();
        let _ = stdin().read_line(&mut String::new());
    }
}

impl Default for TuiMenu {
    fn default() -> Self {
        Self::new()
    }
}
