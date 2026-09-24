//! Modul Dynamic Range Control (DRC) Audio Engine

pub mod compressor;
pub mod envelope;
pub mod limiter;

pub use compressor::{Compressor, CompressorConfig};
pub use envelope::{DetectionMode, EnvelopeFollower};
pub use limiter::BrickwallLimiter;
