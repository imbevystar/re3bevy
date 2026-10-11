//! re3 reference: references/re-GTA/src/fakerw/rpworld.h
//!
//! RenderWare world/clump/atomic facade.

use super::rwcore::{RwFrame, RwRaster};
use super::rwplcore::{RwBBox, RwSphere};

#[derive(Debug, Clone, PartialEq)]
pub struct RpWorld {
    pub bounding_box: Option<RwBBox>,
    pub atomics: Vec<RpAtomic>,
    pub clumps: Vec<RpClump>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpClump {
    pub frame: Option<RwFrame>,
    pub atomics: Vec<RpAtomic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpAtomic {
    pub frame: Option<RwFrame>,
    pub geometry: Option<RpGeometry>,
    pub flags: u32,
    pub bounding_sphere: RwSphere,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpGeometry {
    pub num_vertices: usize,
    pub num_triangles: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpMaterial {
    pub texture: Option<RwRaster>,
}

pub fn RpWorldCreate(bounding_box: Option<RwBBox>) -> RpWorld {
    RpWorld {
        bounding_box,
        atomics: Vec::new(),
        clumps: Vec::new(),
    }
}

pub fn RpWorldPluginAttach() -> bool {
    true
}

pub fn RpClumpCreate() -> RpClump {
    RpClump {
        frame: None,
        atomics: Vec::new(),
    }
}

pub fn RpClumpGetNumAtomics(clump: &RpClump) -> i32 {
    clump.atomics.len() as i32
}

pub fn RpAtomicCreate() -> RpAtomic {
    RpAtomic {
        frame: None,
        geometry: None,
        flags: 0,
        bounding_sphere: RwSphere::default(),
    }
}

pub fn RpAtomicSetFlags(atomic: &mut RpAtomic, flags: u32) {
    atomic.flags = flags;
}

pub fn RpAtomicGetFlags(atomic: &RpAtomic) -> u32 {
    atomic.flags
}
