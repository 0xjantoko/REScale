//! Modul Filter Digital IIR & Equalizer (Zero Dependency)

pub mod biquad;
pub mod coefficients;
pub mod equalizer;

pub use biquad::BiquadFilter;
pub use coefficients::BiquadCoefficients;
pub use equalizer::ParametricEqualizer3Band;
