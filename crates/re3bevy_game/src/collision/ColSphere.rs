//! re3 reference: references/re-GTA/src/collision/ColSphere.cpp
//! re3 reference: references/re-GTA/src/collision/ColSphere.h
//!
//! C++ file-name facade for the collision sphere value type.

pub use super::ColModel::CColSphere;

impl CColSphere {
    pub fn Set(
        &mut self,
        radius: f32,
        center: super::ColModel::CColVector,
        surface: u8,
        piece: u8,
    ) {
        self.radius = radius;
        self.center = center;
        self.surface = surface;
        self.piece = piece;
    }
}
