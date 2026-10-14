//! re3 reference: references/re-GTA/src/control/AutoPilot.cpp
//! re3 reference: references/re-GTA/src/control/AutoPilot.h
//!
//! Autopilot state used by vehicles.

use rust_gta_3_core::core::Coordinates::CVector;

use crate::control::Curves::CCurves;

pub const NUM_PATH_NODES_IN_AUTOPILOT: usize = 8;
pub const LANE_WIDTH: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarPathLinkSnapshot {
    pub x: f32,
    pub y: f32,
    pub dir_x: f32,
    pub dir_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eCarMission {
    MISSION_NONE = 0,
    MISSION_CRUISE = 1,
    MISSION_RAMPLAYER_FARAWAY = 2,
    MISSION_RAMPLAYER_CLOSE = 3,
    MISSION_BLOCKPLAYER_FARAWAY = 4,
    MISSION_BLOCKPLAYER_CLOSE = 5,
    MISSION_BLOCKPLAYER_HANDBRAKESTOP = 6,
    MISSION_WAITFORDELETION = 7,
    MISSION_GOTOCOORDS = 8,
    MISSION_GOTOCOORDS_STRAIGHT = 9,
    MISSION_EMERGENCYVEHICLE_STOP = 10,
    MISSION_STOP_FOREVER = 11,
    MISSION_GOTOCOORDS_ACCURATE = 12,
    MISSION_GOTO_COORDS_STRAIGHT_ACCURATE = 13,
    MISSION_GOTOCOORDS_ASTHECROWSWIMS = 14,
    MISSION_RAMCAR_FARAWAY = 15,
    MISSION_RAMCAR_CLOSE = 16,
    MISSION_BLOCKCAR_FARAWAY = 17,
    MISSION_BLOCKCAR_CLOSE = 18,
    MISSION_BLOCKCAR_HANDBRAKESTOP = 19,
}

pub(crate) fn script_car_mission(value: i32) -> eCarMission {
    match value {
        1 => eCarMission::MISSION_CRUISE,
        2 => eCarMission::MISSION_RAMPLAYER_FARAWAY,
        3 => eCarMission::MISSION_RAMPLAYER_CLOSE,
        4 => eCarMission::MISSION_BLOCKPLAYER_FARAWAY,
        5 => eCarMission::MISSION_BLOCKPLAYER_CLOSE,
        6 => eCarMission::MISSION_BLOCKPLAYER_HANDBRAKESTOP,
        7 => eCarMission::MISSION_WAITFORDELETION,
        8 => eCarMission::MISSION_GOTOCOORDS,
        9 => eCarMission::MISSION_GOTOCOORDS_STRAIGHT,
        10 => eCarMission::MISSION_EMERGENCYVEHICLE_STOP,
        11 => eCarMission::MISSION_STOP_FOREVER,
        12 => eCarMission::MISSION_GOTOCOORDS_ACCURATE,
        13 => eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE,
        14 => eCarMission::MISSION_GOTOCOORDS_ASTHECROWSWIMS,
        15 => eCarMission::MISSION_RAMCAR_FARAWAY,
        16 => eCarMission::MISSION_RAMCAR_CLOSE,
        17 => eCarMission::MISSION_BLOCKCAR_FARAWAY,
        18 => eCarMission::MISSION_BLOCKCAR_CLOSE,
        19 => eCarMission::MISSION_BLOCKCAR_HANDBRAKESTOP,
        _ => eCarMission::MISSION_NONE,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eCarTempAction {
    TEMPACT_NONE = 0,
    TEMPACT_WAIT = 1,
    TEMPACT_REVERSE = 2,
    TEMPACT_HANDBRAKETURNLEFT = 3,
    TEMPACT_HANDBRAKETURNRIGHT = 4,
    TEMPACT_HANDBRAKESTRAIGHT = 5,
    TEMPACT_TURNLEFT = 6,
    TEMPACT_TURNRIGHT = 7,
    TEMPACT_GOFORWARD = 8,
    TEMPACT_SWERVELEFT = 9,
    TEMPACT_SWERVERIGHT = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eCarDrivingStyle {
    DRIVINGSTYLE_STOP_FOR_CARS = 0,
    DRIVINGSTYLE_SLOW_DOWN_FOR_CARS = 1,
    DRIVINGSTYLE_AVOID_CARS = 2,
    DRIVINGSTYLE_PLOUGH_THROUGH = 3,
    DRIVINGSTYLE_STOP_FOR_CARS_IGNORE_LIGHTS = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CAutoPilot {
    pub current_route_node: i32,
    pub next_route_node: i32,
    pub prev_route_node: i32,
    pub time_entered_curve: i32,
    pub time_to_spend_on_current_curve: i32,
    pub current_path_node_info: u32,
    pub next_path_node_info: u32,
    pub previous_path_node_info: u32,
    pub anti_reverse_timer: u32,
    pub time_to_start_mission: u32,
    pub previous_direction: i8,
    pub current_direction: i8,
    pub next_direction: i8,
    pub current_lane: i8,
    pub next_lane: i8,
    pub driving_style: eCarDrivingStyle,
    pub car_mission: eCarMission,
    pub temp_action: eCarTempAction,
    pub time_temp_action: u32,
    pub max_traffic_speed: f32,
    pub cruise_speed: u8,
    pub slowed_down_because_of_cars: bool,
    pub slowed_down_because_of_peds: bool,
    pub stay_in_current_level: bool,
    pub stay_in_fast_lane: bool,
    pub ignore_pathfinding: bool,
    pub destination_coors: CVector,
    pub path_find_nodes: [Option<usize>; NUM_PATH_NODES_IN_AUTOPILOT],
    pub path_find_nodes_count: i16,
    pub target_car: Option<usize>,
}

impl CAutoPilot {
    /// CCarCtrl::SwitchVehicleToRealPhysics; status is set by the caller.
    pub(crate) fn switch_to_real_physics(&mut self, now_ms: u32) {
        self.car_mission = eCarMission::MISSION_CRUISE;
        self.temp_action = eCarTempAction::TEMPACT_NONE;
        self.anti_reverse_timer = now_ms;
        self.time_temp_action = now_ms;
    }

    pub fn new(time_ms: u32) -> Self {
        Self {
            current_route_node: 0,
            next_route_node: 0,
            prev_route_node: 0,
            time_entered_curve: 0,
            time_to_spend_on_current_curve: 1000,
            current_path_node_info: 0,
            next_path_node_info: 0,
            previous_path_node_info: 0,
            anti_reverse_timer: time_ms,
            time_to_start_mission: time_ms,
            previous_direction: 0,
            current_direction: 1,
            next_direction: 1,
            current_lane: 0,
            next_lane: 0,
            driving_style: eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS,
            car_mission: eCarMission::MISSION_NONE,
            temp_action: eCarTempAction::TEMPACT_NONE,
            time_temp_action: 0,
            max_traffic_speed: 10.0,
            cruise_speed: 10,
            slowed_down_because_of_cars: false,
            slowed_down_because_of_peds: false,
            stay_in_current_level: false,
            stay_in_fast_lane: false,
            ignore_pathfinding: false,
            destination_coors: CVector::ZERO,
            path_find_nodes: [None; NUM_PATH_NODES_IN_AUTOPILOT],
            path_find_nodes_count: 0,
            target_car: None,
        }
    }

    pub fn ModifySpeed(
        &mut self,
        speed: f32,
        current_time_ms: u32,
        current_link: CarPathLinkSnapshot,
        next_link: CarPathLinkSnapshot,
    ) {
        self.max_traffic_speed = speed.max(0.01);
        let position_between_nodes = (current_time_ms.wrapping_sub(self.time_entered_curve as u32))
            as f32
            / self.time_to_spend_on_current_curve as f32;
        let current_forward_x = self.current_direction as f32 * current_link.dir_x;
        let current_forward_y = self.current_direction as f32 * current_link.dir_y;
        let next_forward_x = self.next_direction as f32 * next_link.dir_x;
        let next_forward_y = self.next_direction as f32 * next_link.dir_y;
        let current_position = CVector::new(
            current_link.x + (self.current_lane as f32 + 0.5) * LANE_WIDTH * current_forward_y,
            current_link.y - (self.current_lane as f32 + 0.5) * LANE_WIDTH * current_forward_x,
            0.0,
        );
        let next_position = CVector::new(
            next_link.x + (self.next_lane as f32 + 0.5) * LANE_WIDTH * next_forward_y,
            next_link.y - (self.next_lane as f32 + 0.5) * LANE_WIDTH * next_forward_x,
            0.0,
        );
        self.time_to_spend_on_current_curve = (CCurves::CalcSpeedScaleFactor(
            current_position,
            next_position,
            current_forward_x,
            current_forward_y,
            next_forward_x,
            next_forward_y,
        ) * (1000.0 / self.max_traffic_speed)) as i32;
        // FIX_BUGS path preserves progress without converting the absolute
        // timer to float.
        self.time_entered_curve = current_time_ms.wrapping_sub(
            (position_between_nodes * self.time_to_spend_on_current_curve as f32) as u32,
        ) as i32;
    }

    pub fn RemoveOnePathNode(&mut self) {
        self.path_find_nodes_count -= 1;
        if self.path_find_nodes_count > 0 {
            let count = self.path_find_nodes_count as usize;
            for index in 0..count.min(NUM_PATH_NODES_IN_AUTOPILOT - 1) {
                self.path_find_nodes[index] = self.path_find_nodes[index + 1];
            }
        }
        // The original leaves the now-unused tail pointer untouched.
    }
}
