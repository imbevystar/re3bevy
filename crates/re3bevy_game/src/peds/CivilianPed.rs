//! re3 reference: references/re-GTA/src/peds/CivilianPed.cpp
//! re3 reference: references/re-GTA/src/peds/CivilianPed.h
//!
//! Civilian ped subtype. re3 adds no fields beyond `CPed`; behaviour is in
//! `CivilianAI`/`ProcessControl`, not in an invented civilian state payload.

use bevy::prelude::Component;
use re3bevy_core::core::General::CGeneral;

use super::{Ped::CPed, PedType::ePedType};

#[derive(Debug, Clone, PartialEq, Component)]
pub struct CCivilianPed {
    pub ped: CPed,
}

impl CCivilianPed {
    pub fn new(ped_type: ePedType, model_index: usize, general: &mut CGeneral) -> Self {
        let mut ped = CPed::new(ped_type, general);
        ped.physical.entity.set_model_index_no_create(model_index);
        ped.near_peds.fill(None);
        Self { ped }
    }
}
