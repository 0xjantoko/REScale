//! Modul untuk konversi format sampel audio antar representasi integer dan float.

pub trait SampleConvert {
    fn to_f32(self) -> f32;
    fn from_f32(sample: f32) -> Self;
}

impl SampleConvert for i16 {
    #[inline(always)]
    fn to_f32(self) -> f32 {
        if self < 0 {
            self as f32 / 32768.0
        } else {
            self as f32 / 32767.0
        }
    }

    #[inline(always)]
    fn from_f32(sample: f32) -> Self {
        let clamped = sample.clamp(-1.0, 1.0);
        if clamped < 0.0 {
            (clamped * 32768.0).round() as i16
        } else {
            (clamped * 32767.0).round() as i16
        }
    }
}

// Catatan: literal `2147483647.0` (dan `2147483648.0`) keduanya bulat ke nilai
// f32 yang sama (2^31), karena f32 tidak dapat merepresentasikan 2147483647.
// Karena itu pembagian/penggandaan memakai 2^31 untuk kedua tanda; untuk nilai
// +1.0 hasil `as i32` saturasi ke i32::MAX (2147483647) secara otomatis.
impl SampleConvert for i32 {
    #[inline(always)]
    fn to_f32(self) -> f32 {
        self as f32 / 2147483648.0
    }

    #[inline(always)]
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * 2147483648.0).round() as i32
    }
}

impl SampleConvert for f32 {
    #[inline(always)]
    fn to_f32(self) -> f32 {
        self
    }

    #[inline(always)]
    fn from_f32(sample: f32) -> Self {
        sample
    }
}
