//! CLI Tool untuk ReScale / Normalize / Master File Audio menggunakan REScale DSP Engine

use rescale_core::{
    AudioBlock, AudioProcessor, AudioSpec, CompressorConfig, CompressorProcessor, DetectionMode,
    DitherMode, Gain, LimiterProcessor, Normalizer, ProcessorChain, TuiMenu, WavReader,
    WavSampleFormat, WavWriteOptions, WavWriter,
};
use std::env;
use std::io::{self, stdout, Write};
use std::path::Path;
use std::process::Command;

/// Inisialisasi konsol Windows: set code page ke UTF-8 + aktifkan ANSI VT processing.
///
/// Tanpa ini, karakter Unicode multi-byte yang dikeluarkan program (box-drawing
/// ┌─┐, simbol ✓ → █ ⬤) tampil sebagai karakter terkorupsi di konsol Windows
/// yang memakai code page lama (CP437/CP850), dan escape ANSI warna TUI tidak
/// dirender (muncul sebagai teks mentah `\x1b[...]`).
///
/// Zero dependency: hanya FFI ke `kernel32` milik sistem.
#[cfg(windows)]
fn setup_windows_console() {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n_std_handle: u32) -> *mut c_void;
        fn SetConsoleOutputCP(code_page: u32) -> i32;
        fn SetConsoleCP(code_page: u32) -> i32;
        fn GetConsoleMode(handle: *mut c_void, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: *mut c_void, mode: u32) -> i32;
    }

    // STD_OUTPUT_HANDLE = (DWORD)-11, STD_ERROR_HANDLE = (DWORD)-12, STD_INPUT_HANDLE = (DWORD)-10
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
    const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;
    const STD_INPUT_HANDLE: u32 = 0xFFFF_FFF6;
    const CP_UTF8: u32 = 65001;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;

    unsafe {
        // 1. Code page input & output -> UTF-8 (memperbaiki mojibake karakter Unicode).
        let _ = SetConsoleOutputCP(CP_UTF8);
        let _ = SetConsoleCP(CP_UTF8);

        // 2. Aktifkan ANSI escape sequences pada stdout & stderr (warna TUI,
        //    cursor hide/show, clear screen) tanpa mengubah mode yang sudah ada.
        for handle in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let h = GetStdHandle(handle);
            let mut mode: u32 = 0;
            if GetConsoleMode(h, &mut mode) != 0 {
                let _ = SetConsoleMode(
                    h,
                    mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT,
                );
            }
        }

        // Referensikan STD_INPUT_HANDLE agar input CRT juga mengikuti CP UTF-8
        // yang sudah di-set via SetConsoleCP; tidak diperlukan tindakan lebih lanjut.
        let _ = GetStdHandle(STD_INPUT_HANDLE);
    }
}

#[cfg(not(windows))]
fn setup_windows_console() {}

fn main() {
    setup_windows_console();
    if let Err(error) = run() {
        eprintln!("\x1b[1;31m[ERROR]\x1b[0m {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = CliOptions::parse(env::args().skip(1))?;
    if options.launch_menu {
        let menu = TuiMenu::new();
        menu.run();
        return Ok(());
    }
    if options.show_help {
        print_help();
        return Ok(());
    }

    run_rescale(&options)
}

struct CliOptions {
    input_file: String,
    target_dbfs: f32,
    output_format: WavSampleFormat,
    dither: DitherMode,
    no_dsp: bool,
    show_help: bool,
    launch_menu: bool,
}

impl CliOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let Some(first_arg) = args.next() else {
            return Ok(Self::interactive());
        };
        if first_arg == "--help" || first_arg == "-h" {
            return Ok(Self::help());
        }
        if first_arg == "--menu"
            || first_arg == "-m"
            || first_arg == "-i"
            || first_arg == "--interactive"
        {
            return Ok(Self::interactive());
        }

        let mut options = Self {
            input_file: first_arg,
            target_dbfs: -1.0,
            output_format: WavSampleFormat::Pcm24Bit,
            dither: DitherMode::Tpdf,
            no_dsp: false,
            show_help: false,
            launch_menu: false,
        };
        let mut target_was_set = false;

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--menu" | "-m" | "-i" | "--interactive" => {
                    options.launch_menu = true;
                }
                "--no-dsp" | "--pristine" => {
                    options.no_dsp = true;
                }
                "--bit-depth" => {
                    let value = args
                        .next()
                        .ok_or("--bit-depth requires a value: 16, 24, or 32f")?;
                    options.output_format = match value.as_str() {
                        "16" => WavSampleFormat::Pcm16Bit,
                        "24" => WavSampleFormat::Pcm24Bit,
                        "32f" => WavSampleFormat::Pcm32BitFloat,
                        _ => return Err(format!("Unsupported bit depth: {value}")),
                    };
                }
                "--dither" => {
                    let value = args
                        .next()
                        .ok_or("--dither requires a value: none or tpdf")?;
                    options.dither = match value.as_str() {
                        "none" => DitherMode::None,
                        "tpdf" => DitherMode::Tpdf,
                        _ => return Err(format!("Unsupported dither mode: {value}")),
                    };
                }
                value if !target_was_set => {
                    options.target_dbfs = value
                        .parse()
                        .map_err(|_| format!("Invalid target dBFS: {value}"))?;
                    target_was_set = true;
                }
                value => return Err(format!("Unknown argument: {value}")),
            }
        }

        if !options.target_dbfs.is_finite() || !(-60.0..=0.0).contains(&options.target_dbfs) {
            return Err("Target dBFS must be between -60.0 and 0.0".to_string());
        }
        if options.output_format == WavSampleFormat::Pcm32BitFloat {
            options.dither = DitherMode::None;
        }
        Ok(options)
    }

    fn interactive() -> Self {
        Self {
            input_file: String::new(),
            target_dbfs: -1.0,
            output_format: WavSampleFormat::Pcm24Bit,
            dither: DitherMode::Tpdf,
            no_dsp: false,
            show_help: false,
            launch_menu: true,
        }
    }

    fn help() -> Self {
        Self {
            input_file: String::new(),
            target_dbfs: -1.0,
            output_format: WavSampleFormat::Pcm24Bit,
            dither: DitherMode::Tpdf,
            no_dsp: false,
            show_help: true,
            launch_menu: false,
        }
    }
}

fn print_help() {
    println!("\x1b[1;36m");
    println!("  ██████╗ ███████╗███████╗ ██████╗ █████╗ ██╗     ███████╗");
    println!("  ██╔══██╗██╔════╝██╔════╝██╔════╝██╔══██╗██║     ██╔════╝");
    println!("  ██████╔╝█████╗  ███████╗██║     ███████║██║     █████╗  ");
    println!("  ██╔══██╗██╔══╝  ╚════██║██║     ██╔══██║██║     ██╔══╝  ");
    println!("  ██║  ██║███████╗███████║╚██████╗██║  ██║███████╗███████╗");
    println!("  ╚═╝  ╚═╝╚══════╝╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝");
    println!("\x1b[0m");
    println!("\x1b[1;37mAudio ReScale & Mastering Engine — Pure Rust / Zero Dependency\x1b[0m");
    println!();
    println!("\x1b[1;33mUsage:\x1b[0m");
    println!("  rescale_cli                                          (Open Interactive TUI Menu)");
    println!("  rescale_cli --menu                                   (Open Interactive TUI Menu)");
    println!(
        "  rescale_cli <input_file> [target_dbfs] [--bit-depth 16|24|32f] [--dither none|tpdf] [--no-dsp]"
    );
    println!();
    println!("\x1b[1;33mArguments:\x1b[0m");
    println!("  --menu, -m     Open the interactive TUI menu");
    println!("  input_file     Path to the audio file (.wav or .mp3)");
    println!("  target_dbfs    Target output, -60.0 to 0.0 (default: -1.0 dBFS)");
    println!("  --bit-depth    Output WAV: 16, 24, or 32f (default: 24)");
    println!("  --dither       PCM dither: none or tpdf (default: tpdf)");
    println!(
        "  --no-dsp       Pristine mode: skip normalize/compressor/limiter (transparent conversion)"
    );
    println!();
    println!("\x1b[1;33mExamples:\x1b[0m");
    println!("  cargo run --bin rescale_cli                           # Interactive TUI menu");
    println!("  cargo run --bin rescale_cli -- track.mp3 -1.0 --bit-depth 24");
    println!("  cargo run --bin rescale_cli -- recording.wav -0.3 --bit-depth 32f");
    println!("  cargo run --bin rescale_cli -- track.mp3 --no-dsp --bit-depth 32f  # pristine");
    println!();
    println!(
        "\x1b[90mOutput: REScale-[TrackName].wav saved in the same folder as the input.\x1b[0m"
    );
}

fn run_rescale(options: &CliOptions) -> Result<(), Box<dyn std::error::Error>> {
    let target_linear = Gain::db_to_linear(options.target_dbfs);
    let path = Path::new(&options.input_file);

    // --- Validasi file input ada ---
    if !path.exists() {
        return Err(format!("File not found: {}", options.input_file).into());
    }

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Validasi ekstensi yang didukung
    if ext != "wav" && ext != "mp3" {
        return Err(format!("Format '{ext}' is not supported. Use .wav or .mp3.").into());
    }

    // Ambil nama lagu dan folder output yang sama dengan input
    let song_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("audio");
    let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let output_wav_path = parent_dir.join(format!("REScale-{}.wav", song_stem));

    if output_wav_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Output already exists: {}", output_wav_path.display()),
        )
        .into());
    }

    print_header(song_stem, &options.input_file, options.target_dbfs);

    // --- Langkah 1: Decode ke WAV jika MP3 ---
    let wav_source_path_buf;
    let wav_source: &str = if ext == "mp3" {
        log_step(
            1,
            5,
            "Decoding MP3 to PCM float32 via ffmpeg (-c:a pcm_f32le)...",
        );

        wav_source_path_buf = parent_dir.join(format!("_tmp_{}.wav", song_stem));
        let tmp_path_str = wav_source_path_buf.to_str().unwrap_or("_tmp.wav");

        let result = Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                &options.input_file,
                "-c:a",
                "pcm_f32le",
                tmp_path_str,
            ])
            .stderr(std::process::Stdio::null())
            .output();

        match result {
            Ok(out) if out.status.success() => {
                log_ok("MP3 decoded to WAV float32 - full precision, no intermediate 16-bit quantization.");
                tmp_path_str
            }
            _ => {
                return Err(
                    "ffmpeg not found or failed to decode MP3. Install ffmpeg in PATH or use WAV."
                        .into(),
                );
            }
        }
    } else {
        wav_source_path_buf = path.to_path_buf();
        &options.input_file
    };

    // --- Langkah 2: Baca binary WAV stream ---
    log_step(2, 5, "Reading binary audio stream...");
    let (header, mut channels) = WavReader::read_from_file(wav_source).map_err(|e| {
        eprintln!("\x1b[1;31m[ERROR]\x1b[0m Failed to read WAV file: {}", e);
        e
    })?;
    let sample_rate = header.sample_rate as f32;

    println!(
        "         \x1b[90m{} Hz | {} Ch | {} samples/ch\x1b[0m",
        header.sample_rate, header.num_channels, header.num_samples_per_channel
    );

    // --- Langkah 3: Analisis Level Awal ---
    log_step(3, 5, "Analyzing initial signal level and energy...");
    let initial_peak = channels
        .iter()
        .map(|ch| Normalizer::find_peak(ch))
        .fold(0.0_f32, f32::max);
    let initial_rms = channels
        .iter()
        .map(|ch| Normalizer::calculate_rms(ch))
        .fold(0.0_f32, f32::max);
    println!(
        "         \x1b[90mPeak: {:.4} ({:+.2} dBFS) | RMS: {:.4} ({:+.2} dBFS)\x1b[0m",
        initial_peak,
        Gain::linear_to_db(initial_peak),
        initial_rms,
        Gain::linear_to_db(initial_rms)
    );

    // --- Langkah 4: DSP ReScaling Chain (bypass penuh di mode pristine) ---
    let final_peak: f32;
    let final_rms: f32;
    if options.no_dsp {
        log_step(
            4,
            5,
            "Pristine mode (--no-dsp): normalize/compressor/limiter skipped...",
        );
        if initial_peak > 1.0 {
            log_warn(&format!(
                "Input peak {:.4} ({:+.2} dBFS) exceeds 0 dBFS - output is clamped to +/-1.0 when written.",
                initial_peak,
                Gain::linear_to_db(initial_peak)
            ));
        }
        final_peak = initial_peak;
        final_rms = initial_rms;
        println!(
            "         \x1b[90mPeak: {:.4} ({:+.2} dBFS) | RMS: {:.4} ({:+.2} dBFS)\x1b[0m",
            final_peak,
            Gain::linear_to_db(final_peak),
            final_rms,
            Gain::linear_to_db(final_rms)
        );
        log_ok("Pristine pass-through done (no DSP).");
    } else {
        log_step(4, 5, "Processing DSP ReScaling Chain...");

        // A. Peak Normalization ke target headroom
        Normalizer::normalize_channels_to_peak(&mut channels, target_linear);

        // B. Dynamic Compressor (meratakan dinamika, bukan memotong)
        let comp_config = CompressorConfig {
            threshold_db: options.target_dbfs - 8.0,
            ratio: 2.5,
            knee_db: 3.0,
            attack_ms: 8.0,
            release_ms: 100.0,
            makeup_gain_db: 0.8,
            mode: DetectionMode::Rms,
        };
        // B/C. State DSP terisolasi per channel melalui processor chain.
        let mut chain = ProcessorChain::new();
        chain.push(CompressorProcessor::new(comp_config));
        chain.push(LimiterProcessor::new(options.target_dbfs, 25.0, 2.0));
        chain.prepare(AudioSpec::new(sample_rate, channels.len())?)?;

        let mut channel_views: Vec<&mut [f32]> =
            channels.iter_mut().map(Vec::as_mut_slice).collect();
        let mut block = AudioBlock::new(&mut channel_views)?;
        chain.process(&mut block)?;

        final_peak = channels
            .iter()
            .map(|ch| Normalizer::find_peak(ch))
            .fold(0.0_f32, f32::max);
        final_rms = channels
            .iter()
            .map(|ch| Normalizer::calculate_rms(ch))
            .fold(0.0_f32, f32::max);
        println!(
            "         \x1b[90mPeak: {:.4} ({:+.2} dBFS) | RMS: {:.4} ({:+.2} dBFS)\x1b[0m",
            final_peak,
            Gain::linear_to_db(final_peak),
            final_rms,
            Gain::linear_to_db(final_rms)
        );
        log_ok("ReScaling done.");
    }

    // --- Langkah 5: Simpan Output ---
    log_step(5, 5, "Saving output file...");
    WavWriter::write_to_file_with_options(
        &output_wav_path,
        &channels,
        header.sample_rate,
        WavWriteOptions::new(options.output_format, options.dither),
    )?;
    log_ok(&format!(
        "WAV saved → \x1b[1;37m{}\x1b[0m",
        output_wav_path.display()
    ));

    // Encode balik ke MP3 jika input aslinya MP3
    if ext == "mp3" {
        let mp3_out = parent_dir.join(format!("REScale-{}.mp3", song_stem));
        let enc = Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                output_wav_path.to_str().unwrap_or(""),
                "-b:a",
                "320k",
                mp3_out.to_str().unwrap_or(""),
            ])
            .stderr(std::process::Stdio::null())
            .output();

        if let Ok(out) = enc {
            if out.status.success() {
                log_ok(&format!(
                    "MP3 320kbps saved → \x1b[1;37m{}\x1b[0m",
                    mp3_out.display()
                ));
            }
        }
        // Hapus file temp WAV decode
        let _ = std::fs::remove_file(&wav_source_path_buf);
    }

    // --- Ringkasan Akhir ---
    print_summary(
        song_stem,
        initial_peak,
        Gain::linear_to_db(initial_peak),
        final_peak,
        Gain::linear_to_db(final_peak),
        options.target_dbfs,
        &channels,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helper Tampilan
// ---------------------------------------------------------------------------

fn print_header(song_stem: &str, input_file: &str, target_dbfs: f32) {
    println!("\n\x1b[1;36m┌─────────────────────────────────────────────────────────┐\x1b[0m");
    println!(
        "\x1b[1;36m│\x1b[0m \x1b[1;37m REScale Audio Engine\x1b[0m{:>35}\x1b[1;36m│\x1b[0m",
        ""
    );
    println!("\x1b[1;36m├─────────────────────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[1;36m│\x1b[0m  \x1b[90mFile   :\x1b[0m {:<46}\x1b[1;36m│\x1b[0m",
        truncate(input_file, 46)
    );
    println!(
        "\x1b[1;36m│\x1b[0m  \x1b[90mOutput :\x1b[0m {:<46}\x1b[1;36m│\x1b[0m",
        truncate(&format!("REScale-{}.wav", song_stem), 46)
    );
    println!(
        "\x1b[1;36m│\x1b[0m  \x1b[90mTarget :\x1b[0m {:<46}\x1b[1;36m│\x1b[0m",
        format!("{:.1} dBFS", target_dbfs)
    );
    println!("\x1b[1;36m└─────────────────────────────────────────────────────────┘\x1b[0m\n");
    let _ = stdout().flush();
}

fn print_summary(
    song_stem: &str,
    before_peak: f32,
    before_db: f32,
    after_peak: f32,
    after_db: f32,
    target_dbfs: f32,
    channels: &[Vec<f32>],
) {
    let peak_l = Normalizer::find_peak(&channels[0]);
    let peak_r = if channels.len() > 1 {
        Normalizer::find_peak(&channels[1])
    } else {
        peak_l
    };

    println!("\n\x1b[1;32m┌─────────────────────────────────────────────────────────┐\x1b[0m");
    println!(
        "\x1b[1;32m│  ✅ DONE   : REScale-{:<36}\x1b[1;32m│\x1b[0m",
        format!("{}.wav", song_stem)
    );
    println!("\x1b[1;32m├─────────────────────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[1;32m│\x1b[0m  \x1b[90mBefore  :\x1b[0m Peak {:.4} ({:+.2} dBFS){:>21}\x1b[1;32m│\x1b[0m",
        before_peak, before_db, ""
    );
    println!(
        "\x1b[1;32m│\x1b[0m  \x1b[90mAfter   :\x1b[0m Peak {:.4} ({:+.2} dBFS){:>21}\x1b[1;32m│\x1b[0m",
        after_peak, after_db, ""
    );
    println!(
        "\x1b[1;32m│\x1b[0m  \x1b[90mTarget  :\x1b[0m {:.1} dBFS{:>36}\x1b[1;32m│\x1b[0m",
        target_dbfs, ""
    );
    println!("\x1b[1;32m├─────────────────────────────────────────────────────────┤\x1b[0m");

    // VU Meter inline (tidak menggunakan absolute cursor — aman di semua terminal)
    println!(
        "\x1b[1;32m│\x1b[0m  {}{}\x1b[1;32m│\x1b[0m",
        vu_bar("L", peak_l, 30),
        " ".repeat(7)
    );
    println!(
        "\x1b[1;32m│\x1b[0m  {}{}\x1b[1;32m│\x1b[0m",
        vu_bar("R", peak_r, 30),
        " ".repeat(7)
    );
    println!("\x1b[1;32m└─────────────────────────────────────────────────────────┘\x1b[0m\n");
    let _ = stdout().flush();
}

/// Render VU Bar sebagai String biasa (tidak menggunakan absolute cursor)
fn vu_bar(label: &str, level: f32, width: usize) -> String {
    let clamped = level.clamp(0.0, 1.0);
    let filled = (clamped * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    let db = if level < 1e-4 {
        -90.0
    } else {
        20.0 * level.log10()
    };

    let color = if level > 0.99 {
        "\x1b[1;31m"
    } else if level > 0.75 {
        "\x1b[1;33m"
    } else {
        "\x1b[1;32m"
    };

    format!(
        "\x1b[90m{} :\x1b[0m [{}{}{}{}{}] {:>+6.2} dBFS",
        label,
        color,
        "█".repeat(filled),
        "\x1b[90m",
        "░".repeat(empty),
        "\x1b[0m",
        db
    )
}

fn log_step(step: u8, total: u8, msg: &str) {
    println!("\x1b[1;34m[{}/{}]\x1b[0m {}", step, total, msg);
    let _ = stdout().flush();
}

fn log_ok(msg: &str) {
    println!("  \x1b[1;32m✓\x1b[0m  {}", msg);
    let _ = stdout().flush();
}

fn log_warn(msg: &str) {
    println!("  \x1b[1;33m[WARN]\x1b[0m {}", msg);
    let _ = stdout().flush();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("...{}", &s[s.len().saturating_sub(max - 3)..])
    }
}
