//! re3 reference: references/re-GTA/src/math/Rect.cpp
//! re3 reference: references/re-GTA/src/math/Rect.h
//!
//! `CRect` with re3's left/top/right/bottom coordinate convention.

use super::{Vector::CVector, Vector2D::CVector2D};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CRect {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

impl Default for CRect {
    fn default() -> Self {
        Self {
            left: 1_000_000.0,
            top: 1_000_000.0,
            right: -1_000_000.0,
            bottom: -1_000_000.0,
        }
    }
}

impl CRect {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn ContainPoint(&mut self, v: CVector) {
        if v.x < self.left {
            self.left = v.x;
        }
        if v.x > self.right {
            self.right = v.x;
        }
        if v.y < self.top {
            self.top = v.y;
        }
        if v.y > self.bottom {
            self.bottom = v.y;
        }
    }

    pub fn ContainRect(&mut self, r: CRect) {
        if r.left < self.left {
            self.left = r.left;
        }
        if r.right > self.right {
            self.right = r.right;
        }
        if r.top < self.top {
            self.top = r.top;
        }
        if r.bottom > self.bottom {
            self.bottom = r.bottom;
        }
    }

    pub fn IsPointInside(&self, p: CVector2D) -> bool {
        p.x >= self.left && p.x <= self.right && p.y >= self.top && p.y <= self.bottom
    }

    pub fn IsPointInsideWithRadius(&self, p: CVector2D, extra_radius: f32) -> bool {
        p.x >= self.left - extra_radius
            && p.x <= self.right + extra_radius
            && p.y >= self.top - extra_radius
            && p.y <= self.bottom + extra_radius
    }

    pub fn Translate(&mut self, x: f32, y: f32) {
        self.left += x;
        self.right += x;
        self.bottom += y;
        self.top += y;
    }

    pub fn Grow(&mut self, r: f32) {
        self.left -= r;
        self.right += r;
        self.top -= r;
        self.bottom += r;
    }

    pub fn GrowLR(&mut self, l: f32, r: f32) {
        self.left -= l;
        self.top -= l;
        self.right += r;
        self.bottom += r;
    }

    pub fn GrowLTRB(&mut self, l: f32, r: f32, t: f32, b: f32) {
        self.left -= l;
        self.top -= t;
        self.right += r;
        self.bottom += b;
    }

    pub fn GetWidth(&self) -> f32 {
        self.right - self.left
    }

    pub fn GetHeight(&self) -> f32 {
        self.bottom - self.top
    }
}
