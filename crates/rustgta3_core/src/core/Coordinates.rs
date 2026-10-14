//! re3 reference: references/re-GTA/src/math/Vector.cpp
//! re3 reference: references/re-GTA/src/math/Vector.h
//!
//! GTA keeps Z as up. Bevy keeps Y as up and looks along -Z.

use bevy::prelude::*;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl CVector {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn magnitude(self) -> f32 {
        self.magnitude_sqr().sqrt()
    }

    pub fn magnitude_sqr(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    pub fn to_bevy(self) -> Vec3 {
        Vec3::new(self.x, self.z, -self.y)
    }

    pub fn from_bevy(value: Vec3) -> Self {
        Self {
            x: value.x,
            y: -value.z,
            z: value.y,
        }
    }
}

impl Sub for CVector {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Add for CVector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for CVector {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for CVector {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for CVector {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<CVector> for f32 {
    type Output = CVector;

    fn mul(self, rhs: CVector) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<f32> for CVector {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for CVector {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Neg for CVector {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CVector2D {
    pub x: f32,
    pub y: f32,
}

#[allow(non_snake_case)]
impl CVector2D {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn from_vector(v: CVector) -> Self {
        Self { x: v.x, y: v.y }
    }

    pub fn Heading(self) -> f32 {
        (-self.x).atan2(self.y)
    }

    pub fn Magnitude(self) -> f32 {
        self.MagnitudeSqr().sqrt()
    }

    pub fn magnitude_sqr(self) -> f32 {
        self.MagnitudeSqr()
    }

    pub fn MagnitudeSqr(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn Normalise(&mut self) {
        let invsqrt = self.MagnitudeSqr().sqrt().recip();
        self.x *= invsqrt;
        self.y *= invsqrt;
    }

    pub fn NormaliseSafe(&mut self) {
        let sq = self.MagnitudeSqr();
        if sq > 0.0 {
            let invsqrt = sq.sqrt().recip();
            self.x *= invsqrt;
            self.y *= invsqrt;
        } else {
            self.x = 1.0;
        }
    }
}

impl From<CVector> for CVector2D {
    fn from(value: CVector) -> Self {
        Self::from_vector(value)
    }
}

impl Add for CVector2D {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for CVector2D {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for CVector2D {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for CVector2D {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for CVector2D {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul<CVector2D> for f32 {
    type Output = CVector2D;

    fn mul(self, rhs: CVector2D) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<f32> for CVector2D {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for CVector2D {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<f32> for CVector2D {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

pub fn DotProduct2D(v1: CVector2D, v2: CVector2D) -> f32 {
    v1.x * v2.x + v1.y * v2.y
}

pub fn CrossProduct2D(v1: CVector2D, v2: CVector2D) -> f32 {
    v1.x * v2.y - v1.y * v2.x
}

pub fn Distance2D(v: CVector2D, x: f32, y: f32) -> f32 {
    DistanceSqr2D(v, x, y).sqrt()
}

pub fn DistanceSqr2D(v: CVector2D, x: f32, y: f32) -> f32 {
    (v.x - x) * (v.x - x) + (v.y - y) * (v.y - y)
}

pub fn NormalizeXY(x: &mut f32, y: &mut f32) {
    let l = (*x * *x + *y * *y).sqrt();
    if l != 0.0 {
        *x /= l;
        *y /= l;
    } else {
        *x = 1.0;
    }
}
