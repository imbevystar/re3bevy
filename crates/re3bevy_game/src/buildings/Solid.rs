//! re3 reference: references/re-GTA/src/buildings/Solid.h
//!
//! Solid building entity state.

use re3bevy_core::core::General::CGeneral;

use crate::entities::Entity::{CEntity, eEntityType};

#[derive(Debug, Clone, PartialEq)]
pub struct CSolid {
    pub entity: CEntity,
}

impl CSolid {
    pub fn new(general: &mut CGeneral) -> Self {
        let mut entity = CEntity::new(general);
        entity.set_type(eEntityType::ENTITY_TYPE_BUILDING);
        entity.flags.uses_collision = true;
        Self { entity }
    }
}
