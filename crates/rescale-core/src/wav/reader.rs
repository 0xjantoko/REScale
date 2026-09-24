//! Binary Reader untuk mem-parsing file RIFF WAV ke f32 channel buffer.

use super::header::{WavHeader, WavSampleFormat};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub struct WavReader;

impl WavReader {
    /// Membaca file WAV dari path filesystem
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> io::Result<(WavHeader, Vec<Vec<f32>>)> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        Self::read_from_stream(&mut reader)
    }

    /// Mem-parsing stream WAV binary dari memory / file reader
    pub fn read_from_stream<R: Read + Seek>(
        reader: &mut R,
    ) -> io::Result<(WavHeader, Vec<Vec<f32>>)> {
        let mut fourcc = [0u8; 4];

        // 1. Periksa header RIFF
        reader.read_exact(&mut fourcc)?;
        if &fourcc != b"RIFF" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a valid RIFF file",
            ));
        }

        let mut riff_size_bytes = [0u8; 4];
        reader.read_exact(&mut riff_size_bytes)?;

        reader.read_exact(&mut fourcc)?;
        if &fourcc != b"WAVE" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a WAVE format",
            ));
        }

        let mut audio_format: u16 = 0;
        let mut num_channels: u16 = 0;
        let mut sample_rate: u32 = 0;
        let mut bits_per_sample: u16 = 0;
        let mut data_chunk_size: u32 = 0;
        let mut fmt_found = false;

        // Loop melalui chunks sampai menemukan "fmt " dan "data"
        loop {
            if reader.read_exact(&mut fourcc).is_err() {
                break;
            }

            let mut chunk_size_bytes = [0u8; 4];
            reader.read_exact(&mut chunk_size_bytes)?;
            let chunk_size = u32::from_le_bytes(chunk_size_bytes);

            match &fourcc {
                b"fmt " => {
                    let mut format_tag_bytes = [0u8; 2];
                    let mut channels_bytes = [0u8; 2];
                    let mut samplerate_bytes = [0u8; 4];
                    let mut byterate_bytes = [0u8; 4];
                    let mut blockalign_bytes = [0u8; 2];
                    let mut bitspersample_bytes = [0u8; 2];

                    reader.read_exact(&mut format_tag_bytes)?;
                    reader.read_exact(&mut channels_bytes)?;
                    reader.read_exact(&mut samplerate_bytes)?;
                    reader.read_exact(&mut byterate_bytes)?;
                    reader.read_exact(&mut blockalign_bytes)?;
                    reader.read_exact(&mut bitspersample_bytes)?;

                    audio_format = u16::from_le_bytes(format_tag_bytes);
                    num_channels = u16::from_le_bytes(channels_bytes);
                    sample_rate = u32::from_le_bytes(samplerate_bytes);
                    bits_per_sample = u16::from_le_bytes(bitspersample_bytes);

                    // Lewati byte ekstra jika chunk_size > 16
                    if chunk_size > 16 {
                        reader.seek(SeekFrom::Current((chunk_size - 16) as i64))?;
                    }
                    fmt_found = true;
                }
                b"data" => {
                    data_chunk_size = chunk_size;
                    break; // Mulai baca data audio
                }
                _ => {
                    // Lewati chunk lain (misal "LIST", "JUNK", dll.)
                    reader.seek(SeekFrom::Current(chunk_size as i64))?;
                }
            }
        }

        if !fmt_found {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing 'fmt ' chunk in WAV file",
            ));
        }

        let sample_format = match (audio_format, bits_per_sample) {
            (1, 16) => WavSampleFormat::Pcm16Bit,
            (1, 24) => WavSampleFormat::Pcm24Bit,
            (3, 32) => WavSampleFormat::Pcm32BitFloat,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "Unsupported WAV format: tag={}, bits={}",
                        audio_format, bits_per_sample
                    ),
                ));
            }
        };

        let bytes_per_sample = (bits_per_sample / 8) as usize;
        let total_samples = (data_chunk_size as usize) / bytes_per_sample;
        let samples_per_channel = total_samples / (num_channels as usize);

        let mut channels: Vec<Vec<f32>> =
            vec![Vec::with_capacity(samples_per_channel); num_channels as usize];

        // De-interleave data dari file binary
        for _ in 0..samples_per_channel {
            for channel in channels.iter_mut().take(num_channels as usize) {
                let sample_val: f32 = match sample_format {
                    WavSampleFormat::Pcm16Bit => {
                        let mut b = [0u8; 2];
                        reader.read_exact(&mut b)?;
                        let i = i16::from_le_bytes(b);
                        if i < 0 {
                            i as f32 / 32768.0
                        } else {
                            i as f32 / 32767.0
                        }
                    }
                    WavSampleFormat::Pcm24Bit => {
                        let mut b = [0u8; 3];
                        reader.read_exact(&mut b)?;
                        // Sign extend 24-bit ke int 32-bit
                        let sign = if b[2] & 0x80 != 0 { 0xFF } else { 0x00 };
                        let i = i32::from_le_bytes([b[0], b[1], b[2], sign]);
                        if i < 0 {
                            i as f32 / 8388608.0
                        } else {
                            i as f32 / 8388607.0
                        }
                    }
                    WavSampleFormat::Pcm32BitFloat => {
                        let mut b = [0u8; 4];
                        reader.read_exact(&mut b)?;
                        f32::from_le_bytes(b)
                    }
                };
                channel.push(sample_val);
            }
        }

        let header = WavHeader::new(
            num_channels,
            sample_rate,
            sample_format,
            samples_per_channel,
        );
        Ok((header, channels))
    }
}
