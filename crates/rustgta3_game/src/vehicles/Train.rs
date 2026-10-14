//! re3 reference: references/re-GTA/src/vehicles/Train.cpp
//! re3 reference: references/re-GTA/src/vehicles/Train.h
//!
//! Train vehicle state.

use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};

use crate::{
    entities::Entity::eEntityStatus,
    modelinfo::VehicleModelInfo::eVehicleType,
    vehicles::{
        Door::{CTrainDoor, eDoorState},
        Vehicle::CVehicle,
    },
};

pub const TRACK_ELTRAIN: u8 = 0;
pub const TRACK_SUBWAY: u8 = 1;
pub const TRAIN_DOOR_CLOSED: i16 = 0;
pub const TRAIN_DOOR_OPENING: i16 = 1;
pub const TRAIN_DOOR_OPEN: i16 = 2;
pub const TRAIN_DOOR_CLOSING: i16 = 3;
pub const TRAIN_DOOR_LHS: usize = 1;
pub const TRAIN_DOOR_RHS: usize = 2;
pub const NUM_TRAIN_NODES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CTrainNode {
    pub p: CVector,
    pub t: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CTrainInterpolationLine {
    pub line_type: u8,
    pub time: f32,
    pub position: f32,
    pub speed: f32,
    pub acceleration: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CTrain {
    pub vehicle: CVehicle,
    pub wagon_position: f32,
    pub wagon_id: i16,
    pub is_far_away: i16,
    pub cur_track_node: i16,
    pub wagon_group: i16,
    pub speed: f32,
    pub process_door: bool,
    pub train_stopping: bool,
    pub is_first_wagon: bool,
    pub is_last_wagon: bool,
    pub track_id: u8,
    pub door_timer: u32,
    pub door_state: i16,
    pub doors: [CTrainDoor; 2],
    pub train_nodes_loaded: [bool; NUM_TRAIN_NODES],
}

impl CTrain {
    pub fn new(id: i32, created_by: u8, time_ms: u32, general: &mut CGeneral) -> Self {
        let mut vehicle = CVehicle::new(created_by, time_ms, general);
        vehicle.vehicle_type = eVehicleType::VEHICLE_TYPE_TRAIN;
        vehicle
            .physical
            .entity
            .set_model_index_no_create(id as usize);
        vehicle.physical.mass = 100000000.0;
        vehicle.physical.turn_mass = 100000000.0;
        vehicle.physical.air_resistance = 0.9994;
        vehicle.physical.elasticity = 0.05;
        vehicle.physical.entity.flags.uses_collision = true;
        vehicle.set_status(eEntityStatus::STATUS_TRAIN_MOVING);
        vehicle.entry.num_max_passengers = 5;

        let mut doors = [CTrainDoor::default(), CTrainDoor::default()];
        doors[0].init(0.8, 0.0, 1, 0);
        doors[1].init(-0.8, 0.0, 0, 0);
        doors[0].door_state = eDoorState::DOORST_CLOSED;
        doors[1].door_state = eDoorState::DOORST_CLOSED;

        Self {
            vehicle,
            wagon_position: 0.0,
            wagon_id: 0,
            is_far_away: 1,
            cur_track_node: 0,
            wagon_group: 0,
            speed: 0.0,
            process_door: true,
            train_stopping: false,
            is_first_wagon: false,
            is_last_wagon: false,
            track_id: TRACK_ELTRAIN,
            door_timer: time_ms,
            door_state: TRAIN_DOOR_CLOSED,
            doors,
            train_nodes_loaded: [false; NUM_TRAIN_NODES],
        }
    }
}
