//! re3 reference: references/re-GTA/src/collision/CompressedVector.h
//!
//! Collision vector storage. re3's PC path stores floats unless
//! `COMPRESSED_COL_VECTORS` is enabled.

use super::ColModel::CColVector;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CompressedVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl CompressedVector {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn Get(&self) -> CColVector {
        CColVector::new(self.x, self.y, self.z)
    }

    pub fn Set(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
}

impl From<CColVector> for CompressedVector {
    fn from(value: CColVector) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}
