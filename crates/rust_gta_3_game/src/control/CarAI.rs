//! re3 reference: references/re-GTA/src/control/CarAI.cpp
//! re3 reference: references/re-GTA/src/control/CarAI.h
//!
//! Vehicle AI command state and deterministic chase helpers.

use bevy::prelude::{Entity, Resource};
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};

use crate::{
    control::AutoPilot::{CAutoPilot, eCarDrivingStyle, eCarMission, eCarTempAction},
    entities::Entity::eEntityStatus,
    vehicles::Vehicle::{MISSION_VEHICLE, RANDOM_VEHICLE},
};

pub const GAME_SPEED_TO_CARAI_SPEED: f32 = 60.0;
pub const DISTANCE_TO_SWITCH_DISTANCE_GOTO: f32 = 20.0;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CCarAIState {
    pub vehicle: Entity,
    pub mission: eCarMission,
    pub target: Option<Entity>,
    pub cruise_speed: i32,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Resource)]
pub struct CCarAI {
    pub cars: Vec<CCarAIState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarCommandResult {
    pub reference_target: Entity,
    pub mission: eCarMission,
    pub engine_on: bool,
    pub cruise_speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoToCoorsResult {
    pub distance: f32,
    pub status_physics: bool,
    pub path_join_attempted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarAiMissionContext {
    pub vehicle_position: CVector,
    pub vehicle_speed: CVector,
    pub is_law_enforcer: bool,
    pub is_ambulance_on_duty: bool,
    pub is_firetruck_on_duty: bool,
    pub target_car_position: Option<CVector>,
    pub target_car_collided: bool,
    pub frame_counter: u32,
    pub current_time_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CarAiMissionResult {
    pub join_road_system: bool,
    pub join_road_system_goto: bool,
    pub occupants_leave: bool,
    pub siren: Option<bool>,
    pub horn_timer: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SirenVehicleSnapshot {
    pub slot: usize,
    pub position: CVector,
    pub move_speed: CVector,
    pub forward: CVector,
    pub is_car_or_bike: bool,
    pub simple_or_physics: bool,
    pub random_vehicle: bool,
    pub emergency_vehicle: bool,
    pub temp_action: eCarTempAction,
    pub time_temp_action: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SirenMakeWayIntent {
    pub slot: usize,
    pub temp_action: eCarTempAction,
    pub time_temp_action: u32,
    pub set_status_physics: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarAiStepInput {
    pub status: eEntityStatus,
    pub position: CVector,
    pub move_speed: CVector,
    pub forward: CVector,
    pub up_z: f32,
    pub is_law_enforcer: bool,
    pub uses_siren: bool,
    pub siren_or_alarm: bool,
    pub is_rhino: bool,
    pub is_ambulance_on_duty: bool,
    pub is_fire_truck_on_duty: bool,
    pub created_by: u8,
    pub random_seed: u16,
    pub last_collision_ms: u32,
    pub time_blocked_ms: u32,
    pub now_ms: u32,
    pub previous_ms: u32,
    pub time_step_ms: f32,
    pub frame_counter: u32,
    pub player_position: CVector,
    pub player_vehicle_speed: Option<f32>,
    pub player_move_speed: CVector,
    pub player_forward: CVector,
    pub player_vehicle_upside_down: bool,
    pub player_vehicle_collided: bool,
    pub driver_kill_player_any_means: bool,
    pub player_ignored_by_everyone: bool,
    pub player_ignored_by_cops: bool,
    pub wanted_level: u8,
    pub no_police_zone: bool,
    pub bridge_requires_stop: bool,
    pub target_car_position: Option<CVector>,
    pub target_car_is_player_vehicle: bool,
    pub target_car_collided: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CarAiStepResult {
    pub time_blocked_ms: u32,
    pub join_road_system: bool,
    pub join_goto_target: bool,
    pub occupants_leave: bool,
    pub siren: Option<bool>,
    pub horn: bool,
    pub horn_timer: Option<u32>,
    /// re3 `UpdateCarAI` directly promotes SIMPLE law-enforcement cars to
    /// `STATUS_PHYSICS` when they begin a wanted-level pursuit (or stop to
    /// unload nearby cops). This is deliberately separate from
    /// `switch_to_real_physics`, which models `CCarCtrl::SwitchVehicleToRealPhysics`
    /// and resets the autopilot back to cruise.
    pub promote_to_physics: bool,
    /// UpdateCarAI already applied the autopilot hand-off before its escape
    /// action. The ECS caller changes status without repeating that reset.
    pub switch_to_real_physics: bool,
}

impl CCarAI {
    /// Entity-independent core of `CCarAI::UpdateCarAI`. Path joins and occupant
    /// objectives are returned to the ECS owner instead of dereferencing C++ pointers.
    pub fn UpdateCarAI(
        auto_pilot: &mut CAutoPilot,
        input: CarAiStepInput,
        transmission_max_velocity: f32,
        general: &mut CGeneral,
    ) -> CarAiStepResult {
        let mut out = CarAiStepResult {
            time_blocked_ms: input.time_blocked_ms,
            ..Default::default()
        };
        if input.is_law_enforcer
            && matches!(
                auto_pilot.car_mission,
                eCarMission::MISSION_BLOCKCAR_FARAWAY
                    | eCarMission::MISSION_RAMPLAYER_FARAWAY
                    | eCarMission::MISSION_BLOCKPLAYER_CLOSE
                    | eCarMission::MISSION_RAMPLAYER_CLOSE
            )
        {
            auto_pilot.cruise_speed = Self::FindPoliceCarSpeedForWantedLevel(
                input.wanted_level,
                transmission_max_velocity,
                general,
            ) as u8;
        }

        if matches!(
            input.status,
            eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED
        ) {
            auto_pilot.car_mission = eCarMission::MISSION_NONE;
            auto_pilot.cruise_speed = 0;
            return out;
        }
        if !matches!(
            input.status,
            eEntityStatus::STATUS_SIMPLE | eEntityStatus::STATUS_PHYSICS
        ) {
            return out;
        }

        let player_distance = distance_2d(input.position, input.player_position);
        let stop_chase = input.player_ignored_by_everyone
            || (input.is_law_enforcer
                && (input.wanted_level == 0
                    || input.player_ignored_by_cops
                    || input.no_police_zone));
        match auto_pilot.car_mission {
            eCarMission::MISSION_RAMPLAYER_FARAWAY | eCarMission::MISSION_BLOCKPLAYER_FARAWAY => {
                if player_distance < Self::FindSwitchDistanceClose(input.is_law_enforcer)
                    || auto_pilot.ignore_pathfinding
                {
                    auto_pilot.car_mission =
                        if auto_pilot.car_mission == eCarMission::MISSION_RAMPLAYER_FARAWAY {
                            eCarMission::MISSION_RAMPLAYER_CLOSE
                        } else {
                            eCarMission::MISSION_BLOCKPLAYER_CLOSE
                        };
                    if input.uses_siren {
                        out.siren = Some(true);
                    }
                }
                if stop_chase {
                    end_police_chase(auto_pilot, input.no_police_zone, &mut out);
                }
            }
            eCarMission::MISSION_RAMPLAYER_CLOSE | eCarMission::MISSION_BLOCKPLAYER_CLOSE => {
                if player_distance <= Self::FindSwitchDistanceFar(input.is_law_enforcer)
                    || auto_pilot.ignore_pathfinding
                {
                    let player_stopped =
                        input.player_vehicle_speed.is_some_and(|speed| speed < 0.05);
                    if auto_pilot.car_mission == eCarMission::MISSION_RAMPLAYER_CLOSE
                        && input.player_vehicle_collided
                        && !matches!(
                            auto_pilot.temp_action,
                            eCarTempAction::TEMPACT_TURNLEFT | eCarTempAction::TEMPACT_TURNRIGHT
                        )
                    {
                        auto_pilot.temp_action = eCarTempAction::TEMPACT_REVERSE;
                        auto_pilot.time_temp_action = input
                            .now_ms
                            .wrapping_add(if player_stopped { 800 } else { 50 });
                    }
                    if player_stopped {
                        out.time_blocked_ms = out
                            .time_blocked_ms
                            .saturating_add(input.time_step_ms.max(0.0) as u32);
                    } else {
                        out.time_blocked_ms = 0;
                    }
                    if input.player_vehicle_speed.is_none()
                        || input.player_vehicle_upside_down
                        || (player_stopped && out.time_blocked_ms > 2500)
                    {
                        if input.is_law_enforcer
                            && (!input.is_rhino || input.random_seed > 10_000)
                            && player_distance < 10.0
                        {
                            out.occupants_leave = true;
                            auto_pilot.cruise_speed = 0;
                            auto_pilot.car_mission = eCarMission::MISSION_NONE;
                            if input.wanted_level <= 1 {
                                out.siren = Some(false);
                            }
                        }
                    }
                } else {
                    out.join_goto_target = true;
                    auto_pilot.car_mission =
                        if auto_pilot.car_mission == eCarMission::MISSION_RAMPLAYER_CLOSE {
                            eCarMission::MISSION_RAMPLAYER_FARAWAY
                        } else {
                            eCarMission::MISSION_BLOCKPLAYER_FARAWAY
                        };
                    out.siren = Some(false);
                }
                if stop_chase {
                    end_police_chase(auto_pilot, input.no_police_zone, &mut out);
                } else if input.is_law_enforcer {
                    Self::MellowOutChaseSpeed(
                        input.wanted_level,
                        player_distance,
                        input.player_vehicle_speed.is_some(),
                        auto_pilot,
                    );
                }
            }
            eCarMission::MISSION_GOTOCOORDS | eCarMission::MISSION_GOTOCOORDS_ACCURATE => {
                if distance_2d(input.position, auto_pilot.destination_coors) < 20.0
                    || auto_pilot.ignore_pathfinding
                {
                    auto_pilot.car_mission =
                        if auto_pilot.car_mission == eCarMission::MISSION_GOTOCOORDS {
                            eCarMission::MISSION_GOTOCOORDS_STRAIGHT
                        } else {
                            eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE
                        };
                }
            }
            eCarMission::MISSION_GOTOCOORDS_STRAIGHT
            | eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE => {
                let accurate =
                    auto_pilot.car_mission == eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE;
                let distance = distance_2d(input.position, auto_pilot.destination_coors);
                if !accurate
                    && (input.is_ambulance_on_duty || input.is_fire_truck_on_duty)
                    && distance < 20.0
                {
                    auto_pilot.car_mission = eCarMission::MISSION_EMERGENCYVEHICLE_STOP;
                }
                if distance < if accurate { 1.0 } else { 5.0 } {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                    auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                } else if distance > Self::FindSwitchDistanceFarNormalVehicle(input.is_law_enforcer)
                    && !auto_pilot.ignore_pathfinding
                    && input.frame_counter & 7 == 0
                {
                    auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                    out.join_goto_target = true;
                    auto_pilot.car_mission = if accurate {
                        eCarMission::MISSION_GOTOCOORDS_ACCURATE
                    } else {
                        eCarMission::MISSION_GOTOCOORDS
                    };
                }
            }
            eCarMission::MISSION_RAMCAR_FARAWAY | eCarMission::MISSION_BLOCKCAR_FARAWAY => {
                let blocking = auto_pilot.car_mission == eCarMission::MISSION_BLOCKCAR_FARAWAY;
                if let Some(target) = input.target_car_position {
                    if distance_2d(input.position, target)
                        < Self::FindSwitchDistanceClose(input.is_law_enforcer)
                        || auto_pilot.ignore_pathfinding
                    {
                        auto_pilot.car_mission = if blocking {
                            eCarMission::MISSION_BLOCKCAR_CLOSE
                        } else {
                            eCarMission::MISSION_RAMCAR_CLOSE
                        };
                        if blocking && input.uses_siren {
                            out.siren = Some(true);
                        }
                    }
                } else {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                }
            }
            eCarMission::MISSION_RAMCAR_CLOSE | eCarMission::MISSION_BLOCKCAR_CLOSE => {
                let ramming = auto_pilot.car_mission == eCarMission::MISSION_RAMCAR_CLOSE;
                let blocking = !ramming;
                if let Some(target) = input.target_car_position {
                    // FIX_BUGS in re3 limits this wanted/ignore escape to the
                    // player vehicle.  Scripted RAMCAR targets must keep their
                    // mission even if the player happens to be ignored.
                    if ramming && input.target_car_is_player_vehicle && stop_chase {
                        end_police_chase(auto_pilot, input.no_police_zone, &mut out);
                    }
                    let distance = distance_2d(input.position, target);
                    if distance > Self::FindSwitchDistanceFar(input.is_law_enforcer)
                        && !auto_pilot.ignore_pathfinding
                    {
                        auto_pilot.car_mission = if blocking {
                            eCarMission::MISSION_BLOCKCAR_FARAWAY
                        } else {
                            eCarMission::MISSION_RAMCAR_FARAWAY
                        };
                        out.join_road_system = true;
                        if blocking {
                            out.siren = Some(false);
                            out.horn_timer = Some(0);
                        }
                    } else if input.target_car_collided
                        && ramming
                        && magnitude(input.move_speed) < 0.04
                    {
                        auto_pilot.temp_action = eCarTempAction::TEMPACT_REVERSE;
                        auto_pilot.time_temp_action = input.now_ms.saturating_add(800);
                    }
                } else {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                }
            }
            _ => {
                if input.is_law_enforcer && input.wanted_level > 0 && !input.no_police_zone {
                    if (input.player_position.x - input.position.x).abs() > 10.0
                        || (input.player_position.y - input.position.y).abs() > 10.0
                    {
                        out.promote_to_physics = true;
                        auto_pilot.cruise_speed = Self::FindPoliceCarSpeedForWantedLevel(
                            input.wanted_level,
                            transmission_max_velocity,
                            general,
                        ) as u8;
                        auto_pilot.car_mission =
                            Self::FindPoliceCarMissionForWantedLevel(input.wanted_level, general);
                        auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                        auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
                    } else if auto_pilot.car_mission == eCarMission::MISSION_CRUISE {
                        out.promote_to_physics = true;
                        out.occupants_leave = true;
                        auto_pilot.cruise_speed = 0;
                        auto_pilot.car_mission = eCarMission::MISSION_NONE;
                        if input.wanted_level <= 1 {
                            out.siren = Some(false);
                        }
                    }
                }
            }
        }

        let flat_speed_sqr =
            input.move_speed.x * input.move_speed.x + input.move_speed.y * input.move_speed.y;
        if flat_speed_sqr > 0.018 * 0.018 {
            auto_pilot.time_to_start_mission = input.now_ms;
            auto_pilot.anti_reverse_timer = input.now_ms;
        }
        if input.status == eEntityStatus::STATUS_PHYSICS
            && auto_pilot.temp_action == eCarTempAction::TEMPACT_NONE
            && auto_pilot.car_mission != eCarMission::MISSION_NONE
            && auto_pilot.car_mission != eCarMission::MISSION_STOP_FOREVER
            && auto_pilot.cruise_speed != 0
            && (input.created_by != RANDOM_VEHICLE
                || auto_pilot.car_mission != eCarMission::MISSION_CRUISE)
            && auto_pilot.driving_style != eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS
        {
            if input.now_ms.wrapping_sub(input.last_collision_ms) > 500 {
                auto_pilot.anti_reverse_timer = input.now_ms;
            }
            if flat_speed_sqr < 0.018 * 0.018
                && input.now_ms.wrapping_sub(auto_pilot.anti_reverse_timer) > 2000
            {
                auto_pilot.temp_action = eCarTempAction::TEMPACT_REVERSE;
                auto_pilot.time_temp_action = input.now_ms.saturating_add(
                    if (auto_pilot.car_mission != eCarMission::MISSION_NONE
                        && auto_pilot.car_mission != eCarMission::MISSION_CRUISE)
                        || input.created_by == MISSION_VEHICLE
                    {
                        1500
                    } else {
                        750
                    },
                );
                auto_pilot.anti_reverse_timer = input.now_ms;
                if input.created_by == RANDOM_VEHICLE
                    && (auto_pilot.driving_style as u8)
                        < eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS as u8
                {
                    auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
                }
                out.horn = true;
            }
        }
        if (input.random_seed & 7) == 0
            && input.now_ms.wrapping_sub(auto_pilot.time_to_start_mission) > 30_000
            && input
                .previous_ms
                .wrapping_sub(auto_pilot.time_to_start_mission)
                <= 30_000
            && auto_pilot.car_mission == eCarMission::MISSION_CRUISE
            && !input.bridge_requires_stop
        {
            auto_pilot.switch_to_real_physics(input.now_ms);
            auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
            auto_pilot.temp_action = eCarTempAction::TEMPACT_REVERSE;
            auto_pilot.time_temp_action = input.now_ms.wrapping_add(400);
            out.switch_to_real_physics = true;
        }
        if input.up_z < -0.7 {
            auto_pilot.temp_action = eCarTempAction::TEMPACT_WAIT;
            auto_pilot.time_temp_action = input.now_ms.saturating_add(1000);
        }
        if auto_pilot.temp_action == eCarTempAction::TEMPACT_NONE
            && matches!(
                auto_pilot.car_mission,
                eCarMission::MISSION_RAMPLAYER_FARAWAY
                    | eCarMission::MISSION_RAMPLAYER_CLOSE
                    | eCarMission::MISSION_BLOCKPLAYER_FARAWAY
                    | eCarMission::MISSION_BLOCKPLAYER_CLOSE
            )
            && input.player_vehicle_speed.is_some()
            && magnitude(input.player_move_speed) > magnitude(input.move_speed)
            && magnitude(input.player_move_speed) > 0.1
            && input.player_forward.x * input.forward.x + input.player_forward.y * input.forward.y
                > 0.0
        {
            let distance = sub(input.position, input.player_position);
            if 0.5 * magnitude(distance) * magnitude(input.player_move_speed)
                < distance.x * input.player_move_speed.x + distance.y * input.player_move_speed.y
                && magnitude(sub(input.player_position, input.position)) > 12.0
            {
                auto_pilot.temp_action = eCarTempAction::TEMPACT_WAIT;
                auto_pilot.time_temp_action = input.now_ms.wrapping_add(500);
            }
        }
        if input.driver_kill_player_any_means
            && magnitude(sub(input.position, input.player_position)) < 15.0
            && (input.player_vehicle_speed.is_none() || input.player_vehicle_collided)
        {
            auto_pilot.temp_action = eCarTempAction::TEMPACT_WAIT;
            auto_pilot.time_temp_action = input.now_ms.wrapping_add(3_000);
        }
        if input.siren_or_alarm
            && ((input.random_seed as u8) ^ general.get_random_number() as u8) == 0xAD
        {
            out.horn_timer = Some(45);
        }
        out
    }

    pub fn FindSwitchDistanceClose(_is_law_enforcer: bool) -> f32 {
        30.0
    }

    pub fn FindSwitchDistanceFarNormalVehicle(_is_law_enforcer: bool) -> f32 {
        Self::FindSwitchDistanceClose(false) + 5.0
    }

    pub fn FindSwitchDistanceFar(is_law_enforcer: bool) -> f32 {
        if is_law_enforcer {
            50.0
        } else {
            Self::FindSwitchDistanceFarNormalVehicle(false)
        }
    }

    pub fn CarHasReasonToStop(auto_pilot: &mut CAutoPilot, time_ms: u32) {
        auto_pilot.anti_reverse_timer = time_ms;
    }

    /// `TellCarToRamOtherCar`; the returned target is registered with the ECS
    /// reference owner by the caller that owns the vehicle relation component.
    pub fn TellCarToRamOtherCar(
        auto_pilot: &mut CAutoPilot,
        target: Entity,
        target_pool_index: usize,
    ) -> CarCommandResult {
        auto_pilot.target_car = Some(target_pool_index);
        auto_pilot.car_mission = eCarMission::MISSION_RAMCAR_FARAWAY;
        auto_pilot.cruise_speed = auto_pilot.cruise_speed.max(6);
        CarCommandResult {
            reference_target: target,
            mission: auto_pilot.car_mission,
            engine_on: true,
            cruise_speed: auto_pilot.cruise_speed,
        }
    }

    pub fn TellCarToBlockOtherCar(
        auto_pilot: &mut CAutoPilot,
        target: Entity,
        target_pool_index: usize,
    ) -> CarCommandResult {
        auto_pilot.target_car = Some(target_pool_index);
        auto_pilot.car_mission = eCarMission::MISSION_BLOCKCAR_FARAWAY;
        auto_pilot.cruise_speed = auto_pilot.cruise_speed.max(6);
        CarCommandResult {
            reference_target: target,
            mission: auto_pilot.car_mission,
            engine_on: true,
            cruise_speed: auto_pilot.cruise_speed,
        }
    }

    pub fn FindPoliceCarMissionForWantedLevel(
        wanted_level: u8,
        general: &mut CGeneral,
    ) -> eCarMission {
        match wanted_level {
            0 | 1 => eCarMission::MISSION_BLOCKPLAYER_FARAWAY,
            2 => {
                if general.get_random_number() & 3 >= 3 {
                    eCarMission::MISSION_RAMPLAYER_FARAWAY
                } else {
                    eCarMission::MISSION_BLOCKPLAYER_FARAWAY
                }
            }
            3 => {
                if general.get_random_number() & 3 >= 2 {
                    eCarMission::MISSION_RAMPLAYER_FARAWAY
                } else {
                    eCarMission::MISSION_BLOCKPLAYER_FARAWAY
                }
            }
            4..=6 => {
                if general.get_random_number() & 3 >= 1 {
                    eCarMission::MISSION_RAMPLAYER_FARAWAY
                } else {
                    eCarMission::MISSION_BLOCKPLAYER_FARAWAY
                }
            }
            _ => eCarMission::MISSION_BLOCKPLAYER_FARAWAY,
        }
    }

    pub fn FindPoliceCarSpeedForWantedLevel(
        wanted_level: u8,
        transmission_max_velocity: f32,
        general: &mut CGeneral,
    ) -> i32 {
        match wanted_level {
            0 => general.get_random_number_in_range_i32(12, 16),
            1 => 25,
            2 => 34,
            3 => (GAME_SPEED_TO_CARAI_SPEED * transmission_max_velocity * 0.9) as i32,
            4 => (GAME_SPEED_TO_CARAI_SPEED * transmission_max_velocity * 1.2) as i32,
            5 => (GAME_SPEED_TO_CARAI_SPEED * transmission_max_velocity * 1.25) as i32,
            6 => (GAME_SPEED_TO_CARAI_SPEED * transmission_max_velocity * 1.3) as i32,
            _ => 0,
        }
    }

    pub fn MellowOutChaseSpeed(
        wanted_level: u8,
        distance_to_player: f32,
        player_in_vehicle: bool,
        auto_pilot: &mut CAutoPilot,
    ) {
        let speed = match (wanted_level, player_in_vehicle) {
            (1, true) if distance_to_player < 10.0 => 15,
            (1, true) if distance_to_player < 20.0 => 22,
            (1, true) => 25,
            (1, false) if distance_to_player < 20.0 => 5,
            (1, false) if distance_to_player < 40.0 => 13,
            (1, false) => 25,
            (2, true) if distance_to_player < 10.0 => 27,
            (2, true) if distance_to_player < 20.0 => 30,
            (2, true) => 34,
            (2, false) if distance_to_player < 20.0 => 5,
            (2, false) if distance_to_player < 40.0 => 18,
            (2, false) => 34,
            _ => return,
        };
        auto_pilot.cruise_speed = speed;
    }

    /// `GetCarToGoToCoors`. `join_road_system` is the real path owner callback
    /// and returns the C++ `JoinCarWithRoadSystemGotoCoors` decision.
    pub fn GetCarToGoToCoors<F>(
        auto_pilot: &mut CAutoPilot,
        vehicle_position: CVector,
        target: CVector,
        time_ms: u32,
        join_road_system: F,
    ) -> GoToCoorsResult
    where
        F: FnOnce(CVector, bool) -> bool,
    {
        let mut path_join_attempted = false;
        let mut status_physics = false;
        if auto_pilot.car_mission != eCarMission::MISSION_GOTOCOORDS
            && auto_pilot.car_mission != eCarMission::MISSION_GOTOCOORDS_STRAIGHT
        {
            auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS;
            auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
            auto_pilot.cruise_speed = 20;
            auto_pilot.anti_reverse_timer = time_ms;
            status_physics = true;
            path_join_attempted = true;
            auto_pilot.car_mission = if join_road_system(target, false) {
                eCarMission::MISSION_GOTOCOORDS_STRAIGHT
            } else {
                eCarMission::MISSION_GOTOCOORDS
            };
        } else if (target.x - auto_pilot.destination_coors.x).abs() > 2.0
            || (target.y - auto_pilot.destination_coors.y).abs() > 2.0
        {
            auto_pilot.destination_coors = target;
        }
        let dx = vehicle_position.x - target.x;
        let dy = vehicle_position.y - target.y;
        GoToCoorsResult {
            distance: (dx * dx + dy * dy).sqrt(),
            status_physics,
            path_join_attempted,
        }
    }

    /// Mission-local portion of `UpdateCarAI`. Path joining remains an intent
    /// because `CCarCtrl` owns path-node mutation; all thresholds and mission
    /// transitions here are the original re3 branches.
    pub fn UpdateMission(
        auto_pilot: &mut CAutoPilot,
        context: CarAiMissionContext,
    ) -> CarAiMissionResult {
        let mut result = CarAiMissionResult::default();
        let distance_to_destination =
            magnitude_2d(sub(auto_pilot.destination_coors, context.vehicle_position));
        match auto_pilot.car_mission {
            eCarMission::MISSION_GOTOCOORDS => {
                if distance_to_destination < DISTANCE_TO_SWITCH_DISTANCE_GOTO
                    || auto_pilot.ignore_pathfinding
                {
                    auto_pilot.car_mission = eCarMission::MISSION_GOTOCOORDS_STRAIGHT;
                }
            }
            eCarMission::MISSION_GOTOCOORDS_STRAIGHT => {
                if (context.is_ambulance_on_duty || context.is_firetruck_on_duty)
                    && distance_to_destination < 20.0
                {
                    auto_pilot.car_mission = eCarMission::MISSION_EMERGENCYVEHICLE_STOP;
                }
                if distance_to_destination < 5.0 {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                    auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                } else if distance_to_destination > 35.0
                    && !auto_pilot.ignore_pathfinding
                    && (context.frame_counter & 7) == 0
                {
                    auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                    result.join_road_system_goto = true;
                }
            }
            eCarMission::MISSION_GOTOCOORDS_ACCURATE => {
                if distance_to_destination < 20.0 || auto_pilot.ignore_pathfinding {
                    auto_pilot.car_mission = eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE;
                }
            }
            eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE => {
                if distance_to_destination < 1.0 {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                    auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                } else if distance_to_destination > 35.0
                    && !auto_pilot.ignore_pathfinding
                    && (context.frame_counter & 7) == 0
                {
                    auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                    result.join_road_system_goto = true;
                }
            }
            eCarMission::MISSION_RAMCAR_FARAWAY => {
                if let Some(target) = context.target_car_position {
                    if magnitude_2d(sub(context.vehicle_position, target)) < 30.0
                        || auto_pilot.ignore_pathfinding
                    {
                        auto_pilot.car_mission = eCarMission::MISSION_RAMCAR_CLOSE;
                    }
                } else {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                }
            }
            eCarMission::MISSION_RAMCAR_CLOSE => {
                if let Some(target) = context.target_car_position {
                    let distance = magnitude_2d(sub(target, context.vehicle_position));
                    if distance <= if context.is_law_enforcer { 50.0 } else { 35.0 }
                        || auto_pilot.ignore_pathfinding
                    {
                        if context.target_car_collided && magnitude(context.vehicle_speed) < 0.04 {
                            auto_pilot.temp_action = eCarTempAction::TEMPACT_REVERSE;
                            auto_pilot.time_temp_action = context.current_time_ms.wrapping_add(800);
                        }
                    } else {
                        auto_pilot.car_mission = eCarMission::MISSION_RAMCAR_FARAWAY;
                        result.join_road_system = true;
                    }
                } else {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                }
            }
            eCarMission::MISSION_BLOCKCAR_FARAWAY => {
                if let Some(target) = context.target_car_position {
                    if magnitude_2d(sub(target, context.vehicle_position)) < 30.0
                        || auto_pilot.ignore_pathfinding
                    {
                        auto_pilot.car_mission = eCarMission::MISSION_BLOCKCAR_CLOSE;
                        result.siren = Some(true);
                    }
                } else {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                }
            }
            eCarMission::MISSION_BLOCKCAR_CLOSE => {
                if let Some(target) = context.target_car_position {
                    let far = if context.is_law_enforcer { 50.0 } else { 35.0 };
                    if magnitude_2d(sub(target, context.vehicle_position)) > far
                        && !auto_pilot.ignore_pathfinding
                    {
                        auto_pilot.car_mission = eCarMission::MISSION_BLOCKCAR_FARAWAY;
                        result.siren = Some(false);
                        result.horn_timer = Some(0);
                        result.join_road_system = true;
                    }
                } else {
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                }
            }
            _ => {}
        }
        result
    }

    /// Native decision inputs in descending pool order; no actor state is owned.
    pub fn MakeWayForCarWithSiren(
        siren_position: CVector,
        siren_move_speed: CVector,
        siren_slot: usize,
        candidates: impl IntoIterator<Item = SirenVehicleSnapshot>,
        current_time_ms: u32,
    ) -> Vec<SirenMakeWayIntent> {
        candidates
            .into_iter()
            .filter_map(|vehicle| {
                Self::make_way_for_siren_vehicle(
                    siren_position,
                    siren_move_speed,
                    siren_slot,
                    vehicle,
                    current_time_ms,
                )
            })
            .collect()
    }

    /// Single live candidate branch of `CCarAI::MakeWayForCarWithSiren`.
    pub fn make_way_for_siren_vehicle(
        siren_position: CVector,
        siren_move_speed: CVector,
        siren_slot: usize,
        vehicle: SirenVehicleSnapshot,
        current_time_ms: u32,
    ) -> Option<SirenMakeWayIntent> {
        let flat_speed = magnitude_2d(siren_move_speed);
        if flat_speed < 0.1 {
            return None;
        }
        let forward_x = siren_move_speed.x / flat_speed;
        let forward_y = siren_move_speed.y / flat_speed;
        let projection = flat_speed * 45.0 + 20.0;
        if vehicle.slot == siren_slot
            || !vehicle.is_car_or_bike
            || !vehicle.simple_or_physics
            || !vehicle.random_vehicle
            || vehicle.emergency_vehicle
            || (siren_position.z - vehicle.position.z).abs() >= 5.0
        {
            return None;
        }
        let dx = vehicle.position.x - siren_position.x;
        let dy = vehicle.position.y - siren_position.y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance >= projection || magnitude_2d(vehicle.move_speed) <= 0.05 {
            return None;
        }
        let correlation = (forward_x * dx + forward_y * dy) / distance;
        if correlation <= 0.0 {
            return None;
        }
        let same_direction = forward_x * vehicle.forward.x + forward_y * vehicle.forward.y > 0.7;
        if correlation > 0.8 && same_direction {
            let action = if matches!(
                vehicle.temp_action,
                eCarTempAction::TEMPACT_SWERVELEFT | eCarTempAction::TEMPACT_SWERVERIGHT
            ) {
                vehicle.temp_action
            } else if dx * forward_y - dy * forward_x > 0.0 {
                eCarTempAction::TEMPACT_SWERVELEFT
            } else {
                eCarTempAction::TEMPACT_SWERVERIGHT
            };
            return Some(SirenMakeWayIntent {
                slot: vehicle.slot,
                temp_action: action,
                time_temp_action: if action == vehicle.temp_action {
                    vehicle.time_temp_action
                } else {
                    current_time_ms.wrapping_add(2_000)
                },
                set_status_physics: true,
            });
        } else if vehicle.move_speed.x * dx + vehicle.move_speed.y * dy < 0.0
            && vehicle.temp_action != eCarTempAction::TEMPACT_WAIT
        {
            return Some(SirenMakeWayIntent {
                slot: vehicle.slot,
                temp_action: eCarTempAction::TEMPACT_WAIT,
                time_temp_action: current_time_ms.wrapping_add(2_000),
                set_status_physics: false,
            });
        }
        None
    }
}

fn sub(left: CVector, right: CVector) -> CVector {
    CVector::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn magnitude_2d(vector: CVector) -> f32 {
    (vector.x * vector.x + vector.y * vector.y).sqrt()
}

fn magnitude(vector: CVector) -> f32 {
    (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt()
}

fn distance_2d(a: CVector, b: CVector) -> f32 {
    let x = a.x - b.x;
    let y = a.y - b.y;
    (x * x + y * y).sqrt()
}

fn end_police_chase(auto_pilot: &mut CAutoPilot, no_police_zone: bool, out: &mut CarAiStepResult) {
    out.join_road_system = true;
    auto_pilot.car_mission = if no_police_zone {
        eCarMission::MISSION_NONE
    } else {
        eCarMission::MISSION_CRUISE
    };
    auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS;
    out.siren = Some(false);
}

#[cfg(test)]
mod physics_handoff_tests {
    use super::*;

    fn cruise_input(status: eEntityStatus) -> CarAiStepInput {
        CarAiStepInput {
            status, position: CVector::ZERO, move_speed: CVector::ZERO,
            forward: CVector::new(0.0, 1.0, 0.0), up_z: 1.0,
            is_law_enforcer: false, uses_siren: false, siren_or_alarm: false,
            is_rhino: false, is_ambulance_on_duty: false, is_fire_truck_on_duty: false,
            created_by: RANDOM_VEHICLE, random_seed: 8, last_collision_ms: 30_101,
            time_blocked_ms: 0, now_ms: 30_101, previous_ms: 30_100,
            time_step_ms: 1.0, frame_counter: 1,
            player_position: CVector::new(1000.0, 0.0, 0.0), player_vehicle_speed: None,
            player_move_speed: CVector::ZERO, player_forward: CVector::new(0.0, 1.0, 0.0),
            player_vehicle_upside_down: false, player_vehicle_collided: false,
            driver_kill_player_any_means: false, player_ignored_by_everyone: false,
            player_ignored_by_cops: false, wanted_level: 0, no_police_zone: false,
            bridge_requires_stop: false, target_car_position: None,
            target_car_is_player_vehicle: false, target_car_collided: false,
        }
    }

    #[test]
    fn cruise_escape_resets_before_reverse_for_both_native_control_statuses() {
        for status in [eEntityStatus::STATUS_SIMPLE, eEntityStatus::STATUS_PHYSICS] {
            let mut pilot = CAutoPilot::new(100);
            pilot.car_mission = eCarMission::MISSION_CRUISE;
            pilot.anti_reverse_timer = 55;
            pilot.current_route_node = 23;
            pilot.cruise_speed = 17;
            let input = cruise_input(status);
            let result = CCarAI::UpdateCarAI(&mut pilot, input, 0.6, &mut CGeneral::with_seed(9));
            assert!(result.switch_to_real_physics);
            assert_eq!(pilot.anti_reverse_timer, input.now_ms);
            assert_eq!(pilot.temp_action, eCarTempAction::TEMPACT_REVERSE);
            assert_eq!(pilot.time_temp_action, input.now_ms + 400);
            assert_eq!(pilot.current_route_node, 23);
            assert_eq!(pilot.cruise_speed, 17);
        }
    }

    #[test]
    fn bridge_hold_does_not_trigger_the_thirty_second_escape_handoff() {
        let mut pilot = CAutoPilot::new(100);
        pilot.car_mission = eCarMission::MISSION_CRUISE;
        pilot.anti_reverse_timer = 55;
        let mut input = cruise_input(eEntityStatus::STATUS_SIMPLE);
        input.bridge_requires_stop = true;
        let result = CCarAI::UpdateCarAI(&mut pilot, input, 0.6, &mut CGeneral::with_seed(9));
        assert!(!result.switch_to_real_physics);
        assert_eq!(pilot.anti_reverse_timer, 55);
        assert_eq!(pilot.temp_action, eCarTempAction::TEMPACT_NONE);
    }
}
