//! re3 reference: references/re-GTA/src/buildings/Building.cpp
//! re3 reference: references/re-GTA/src/buildings/Building.h
//!
//! Building entity state.

use rust_gta_3_core::core::General::CGeneral;

use crate::entities::Entity::{CEntity, eEntityType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CBuildingReplaceResult {
    pub old_model_index: i16,
    pub new_model_index: i16,
    pub deleted_rw_object: bool,
    pub should_remove_old_model: bool,
    pub should_request_new_model_dont_remove: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CBuilding {
    pub entity: CEntity,
}

impl CBuilding {
    pub fn new(general: &mut CGeneral) -> Self {
        let mut entity = CEntity::new(general);
        entity.set_type(eEntityType::ENTITY_TYPE_BUILDING);
        entity.flags.uses_collision = true;
        Self { entity }
    }

    pub fn replace_with_new_model(
        &mut self,
        id: i32,
        old_model_has_no_refs: bool,
        current_level: u16,
    ) -> CBuildingReplaceResult {
        let old_model_index = self.entity.model_index;
        let deleted_rw_object = self.entity.rw_object_loaded;
        self.entity.delete_rw_object_marker();
        self.entity.model_index = id as i16;

        let should_request_new_model_dont_remove = self.entity.flags.is_big_building
            && (self.entity.level == 0 || self.entity.level == current_level);

        CBuildingReplaceResult {
            old_model_index,
            new_model_index: self.entity.model_index,
            deleted_rw_object,
            should_remove_old_model: old_model_has_no_refs,
            should_request_new_model_dont_remove,
        }
    }

    pub const fn get_is_a_treadable(&self) -> bool {
        false
    }
}
