//! re3 reference: references/re-GTA/src/collision/ColLine.cpp
//! re3 reference: references/re-GTA/src/collision/ColLine.h
//!
//! C++ file-name facade for the collision line value type.

pub use super::ColModel::CColLine;

impl CColLine {
    pub fn Set(&mut self, p0: super::ColModel::CColVector, p1: super::ColModel::CColVector) {
        self.p0 = p0;
        self.p1 = p1;
    }
}
