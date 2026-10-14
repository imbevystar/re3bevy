//! re3 reference: references/re-GTA/src/objects/DummyObject.cpp
//! re3 reference: references/re-GTA/src/objects/DummyObject.h
//!
//! Dummy object state.

use rust_gta_3_core::core::General::CGeneral;

use crate::{entities::Dummy::CDummy, objects::Object::CObject};

#[derive(Debug, Clone, PartialEq)]
pub struct CDummyObject {
    pub dummy: CDummy,
}

impl CDummyObject {
    pub fn new(general: &mut CGeneral) -> Self {
        Self {
            dummy: CDummy::new(general),
        }
    }

    pub fn from_object(object: &mut CObject, general: &mut CGeneral) -> Self {
        let mut dummy = CDummy::new(general);
        dummy.entity.model_index = object.physical.entity.model_index;
        if object.physical.entity.rw_object_loaded {
            dummy.entity.rw_object_loaded = true;
        }
        object.physical.entity.delete_rw_object_marker();
        dummy.entity.level = object.physical.entity.level;
        Self { dummy }
    }
}
