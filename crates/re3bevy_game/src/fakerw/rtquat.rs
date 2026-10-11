//! re3 reference: references/re-GTA/src/fakerw/rtquat.h
//!
//! RenderWare toolkit quaternion facade.

use super::rwplcore::{RwMatrix, RwOpCombineType, RwReal, RwV3d};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RtQuat {
    pub imag: RwV3d,
    pub real: RwReal,
}

pub fn RtQuatRotate(
    quat: &mut RtQuat,
    axis: &RwV3d,
    angle_degrees: RwReal,
    combine: RwOpCombineType,
) {
    let angle = angle_degrees.to_radians() * 0.5;
    let s = angle.sin();
    let rot = RtQuat {
        imag: RwV3d {
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
        },
        real: angle.cos(),
    };
    *quat = match combine {
        RwOpCombineType::rwCOMBINEREPLACE => rot,
        RwOpCombineType::rwCOMBINEPRECONCAT => multiply(rot, *quat),
        RwOpCombineType::rwCOMBINEPOSTCONCAT => multiply(*quat, rot),
    };
}

pub fn RtQuatConvertToMatrix(quat: &RtQuat, matrix: &mut RwMatrix) {
    let x = quat.imag.x;
    let y = quat.imag.y;
    let z = quat.imag.z;
    let w = quat.real;
    matrix.right = RwV3d {
        x: 1.0 - 2.0 * (y * y + z * z),
        y: 2.0 * (x * y + z * w),
        z: 2.0 * (x * z - y * w),
    };
    matrix.up = RwV3d {
        x: 2.0 * (x * y - z * w),
        y: 1.0 - 2.0 * (x * x + z * z),
        z: 2.0 * (y * z + x * w),
    };
    matrix.at = RwV3d {
        x: 2.0 * (x * z + y * w),
        y: 2.0 * (y * z - x * w),
        z: 1.0 - 2.0 * (x * x + y * y),
    };
}

fn multiply(a: RtQuat, b: RtQuat) -> RtQuat {
    RtQuat {
        real: a.real * b.real - a.imag.x * b.imag.x - a.imag.y * b.imag.y - a.imag.z * b.imag.z,
        imag: RwV3d {
            x: a.real * b.imag.x + a.imag.x * b.real + a.imag.y * b.imag.z - a.imag.z * b.imag.y,
            y: a.real * b.imag.y - a.imag.x * b.imag.z + a.imag.y * b.real + a.imag.z * b.imag.x,
            z: a.real * b.imag.z + a.imag.x * b.imag.y - a.imag.y * b.imag.x + a.imag.z * b.real,
        },
    }
}
