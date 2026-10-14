//! re3 reference: references/re-GTA/src/core/Range2D.cpp
//! re3 reference: references/re-GTA/src/core/Range2D.h
//!
//! Rectangular 2D range helper used by mission/gameplay code.

use rust_gta_3_core::core::{Coordinates::CVector2D, General::CGeneral};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CRange2D {
    pub min: CVector2D,
    pub max: CVector2D,
}

impl CRange2D {
    pub const fn new(min: CVector2D, max: CVector2D) -> Self {
        Self { min, max }
    }

    pub fn IsInRange(&self, vec: CVector2D) -> bool {
        self.min.x < vec.x && self.max.x > vec.x && self.min.y < vec.y && self.max.y > vec.y
    }

    pub fn DebugShowRange(&self, _height: f32, _colour: i32) {}

    pub fn GetRandomPointInRange(&self, general: &mut CGeneral) -> CVector2D {
        let dist_x = (self.max.x - self.min.x).abs() as i32;
        let dist_y = (self.max.y - self.min.y).abs() as i32;
        let out_x = random_mod(general, dist_x) as f32 + self.min.x;
        let out_y = random_mod(general, dist_y) as f32 + self.min.y;
        CVector2D::new(out_x, out_y)
    }
}

fn random_mod(general: &mut CGeneral, modulus: i32) -> i32 {
    if modulus <= 0 {
        0
    } else {
        general.myrand() % modulus
    }
}
