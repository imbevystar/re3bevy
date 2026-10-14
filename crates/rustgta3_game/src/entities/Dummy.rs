//! re3 reference: references/re-GTA/src/entities/Dummy.cpp
//! re3 reference: references/re-GTA/src/entities/Dummy.h
//!
//! Dummy entity state.

use rustgta3_core::core::General::CGeneral;

use crate::entities::Entity::{CEntity, eEntityType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CEntryInfoHandle {
    pub sector: usize,
    pub list: usize,
    pub node: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CDummy {
    pub entity: CEntity,
    pub entry_info_list: Vec<CEntryInfoHandle>,
}

impl CDummy {
    pub fn new(general: &mut CGeneral) -> Self {
        let mut entity = CEntity::new(general);
        entity.set_type(eEntityType::ENTITY_TYPE_DUMMY);
        Self {
            entity,
            entry_info_list: Vec::new(),
        }
    }

    pub fn add_entry_info(&mut self, sector: usize, list: usize, node: usize) {
        self.entry_info_list
            .push(CEntryInfoHandle { sector, list, node });
    }

    pub fn remove_entry_info(&mut self) {
        self.entry_info_list.clear();
    }
}
