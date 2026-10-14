//! re3 reference: references/re-GTA/src/peds/DummyPed.h
//!
//! Unused re3 `CDummyPed` layout. It derives from `CDummy`, not `CPed`.

use bevy::prelude::Component;
use rustgta3_core::core::General::CGeneral;

use crate::entities::Dummy::CDummy;

#[derive(Debug, Clone, PartialEq, Component)]
pub struct CDummyPed {
    pub dummy: CDummy,
    pub ped_type: i32,
    pub unknown: i32,
}

impl CDummyPed {
    pub fn new(ped_type: i32, unknown: i32, general: &mut CGeneral) -> Self {
        Self {
            dummy: CDummy::new(general),
            ped_type,
            unknown,
        }
    }
}
