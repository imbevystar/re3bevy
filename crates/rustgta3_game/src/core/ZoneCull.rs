//! re3 reference: references/re-GTA/src/core/ZoneCull.cpp
//! re3 reference: references/re-GTA/src/core/ZoneCull.h
//!
//! Zone cull volumes.

use bevy::prelude::{Res, ResMut, Resource};
use rustgta3_core::core::{Coordinates::CVector, Timer::CTimer};

use crate::{
    core::{Camera::CCamera, FileLoader::CFileLoader},
    peds::PlayerPed::CWorldPlayers,
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CZoneCullZone {
    pub min: CVector,
    pub max: CVector,
    pub flags: u32,
}

pub const ATTRZONE_NOPOLICE: u32 = 0x10;

#[derive(Debug, Clone, PartialEq, Default, Resource)]
pub struct CZoneCull {
    pub zones: Vec<CZoneCullZone>,
    pub current_flags_player: u32,
    pub current_wanted_level_drop_player: i32,
}

impl CZoneCull {
    pub fn no_police(&self) -> bool {
        self.current_flags_player & ATTRZONE_NOPOLICE != 0
    }

    pub fn wanted_level_drop(&self) -> i32 {
        self.current_wanted_level_drop_player
    }
}

impl CZoneCullZone {
    #[allow(non_snake_case)]
    pub fn Contains(&self, point: CVector) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.z >= self.min.z
            && point.x <= self.max.x
            && point.y <= self.max.y
            && point.z <= self.max.z
    }
}

// === FAZ10c-zonecull: moved from core/Game.rs (verbatim, parity-preserving) ===

/// Runtime half of re3 `CCullZones::Update` (`core/ZoneCull.cpp:257-283`).
/// The old port loaded cull-zone visibility data but never refreshed the
/// camera/player attribute masks, so indoor no-rain and no-police volumes were
/// effectively ignored after level load.
pub(crate) fn update_cull_zone_runtime(
    mut file_loader: ResMut<CFileLoader>,
    players: CWorldPlayers,
    camera: Option<Res<CCamera>>,
    timer: Res<CTimer>,
) {
    let Some(player) = players.focused() else {
        return;
    };
    let player_position = player.position();
    let camera_position = camera
        .as_ref()
        .map(|camera| camera.position)
        .unwrap_or(player_position);
    let camera_coors = [camera_position.x, camera_position.y, camera_position.z];
    let player_coors = [player_position.x, player_position.y, player_position.z];
    let frame_slot = timer.frame_counter() & 7;

    // ZoneCull.cpp cases 0/4: visibility zone transitions are driven by the
    // current game-camera position. This owns `zone_culled/zone_culled2` on
    // IPL buildings and treadables; streaming alone is not a substitute
    // because the renderer consumes those flags independently.
    if matches!(frame_slot, 0 | 4) {
        file_loader.world.force_cull_zone_coors(camera_coors);
    }
    file_loader.world.cull_zones.update_attributes(
        camera_coors,
        player_coors,
        timer.frame_counter(),
    );

    // ZoneCull.cpp case 2: the subway visibility bit is edge-triggered and
    // writes every subway building/treadable's ordinary visibility flag.
    if frame_slot == 2 {
        let invisible = !file_loader.world.cull_zones.player_subway_visible();
        if invisible != file_loader.world.cull_zones.current_subway_is_invisible {
            file_loader.world.mark_subway_as_visible(!invisible);
            file_loader.world.cull_zones.current_subway_is_invisible = invisible;
        }
    }
}

// FAZ2: moved to vehicles/Automobile.rs verbatim (13 items): fn register_automobile_skidmarks, fn register_automobile_exhaust_particles, struct AutomobileRoadContacts, fn automobile_road_contacts, fn store_automobile_wheel_contacts, fn automobile_static_collision_step, fn first_automobile_static_collision, fn automobile_static_contacts_for_matrix, fn apply_automobile_static_contacts, fn interpolate_collision_matrix, fn automobile_static_process_shift, fn process_automobile_damage_lifecycle, fn process_passive_automobile_physics

// FAZ7a: moved to control/Garages.rs verbatim (13 items): fn update_garages, fn complete_respray, fn stored_car_from_script_vehicle, fn script_vehicle_can_be_deleted, fn cancel_player_entering_removed_car, fn store_and_remove_hideout_cars, fn restore_hideout_cars, fn remove_hideout_blocking_cars_not_inside, fn garage_tidy_vehicle_slots, fn garage_is_empty, fn garage_any_other_car_touching, fn garage_any_car_blocking_door, fn destroy_script_vehicle_and_occupants
