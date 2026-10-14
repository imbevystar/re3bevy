//! re3 reference: references/re-GTA/src/collision/ColBox.cpp
//! re3 reference: references/re-GTA/src/collision/ColBox.h
//!
//! C++ file-name facade for the collision box value type.

pub use super::ColModel::CColBox;

impl CColBox {
    pub fn Set(
        &mut self,
        min: super::ColModel::CColVector,
        max: super::ColModel::CColVector,
        surface: u8,
        piece: u8,
    ) {
        self.min = min;
        self.max = max;
        self.surface = surface;
        self.piece = piece;
    }

    pub fn GetSize(&self) -> super::ColModel::CColVector {
        super::ColModel::CColVector::new(
            self.max.x - self.min.x,
            self.max.y - self.min.y,
            self.max.z - self.min.z,
        )
    }
}
