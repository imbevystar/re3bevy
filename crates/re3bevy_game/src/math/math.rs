//! re3 reference: references/re-GTA/src/math/math.cpp
//! re3 reference: references/re-GTA/src/math/math.h
//!
//! Compatibility facade for re3's global math helpers.

pub use super::maths::{
    Abs, Acos, Asin, Atan, Atan2, Ceil, Cos, Floor, Pow, RecipSqrt, RecipSqrtScaled, Sin, Sqrt, Tan,
};

#[inline]
pub fn SQR(x: f32) -> f32 {
    x * x
}

#[inline]
pub fn sq(x: f32) -> f32 {
    x * x
}

#[inline]
pub fn Min<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

#[inline]
pub fn Max<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

#[inline]
pub fn Clamp<T: PartialOrd + Copy>(v: T, min: T, max: T) -> T {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}
