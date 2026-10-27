//! Modul Dynamic Range Control (DRC) Audio Engine

pub mod compressor;
pub mod envelope;
pub mod limiter;
pub mod max_hold;

pub use compressor::{Compressor, CompressorConfig};
pub use envelope::{DetectionMode, EnvelopeFollower};
pub use limiter::{BrickwallLimiter, LookaheadDetector};
pub use max_hold::SlidingMax;
