//! re3 reference: references/re-GTA/src/math/VuVector.h
//!
//! 16-byte vector used by the PC fallback of VU collision routines.

use std::ops::{Add, AddAssign, Mul, MulAssign, Sub};

use rust_gta_3_core::core::Coordinates::CVector;

use super::Matrix::CMatrix;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CVuVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl CVuVector {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, w: 0.0 }
    }

    pub const fn new4(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn MagnitudeSqr(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn Magnitude(self) -> f32 {
        self.MagnitudeSqr().sqrt()
    }

    pub fn Normalise(&mut self) {
        let sq = self.MagnitudeSqr();
        if sq > 0.0 {
            let inv = 1.0 / sq.sqrt();
            self.x *= inv;
            self.y *= inv;
            self.z *= inv;
        } else {
            self.x = 1.0;
        }
    }

    pub fn xyz(self) -> CVector {
        CVector::new(self.x, self.y, self.z)
    }
}

impl From<CVector> for CVuVector {
    fn from(value: CVector) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl Add for CVuVector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new4(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
            self.w + rhs.w,
        )
    }
}

impl AddAssign for CVuVector {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for CVuVector {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new4(
            self.x - rhs.x,
            self.y - rhs.y,
            self.z - rhs.z,
            self.w - rhs.w,
        )
    }
}

impl Mul<f32> for CVuVector {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new4(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl Mul<CVuVector> for f32 {
    type Output = CVuVector;

    fn mul(self, rhs: CVuVector) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<f32> for CVuVector {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

pub fn DotProduct(a: CVuVector, b: CVuVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

pub fn CrossProduct(a: CVuVector, b: CVuVector) -> CVuVector {
    CVuVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

pub fn TransformPoint(out: &mut CVuVector, mat: &CMatrix, input: CVuVector) {
    *out = CVuVector::from(mat.transform_point(input.xyz()));
    out.w = input.w;
}

pub fn TransformPoints(out: &mut [CVuVector], mat: &CMatrix, input: &[CVuVector]) {
    for (out, input) in out.iter_mut().zip(input.iter().copied()) {
        TransformPoint(out, mat, input);
    }
}
