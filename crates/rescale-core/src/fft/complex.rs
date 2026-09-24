//! Struktur bilangan kompleks f32 untuk komputasi Fourier Transform.

use std::ops::{Add, AddAssign, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    #[inline(always)]
    pub fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    /// Membuat complex number dari representasi polar e^(i * theta)
    #[inline(always)]
    pub fn from_polar(magnitude: f32, theta: f32) -> Self {
        Self {
            re: magnitude * theta.cos(),
            im: magnitude * theta.sin(),
        }
    }

    /// Menghitung magnitudo absolut / modulus |z| = sqrt(re^2 + im^2)
    #[inline(always)]
    pub fn magnitude(self) -> f32 {
        (self.re * self.re + self.im * self.im).sqrt()
    }

    /// Menghitung power spectrum |z|^2 = re^2 + im^2
    #[inline(always)]
    pub fn power(self) -> f32 {
        self.re * self.re + self.im * self.im
    }

    /// Menghitung sudut fase arg(z) = atan2(im, re) dalam radian
    #[inline(always)]
    pub fn phase(self) -> f32 {
        self.im.atan2(self.re)
    }

    /// Konjugat kompleks: conj(a + bi) = a - bi
    #[inline(always)]
    pub fn conjugate(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }
}

impl Add for Complex {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl AddAssign for Complex {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.re += rhs.re;
        self.im += rhs.im;
    }
}

impl Sub for Complex {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl Mul for Complex {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        // (a + bi)(c + di) = (ac - bd) + (ad + bc)i
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl Mul<f32> for Complex {
    type Output = Self;
    #[inline(always)]
    fn mul(self, scalar: f32) -> Self::Output {
        Self {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }
}
