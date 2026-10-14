//! re3 reference: references/re-GTA/src/math/Quaternion.cpp
//! re3 reference: references/re-GTA/src/math/Quaternion.h
//!
//! Quaternion value type used by re3 animation blending.

use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, SubAssign};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CQuaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl CQuaternion {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }

    pub fn normalise(&mut self) {
        let len = self.dot(*self).sqrt();
        if len > 0.0 {
            self.x /= len;
            self.y /= len;
            self.z /= len;
            self.w /= len;
        }
    }

    pub fn normalised(mut self) -> Self {
        self.normalise();
        self
    }

    pub fn slerp(from: Self, to: Self, theta: f32, inv_sin: f32, t: f32) -> Self {
        if theta == 0.0 || inv_sin == 0.0 {
            return from;
        }

        let left = ((1.0 - t) * theta).sin() * inv_sin;
        let right = (t * theta).sin() * inv_sin;
        from * left + to * right
    }
}

impl Add for CQuaternion {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
            self.w + rhs.w,
        )
    }
}

impl AddAssign for CQuaternion {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for CQuaternion {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
        self.w -= rhs.w;
    }
}

impl Mul<f32> for CQuaternion {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

impl MulAssign<f32> for CQuaternion {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Neg for CQuaternion {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}
