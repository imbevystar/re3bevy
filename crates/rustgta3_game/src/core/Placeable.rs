//! re3 reference: references/re-GTA/src/core/Placeable.cpp
//! re3 reference: references/re-GTA/src/core/Placeable.h
//!
//! CPlaceable transform component.

use bevy::prelude::Component;
use rustgta3_core::core::Coordinates::CVector;

use crate::math::Matrix::CMatrix;

#[derive(Debug, Copy, Clone, PartialEq, Component)]
pub struct CPlaceable {
    pub matrix: CMatrix,
}

impl Default for CPlaceable {
    fn default() -> Self {
        Self {
            matrix: CMatrix::unity(),
        }
    }
}

impl CPlaceable {
    #[allow(non_snake_case)]
    pub fn GetPosition(&self) -> CVector {
        self.matrix.position
    }

    #[allow(non_snake_case)]
    pub fn SetPosition(&mut self, position: CVector) {
        self.matrix.position = position;
    }

    #[allow(non_snake_case)]
    pub fn GetRight(&self) -> CVector {
        self.matrix.right
    }

    #[allow(non_snake_case)]
    pub fn GetForward(&self) -> CVector {
        self.matrix.forward
    }

    #[allow(non_snake_case)]
    pub fn GetUp(&self) -> CVector {
        self.matrix.up
    }

    #[allow(non_snake_case)]
    pub fn SetMatrix(&mut self, matrix: CMatrix) {
        self.matrix = matrix;
    }

    #[allow(non_snake_case)]
    pub fn SetHeading(&mut self, angle: f32) {
        let position = self.matrix.position;
        self.matrix.set_rotate_z(angle);
        self.matrix.translate(position);
    }

    #[allow(non_snake_case)]
    pub fn SetOrientation(&mut self, x: f32, y: f32, z: f32) {
        let position = self.matrix.position;
        self.matrix.set_rotate(x, y, z);
        self.matrix.translate(position);
    }

    #[allow(non_snake_case)]
    pub fn IsWithinArea2D(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
        let (x1, x2) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
        let (y1, y2) = if y1 > y2 { (y2, y1) } else { (y1, y2) };
        x1 <= self.matrix.position.x
            && self.matrix.position.x <= x2
            && y1 <= self.matrix.position.y
            && self.matrix.position.y <= y2
    }

    #[allow(non_snake_case)]
    pub fn IsWithinArea3D(&self, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> bool {
        let (x1, x2) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
        let (y1, y2) = if y1 > y2 { (y2, y1) } else { (y1, y2) };
        let (z1, z2) = if z1 > z2 { (z2, z1) } else { (z1, z2) };
        x1 <= self.matrix.position.x
            && self.matrix.position.x <= x2
            && y1 <= self.matrix.position.y
            && self.matrix.position.y <= y2
            && z1 <= self.matrix.position.z
            && self.matrix.position.z <= z2
    }
}
