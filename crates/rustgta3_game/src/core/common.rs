//! re3 reference: references/re-GTA/src/core/common.h
//!
//! Portable subset of re3's global helpers/macros.

pub const RW_VENDOR_ID_ROCKSTAR: u32 = 0x0253F2;
pub const HALFPX: f32 = 0.5;

pub const DEFAULT_SCREEN_WIDTH: f32 = 640.0;
pub const DEFAULT_SCREEN_HEIGHT: f32 = 448.0;
pub const DEFAULT_VIEWWINDOW: f32 = 0.7;
pub const SCREEN_HEIGHT_PAL: f32 = 512.0;
pub const SCREEN_HEIGHT_NTSC: f32 = 448.0;

pub const PI: f32 = std::f32::consts::PI;
pub const TWOPI: f32 = PI * 2.0;
pub const HALFPI: f32 = PI / 2.0;

#[inline]
pub fn Max<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

#[inline]
pub fn Min<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a < b { a } else { b }
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

#[inline]
pub fn Clamp2<T>(v: T, center: T, radius: T) -> T
where
    T: PartialOrd + Copy + std::ops::Add<Output = T> + std::ops::Sub<Output = T>,
{
    if v > center {
        Min(v, center + radius)
    } else {
        Max(v, center - radius)
    }
}

#[inline]
pub fn SQR(x: f32) -> f32 {
    x * x
}

#[inline]
pub fn DEGTORAD(x: f32) -> f32 {
    x * PI / 180.0
}

#[inline]
pub fn RADTODEG(x: f32) -> f32 {
    x * 180.0 / PI
}

#[inline]
pub fn SCREEN_STRETCH_X(a: f32, screen_width: f32) -> f32 {
    a * screen_width / DEFAULT_SCREEN_WIDTH
}

#[inline]
pub fn SCREEN_STRETCH_Y(a: f32, screen_height: f32) -> f32 {
    a * screen_height / DEFAULT_SCREEN_HEIGHT
}

#[inline]
pub fn SCREEN_STRETCH_FROM_RIGHT(a: f32, screen_width: f32) -> f32 {
    screen_width - SCREEN_STRETCH_X(a, screen_width)
}

#[inline]
pub fn SCREEN_STRETCH_FROM_BOTTOM(a: f32, screen_height: f32) -> f32 {
    screen_height - SCREEN_STRETCH_Y(a, screen_height)
}

#[inline]
pub fn SCREEN_SCALE_X(a: f32, screen_width: f32) -> f32 {
    SCREEN_STRETCH_X(a, screen_width)
}

#[inline]
pub fn SCREEN_SCALE_Y(a: f32, screen_height: f32) -> f32 {
    SCREEN_STRETCH_Y(a, screen_height)
}

#[inline]
pub fn SCREEN_SCALE_FROM_RIGHT(a: f32, screen_width: f32) -> f32 {
    screen_width - SCREEN_SCALE_X(a, screen_width)
}

#[inline]
pub fn SCREEN_SCALE_FROM_BOTTOM(a: f32, screen_height: f32) -> f32 {
    screen_height - SCREEN_SCALE_Y(a, screen_height)
}

#[inline]
pub fn SCALE_AND_CENTER_X(x: f32, screen_width: f32) -> f32 {
    if screen_width == DEFAULT_SCREEN_WIDTH {
        x
    } else {
        (screen_width - SCREEN_SCALE_X(DEFAULT_SCREEN_WIDTH, screen_width)) / 2.0
            + SCREEN_SCALE_X(x, screen_width)
    }
}
