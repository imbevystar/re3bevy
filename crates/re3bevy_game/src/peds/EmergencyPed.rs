//! re3 reference: references/re-GTA/src/peds/EmergencyPed.cpp
//! re3 reference: references/re-GTA/src/peds/EmergencyPed.h
//!
//! Emergency ped subtype state used by medic and fireman AI.

use bevy::prelude::Component;
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};

use crate::{
    core::Accident::CAccidentManager,
    modelinfo::ModelIndices::{MI_FIREMAN, MI_MEDIC},
    peds::{
        Ped::{CPed, PedState, eMoveState},
        PedType::ePedType,
    },
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum EmergencyPedState {
    EMERGENCY_PED_READY = 0x000,
    EMERGENCY_PED_DETERMINE_NEXT_STATE = 0x001,
    EMERGENCY_PED_START_CPR = 0x002,
    EMERGENCY_PED_FLAG_4 = 0x004,
    EMERGENCY_PED_FLAG_8 = 0x008,
    EMERGENCY_PED_FACE_TO_PATIENT = 0x010,
    EMERGENCY_PED_PERFORM_CPR = 0x020,
    EMERGENCY_PED_STOP_CPR = 0x040,
    EMERGENCY_PED_STAND_STILL = 0x080,
    EMERGENCY_PED_STOP = 0x100,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct CEmergencyPed {
    pub ped: CPed,
    pub revived_ped: Option<usize>,
    pub emergency_ped_state: EmergencyPedState,
    pub attended_accident: Option<usize>,
    pub attended_fire: Option<usize>,
    pub started_to_cpr: bool,
    pub field_1360: i32,
}

impl CEmergencyPed {
    pub fn new(ped_type: ePedType, general: &mut CGeneral) -> Self {
        let mut ped = CPed::new(ped_type, general);
        match ped_type {
            ePedType::PEDTYPE_EMERGENCY => {
                ped.physical.entity.set_model_index_no_create(MI_MEDIC);
            }
            ePedType::PEDTYPE_FIREMAN => {
                ped.physical.entity.set_model_index_no_create(MI_FIREMAN);
            }
            _ => {}
        }
        Self {
            ped,
            revived_ped: None,
            emergency_ped_state: EmergencyPedState::EMERGENCY_PED_READY,
            attended_accident: None,
            attended_fire: None,
            started_to_cpr: false,
            field_1360: 0,
        }
    }

    pub fn in_range(&self, victim_position: CVector, vehicle_position: Option<CVector>) -> bool {
        let Some(vehicle_position) = vehicle_position else {
            return true;
        };
        let dx = vehicle_position.x - victim_position.x;
        let dy = vehicle_position.y - victim_position.y;
        let dz = vehicle_position.z - victim_position.z;
        dx * dx + dy * dy + dz * dz <= 30.0 * 30.0
    }

    pub fn begin_attending_accident(
        &mut self,
        accident_slot: usize,
        victim: usize,
        accidents: &mut CAccidentManager,
    ) -> bool {
        let Some(accident) = accidents.accidents.get_mut(accident_slot) else {
            return false;
        };
        if accident.victim.is_none() {
            return false;
        }
        self.revived_ped = Some(victim);
        self.attended_accident = Some(accident_slot);
        self.emergency_ped_state = EmergencyPedState::EMERGENCY_PED_DETERMINE_NEXT_STATE;
        self.ped.flags.is_running = true;
        self.field_1360 = 0;
        accident.medics_attending = accident.medics_attending.wrapping_add(1);
        true
    }

    pub fn start_cpr(&mut self, accidents: &mut CAccidentManager, now_ms: u32) {
        let Some(slot) = self.attended_accident else {
            self.emergency_ped_state = EmergencyPedState::EMERGENCY_PED_DETERMINE_NEXT_STATE;
            return;
        };
        let Some(accident) = accidents.accidents.get_mut(slot) else {
            self.emergency_ped_state = EmergencyPedState::EMERGENCY_PED_DETERMINE_NEXT_STATE;
            return;
        };

        self.ped.move_state = eMoveState::PEDMOVE_STILL;
        self.ped.ped_state = PedState::PED_CPR;
        self.ped.last_ped_state = PedState::PED_CPR;
        self.ped.look_timer = now_ms.wrapping_add(2_000);
        if accident.medics_performing_cpr != 0 {
            self.ped.ped_state = PedState::PED_IDLE;
            self.emergency_ped_state = EmergencyPedState::EMERGENCY_PED_STAND_STILL;
        } else {
            self.emergency_ped_state = EmergencyPedState::EMERGENCY_PED_FACE_TO_PATIENT;
            self.ped.flags.is_ducking = true;
        }
        accident.medics_performing_cpr = accident.medics_performing_cpr.wrapping_add(1);
        self.started_to_cpr = true;
    }

    pub fn stop_attending(&mut self, accidents: &mut CAccidentManager) {
        self.started_to_cpr = false;
        self.ped.ped_state = PedState::PED_NONE;
        if let Some(slot) = self.attended_accident.take() {
            if let Some(accident) = accidents.accidents.get_mut(slot) {
                accident.victim = None;
                accident.medics_attending = accident.medics_attending.wrapping_sub(1);
            }
        }
        self.revived_ped = None;
        self.emergency_ped_state = EmergencyPedState::EMERGENCY_PED_READY;
        self.ped.move_state = eMoveState::PEDMOVE_WALK;
    }
}
