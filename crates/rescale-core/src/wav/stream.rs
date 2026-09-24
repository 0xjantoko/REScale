//! Streaming I/O untuk file RIFF WAV berbasis block (Zero-Allocation on process).

use super::header::{WavHeader, WavSampleFormat};
use super::writer::{DitherMode, WavWriteOptions};
use crate::engine::{AudioBlock, AudioSpec, EngineError};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Reader streaming berbasis blok untuk file RIFF WAV.
pub struct WavStreamReader<R> {
    reader: R,
    header: WavHeader,
    samples_read_per_channel: usize,
    data_chunk_offset: u64,
}

impl WavStreamReader<BufReader<File>> {
    /// Membuka file WAV dari path filesystem dalam mode streaming reader.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::new(reader)
    }
}

impl<R: Read + Seek> WavStreamReader<R> {
    /// Membuat streaming reader dari reader yang mengimplementasikan `Read + Seek`.
    pub fn new(mut reader: R) -> io::Result<Self> {
        let mut fourcc = [0u8; 4];

        // 1. Validasi header RIFF
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

        // 2. Scan chunks sampai menemukan 'fmt ' dan 'data'
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

                    if chunk_size > 16 {
                        reader.seek(SeekFrom::Current((chunk_size - 16) as i64))?;
                    }
                    fmt_found = true;
                }
                b"data" => {
                    data_chunk_size = chunk_size;
                    break;
                }
                _ => {
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
                    format!("Unsupported WAV format: tag={audio_format}, bits={bits_per_sample}"),
                ));
            }
        };

        let bytes_per_sample = (bits_per_sample / 8) as usize;
        let total_samples = (data_chunk_size as usize) / bytes_per_sample;
        let samples_per_channel = if num_channels > 0 {
            total_samples / (num_channels as usize)
        } else {
            0
        };

        let data_chunk_offset = reader.stream_position()?;
        let header = WavHeader::new(
            num_channels,
            sample_rate,
            sample_format,
            samples_per_channel,
        );

        Ok(Self {
            reader,
            header,
            samples_read_per_channel: 0,
            data_chunk_offset,
        })
    }

    pub fn header(&self) -> &WavHeader {
        &self.header
    }

    pub fn spec(&self) -> Result<AudioSpec, EngineError> {
        AudioSpec::new(
            self.header.sample_rate as f32,
            self.header.num_channels as usize,
        )
    }

    pub fn total_frames(&self) -> usize {
        self.header.num_samples_per_channel
    }

    pub fn frames_read(&self) -> usize {
        self.samples_read_per_channel
    }

    pub fn frames_remaining(&self) -> usize {
        self.total_frames()
            .saturating_sub(self.samples_read_per_channel)
    }

    pub fn is_finished(&self) -> bool {
        self.frames_remaining() == 0
    }

    /// Membaca blok planar langsung ke dalam `AudioBlock`.
    /// Mengembalikan jumlah frame yang berhasil dibaca (0 jika mencapai EOF).
    pub fn read_block(&mut self, block: &mut AudioBlock<'_>) -> io::Result<usize> {
        if block.channels() != self.header.num_channels as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "AudioBlock channels ({}) does not match WAV channels ({})",
                    block.channels(),
                    self.header.num_channels
                ),
            ));
        }

        let frames_to_read = block.frames().min(self.frames_remaining());
        if frames_to_read == 0 {
            return Ok(0);
        }

        let num_channels = self.header.num_channels as usize;
        for frame in 0..frames_to_read {
            for ch in 0..num_channels {
                let sample_val = match self.header.format {
                    WavSampleFormat::Pcm16Bit => {
                        let mut b = [0u8; 2];
                        self.reader.read_exact(&mut b)?;
                        let i = i16::from_le_bytes(b);
                        if i < 0 {
                            i as f32 / 32768.0
                        } else {
                            i as f32 / 32767.0
                        }
                    }
                    WavSampleFormat::Pcm24Bit => {
                        let mut b = [0u8; 3];
                        self.reader.read_exact(&mut b)?;
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
                        self.reader.read_exact(&mut b)?;
                        f32::from_le_bytes(b)
                    }
                };

                let channel_slice = block.channel_mut(ch).expect("validated channel count");
                channel_slice[frame] = sample_val;
            }
        }

        self.samples_read_per_channel += frames_to_read;
        Ok(frames_to_read)
    }

    /// Reset posisi pembacaan kembali ke awal chunk data.
    pub fn rewind(&mut self) -> io::Result<()> {
        self.reader.seek(SeekFrom::Start(self.data_chunk_offset))?;
        self.samples_read_per_channel = 0;
        Ok(())
    }
}

/// Writer streaming berbasis blok untuk menulis file RIFF WAV secara bertahap.
pub struct WavStreamWriter<W> {
    writer: W,
    options: WavWriteOptions,
    num_channels: u16,
    sample_rate: u32,
    frames_written: usize,
    dither_state: u32,
}

impl WavStreamWriter<BufWriter<File>> {
    /// Membuat file WAV baru untuk streaming write.
    pub fn create<P: AsRef<Path>>(
        path: P,
        num_channels: u16,
        sample_rate: u32,
        options: WavWriteOptions,
    ) -> io::Result<Self> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        Self::new(writer, num_channels, sample_rate, options)
    }
}

impl<W: Write + Seek> WavStreamWriter<W> {
    /// Inisialisasi streaming writer dan menulis placeholder header 44-byte.
    pub fn new(
        mut writer: W,
        num_channels: u16,
        sample_rate: u32,
        options: WavWriteOptions,
    ) -> io::Result<Self> {
        if num_channels == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Channels must be at least 1",
            ));
        }

        // Tulis placeholder header
        let header = WavHeader::new(num_channels, sample_rate, options.format, 0);
        let bits_per_sample = header.bits_per_sample();
        let block_align = num_channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * (block_align as u32);

        // 1. RIFF descriptor placeholder
        writer.write_all(b"RIFF")?;
        writer.write_all(&0u32.to_le_bytes())?; // Placeholder ukuran RIFF
        writer.write_all(b"WAVE")?;

        // 2. fmt chunk
        writer.write_all(b"fmt ")?;
        writer.write_all(&16u32.to_le_bytes())?;
        writer.write_all(&header.audio_format_tag().to_le_bytes())?;
        writer.write_all(&num_channels.to_le_bytes())?;
        writer.write_all(&sample_rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        writer.write_all(&block_align.to_le_bytes())?;
        writer.write_all(&bits_per_sample.to_le_bytes())?;

        // 3. data chunk placeholder
        writer.write_all(b"data")?;
        writer.write_all(&0u32.to_le_bytes())?; // Placeholder ukuran data

        Ok(Self {
            writer,
            options,
            num_channels,
            sample_rate,
            frames_written: 0,
            dither_state: 0xA5A5_1F3D,
        })
    }

    pub fn frames_written(&self) -> usize {
        self.frames_written
    }

    fn uniform_random(&mut self) -> f32 {
        self.dither_state = self
            .dither_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.dither_state as f32 / u32::MAX as f32
    }

    fn apply_dither(&mut self, sample: f32, scale: f32) -> f32 {
        if self.options.dither == DitherMode::Tpdf {
            (sample + (self.uniform_random() - self.uniform_random()) / scale).clamp(-1.0, 1.0)
        } else {
            sample
        }
    }

    /// Menulis blok audio planar ke stream file.
    pub fn write_block(&mut self, block: &AudioBlock<'_>) -> io::Result<()> {
        self.write_block_frames(block, block.frames())
    }

    /// Menulis sejumlah `frame_count` dari `AudioBlock` ke stream file.
    pub fn write_block_frames(
        &mut self,
        block: &AudioBlock<'_>,
        frame_count: usize,
    ) -> io::Result<()> {
        if block.channels() != self.num_channels as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "AudioBlock channels ({}) does not match writer channels ({})",
                    block.channels(),
                    self.num_channels
                ),
            ));
        }

        let frames = frame_count.min(block.frames());
        let num_ch = self.num_channels as usize;

        for frame in 0..frames {
            for ch in 0..num_ch {
                // AudioBlock::channel_mut butuh &mut, jadi dipakai indeks slice planar
                // Borrow aman karena tiap channel berbeda
                let sample_val =
                    block.channel(ch).expect("validated channel count")[frame].clamp(-1.0, 1.0);

                match self.options.format {
                    WavSampleFormat::Pcm16Bit => {
                        let dithered = self.apply_dither(sample_val, 32768.0);
                        let val: i16 = if dithered < 0.0 {
                            (dithered * 32768.0).round() as i16
                        } else {
                            (dithered * 32767.0).round() as i16
                        };
                        self.writer.write_all(&val.to_le_bytes())?;
                    }
                    WavSampleFormat::Pcm24Bit => {
                        let dithered = self.apply_dither(sample_val, 8388608.0);
                        let val: i32 = if dithered < 0.0 {
                            (dithered * 8388608.0).round() as i32
                        } else {
                            (dithered * 8388607.0).round() as i32
                        };
                        let bytes = val.to_le_bytes();
                        self.writer.write_all(&bytes[0..3])?;
                    }
                    WavSampleFormat::Pcm32BitFloat => {
                        self.writer.write_all(&sample_val.to_le_bytes())?;
                    }
                }
            }
        }

        self.frames_written += frames;
        Ok(())
    }

    /// Menulis planar slices langsung.
    pub fn write_planar(&mut self, channels: &[&[f32]]) -> io::Result<()> {
        if channels.len() != self.num_channels as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Channel count mismatch",
            ));
        }
        if channels.is_empty() {
            return Ok(());
        }

        let frames = channels[0].len();
        for ch in channels {
            if ch.len() != frames {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "All channel slices must have the same length",
                ));
            }
        }

        for frame in 0..frames {
            for channel in channels.iter() {
                let sample_val = channel[frame].clamp(-1.0, 1.0);
                match self.options.format {
                    WavSampleFormat::Pcm16Bit => {
                        let dithered = self.apply_dither(sample_val, 32768.0);
                        let val: i16 = if dithered < 0.0 {
                            (dithered * 32768.0).round() as i16
                        } else {
                            (dithered * 32767.0).round() as i16
                        };
                        self.writer.write_all(&val.to_le_bytes())?;
                    }
                    WavSampleFormat::Pcm24Bit => {
                        let dithered = self.apply_dither(sample_val, 8388608.0);
                        let val: i32 = if dithered < 0.0 {
                            (dithered * 8388608.0).round() as i32
                        } else {
                            (dithered * 8388607.0).round() as i32
                        };
                        let bytes = val.to_le_bytes();
                        self.writer.write_all(&bytes[0..3])?;
                    }
                    WavSampleFormat::Pcm32BitFloat => {
                        self.writer.write_all(&sample_val.to_le_bytes())?;
                    }
                }
            }
        }

        self.frames_written += frames;
        Ok(())
    }

    /// Menutup stream, menulis ukuran final pada header RIFF dan data chunk, dan melakukan flush.
    pub fn finalize(mut self) -> io::Result<WavHeader> {
        let header = WavHeader::new(
            self.num_channels,
            self.sample_rate,
            self.options.format,
            self.frames_written,
        );

        let bits_per_sample = header.bits_per_sample();
        let block_align = self.num_channels as u32 * (bits_per_sample as u32 / 8);
        let data_chunk_size = (self.frames_written as u32) * block_align;
        let riff_chunk_size = 36 + data_chunk_size;

        // 1. Update ukuran RIFF (offset 4)
        self.writer.seek(SeekFrom::Start(4))?;
        self.writer.write_all(&riff_chunk_size.to_le_bytes())?;

        // 2. Update ukuran data chunk (offset 40)
        self.writer.seek(SeekFrom::Start(40))?;
        self.writer.write_all(&data_chunk_size.to_le_bytes())?;

        // 3. Posisikan ke akhir dan flush
        self.writer.seek(SeekFrom::End(0))?;
        self.writer.flush()?;

        Ok(header)
    }
}
