//! re3 reference: references/re-GTA/src/collision/ColTriangle.cpp
//! re3 reference: references/re-GTA/src/collision/ColTriangle.h
//!
//! Triangle indices and collision plane setup.

use super::{ColModel::CColVector, CompressedVector::CompressedVector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    DIR_X_POS = 0,
    DIR_X_NEG = 1,
    DIR_Y_POS = 2,
    DIR_Y_NEG = 3,
    DIR_Z_POS = 4,
    DIR_Z_NEG = 5,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CColTriangle {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub surface: u8,
}

impl CColTriangle {
    pub fn Set(
        &mut self,
        vertices: &[CompressedVector],
        a: usize,
        b: usize,
        c: usize,
        surface: u8,
        _piece: u8,
    ) {
        self.a = a as u16;
        self.b = b as u16;
        self.c = c as u16;
        self.surface = surface;
        let _ = vertices;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CColTrianglePlane {
    pub normal: CColVector,
    pub dist: f32,
    pub dir: Direction,
}

impl Default for CColTrianglePlane {
    fn default() -> Self {
        Self {
            normal: CColVector::new(0.0, 0.0, 1.0),
            dist: 0.0,
            dir: Direction::DIR_Z_POS,
        }
    }
}

impl CColTrianglePlane {
    pub fn Set(&mut self, va: CColVector, vb: CColVector, vc: CColVector) {
        let ab = sub(vb, va);
        let ac = sub(vc, va);
        let mut normal = cross(ab, ac);
        normalise(&mut normal);
        self.dist = dot(normal, va);
        self.normal = normal;
        self.dir = dominant_direction(normal);
    }

    pub fn SetFromTriangle(&mut self, vertices: &[CompressedVector], tri: CColTriangle) {
        let Some(va) = vertices.get(tri.a as usize).map(CompressedVector::Get) else {
            return;
        };
        let Some(vb) = vertices.get(tri.b as usize).map(CompressedVector::Get) else {
            return;
        };
        let Some(vc) = vertices.get(tri.c as usize).map(CompressedVector::Get) else {
            return;
        };
        self.Set(va, vb, vc);
    }

    pub fn GetNormal(&self) -> CColVector {
        self.normal
    }

    pub fn CalcPoint(&self, v: CColVector) -> f32 {
        dot(self.normal, v) - self.dist
    }
}

fn sub(a: CColVector, b: CColVector) -> CColVector {
    CColVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn cross(a: CColVector, b: CColVector) -> CColVector {
    CColVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn dot(a: CColVector, b: CColVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn normalise(v: &mut CColVector) {
    let mag = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if mag != 0.0 {
        v.x /= mag;
        v.y /= mag;
        v.z /= mag;
    }
}

fn dominant_direction(normal: CColVector) -> Direction {
    let ax = normal.x.abs();
    let ay = normal.y.abs();
    let az = normal.z.abs();
    if ax > ay && ax > az {
        if normal.x > 0.0 {
            Direction::DIR_X_POS
        } else {
            Direction::DIR_X_NEG
        }
    } else if ay > az {
        if normal.y > 0.0 {
            Direction::DIR_Y_POS
        } else {
            Direction::DIR_Y_NEG
        }
    } else if normal.z > 0.0 {
        Direction::DIR_Z_POS
    } else {
        Direction::DIR_Z_NEG
    }
}
