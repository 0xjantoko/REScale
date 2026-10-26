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
    /// Sisa frame tail yang belum ditiriskan oleh [`AudioProcessor::flush`].
    tail_frames_remaining: usize,
}

impl ProcessorChain {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            tail_frames_remaining: 0,
        }
    }

    pub fn push<P: AudioProcessor + 'static>(&mut self, processor: P) {
        self.processors.push(Box::new(processor));
    }

    /// Menjalankan seluruh stage tanpa menyentuh penghitung tail.
    ///
    /// Dipakai bersama oleh `process` dan `flush`: keduanya mengalirkan block
    /// melalui rantai yang sama, yang berbeda hanya asal data block dan cara
    /// penghitung tail diperbarui.
    fn process_stages(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError> {
        for processor in &mut self.processors {
            processor.process(block)?;
        }
        Ok(())
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
        self.tail_frames_remaining = self.latency_samples();
        Ok(())
    }

    fn process(&mut self, block: &mut AudioBlock<'_>) -> Result<(), EngineError> {
        self.process_stages(block)?;
        // Setiap block baru menambah tail sepanjang total latency chain.
        self.tail_frames_remaining = self.latency_samples();
        Ok(())
    }

    fn reset(&mut self) {
        for processor in &mut self.processors {
            processor.reset();
        }
        self.tail_frames_remaining = self.latency_samples();
    }

    fn latency_samples(&self) -> usize {
        self.processors
            .iter()
            .map(|processor| processor.latency_samples())
            .sum()
    }

    /// Meniriskan tail seluruh chain setelah input habis.
    ///
    /// Tail sebuah rantai processor sepanjang jumlah latency semua stage. Karena
    /// itu cukup mengalirkan block berisi senyap melewati seluruh stage: setiap
    /// delay internal (lookahead limiter, dst.) mengeluarkan sampel yang masih
    /// tertahan, lalu stage berikutnya memproses tail tersebut seperti sinyal
    /// biasa.
    ///
    /// Pendekatan ini benar untuk urutan stage apa pun — termasuk bila processor
    /// tanpa latency berada sebelum maupun sesudah processor berlatency.
    fn flush(&mut self, block: &mut AudioBlock<'_>) -> Result<usize, EngineError> {
        if self.tail_frames_remaining == 0 {
            return Ok(0);
        }

        let frames_flushed = block.frames().min(self.tail_frames_remaining);
        if frames_flushed == 0 {
            return Ok(0);
        }

        // Input senyap: tail dibangkitkan oleh state internal setiap stage.
        for channel_index in 0..block.channels() {
            block
                .channel_mut(channel_index)
                .expect("validated channel count")
                .fill(0.0);
        }
        self.process_stages(block)?;
        self.tail_frames_remaining -= frames_flushed;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drc::DetectionMode;

    const SAMPLE_RATE: f32 = 44_100.0;
    const CHANNELS: usize = 1;
    const LIMITER_CEILING_DB: f32 = -1.0;
    const LIMITER_RELEASE_MS: f32 = 50.0;
    const LIMITER_LOOKAHEAD_MS: f32 = 2.0;
    const TEST_SIGNAL_FRAMES: usize = 2_048;

    fn audio_spec() -> AudioSpec {
        AudioSpec::new(SAMPLE_RATE, CHANNELS).expect("spec audio valid")
    }

    fn limiter_latency_samples() -> usize {
        (LIMITER_LOOKAHEAD_MS * 0.001 * SAMPLE_RATE).round() as usize
    }

    fn limiter() -> LimiterProcessor {
        LimiterProcessor::new(LIMITER_CEILING_DB, LIMITER_RELEASE_MS, LIMITER_LOOKAHEAD_MS)
    }

    fn transparent_compressor() -> CompressorProcessor {
        CompressorProcessor::new(CompressorConfig {
            threshold_db: 0.0,
            ratio: 1.0,
            knee_db: 0.0,
            attack_ms: 1.0,
            release_ms: 10.0,
            makeup_gain_db: 0.0,
            mode: DetectionMode::Peak,
        })
    }

    fn full_mastering_chain() -> ProcessorChain {
        let mut chain = ProcessorChain::new();
        chain.push(GainProcessor::new(-3.0));
        chain.push(CompressorProcessor::new(CompressorConfig {
            threshold_db: -12.0,
            ratio: 3.0,
            knee_db: 4.0,
            attack_ms: 5.0,
            release_ms: 80.0,
            makeup_gain_db: 0.0,
            mode: DetectionMode::Peak,
        }));
        chain.push(limiter());
        chain.prepare(audio_spec()).expect("prepare chain");
        chain
    }

    /// Chain dengan/tanpa processor tanpa latency di depan limiter.
    fn limiter_chain(with_prefix_processor: bool) -> ProcessorChain {
        let mut chain = ProcessorChain::new();
        if with_prefix_processor {
            chain.push(GainProcessor::new(0.0));
        }
        chain.push(transparent_compressor());
        chain.push(limiter());
        chain.prepare(audio_spec()).expect("prepare chain");
        chain
    }

    fn process_block(chain: &mut ProcessorChain, samples: &mut [f32]) {
        let mut channel_views: Vec<&mut [f32]> = vec![samples];
        let mut block = AudioBlock::new(&mut channel_views).expect("block valid");
        chain.process(&mut block).expect("process block");
    }

    /// Meniriskan tail sampai habis, mengembalikan sampel yang dihasilkan.
    fn flush_all(chain: &mut ProcessorChain, block_size: usize) -> Vec<f32> {
        let mut tail = Vec::new();
        loop {
            let mut samples = vec![0.0_f32; block_size];
            let mut channel_views: Vec<&mut [f32]> = vec![samples.as_mut_slice()];
            let mut block = AudioBlock::new(&mut channel_views).expect("flush block");
            let flushed = chain.flush(&mut block).expect("flush");
            if flushed == 0 {
                break;
            }
            tail.extend_from_slice(&samples[..flushed]);
        }
        tail
    }

    fn test_signal(frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                let time = index as f32 / SAMPLE_RATE;
                (2.0 * std::f32::consts::PI * 220.0 * time).sin() * 0.3
            })
            .collect()
    }

    /// Merender sinyal uji dengan ukuran block tertentu, termasuk tail hasil flush.
    fn render_with_block_size(block_size: usize) -> Vec<f32> {
        let input = test_signal(TEST_SIGNAL_FRAMES);
        let mut chain = full_mastering_chain();
        let mut output = Vec::with_capacity(TEST_SIGNAL_FRAMES + limiter_latency_samples());

        for chunk in input.chunks(block_size) {
            let mut samples = chunk.to_vec();
            process_block(&mut chain, &mut samples);
            output.extend_from_slice(&samples);
        }
        output.extend(flush_all(&mut chain, block_size));
        output
    }

    #[test]
    fn flush_drains_limiter_tail_regardless_of_prefix_processors() {
        let latency = limiter_latency_samples();
        let mut tails = Vec::new();

        for with_prefix_processor in [false, true] {
            let mut chain = limiter_chain(with_prefix_processor);
            let mut impulse = vec![0.0_f32; 64];
            impulse[0] = 0.5;
            process_block(&mut chain, &mut impulse);
            tails.push(flush_all(&mut chain, latency.max(1)));
        }

        assert_eq!(
            tails[0].len(),
            latency,
            "tail harus sepanjang latency limiter"
        );
        assert_eq!(tails[1].len(), latency);

        // Gain 0 dB tidak boleh mengubah tail: hasil kedua chain harus identik.
        for (plain, prefixed) in tails[0].iter().zip(tails[1].iter()) {
            assert!((plain - prefixed).abs() < 1e-6);
        }

        let impulse_survives = tails[0]
            .iter()
            .any(|sample| (sample.abs() - 0.5).abs() < 1e-3);
        assert!(
            impulse_survives,
            "tail harus memuat impulse 0.5 yang tertahan lookahead"
        );
    }

    #[test]
    fn flush_drains_the_tail_of_every_latency_stage() {
        let mut chain = ProcessorChain::new();
        chain.push(LimiterProcessor::new(
            LIMITER_CEILING_DB,
            LIMITER_RELEASE_MS,
            1.0,
        ));
        chain.push(limiter());
        chain.prepare(audio_spec()).expect("prepare chain");

        let expected_latency = chain.latency_samples();
        assert_eq!(
            expected_latency,
            chain.latency_samples(),
            "total latency = jumlah latency setiap stage"
        );

        let mut impulse = vec![0.0_f32; 32];
        impulse[0] = 0.5;
        process_block(&mut chain, &mut impulse);

        let tail = flush_all(&mut chain, expected_latency + 16);
        assert_eq!(
            tail.len(),
            expected_latency,
            "flush harus meniriskan tail dari semua stage berlatency"
        );
        let impulse_survives = tail.iter().any(|sample| (sample.abs() - 0.5).abs() < 1e-3);
        assert!(impulse_survives, "impulse harus keluar dari kedua limiter");
    }

    #[test]
    fn flush_returns_zero_when_chain_has_no_latency() {
        let mut chain = ProcessorChain::new();
        chain.push(GainProcessor::new(0.0));
        chain.prepare(audio_spec()).expect("prepare chain");

        let mut samples = vec![0.1_f32; 16];
        process_block(&mut chain, &mut samples);

        assert_eq!(chain.latency_samples(), 0);
        assert!(flush_all(&mut chain, 16).is_empty());
    }

    #[test]
    fn block_size_does_not_change_output() {
        let reference = render_with_block_size(TEST_SIGNAL_FRAMES);

        for block_size in [64, 256, 1_024] {
            let rendered = render_with_block_size(block_size);

            assert_eq!(
                rendered.len(),
                reference.len(),
                "panjang output berbeda pada blok {block_size}"
            );
            for (frame, (expected, actual)) in reference.iter().zip(rendered.iter()).enumerate() {
                assert!(
                    (expected - actual).abs() < 1e-6,
                    "blok {block_size}: frame {frame} berbeda ({expected} vs {actual})"
                );
            }
        }
    }
}
