//! re3 reference: references/re-GTA/src/fakerw/rphanim.h
//!
//! HAnim hierarchy metadata facade.

use super::rwplcore::RwMatrix;

#[derive(Debug, Clone, PartialEq)]
pub struct RpHAnimHierarchy {
    pub flags: u32,
    pub current_time: f32,
    pub matrices: Vec<RwMatrix>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RpHAnimAnimation {
    pub type_id: i32,
    pub num_frames: i32,
    pub flags: i32,
    pub duration: f32,
}

pub fn RpHAnimPluginAttach() -> bool {
    true
}

pub fn RpHAnimHierarchySetFlags(hierarchy: &mut RpHAnimHierarchy, flags: u32) {
    hierarchy.flags = flags;
}

pub fn RpHAnimHierarchyAddAnimTime(hierarchy: &mut RpHAnimHierarchy, time: f32) -> bool {
    hierarchy.current_time += time;
    true
}

pub fn RpHAnimHierarchyGetMatrixArray(hierarchy: &mut RpHAnimHierarchy) -> &mut [RwMatrix] {
    &mut hierarchy.matrices
}

pub fn RpHAnimHierarchyUpdateMatrices(_hierarchy: &mut RpHAnimHierarchy) -> bool {
    true
}

pub fn RpHAnimAnimationCreate(
    type_id: i32,
    num_frames: i32,
    flags: i32,
    duration: f32,
) -> RpHAnimAnimation {
    RpHAnimAnimation {
        type_id,
        num_frames,
        flags,
        duration,
    }
}
