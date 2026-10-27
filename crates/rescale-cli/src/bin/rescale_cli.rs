//! CLI Tool untuk ReScale / Normalize / Master File Audio menggunakan REScale DSP Engine

use rescale_core::{
    AudioBlock, AudioProcessor, AudioSpec, CompressorConfig, CompressorProcessor, DetectionMode,
    DitherMode, Gain, LimiterProcessor, Normalizer, ProcessorChain, TuiMenu, WavSampleFormat,
    WavStreamReader, WavStreamWriter, WavWriteOptions,
};
use std::env;
use std::io::{self, stdout, Read, Seek, Write};
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
                // Flag panjang tak dikenal tidak boleh ditelan sebagai target dBFS.
                value if value.starts_with("--") => {
                    return Err(format!("Unknown argument: {value}"))
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

    // --- Langkah 2: Analisis awal (pass 1, read-only) ---
    log_step(
        2,
        5,
        "Reading audio stream (pass 1: header + level scan)...",
    );
    let analysis = analyze_input(Path::new(wav_source))?;
    log_ok(&format!(
        "{} Hz | {} Ch | {} frames (~{:.2} s)",
        analysis.sample_rate,
        analysis.channels,
        analysis.total_frames,
        analysis.duration_seconds()
    ));

    // --- Langkah 3: Analisis Level Awal ---
    log_step(3, 5, "Analyzing initial signal level and energy...");
    println!(
        "         \x1b[90mPeak: {:.4} ({:+.2} dBFS) | RMS: {:.4} ({:+.2} dBFS)\x1b[0m",
        analysis.peak,
        Gain::linear_to_db(analysis.peak),
        analysis.rms,
        Gain::linear_to_db(analysis.rms)
    );

    if options.no_dsp && analysis.peak > 1.0 {
        log_warn(&format!(
            "Input peak {:.4} ({:+.2} dBFS) exceeds 0 dBFS - output is clamped to +/-1.0 when written.",
            analysis.peak,
            Gain::linear_to_db(analysis.peak)
        ));
    }

    // --- Langkah 4: DSP ReScaling Chain + streaming write ---
    let spec = AudioSpec::new(analysis.sample_rate as f32, analysis.channels)?;
    let normalization_gain = if options.no_dsp {
        log_step(
            4,
            5,
            "Pristine mode (--no-dsp): normalize/compressor/limiter skipped...",
        );
        1.0
    } else {
        log_step(
            4,
            5,
            "Processing DSP ReScaling Chain (streaming, latency-compensated)...",
        );
        normalization_gain(&analysis, target_linear)
    };

    let mut chain = if options.no_dsp {
        None
    } else {
        Some(build_processing_chain(options, spec)?)
    };
    let latency_samples = chain
        .as_ref()
        .map(ProcessorChain::latency_samples)
        .unwrap_or(0);
    let (frames_written, metrics) = render_streaming(
        Path::new(wav_source),
        &output_wav_path,
        WavWriteOptions::new(options.output_format, options.dither),
        chain.as_mut(),
        normalization_gain,
        latency_samples,
    )?;

    println!(
        "         \x1b[90mPeak: {:.4} ({:+.2} dBFS) | RMS: {:.4} ({:+.2} dBFS)\x1b[0m",
        metrics.peak,
        Gain::linear_to_db(metrics.peak),
        metrics.rms(),
        Gain::linear_to_db(metrics.rms())
    );
    if options.no_dsp {
        log_ok("Pristine pass-through done (no DSP).");
    } else {
        log_ok(&format!(
            "ReScaling done ({latency_samples} samples latency compensated)."
        ));
    }

    // --- Langkah 5: Simpan Output ---
    log_step(5, 5, "Saving output file...");
    log_ok(&format!(
        "WAV saved → \x1b[1;37m{}\x1b[0m ({} frames)",
        output_wav_path.display(),
        frames_written
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
        analysis.peak,
        Gain::linear_to_db(analysis.peak),
        &metrics,
        options.target_dbfs,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Pipeline Streaming (pass 1: analisis, pass 2: proses + tulis)
// ---------------------------------------------------------------------------

/// Ukuran blok streaming: cukup besar untuk mengamortisasi overhead I/O, cukup
/// kecil untuk menjaga pemakaian memori konstan pada file berdurasi panjang.
const STREAM_BLOCK_FRAMES: usize = 4_096;

/// Hasil pass-1 read-only: header dan level awal seluruh file.
struct InputAnalysis {
    sample_rate: u32,
    channels: usize,
    total_frames: usize,
    peak: f32,
    rms: f32,
}

impl InputAnalysis {
    fn duration_seconds(&self) -> f64 {
        self.total_frames as f64 / f64::from(self.sample_rate)
    }
}

/// Akumulator metrik output; memungkinkan ringkasan tanpa menahan audio di memori.
struct OutputMetrics {
    peak: f32,
    per_channel_peak: Vec<f32>,
    sum_squares: f64,
    sample_count: usize,
}

impl OutputMetrics {
    fn new(channels: usize) -> Self {
        Self {
            peak: 0.0,
            per_channel_peak: vec![0.0; channels],
            sum_squares: 0.0,
            sample_count: 0,
        }
    }

    /// Mencatat sampel yang benar-benar ditulis ke file (bukan sampel yang dibuang).
    fn record(&mut self, block: &AudioBlock<'_>, frames: usize) {
        for channel_index in 0..block.channels() {
            let channel = block
                .channel(channel_index)
                .expect("validated channel count");
            let written = &channel[..frames];
            let channel_peak = Normalizer::find_peak(written);
            self.per_channel_peak[channel_index] =
                self.per_channel_peak[channel_index].max(channel_peak);
            self.peak = self.peak.max(channel_peak);
            for &sample in written {
                self.sum_squares += f64::from(sample) * f64::from(sample);
                self.sample_count += 1;
            }
        }
    }

    fn rms(&self) -> f32 {
        if self.sample_count == 0 {
            return 0.0;
        }
        (self.sum_squares / self.sample_count as f64).sqrt() as f32
    }
}

/// Pass-1: memindai header dan level puncak/RMS tanpa menyimpan audio.
fn analyze_input(path: &Path) -> Result<InputAnalysis, Box<dyn std::error::Error>> {
    let mut reader = WavStreamReader::open(path)?;
    let header = reader.header().clone();
    let channels = header.num_channels as usize;

    let mut buffers = vec![vec![0.0_f32; STREAM_BLOCK_FRAMES]; channels];
    let mut peak = 0.0_f32;
    let mut sum_squares = 0.0_f64;
    let mut sample_count = 0_usize;

    loop {
        let frames_read = read_block_into(&mut reader, &mut buffers)?;
        if frames_read == 0 {
            break;
        }
        let mut channel_views: Vec<&mut [f32]> = buffers
            .iter_mut()
            .map(|buffer| &mut buffer[..frames_read])
            .collect();
        let block = AudioBlock::new(&mut channel_views)?;
        for channel_index in 0..block.channels() {
            let channel = block
                .channel(channel_index)
                .expect("validated channel count");
            peak = peak.max(Normalizer::find_peak(channel));
            for &sample in channel {
                sum_squares += f64::from(sample) * f64::from(sample);
                sample_count += 1;
            }
        }
    }

    let rms = if sample_count == 0 {
        0.0
    } else {
        (sum_squares / sample_count as f64).sqrt() as f32
    };

    Ok(InputAnalysis {
        sample_rate: header.sample_rate,
        channels,
        total_frames: header.num_samples_per_channel,
        peak,
        rms,
    })
}

/// Gain normalisasi identik dengan `Normalizer::normalize_channels_to_peak`.
fn normalization_gain(analysis: &InputAnalysis, target_peak: f32) -> f32 {
    if analysis.peak <= 1e-6 {
        1.0
    } else {
        target_peak / analysis.peak
    }
}

fn build_processing_chain(
    options: &CliOptions,
    spec: AudioSpec,
) -> Result<ProcessorChain, Box<dyn std::error::Error>> {
    let compressor = CompressorProcessor::new(CompressorConfig {
        threshold_db: options.target_dbfs - 8.0,
        ratio: 2.5,
        knee_db: 3.0,
        attack_ms: 8.0,
        release_ms: 100.0,
        makeup_gain_db: 0.8,
        mode: DetectionMode::Rms,
    });

    let mut chain = ProcessorChain::new();
    chain.push(compressor);
    chain.push(LimiterProcessor::new(options.target_dbfs, 25.0, 2.0));
    chain.prepare(spec)?;
    Ok(chain)
}

/// Pass-2: membaca blok, memproses, menulis, lalu meniriskan tail.
///
/// Kompensasi latency: `latency_samples` frame pertama dibuang dan tail yang
/// setara ditarik lewat `flush()` di EOF, sehingga jumlah frame output sama
/// dengan input dan tidak ada time-shift. Mengembalikan jumlah frame tertulis.
fn render_streaming(
    input_path: &Path,
    output_path: &Path,
    write_options: WavWriteOptions,
    mut chain: Option<&mut ProcessorChain>,
    normalization_gain: f32,
    latency_samples: usize,
) -> Result<(usize, OutputMetrics), Box<dyn std::error::Error>> {
    let mut reader = WavStreamReader::open(input_path)?;
    let header = reader.header().clone();
    let channels = header.num_channels as usize;

    let mut writer = WavStreamWriter::create(
        output_path,
        header.num_channels,
        header.sample_rate,
        write_options,
    )?;

    let mut buffers = vec![vec![0.0_f32; STREAM_BLOCK_FRAMES]; channels];
    let mut metrics = OutputMetrics::new(channels);
    let mut frames_to_discard = latency_samples;

    loop {
        let frames_read = read_block_into(&mut reader, &mut buffers)?;
        if frames_read == 0 {
            break;
        }
        let mut channel_views: Vec<&mut [f32]> = buffers
            .iter_mut()
            .map(|buffer| &mut buffer[..frames_read])
            .collect();
        let mut block = AudioBlock::new(&mut channel_views)?;
        apply_normalization(&mut block, normalization_gain);
        if let Some(chain) = chain.as_deref_mut() {
            chain.process(&mut block)?;
        }
        write_compensated(
            &mut writer,
            &mut block,
            frames_read,
            &mut frames_to_discard,
            &mut metrics,
        )?;
    }

    if let Some(chain) = chain {
        let mut flush_buffers = vec![vec![0.0_f32; STREAM_BLOCK_FRAMES]; channels];
        loop {
            let mut channel_views: Vec<&mut [f32]> = flush_buffers
                .iter_mut()
                .map(|buffer| buffer.as_mut_slice())
                .collect();
            let mut block = AudioBlock::new(&mut channel_views)?;
            let frames_flushed = chain.flush(&mut block)?;
            if frames_flushed == 0 {
                break;
            }
            write_compensated(
                &mut writer,
                &mut block,
                frames_flushed,
                &mut frames_to_discard,
                &mut metrics,
            )?;
        }
    }

    let written_header = writer.finalize()?;
    Ok((written_header.num_samples_per_channel, metrics))
}

fn read_block_into<R: Read + Seek>(
    reader: &mut WavStreamReader<R>,
    buffers: &mut [Vec<f32>],
) -> io::Result<usize> {
    let mut channel_views: Vec<&mut [f32]> = buffers.iter_mut().map(Vec::as_mut_slice).collect();
    let mut block = AudioBlock::new(&mut channel_views)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    reader.read_block(&mut block)
}

fn apply_normalization(block: &mut AudioBlock<'_>, gain_linear: f32) {
    if (gain_linear - 1.0).abs() <= f32::EPSILON {
        return;
    }
    for channel_index in 0..block.channels() {
        let channel = block
            .channel_mut(channel_index)
            .expect("validated channel count");
        Gain::apply_linear(channel, gain_linear);
    }
}

/// Menulis `frames` frame sambil membuang `frames_to_discard` frame awal.
fn write_compensated<W: Write + Seek>(
    writer: &mut WavStreamWriter<W>,
    block: &mut AudioBlock<'_>,
    frames: usize,
    frames_to_discard: &mut usize,
    metrics: &mut OutputMetrics,
) -> io::Result<()> {
    let discarded = (*frames_to_discard).min(frames);
    *frames_to_discard -= discarded;
    let kept = frames - discarded;
    if kept == 0 {
        return Ok(());
    }

    if discarded > 0 {
        for channel_index in 0..block.channels() {
            let channel = block
                .channel_mut(channel_index)
                .expect("validated channel count");
            channel.copy_within(discarded..frames, 0);
        }
    }

    metrics.record(block, kept);
    writer.write_block_frames(block, kept)
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
    metrics: &OutputMetrics,
    target_dbfs: f32,
) {
    let peak_l = metrics.per_channel_peak.first().copied().unwrap_or(0.0);
    let peak_r = metrics.per_channel_peak.get(1).copied().unwrap_or(peak_l);
    let after_peak = metrics.peak;
    let after_db = Gain::linear_to_db(after_peak);

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
