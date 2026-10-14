//! re3 reference: references/re-GTA/src/control/Curves.cpp
//! re3 reference: references/re-GTA/src/control/Curves.h
//!
//! Cubic curve helpers used by cameras, missions, and path effects.

use rust_gta_3_core::core::Coordinates::CVector;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CCurvePoint {
    pub position: CVector,
    pub tangent: CVector,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CCurve {
    pub points: Vec<CCurvePoint>,
}

impl CCurve {
    #[allow(non_snake_case)]
    pub fn AddPoint(&mut self, position: CVector, tangent: CVector) {
        self.points.push(CCurvePoint { position, tangent });
    }

    #[allow(non_snake_case)]
    pub fn GetPoint(&self, index: usize) -> Option<CCurvePoint> {
        self.points.get(index).copied()
    }
}

pub struct CCurves;

impl CCurves {
    pub fn CalcSpeedScaleFactor(
        point1: CVector,
        point2: CVector,
        dir1_x: f32,
        dir1_y: f32,
        dir2_x: f32,
        dir2_y: f32,
    ) -> f32 {
        let dx = point1.x - point2.x;
        let dy = point1.y - point2.y;
        let distance = (dx * dx + dy * dy).sqrt();
        let dot = dir1_x * dir2_x + dir1_y * dir2_y;
        if dot > 0.9 {
            distance
                + ((point1.x * dir1_y - point1.y * dir1_x)
                    - (point2.x * dir1_y - point2.y * dir1_x))
                    .abs()
        } else {
            ((1.0 - dot) * 0.2 + 1.0) * distance
        }
    }

    pub fn CalcCurvePoint(
        pos1: CVector,
        pos2: CVector,
        dir1: CVector,
        dir2: CVector,
        between: f32,
        time_on_curve_ms: i32,
    ) -> (CVector, CVector) {
        let scale = Self::CalcSpeedScaleFactor(pos1, pos2, dir1.x, dir1.y, dir2.x, dir2.y);
        let dir1_x = dir1.x * scale;
        let dir1_y = dir1.y * scale;
        let dir2_x = dir2.x * scale;
        let dir2_y = dir2.y * scale;
        // Keep re3's literal 3.1415 rather than substituting PI.
        let curve_coefficient = 0.5 - 0.5 * (3.1415 * between).cos();
        let inverse = 1.0 - curve_coefficient;
        let position = CVector::new(
            (pos1.x + between * dir1_x) * inverse
                + (pos2.x - (1.0 - between) * dir2_x) * curve_coefficient,
            (pos1.y + between * dir1_y) * inverse
                + (pos2.y - (1.0 - between) * dir2_y) * curve_coefficient,
            0.0,
        );
        let seconds = time_on_curve_ms as f32 * 0.001;
        let direction = CVector::new(
            (dir1_x * inverse + dir2_x * curve_coefficient) / seconds,
            (dir1_y * inverse + dir2_y * curve_coefficient) / seconds,
            0.0,
        );
        (position, direction)
    }
}
