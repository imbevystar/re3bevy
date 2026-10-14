//! re3 reference: references/re-GTA/src/vehicles/Bike.h
//!
//! Miami bike leftover state.

use rust_gta_3_core::core::General::CGeneral;

use crate::vehicles::Vehicle::CVehicle;

pub const BIKE_NUM_NODES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum eBikeNodes {
    BIKE_NODE_NONE = 0,
    BIKE_CHASSIS = 1,
    BIKE_FORKS_FRONT = 2,
    BIKE_FORKS_REAR = 3,
    BIKE_WHEEL_FRONT = 4,
    BIKE_WHEEL_REAR = 5,
    BIKE_MUDGUARD = 6,
    BIKE_HANDLEBARS = 7,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CBike {
    pub vehicle: CVehicle,
    pub bike_nodes_loaded: [bool; BIKE_NUM_NODES],
    pub bike_sit_animation: i32,
    pub suspension_spring_ratio: [f32; 4],
    pub suspension_spring_ratio_prev: [f32; 4],
    pub wheel_timer: [f32; 4],
    pub wheel_skidmark_type: [i32; 2],
    pub wheel_skidmark_bloody: [bool; 2],
    pub wheel_skidmark_unk: [bool; 2],
    pub wheel_rotation: [f32; 2],
    pub wheel_speed: [f32; 2],
    pub wheel_position: [f32; 2],
    pub wheel_base_position: [f32; 2],
    pub suspension_spring_length: [f32; 4],
    pub suspension_line_length: [f32; 4],
    pub height_above_road: f32,
    pub traction: f32,
}

impl CBike {
    pub fn new(created_by: u8, time_ms: u32, general: &mut CGeneral) -> Self {
        Self {
            vehicle: CVehicle::new(created_by, time_ms, general),
            bike_nodes_loaded: [false; BIKE_NUM_NODES],
            bike_sit_animation: 0,
            suspension_spring_ratio: [0.0; 4],
            suspension_spring_ratio_prev: [0.0; 4],
            wheel_timer: [0.0; 4],
            wheel_skidmark_type: [0; 2],
            wheel_skidmark_bloody: [false; 2],
            wheel_skidmark_unk: [false; 2],
            wheel_rotation: [0.0; 2],
            wheel_speed: [0.0; 2],
            wheel_position: [0.0; 2],
            wheel_base_position: [0.0; 2],
            suspension_spring_length: [0.0; 4],
            suspension_line_length: [0.0; 4],
            height_above_road: 0.0,
            traction: 0.0,
        }
    }
}
