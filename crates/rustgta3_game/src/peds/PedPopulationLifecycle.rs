//! CPopulation::ManagePopulation retirement over canonical ped identities.

use bevy::prelude::*;
use rustgta3_core::core::Timer::CTimer;

use crate::{
    control::ScriptEntities::{
        ScriptEntityIndex, ScriptPedIdentity, ScriptVehicleIdentity, retire_script_ped,
    },
    core::{
        Cam::{LOOKING_FORWARD, eCamMode},
        Camera::CCamera,
        Pools::CPools,
    },
    peds::{
        Ped::{CPed, PedState, RANDOM_CHAR},
        PedNearby::remove_near_ped_reference,
        PedType::ePedType,
        PedVitality::PedVitality,
        PlayerPed::{CWorldPlayerState, PlayerPedState},
        Population::{
            CPopulation, CREATION_RANGE, MIN_CREATION_DIST, OFFSCREEN_CREATION_MULT,
            PED_REMOVE_DIST, PED_REMOVE_DIST_SPECIAL, remove_ped_from_population_counts,
        },
        VehicleRelations::InVehicle,
    },
    vehicles::Vehicle::VehicleCore,
};

/// Fade random peds and reclaim offscreen actors. Player position, vehicle
/// speed, health and seating are read-only. Script identity retirement changes
/// the index immediately, so a reused native pool slot cannot resolve an old
/// actor while Bevy despawn commands await the schedule barrier.
pub(crate) fn process_ambient_ped_cleanup(
    mut index: ResMut<ScriptEntityIndex>,
    mut peds: Query<
        (
            Entity,
            &ScriptPedIdentity,
            &mut CPed,
            &PedVitality,
            Has<InVehicle>,
        ),
        Without<PlayerPedState>,
    >,
    players: Query<(&CPed, Option<&InVehicle>), With<PlayerPedState>>,
    player_slots: Option<Res<CWorldPlayerState>>,
    vehicles: Query<(&ScriptVehicleIdentity, &VehicleCore)>,
    mut pools: ResMut<CPools>,
    mut population: Option<ResMut<CPopulation>>,
    camera: Option<Res<CCamera>>,
    timer: Res<CTimer>,
    mut commands: Commands,
    mut doomed: Local<Vec<(i32, Entity, ePedType)>>,
) {
    let (Some(slots), Some(camera)) = (player_slots.as_deref(), camera.as_deref()) else {
        return;
    };
    let Some(player_entity) = slots.entities.get(slots.player_in_focus).copied().flatten() else {
        return;
    };
    let Ok((player, player_vehicle)) = players.get(player_entity) else {
        return;
    };
    let player_position = player.physical.entity.get_position();
    let creation_dist_multiplier = player_vehicle
        .and_then(|relation| {
            let (identity, core) = vehicles.get(relation.vehicle).ok()?;
            (index.vehicle(identity.handle) == Some(relation.vehicle)).then_some(core)
        })
        .map(|core| {
            let speed = core.physical.move_speed;
            ((speed.x * speed.x + speed.y * speed.y).sqrt() + 0.9).clamp(1.0, 1.5)
        })
        .unwrap_or(1.0);
    let generation_dist_multiplier = camera.generation_dist_multiplier;
    let allow_offscreen_reclaim = camera.cams.get(camera.active_cam).is_some_and(|cam| {
        cam.mode != eCamMode::MODE_SNIPER && cam.direction_was_looking == LOOKING_FORWARD
    });
    let offscreen_remove_distance =
        creation_dist_multiplier * (MIN_CREATION_DIST + CREATION_RANGE) * OFFSCREEN_CREATION_MULT;
    let now_ms = timer.time_in_milliseconds();
    doomed.clear();

    for (entity, identity, mut ped, vitality, seated) in &mut peds {
        if index.ped(identity.handle) != Some(entity) || ped.char_created_by != RANDOM_CHAR {
            continue;
        }
        if ped.ped_state == PedState::PED_DEAD
            && now_ms.wrapping_sub(vitality.bloody_footprint_count_or_death_time) > 60_000
        {
            ped.flags.fade_out = true;
        }
        // Native fade runs before this frame's distance decision. A newly
        // distant visible actor starts losing alpha on the next pass.
        ped.physical.entity.clump_alpha = if ped.flags.fade_out {
            ped.physical.entity.clump_alpha.saturating_sub(8)
        } else {
            ped.physical.entity.clump_alpha.saturating_add(16)
        };
        if seated {
            continue;
        }
        if ped.flags.fade_out && ped.physical.entity.clump_alpha == 0 {
            doomed.push((identity.handle, entity, ped.ped_type));
            continue;
        }
        let position = ped.physical.entity.get_position();
        let dx = position.x - player_position.x;
        let dy = position.y - player_position.y;
        let distance = (dx * dx + dy * dy).sqrt();
        let remove_distance = creation_dist_multiplier
            * if ped.flags.cull_extra_far_away {
                PED_REMOVE_DIST_SPECIAL
            } else {
                PED_REMOVE_DIST
            }
            * generation_dist_multiplier;
        let on_screen = camera.is_sphere_visible(position, 2.0);
        let far_away = distance > remove_distance
            || (allow_offscreen_reclaim && !on_screen && distance > offscreen_remove_distance);
        if !far_away {
            continue;
        }
        if on_screen {
            ped.flags.fade_out = true;
        } else {
            doomed.push((identity.handle, entity, ped.ped_type));
        }
    }

    for &(handle, entity, ped_type) in doomed.iter() {
        if index.ped(handle) != Some(entity) {
            continue;
        }
        for (_, _, mut ped, _, _) in &mut peds {
            remove_near_ped_reference(&mut ped, handle);
        }
        if let Some(population) = population.as_deref_mut() {
            remove_ped_from_population_counts(population, ped_type);
        }
        retire_script_ped(&mut commands, &mut index, &mut pools, handle);
    }
}
