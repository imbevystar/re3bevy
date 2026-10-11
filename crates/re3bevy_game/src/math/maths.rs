//! re3 reference: references/re-GTA/src/math/maths.h
//!
//! Float math wrappers matching the small global helpers used throughout re3.

#[inline]
pub fn Sin(x: f32) -> f32 {
    x.sin()
}

#[inline]
pub fn Asin(x: f32) -> f32 {
    x.asin()
}

#[inline]
pub fn Cos(x: f32) -> f32 {
    x.cos()
}

#[inline]
pub fn Acos(x: f32) -> f32 {
    x.acos()
}

#[inline]
pub fn Tan(x: f32) -> f32 {
    x.tan()
}

#[inline]
pub fn Atan(x: f32) -> f32 {
    x.atan()
}

#[inline]
pub fn Atan2(y: f32, x: f32) -> f32 {
    y.atan2(x)
}

#[inline]
pub fn Abs(x: f32) -> f32 {
    x.abs()
}

#[inline]
pub fn Sqrt(x: f32) -> f32 {
    x.sqrt()
}

#[inline]
pub fn RecipSqrt(x: f32) -> f32 {
    1.0 / Sqrt(x)
}

#[inline]
pub fn RecipSqrtScaled(x: f32, y: f32) -> f32 {
    x / Sqrt(y)
}

#[inline]
pub fn Pow(x: f32, y: f32) -> f32 {
    x.powf(y)
}

#[inline]
pub fn Floor(x: f32) -> f32 {
    x.floor()
}

#[inline]
pub fn Ceil(x: f32) -> f32 {
    x.ceil()
}
