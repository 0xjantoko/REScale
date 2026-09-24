//! Modul I/O format RIFF WAV (Zero Dependency)

pub mod header;
pub mod reader;
pub mod stream;
pub mod writer;

pub use header::{WavHeader, WavSampleFormat};
pub use reader::WavReader;
pub use stream::{WavStreamReader, WavStreamWriter};
pub use writer::{DitherMode, WavWriteOptions, WavWriter};
