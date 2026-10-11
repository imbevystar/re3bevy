//! re3 reference: references/re-GTA/src/core/Crime.h
//!
//! Crime report state.

use bevy::prelude::Resource;
use re3bevy_core::core::Coordinates::CVector;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum eCrimeType {
    CRIME_NONE = 0,
    CRIME_POSSESSION_GUN = 1,
    CRIME_HIT_PED = 2,
    CRIME_HIT_COP = 3,
    CRIME_SHOOT_PED = 4,
    CRIME_SHOOT_COP = 5,
    CRIME_STEAL_CAR = 6,
    CRIME_RUN_REDLIGHT = 7,
    CRIME_RECKLESS_DRIVING = 8,
    CRIME_SPEEDING = 9,
    CRIME_RUNOVER_PED = 10,
    CRIME_RUNOVER_COP = 11,
    CRIME_SHOOT_HELI = 12,
    CRIME_PED_BURNED = 13,
    CRIME_COP_BURNED = 14,
    CRIME_VEHICLE_BURNED = 15,
    CRIME_DESTROYED_CESSNA = 16,
}

pub const NUM_CRIME_TYPES: usize = 17;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CCrimeBeingQd {
    pub crime_type: eCrimeType,
    pub id: i32,
    pub time_ms: u32,
    pub position: CVector,
    pub reported: bool,
    pub police_doesnt_care: bool,
}

impl Default for CCrimeBeingQd {
    fn default() -> Self {
        Self {
            crime_type: eCrimeType::CRIME_NONE,
            id: 0,
            time_ms: 0,
            position: CVector::ZERO,
            reported: false,
            police_doesnt_care: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Resource)]
pub struct CCrimeReportQueue {
    pub crimes: Vec<CCrimeBeingQd>,
}
