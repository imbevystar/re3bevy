//! re3 reference: references/re-GTA/src/core/Range3D.cpp
//! re3 reference: references/re-GTA/src/core/Range3D.h
//!
//! Box-shaped 3D range helper used by mission/gameplay code.

use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CRange3D {
    pub min: CVector,
    pub max: CVector,
}

impl CRange3D {
    pub const fn new(min: CVector, max: CVector) -> Self {
        Self { min, max }
    }

    pub fn IsInRange(&self, vec: CVector) -> bool {
        self.min.x < vec.x
            && self.max.x > vec.x
            && self.min.y < vec.y
            && self.max.y > vec.y
            && self.min.z < vec.z
            && self.max.z > vec.z
    }

    pub fn DebugShowRange(&self, _height: f32, _colour: i32) {}

    pub fn GetRandomPointInRange(&self, general: &mut CGeneral) -> CVector {
        let dist_x = (self.max.x - self.min.x).abs() as i32;
        let dist_y = (self.max.y - self.min.y).abs() as i32;
        let dist_z = (self.max.z - self.min.z).abs() as i32;
        let out_x = random_mod(general, dist_x) as f32 + self.min.x;
        let out_y = random_mod(general, dist_y) as f32 + self.min.y;
        let out_z = random_mod(general, dist_z) as f32 + self.min.z;
        CVector::new(out_x, out_y, out_z)
    }
}

fn random_mod(general: &mut CGeneral, modulus: i32) -> i32 {
    if modulus <= 0 {
        0
    } else {
        general.myrand() % modulus
    }
}
