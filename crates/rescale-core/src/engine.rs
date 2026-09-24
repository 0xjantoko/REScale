//! Fondasi engine audio untuk pemrosesan block-based dan real-time friendly.

use crate::filter::ParametricEqualizer3Band;
use crate::gain::Gain;
use crate::{BrickwallLimiter, Compressor, CompressorConfig};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioSpec {
    pub sample_rate: f32,
    pub channels: usize,
}

impl AudioSpec {
    pub fn new(sample_rate: f32, channels: usize) -> Result<Self, EngineError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EngineError::InvalidSampleRate(sample_rate));
        }
        if channels == 0 {
            return Err(EngineError::NoChannels);
        }
        Ok(Self {
            sample_rate,
            channels,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EngineError {
    InvalidSampleRate(f32),
    NoChannels,
    ChannelCountMismatch { expected: usize, actual: usize },
    UnequalChannelLengths,
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate(value) => write!(formatter, "Invalid sample rate: {value}"),
            Self::NoChannels => write!(formatter, "Audio requires at least one channel"),
            Self::ChannelCountMismatch { expected, actual } => {
                write!(formatter, "Expected {expected} channels, received {actual}")
            }
            Self::UnequalChannelLengths => write!(formatter, "All channels in a block must match"),
        }
    }
}

impl Error for EngineError {}

/// Tampilan planar atas sebuah block audio. Engine tidak mengambil ownership buffer.
pub struct AudioBlock<'a> {
    pub(crate) channels: &'a mut [&'a mut [f32]],
    frames: usize,
}

impl<'a> AudioBlock<'a> {
    pub fn new(channels: &'a mut [&'a mut [f32]]) -> Result<Self, EngineError> {
        let Some(first_channel) = channels.first() else {
            return Err(EngineError::NoChannels);
        };
        let frames = first_channel.len();
        if channels.iter().any(|channel| channel.len() != frames) {
            return Err(EngineError::UnequalChannelLengths);
        }
        Ok(Self { channels, frames })
    }

    pub fn channels(&self) -> usize {
        self.channels.len()
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn channel(&self, index: usize) -> Option<&[f32]> {
        self.channels.get(index).map(|channel| &**channel as &[f32])
    }

    pub fn channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        self.channels.get_mut(index).map(|channel| &mut **channel)
    }
}

/// Kontrak processor tanpa I/O atau alokasi di jalur pemrosesan.
pub trait AudioProcessor {
    fn prepare(&mut self, spec: AudioSpec) -> Result<(), EngineError>;
    fn process(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError>;
    fn reset(&mut self);
    fn latency_samples(&self) -> usize {
        0
    }
    /// Mengeluarkan sisa sampel internal (misal tail lookahead limiter) setelah input habis.
    /// Mengembalikan jumlah frame valid yang ditulis ke dalam `block`.
    fn flush(&mut self, _block: &mut AudioBlock<'_>) -> Result<usize, EngineError> {
        Ok(0)
    }
}

pub struct ProcessorChain {
    processors: Vec<Box<dyn AudioProcessor>>,
}

impl ProcessorChain {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn push<P: AudioProcessor + 'static>(&mut self, processor: P) {
        self.processors.push(Box::new(processor));
    }
}

impl Default for ProcessorChain {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProcessor for ProcessorChain {
    fn prepare(&mut self, spec: AudioSpec) -> Result<(), EngineError> {
        for processor in &mut self.processors {
            processor.prepare(spec)?;
        }
        Ok(())
    }

    fn process(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError> {
        for processor in &mut self.processors {
            processor.process(block)?;
        }
        Ok(())
    }

    fn reset(&mut self) {
        for processor in &mut self.processors {
            processor.reset();
        }
    }

    fn latency_samples(&self) -> usize {
        self.processors
            .iter()
            .map(|processor| processor.latency_samples())
            .sum()
    }

    fn flush(&mut self, block: &mut AudioBlock<'_>) -> Result<usize, EngineError> {
        let mut frames_flushed = 0;
        for processor in &mut self.processors {
            if frames_flushed == 0 {
                frames_flushed = processor.flush(block)?;
            } else {
                processor.process(block)?;
            }
        }
        Ok(frames_flushed)
    }
}

pub struct GainProcessor {
    gain_db: f32,
    gain_linear: f32,
    channels: usize,
}

impl GainProcessor {
    pub fn new(gain_db: f32) -> Self {
        Self {
            gain_db,
            gain_linear: Gain::db_to_linear(gain_db),
            channels: 0,
        }
    }

    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.gain_db = gain_db;
        self.gain_linear = Gain::db_to_linear(gain_db);
    }
}

impl AudioProcessor for GainProcessor {
    fn prepare(&mut self, spec: AudioSpec) -> Result<(), EngineError> {
        self.channels = spec.channels;
        Ok(())
    }

    fn process(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError> {
        if block.channels() != self.channels {
            return Err(EngineError::ChannelCountMismatch {
                expected: self.channels,
                actual: block.channels(),
            });
        }
        for ch in 0..block.channels() {
            let channel = block.channel_mut(ch).expect("validated channel count");
            Gain::apply_linear(channel, self.gain_linear);
        }
        Ok(())
    }

    fn reset(&mut self) {}
}

pub struct EqualizerProcessor {
    eqs: Vec<ParametricEqualizer3Band>,
    low_freq: f32,
    low_gain_db: f32,
    mid_freq: f32,
    mid_q: f32,
    mid_gain_db: f32,
    high_freq: f32,
    high_gain_db: f32,
}

impl EqualizerProcessor {
    pub fn new(
        low_freq: f32,
        low_gain_db: f32,
        mid_freq: f32,
        mid_q: f32,
        mid_gain_db: f32,
        high_freq: f32,
        high_gain_db: f32,
    ) -> Self {
        Self {
            eqs: Vec::new(),
            low_freq,
            low_gain_db,
            mid_freq,
            mid_q,
            mid_gain_db,
            high_freq,
            high_gain_db,
        }
    }
}

impl AudioProcessor for EqualizerProcessor {
    fn prepare(&mut self, spec: AudioSpec) -> Result<(), EngineError> {
        self.eqs = (0..spec.channels)
            .map(|_| {
                let mut eq = ParametricEqualizer3Band::new(spec.sample_rate);
                eq.set_low_band(self.low_freq, self.low_gain_db);
                eq.set_mid_band(self.mid_freq, self.mid_q, self.mid_gain_db);
                eq.set_high_band(self.high_freq, self.high_gain_db);
                eq
            })
            .collect();
        Ok(())
    }

    fn process(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError> {
        if block.channels() != self.eqs.len() {
            return Err(EngineError::ChannelCountMismatch {
                expected: self.eqs.len(),
                actual: block.channels(),
            });
        }
        for (index, eq) in self.eqs.iter_mut().enumerate() {
            let channel = block.channel_mut(index).expect("validated channel count");
            eq.process_buffer(channel);
        }
        Ok(())
    }

    fn reset(&mut self) {
        for eq in &mut self.eqs {
            eq.reset();
        }
    }
}

pub struct CompressorProcessor {
    config: CompressorConfig,
    processors: Vec<Compressor>,
}

impl CompressorProcessor {
    pub fn new(config: CompressorConfig) -> Self {
        Self {
            config,
            processors: Vec::new(),
        }
    }
}

impl AudioProcessor for CompressorProcessor {
    fn prepare(&mut self, spec: AudioSpec) -> Result<(), EngineError> {
        self.processors = (0..spec.channels)
            .map(|_| Compressor::new(spec.sample_rate, self.config.clone()))
            .collect();
        Ok(())
    }

    fn process(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError> {
        if block.channels() != self.processors.len() {
            return Err(EngineError::ChannelCountMismatch {
                expected: self.processors.len(),
                actual: block.channels(),
            });
        }
        for (index, processor) in self.processors.iter_mut().enumerate() {
            let channel = block
                .channel_mut(index)
                .expect("channel count was validated");
            processor.process_buffer(channel);
        }
        Ok(())
    }

    fn reset(&mut self) {
        for processor in &mut self.processors {
            processor.reset();
        }
    }
}

pub struct LimiterProcessor {
    ceiling_db: f32,
    release_ms: f32,
    lookahead_ms: f32,
    processors: Vec<BrickwallLimiter>,
    latency_samples: usize,
    pending_flush_samples: usize,
}

impl LimiterProcessor {
    pub fn new(ceiling_db: f32, release_ms: f32, lookahead_ms: f32) -> Self {
        Self {
            ceiling_db,
            release_ms,
            lookahead_ms,
            processors: Vec::new(),
            latency_samples: 0,
            pending_flush_samples: 0,
        }
    }
}

impl AudioProcessor for LimiterProcessor {
    fn prepare(&mut self, spec: AudioSpec) -> Result<(), EngineError> {
        self.latency_samples = (self.lookahead_ms * 0.001 * spec.sample_rate)
            .round()
            .max(1.0) as usize;
        self.pending_flush_samples = self.latency_samples;
        self.processors = (0..spec.channels)
            .map(|_| {
                BrickwallLimiter::new(
                    spec.sample_rate,
                    self.ceiling_db,
                    self.release_ms,
                    self.lookahead_ms,
                )
            })
            .collect();
        Ok(())
    }

    fn process(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError> {
        if block.channels() != self.processors.len() {
            return Err(EngineError::ChannelCountMismatch {
                expected: self.processors.len(),
                actual: block.channels(),
            });
        }
        self.pending_flush_samples = self.latency_samples;
        for (index, processor) in self.processors.iter_mut().enumerate() {
            let channel = block
                .channel_mut(index)
                .expect("channel count was validated");
            processor.process_buffer(channel);
        }
        Ok(())
    }

    fn reset(&mut self) {
        for processor in &mut self.processors {
            processor.reset();
        }
        self.pending_flush_samples = self.latency_samples;
    }

    fn latency_samples(&self) -> usize {
        self.latency_samples
    }

    fn flush(&mut self, block: &mut AudioBlock<'_>) -> Result<usize, EngineError> {
        if block.channels() != self.processors.len() {
            return Err(EngineError::ChannelCountMismatch {
                expected: self.processors.len(),
                actual: block.channels(),
            });
        }
        if self.pending_flush_samples == 0 {
            return Ok(0);
        }

        let frames_to_flush = block.frames().min(self.pending_flush_samples);
        for ch in 0..self.processors.len() {
            let channel_slice = block.channel_mut(ch).expect("validated channel count");
            let limiter = &mut self.processors[ch];
            for sample in channel_slice[..frames_to_flush].iter_mut() {
                *sample = limiter.process_sample(0.0);
            }
            for sample in channel_slice[frames_to_flush..].iter_mut() {
                *sample = 0.0;
            }
        }
        self.pending_flush_samples -= frames_to_flush;
        Ok(frames_to_flush)
    }
}
