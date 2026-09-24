//! Binary Writer untuk menulis file RIFF WAV dari raw f32 buffer.

use super::header::{WavHeader, WavSampleFormat};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub struct WavWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMode {
    None,
    Tpdf,
}

#[derive(Debug, Clone, Copy)]
pub struct WavWriteOptions {
    pub format: WavSampleFormat,
    pub dither: DitherMode,
}

impl WavWriteOptions {
    pub const fn new(format: WavSampleFormat, dither: DitherMode) -> Self {
        Self { format, dither }
    }
}

impl WavWriter {
    /// Menulis audio channel data (f32 normalized [-1.0, 1.0]) ke file WAV.
    /// Default dither: TPDF (hanya berlaku pada format PCM; float 32-bit ditulis langsung).
    pub fn write_to_file<P: AsRef<Path>>(
        path: P,
        channels: &[Vec<f32>],
        sample_rate: u32,
        format: WavSampleFormat,
    ) -> io::Result<()> {
        Self::write_to_file_with_options(
            path,
            channels,
            sample_rate,
            WavWriteOptions::new(format, DitherMode::Tpdf),
        )
    }

    pub fn write_to_file_with_options<P: AsRef<Path>>(
        path: P,
        channels: &[Vec<f32>],
        sample_rate: u32,
        options: WavWriteOptions,
    ) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        Self::write_to_stream_with_options(&mut writer, channels, sample_rate, options)
    }

    /// Menulis audio ke writer stream generic.
    /// Default dither: TPDF (hanya berlaku pada format PCM; float 32-bit ditulis langsung).
    pub fn write_to_stream<W: Write>(
        writer: &mut W,
        channels: &[Vec<f32>],
        sample_rate: u32,
        format: WavSampleFormat,
    ) -> io::Result<()> {
        Self::write_to_stream_with_options(
            writer,
            channels,
            sample_rate,
            WavWriteOptions::new(format, DitherMode::Tpdf),
        )
    }

    pub fn write_to_stream_with_options<W: Write>(
        writer: &mut W,
        channels: &[Vec<f32>],
        sample_rate: u32,
        options: WavWriteOptions,
    ) -> io::Result<()> {
        if channels.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Channels cannot be empty",
            ));
        }

        let num_channels = channels.len() as u16;
        let num_samples_per_channel = channels[0].len();

        // Validasi panjang tiap channel sama
        for ch in channels {
            if ch.len() != num_samples_per_channel {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "All channels must have the same length",
                ));
            }
        }

        let header = WavHeader::new(
            num_channels,
            sample_rate,
            options.format,
            num_samples_per_channel,
        );
        let bits_per_sample = header.bits_per_sample();
        let _bytes_per_sample = (bits_per_sample / 8) as u32;
        let block_align = num_channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * (block_align as u32);
        let data_chunk_size = (num_samples_per_channel as u32) * (block_align as u32);
        let riff_chunk_size = 36 + data_chunk_size;

        // 1. Deskriptor Chunk RIFF
        writer.write_all(b"RIFF")?;
        writer.write_all(&riff_chunk_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;

        // 2. Subchunk "fmt "
        writer.write_all(b"fmt ")?;
        writer.write_all(&16u32.to_le_bytes())?; // Subchunk1Size = 16 untuk PCM
        writer.write_all(&header.audio_format_tag().to_le_bytes())?;
        writer.write_all(&num_channels.to_le_bytes())?;
        writer.write_all(&sample_rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        writer.write_all(&block_align.to_le_bytes())?;
        writer.write_all(&bits_per_sample.to_le_bytes())?;

        // 3. Subchunk "data"
        writer.write_all(b"data")?;
        writer.write_all(&data_chunk_size.to_le_bytes())?;

        // Interleaving sample: L0, R0, L1, R1, ...
        let mut dither = TpdfDither::new();
        for i in 0..num_samples_per_channel {
            for channel in channels.iter() {
                let sample = channel[i].clamp(-1.0, 1.0);
                match options.format {
                    WavSampleFormat::Pcm16Bit => {
                        let sample = dither.apply(sample, options.dither, 32768.0);
                        let val: i16 = if sample < 0.0 {
                            (sample * 32768.0).round() as i16
                        } else {
                            (sample * 32767.0).round() as i16
                        };
                        writer.write_all(&val.to_le_bytes())?;
                    }
                    WavSampleFormat::Pcm24Bit => {
                        let sample = dither.apply(sample, options.dither, 8388608.0);
                        let val: i32 = if sample < 0.0 {
                            (sample * 8388608.0).round() as i32
                        } else {
                            (sample * 8388607.0).round() as i32
                        };
                        let bytes = val.to_le_bytes();
                        // Tulis 3 byte (little endian)
                        writer.write_all(&bytes[0..3])?;
                    }
                    WavSampleFormat::Pcm32BitFloat => {
                        writer.write_all(&sample.to_le_bytes())?;
                    }
                }
            }
        }

        writer.flush()?;
        Ok(())
    }
}

struct TpdfDither(u32);

impl TpdfDither {
    fn new() -> Self {
        Self(0xA5A5_1F3D)
    }

    fn uniform(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 as f32 / u32::MAX as f32
    }

    fn apply(&mut self, sample: f32, mode: DitherMode, scale: f32) -> f32 {
        if mode == DitherMode::Tpdf {
            (sample + (self.uniform() - self.uniform()) / scale).clamp(-1.0, 1.0)
        } else {
            sample
        }
    }
}
