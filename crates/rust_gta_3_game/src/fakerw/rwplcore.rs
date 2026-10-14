//! re3 reference: references/re-GTA/src/fakerw/rwplcore.h
//!
//! Minimal RenderWare platform-core value types used by the Rust port.

pub type RwBool = bool;
pub type RwReal = f32;
pub type RwInt8 = i8;
pub type RwUInt8 = u8;
pub type RwInt16 = i16;
pub type RwUInt16 = u16;
pub type RwInt32 = i32;
pub type RwUInt32 = u32;
pub type RwChar = u8;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RwV2d {
    pub x: RwReal,
    pub y: RwReal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RwV3d {
    pub x: RwReal,
    pub y: RwReal,
    pub z: RwReal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RwRGBA {
    pub red: RwUInt8,
    pub green: RwUInt8,
    pub blue: RwUInt8,
    pub alpha: RwUInt8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RwSphere {
    pub center: RwV3d,
    pub radius: RwReal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RwBBox {
    pub sup: RwV3d,
    pub inf: RwV3d,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RwRect {
    pub x: RwInt32,
    pub y: RwInt32,
    pub w: RwInt32,
    pub h: RwInt32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwMatrix {
    pub right: RwV3d,
    pub up: RwV3d,
    pub at: RwV3d,
    pub pos: RwV3d,
}

impl Default for RwMatrix {
    fn default() -> Self {
        Self {
            right: RwV3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            up: RwV3d {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            at: RwV3d {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            pos: RwV3d::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RwPrimitiveType {
    rwPRIMTYPELINELIST = 2,
    rwPRIMTYPEPOLYLINE = 3,
    rwPRIMTYPETRILIST = 4,
    rwPRIMTYPETRISTRIP = 5,
    rwPRIMTYPETRIFAN = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RwOpCombineType {
    rwCOMBINEREPLACE = 0,
    rwCOMBINEPRECONCAT = 1,
    rwCOMBINEPOSTCONCAT = 2,
}

pub fn RwV3dAdd(out: &mut RwV3d, a: &RwV3d, b: &RwV3d) {
    *out = RwV3d {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    };
}

pub fn RwV3dSub(out: &mut RwV3d, a: &RwV3d, b: &RwV3d) {
    *out = RwV3d {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    };
}

pub fn RwV3dScale(out: &mut RwV3d, input: &RwV3d, scalar: RwReal) {
    *out = RwV3d {
        x: input.x * scalar,
        y: input.y * scalar,
        z: input.z * scalar,
    };
}

pub fn RwV3dDotProduct(a: &RwV3d, b: &RwV3d) -> RwReal {
    a.x * b.x + a.y * b.y + a.z * b.z
}

pub fn RwV3dLength(input: &RwV3d) -> RwReal {
    RwV3dDotProduct(input, input).sqrt()
}
