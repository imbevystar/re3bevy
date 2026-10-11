//! re3 reference: references/re-GTA/src/control/Remote.cpp
//! re3 reference: references/re-GTA/src/control/Remote.h
//!
//! Remote-control vehicle ownership state. Vehicle construction/world/camera
//! effects are applied by the ECS consumer in the same order as `CRemote`.

use bevy::prelude::{Res, ResMut, Resource};
use re3bevy_core::core::{Coordinates::CVector, Timer::CTimer};

use crate::{
    audio::DMAudio::cDMAudio,
    control::{
        CarCtrl::CCarCtrl,
        Darkel::CDarkel,
        Replay::{CReplay, ReplayMode},
    },
    core::{Camera::CCamera, Game::ScriptEntityWorld, Pad::CPads, Stats::CStats},
    entities::Entity::eEntityStatus,
    vehicles::Vehicle::RANDOM_VEHICLE,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemoteCarRequest {
    pub player_index: i32,
    pub position: CVector,
    pub rotation_radians: f32,
    pub model_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteCarRelease {
    pub vehicle_handle: i32,
    pub time_lost_remote_car: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Resource)]
pub struct CRemote {
    pub controlled_vehicle_handle: Option<i32>,
    pub previous_player_vehicle_handle: Option<i32>,
    pub active: bool,
    pub time_lost_remote_car: u32,
    pub in_remote_mode: bool,
    pub blow_up_requested: bool,
}

impl CRemote {
    pub fn give_player_remote_controlled_car(
        &mut self,
        vehicle_handle: i32,
        previous_player_vehicle_handle: Option<i32>,
    ) {
        self.controlled_vehicle_handle = Some(vehicle_handle);
        self.previous_player_vehicle_handle = previous_player_vehicle_handle;
        self.active = true;
        self.in_remote_mode = false;
        self.blow_up_requested = false;
    }

    pub fn request_blow_up(&mut self) {
        if self.controlled_vehicle_handle.is_some() {
            self.blow_up_requested = true;
        }
    }

    pub fn take_remote_controlled_car_from_player(
        &mut self,
        now_ms: u32,
    ) -> Option<RemoteCarRelease> {
        let vehicle_handle = self.controlled_vehicle_handle.take()?;
        self.time_lost_remote_car = now_ms;
        self.in_remote_mode = true;
        self.active = false;
        self.blow_up_requested = false;
        Some(RemoteCarRelease {
            vehicle_handle,
            time_lost_remote_car: now_ms,
        })
    }
}

// === FAZ8c-remote: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn process_remote_vehicle(
    pads: Res<CPads>,
    timer: Res<CTimer>,
    mut remote: ResMut<CRemote>,
    mut script_entities: ScriptEntityWorld,
    mut car_ctrl: Option<ResMut<CCarCtrl>>,
    mut camera: Option<ResMut<CCamera>>,
    mut darkel: Option<ResMut<CDarkel>>,
    mut stats: Option<ResMut<CStats>>,
    replay: Option<Res<CReplay>>,
    mut audio: Option<ResMut<cDMAudio>>,
) {
    let Some(handle) = remote.controlled_vehicle_handle else {
        return;
    };
    let Some(vehicle) = script_entities.vehicles.get(&script_entities.index, &handle) else {
        return;
    };
    let should_blow_up = remote.blow_up_requested
        || pads.get_pad(0).car_gun_just_down()
        || vehicle.base().physical.flags.is_in_water;
    if !should_blow_up {
        return;
    }
    let model_id = i32::from(vehicle.base().physical.entity.model_index);
    let Some(release) = remote.take_remote_controlled_car_from_player(timer.time_in_milliseconds())
    else {
        return;
    };
    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &release.vehicle_handle) {
        let mut base = vehicle.base_mut();
        base.vehicle_created_by = RANDOM_VEHICLE;
        base.flags.is_locked = false;
        base.vitality.health = 0.0;
        base.set_status(eEntityStatus::STATUS_WRECKED);
        base.physical.entity.flags.remove_from_world = true;
    }
    if let Some(car_ctrl) = car_ctrl.as_mut() {
        car_ctrl.num_mission_cars -= 1;
        car_ctrl.num_random_cars += 1;
    }
    if let Some(camera) = camera.as_mut() {
        camera.restore();
    }
    if let (Some(darkel), Some(stats)) = (darkel.as_mut(), stats.as_mut()) {
        darkel.register_car_blown_up_by_player(
            model_id,
            replay
                .as_ref()
                .is_some_and(|replay| replay.mode == ReplayMode::Playback),
            stats,
            audio.as_deref_mut(),
        );
    }
}
