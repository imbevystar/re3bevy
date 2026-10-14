//! CPed::DeadPedMakesTyresBloody over live, covered vehicle-sector membership.
use super::{
    Ped::{CPed, PedState},
    PedVitality::PedVitality,
    PlayerPed::{CWorldPlayerState, PlayerPedState},
    VehicleRelations::InVehicle,
};
use crate::{
    animation::CutsceneMgr::CCutsceneMgr,
    audio::{DMAudio::cDMAudio, soundlist::SOUND_SPLATTER},
    collision::{ColModel::CColModel, VehicleSpatial::VehicleCollisionSpatial},
    control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity, ScriptVehicleIdentity},
    core::{
        FileLoader::CFileLoader, Frontend::CMenuManager, Game::ScriptGameplayBridge, Pad::CPads,
    },
    renderer::Shadows::{CShadows, ScriptShadow},
    vehicles::{Automobile::AutomobileState, Vehicle::VehicleCore},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use rustgta3_core::core::{Coordinates::CVector, GameState::CGameState, Timer::CTimer};

#[derive(Default, Resource)]
pub struct PedCorpseDiagnostics {
    pub invalid_sector_queries: u64,
    pub missing_original_vehicle_col: u64,
    pub missing_vehicle_audio_identity: u64,
    pub wheel_contacts: u64,
    pub splatter_requests: u64,
}

/// Permanent native blood-pool decal history is independent of corpse lifetime.
#[derive(Default, Resource)]
pub struct CorpseBloodPools {
    pools: Vec<CorpseBloodPool>,
}

struct CorpseBloodPool {
    ped: Entity,
    position: CVector,
    death_ms: u32,
}

fn blood_pool_extent(elapsed_ms: u32) -> f32 {
    ((elapsed_ms.saturating_sub(2_000) as f32) / 5_000.0 * 0.75).clamp(0.0, 0.75)
}

fn approximate_wheel_offset(col: &CColModel, wheel: usize) -> CVector {
    let bounds = &col.bounding_box;
    CVector::new(
        if wheel < 2 {
            -bounds.max.x
        } else {
            bounds.max.x
        },
        if wheel % 2 == 0 {
            bounds.max.y
        } else {
            bounds.min.y
        },
        0.0,
    )
}

#[derive(SystemParam)]
pub(crate) struct CorpseTyreServices<'w, 's> {
    spatial: Res<'w, VehicleCollisionSpatial>,
    index: Res<'w, ScriptEntityIndex>,
    loader: Res<'w, CFileLoader>,
    game: Res<'w, CGameState>,
    player_state: Res<'w, CWorldPlayerState>,
    occupants: Query<'w, 's, &'static InVehicle, With<PlayerPedState>>,
    audio: ResMut<'w, cDMAudio>,
    pads: ResMut<'w, CPads>,
    menu: Res<'w, CMenuManager>,
    cutscene: Res<'w, CCutsceneMgr>,
    bridge: Res<'w, ScriptGameplayBridge>,
    diagnostics: ResMut<'w, PedCorpseDiagnostics>,
    timer: Res<'w, CTimer>,
    pools: ResMut<'w, CorpseBloodPools>,
    shadows: ResMut<'w, CShadows>,
}

pub(crate) fn process_corpse_tyre_contacts(
    peds: Query<(Entity, &ScriptPedIdentity, &CPed, &PedVitality)>,
    mut cars: Query<(
        &ScriptVehicleIdentity,
        &mut VehicleCore,
        &mut AutomobileState,
    )>,
    mut services: CorpseTyreServices,
    mut candidates: Local<Vec<Entity>>,
) {
    let now_ms = services.timer.time_in_milliseconds();
    let focused_vehicle = services.player_state.entities[services.player_state.player_in_focus]
        .and_then(|entity| services.occupants.get(entity).ok())
        .map(|relation| relation.vehicle);
    for (entity, identity, ped, vitality) in &peds {
        if ped.ped_state != PedState::PED_DEAD
            || services.index.ped(identity.handle) != Some(entity)
        {
            continue;
        }
        let position = ped.physical.entity.get_position();
        if services.game.nasty_game {
            let death_ms = vitality.bloody_footprint_count_or_death_time;
            let elapsed = now_ms.wrapping_sub(death_ms);
            if (2_000..47_000).contains(&elapsed)
                && !services.pools.pools.iter().any(|pool| pool.ped == entity)
            {
                // GTA III grows the blood pool from 2s to 7s after death,
                // then leaves its 0.75m decal for another 40 seconds.
                let ground_z = services.loader.world.find_ground_z_for_3d_coord(
                    position.x, position.y, position.z + 1.0,
                    &services.loader.model_info,
                ).unwrap_or(position.z - super::PedPlacement::PED_FEET_OFFSET);
                services.pools.pools.push(CorpseBloodPool {
                    ped: entity,
                    position: CVector::new(position.x, position.y, ground_z + 0.04),
                    death_ms,
                });
            }
        }
        if !services
            .spatial
            .grid
            .candidates(position, 2.0, &mut candidates)
        {
            services.diagnostics.invalid_sector_queries += 1;
            warn_once!("dead_ped_tyres_invalid_sector_query");
            continue;
        }
        for &car_entity in candidates.iter() {
            let Ok((identity, mut core, mut automobile)) = cars.get_mut(car_entity) else {
                continue;
            };
            if services.index.vehicle(identity.handle) != Some(car_entity) {
                continue;
            }
            let car_position = core.physical.entity.get_position();
            let speed = core.physical.move_speed;
            if (position.x - car_position.x).abs() >= 10.0
                || (position.y - car_position.y).abs() >= 10.0
                || speed.x * speed.x + speed.y * speed.y <= 0.05
            {
                continue;
            }
            let Some(col) = usize::try_from(core.physical.entity.model_index)
                .ok()
                .and_then(|model| services.loader.model_info.col_model(model))
            else {
                services.diagnostics.missing_original_vehicle_col += 1;
                warn_once!("dead_ped_tyres_waiting_original_vehicle_col");
                continue;
            };
            for wheel in 0..4 {
                if automobile.wheel_skidmark_bloody[wheel]
                    || automobile.suspension_spring_ratio[wheel] >= 1.0
                {
                    continue;
                }
                let wheel_position = core
                    .physical
                    .entity
                    .matrix
                    .transform_point(approximate_wheel_offset(col, wheel));
                let delta = wheel_position - position;
                if delta.z.abs() >= 2.0 || delta.x * delta.x + delta.y * delta.y >= 1.0 {
                    continue;
                }
                services.diagnostics.wheel_contacts += 1;
                if services.game.nasty_game {
                    automobile.wheel_skidmark_bloody[wheel] = true;
                    if core.physical.audio_entity_id >= 0 {
                        services.audio.PlayOneShot(
                            core.physical.audio_entity_id,
                            SOUND_SPLATTER as u16,
                            0.0,
                        );
                        services.diagnostics.splatter_requests += 1;
                    } else {
                        services.diagnostics.missing_vehicle_audio_identity += 1;
                        warn_once!("dead_ped_tyres_waiting_vehicle_audio_identity");
                    }
                }
                let force = CVector::new(0.0, 0.0, 50.0);
                core.physical.apply_move_force(force);
                core.physical
                    .apply_turn_force(force, wheel_position - car_position);
                if focused_vehicle == Some(car_entity) {
                    services.pads.get_pad_mut(0).start_shake(
                        300,
                        70,
                        services.menu.prefs_use_vibration != 0,
                        services.cutscene.running,
                        services.bridge.playing_intro,
                    );
                }
            }
        }
    }
    services.pools.pools.retain(|pool| now_ms.wrapping_sub(pool.death_ms) < 47_000);
    if services.game.nasty_game {
        for pool in &services.pools.pools {
            let size = blood_pool_extent(now_ms.wrapping_sub(pool.death_ms));
            if size <= 0.0 { continue; }
            services.shadows.script_shadows.push(ScriptShadow {
                texture: 6, // SHADOWTEX_BLOOD -> bloodpool_64
                position: pool.position,
                front: [size, 0.0],
                side: [0.0, -size],
                intensity: 255,
                color: [255, 0, 0],
                corners: None,
            });
        }
    }
}

#[cfg(test)]
mod blood_pool_tests {
    use super::*;

    #[test]
    fn native_pool_grows_after_two_seconds_and_caps_at_seven() {
        assert_eq!(blood_pool_extent(2_000), 0.0);
        assert!((blood_pool_extent(4_500) - 0.375).abs() < 0.0001);
        assert_eq!(blood_pool_extent(7_000), 0.75);
        assert_eq!(blood_pool_extent(45_000), 0.75);
    }
}
