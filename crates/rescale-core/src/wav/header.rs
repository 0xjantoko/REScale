//! Definisi struktur data RIFF WAVE header.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavSampleFormat {
    Pcm16Bit,
    Pcm24Bit,
    Pcm32BitFloat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WavHeader {
    pub num_channels: u16,
    pub sample_rate: u32,
    pub format: WavSampleFormat,
    pub num_samples_per_channel: usize,
}

impl WavHeader {
    pub fn new(
        num_channels: u16,
        sample_rate: u32,
        format: WavSampleFormat,
        num_samples_per_channel: usize,
    ) -> Self {
        Self {
            num_channels,
            sample_rate,
            format,
            num_samples_per_channel,
        }
    }

    pub fn bits_per_sample(&self) -> u16 {
        match self.format {
            WavSampleFormat::Pcm16Bit => 16,
            WavSampleFormat::Pcm24Bit => 24,
            WavSampleFormat::Pcm32BitFloat => 32,
        }
    }

    pub fn audio_format_tag(&self) -> u16 {
        match self.format {
            WavSampleFormat::Pcm16Bit | WavSampleFormat::Pcm24Bit => 1, // WAVE_FORMAT_PCM
            WavSampleFormat::Pcm32BitFloat => 3,                        // WAVE_FORMAT_IEEE_FLOAT
        }
    }
}
