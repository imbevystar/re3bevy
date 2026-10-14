//! re3 reference: references/re-GTA/src/buildings/Treadable.cpp
//! re3 reference: references/re-GTA/src/buildings/Treadable.h
//!
//! Treadable building state.

use rust_gta_3_core::core::General::CGeneral;

use crate::buildings::Building::CBuilding;

pub const TREADABLE_PATH_TYPES: usize = 2;
pub const TREADABLE_NODE_SLOTS: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct CTreadable {
    pub building: CBuilding,
    pub node_indices: [[i16; TREADABLE_NODE_SLOTS]; TREADABLE_PATH_TYPES],
}

impl CTreadable {
    pub fn new(general: &mut CGeneral) -> Self {
        Self {
            building: CBuilding::new(general),
            node_indices: [[0; TREADABLE_NODE_SLOTS]; TREADABLE_PATH_TYPES],
        }
    }

    pub const fn get_is_a_treadable(&self) -> bool {
        true
    }
}
