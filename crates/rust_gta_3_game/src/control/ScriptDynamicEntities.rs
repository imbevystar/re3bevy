//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Split from control/Script.rs (verbatim, parity-preserving).

#[allow(unused_imports)]
use super::Script::{
    ScriptEntityWorld, ScriptGameplayBridge, ScriptObjectMut, ScriptPedStore, ScriptPickupIndex,
    ScriptRadarBlipIndex, ScriptTaxiLightState, ScriptVehicleEntity, ScriptVehicleStore,
    apply_script_player_clothing_effects, materialize_pickup_object, queue_ped_script_say,
    remove_materialized_pickup, remove_script_object, remove_script_vehicle_and_occupants,
    script_char_say_sound, script_key_string, script_vehicle_base_can_be_deleted,
    set_all_occupants_of_car_leave_car, spawn_script_object,
};
#[allow(unused_imports)]
use super::ScriptTypes::{
    CTheScripts, MAP_Z_LOW_LIMIT, ScriptCommandEffect, ScriptCreateCarRequest,
    ScriptEntitySnapshot, ScriptHandle, ScriptObjectMotionRequest,
};
#[allow(unused_imports)]
use crate::animation::CutsceneMgr::{
    CCutsceneMgr, load_cutscene_head_animation, load_cutscene_runtime_data,
    restore_after_cutscene_clear,
};
#[allow(unused_imports)]
use crate::animation::PedVehicleAnimation::CPedVehicleAnimation;
#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::collision::ColModel::{CColBox, CColSphere, CColVector};
#[allow(unused_imports)]
use crate::collision::Collision::CCollision;
#[allow(unused_imports)]
use crate::control::AutoPilot::{
    eCarDrivingStyle, eCarMission, eCarTempAction, script_car_mission,
};
#[allow(unused_imports)]
use crate::control::Darkel::CDarkel;
#[allow(unused_imports)]
use crate::control::GameLogic::{CGameLogic, GameLogicIntent, ResurrectionKind};
#[allow(unused_imports)]
use crate::control::Garages::{CGarages, garage_contains_vehicle};
#[allow(unused_imports)]
use crate::control::Garages::{CStoredCar, destroy_script_vehicle_and_occupants};
#[allow(unused_imports)]
use crate::control::OnscreenTimer::COnscreenTimer;
#[allow(unused_imports)]
use crate::control::PathFind::CPathFind;
#[allow(unused_imports)]
use crate::control::Pickups::{
    CPacManPickups, PICKUP_COLLECTABLE1, PICKUP_FLOATINGPACKAGE, remove_pacman_object,
    reset_vehicle_pacman_load, spawn_pacman_objects,
};
#[allow(unused_imports)]
use crate::control::Record::{CHASE_CARS, CRecordDataForChase};
#[allow(unused_imports)]
use crate::control::Remote::CRemote;
#[allow(unused_imports)]
use crate::control::Replay::{CReplay, ReplayMode};
#[allow(unused_imports)]
use crate::control::Restart::CRestart;
#[allow(unused_imports)]
use crate::core::Camera::{CCamera, CameraScriptMode};
#[allow(unused_imports)]
use crate::core::EventList::CEventList;
#[allow(unused_imports)]
use crate::core::Fire::CFireManager;
#[allow(unused_imports)]
use crate::core::Frontend::CMenuManager;
#[allow(unused_imports)]
use crate::core::Game::{
    CGameBeforeRw, cutscene_model_snapshot, find_links_to_go_with_these_nodes,
    focused_player_vehicle_handle, join_car_with_road_system, join_car_with_road_system_goto_coors,
    snap_follow_camera_resource,
};
#[allow(unused_imports)]
use crate::core::Pad::{CPads, PLAYERCONTROL_CAMERA};
#[allow(unused_imports)]
use crate::core::PlayerInfo::CPlayerInfo;
#[allow(unused_imports)]
use crate::core::Pools::CPoolHandle;
#[allow(unused_imports)]
use crate::core::References::CReferences;
#[allow(unused_imports)]
use crate::core::Stats::CStats;
#[allow(unused_imports)]
use crate::core::Streaming::{CStreaming, STREAMFLAGS_DEPENDENCY};
#[allow(unused_imports)]
use crate::core::World::EEntityType;
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::entities::Entity::script_entity_status;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_DEADDODO;
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::script_vehicle_base_z;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::{choose_vehicle_optional_components, eVehicleType};
#[allow(unused_imports)]
use crate::objects::Object::TEMP_OBJECT;
#[allow(unused_imports)]
use crate::objects::ObjectData::CObjectInfo;
#[allow(unused_imports)]
use crate::objects::ParticleObject::CParticleObjectRuntime;
#[allow(unused_imports)]
use crate::peds::Gangs::CGangs;
#[allow(unused_imports)]
use crate::peds::Ped::{
    RANDOM_CHAR, add_ped_ammo, apply_ped_objective, apply_ped_wait_state,
    apply_script_ped_wander_path, set_ped_ammo, set_script_ped_chat,
};
#[allow(unused_imports)]
use crate::peds::Ped::{ped_seat_world_position, player_seat_index};
#[allow(unused_imports)]
use crate::peds::PedRoutes::CPedRoutes;
#[allow(unused_imports)]
use crate::peds::PedType::script_ped_type;
#[allow(unused_imports)]
use crate::peds::PedType::{CPedTypeRuntime, PED_STATS_TYPES, ePedType};
#[allow(unused_imports)]
use crate::peds::PlayerPed::CPlayerPed;
#[allow(unused_imports)]
use crate::peds::VehicleRelations::{
    InVehicle, PedEntity, VehicleEntity, queue_clear_vehicle_relation, queue_set_vehicle_relation,
};
#[allow(unused_imports)]
use crate::renderer::Coronas::CCoronas;
#[allow(unused_imports)]
use crate::renderer::Credits::CCredits;
#[allow(unused_imports)]
use crate::renderer::Font::CFont;
#[allow(unused_imports)]
use crate::renderer::Hud::CHud;
#[allow(unused_imports)]
use crate::renderer::Particle::CParticle;
#[allow(unused_imports)]
use crate::renderer::PointLights::CPointLights;

fn mission_zone_level(file_loader: &CFileLoader, position: CVector) -> i8 {
    file_loader
        .world
        .zones
        .get_level_from_position([position.x, position.y, position.z]) as i8
}

/// Mission actors are pinned to the island that owns their collision, just
/// like Script.cpp assigns `m_nZoneLevel` at creation. A far SCM teleport must
/// move that residency tag with the car; otherwise its physics/AI is gated by
/// the old island and the car can appear to drive through unloaded geometry.
fn sync_mission_vehicle_zone_level(
    script_entities: &mut ScriptEntityWorld,
    vehicle_handle: i32,
    zone_level: i8,
) {
    let occupant_handles = script_entities
        .vehicle_entity(vehicle_handle)
        .map(|vehicle| {
            script_entities
                .script_occupant_handles(vehicle)
                .map(|(handle, _)| handle)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
        vehicle.base_mut().physical.zone_level = zone_level;
    }
    for handle in occupant_handles {
        if let Some(ped) = script_entities.peds.get_mut(&script_entities.index, &handle) {
            ped.ped.physical.zone_level = zone_level;
        }
    }
}
#[allow(unused_imports)]
use crate::renderer::Rubbish::CRubbish;
#[allow(unused_imports)]
use crate::renderer::Shadows::{CShadows, ScriptShadow};
#[allow(unused_imports)]
use crate::renderer::SpecialFX::{C3dMarkerState, CSpecialFX, MARKERTYPE_CYLINDER};
#[allow(unused_imports)]
use crate::renderer::WaterLevel::CWaterLevel;
#[allow(unused_imports)]
use crate::renderer::Weather::CWeather;
#[allow(unused_imports)]
use crate::text::Messages::CMessages;
#[allow(unused_imports)]
use crate::text::Pager::CPager;
#[allow(unused_imports)]
use crate::text::Text::CText;
#[allow(unused_imports)]
use crate::vehicles::Automobile::{script_automobile_base_z, script_bomb_type};
#[allow(unused_imports)]
use crate::vehicles::Boat::CBoat;
#[allow(unused_imports)]
use crate::vehicles::CarGen::CTheCarGenerators;
#[allow(unused_imports)]
use crate::vehicles::Cranes::CCranes;
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use crate::vehicles::Heli::CHeliRuntime;
#[allow(unused_imports)]
use crate::vehicles::Heli::{CHeli, HELI_TYPE_CATALINA};
#[allow(unused_imports)]
use crate::vehicles::Plane::CPlane;
#[allow(unused_imports)]
use crate::vehicles::Plane::{CPlaneRuntime, CesnaMissionStatus};
#[allow(unused_imports)]
use crate::vehicles::Vehicle::PERMANENT_VEHICLE;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::{
    MISSION_VEHICLE, PARKED_VEHICLE, RANDOM_VEHICLE, eCarLock, script_car_lock,
};
#[allow(unused_imports)]
use crate::weapons::Explosion::{CExplosionPool, eExplosionType, script_explosion_type};
#[allow(unused_imports)]
use crate::weapons::ProjectileInfo::CProjectileInfoPool;
#[allow(unused_imports)]
use crate::weapons::Weapon::CWeapon;
#[allow(unused_imports)]
use crate::weapons::WeaponInfo::CWeaponInfoStore;
#[allow(unused_imports)]
use crate::weapons::WeaponType::script_weapon_type;
#[allow(unused_imports)]
use bevy::input::{ButtonInput, mouse::MouseButton};
#[allow(unused_imports)]
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
#[allow(unused_imports)]
use bevy::prelude::{KeyCode, error};
#[allow(unused_imports)]
use rust_gta_3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use rust_gta_3_assets::streaming::CdStream::CdStream;
#[allow(unused_imports)]
use rust_gta_3_core::core::GameState::{CGameState, ELevelName};
#[allow(unused_imports)]
use rust_gta_3_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral};

#[allow(unused_imports)]
use crate::audio::soundlist::*;
#[allow(unused_imports)]
use crate::control::CarCtrl::{CCarCtrl, VehicleOfInterest};
#[allow(unused_imports)]
use crate::control::Pickups::{CPickups, PICKUP_IN_SHOP, PICKUP_IN_SHOP_OUT_OF_STOCK};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
// FAZ8a bridge consumed by FAZ8d: ScriptEntityIndex, ScriptEntityWorld,
// ScriptObjectEntity, ScriptPedEntity, ScriptVehicleEntity, spawn_script_object
// and remove_script_object now live in this module.
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPools};
#[allow(unused_imports)]
use crate::entities::Physical::{GTA_PHYSICS_HZ, physical_collision_records};
#[allow(unused_imports)]
use crate::math::Matrix::CMatrix;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_PICKUP_BONUS;
#[allow(unused_imports)]
use crate::objects::Object::{CObject, MISSION_OBJECT};
#[allow(unused_imports)]
use crate::peds::Ped::{
    CPed, MISSION_CHAR, PedState, eMoveState, eObjective, ped_weapon_ammo_snapshot,
    script_ped_can_see_player,
};
#[allow(unused_imports)]
use crate::peds::PlayerPed::{CWorldPlayers, WorldPlayerAccess};
#[allow(unused_imports)]
use crate::peds::Population::{CPopulation, remove_ped_from_population_counts};
#[allow(unused_imports)]
use crate::peds::VehicleRelations::VehicleSeat;
#[allow(unused_imports)]
use crate::vehicles::Automobile::CAutomobile;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::CVehicle;
#[allow(unused_imports)]
use bevy::prelude::*;
#[allow(unused_imports)]
use rust_gta_3_core::core::Timer::CTimer;
use crate::peds::PlayerRead::CWorldPlayersRead;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

/// GET_PLAYER_CHAR returns a ped-pool handle; SET_CHAR_RUNNING must resolve
/// that handle through either live actor owner without confusing it with a
/// player-array index.
fn set_script_char_running(
    entities: &mut ScriptEntityWorld,
    players: &mut CWorldPlayers,
    handle: i32,
    enabled: bool,
) -> bool {
    if let Some(entity) = entities.ped_get_mut(handle) {
        entity.ped.flags.is_running = enabled;
        true
    } else if let Some(player) = players.find_by_pool_handle_mut(handle) {
        player.ped.flags.is_running = enabled;
        true
    } else { false }
}

#[cfg(test)]
mod running_actor_tests {
    use super::*;

    #[test]
    fn running_opcode_resolves_the_player_char_handle_and_can_clear_running() {
        use bevy::ecs::system::SystemState;
        use crate::peds::PlayerPed::CWorldPlayerState;
        let mut world = World::new();
        world.init_resource::<crate::control::ScriptEntities::ScriptEntityIndex>();
        let mut player = CPlayerPed::new(&mut CGeneral::with_seed(1), None);
        player.state.pool_handle = 569;
        let entity = world.spawn((PedEntity, player.into_bundle())).id();
        world.insert_resource(CWorldPlayerState { entities: [Some(entity), None], ..default() });
        let mut state = SystemState::<(ScriptEntityWorld, CWorldPlayers)>::new(&mut world);
        {
            let (mut actors, mut players) = state.get_mut(&mut world).unwrap();
            assert!(set_script_char_running(&mut actors, &mut players, 569, true));
            assert!(players.player(0).unwrap().ped.flags.is_running);
            assert!(set_script_char_running(&mut actors, &mut players, 569, false));
            assert!(!players.player(0).unwrap().ped.flags.is_running);
            assert!(!set_script_char_running(&mut actors, &mut players, 570, true));
        }
        state.apply(&mut world);
    }
}

pub(crate) fn apply_script_dynamic_entity_effects(
    mut scripts: ResMut<CTheScripts>,
    mut script_entities: ScriptEntityWorld,
    mut world_players: CWorldPlayers,
    mut general: ResMut<CGeneral>,
    mut pickups: Option<ResMut<CPickups>>,
    runtime_environment: (
        Option<Res<CWaterLevel>>,
        Option<Res<CWeather>>,
        Option<ResMut<CPopulation>>,
        Option<ResMut<CFireManager>>,
        Option<ResMut<CExplosionPool>>,
        Option<ResMut<CProjectileInfoPool>>,
        Option<ResMut<CShadows>>,
        Option<ResMut<CParticle>>,
        Option<Res<crate::renderer::ParticleMgr::cParticleSystemMgr>>,
        Option<ResMut<CSpecialFX>>,
    ),
    timer: Res<CTimer>,
    mut ped_routes: ResMut<CPedRoutes>,
    mut file_loader: ResMut<CFileLoader>,
    mut events: ResMut<CEventList>,
    pools: Option<ResMut<CPools>>,
    handling_manager: Option<Res<cHandlingDataMgr>>,
    mut remote: ResMut<CRemote>,
    mut camera: Option<ResMut<CCamera>>,
    mut car_ctrl: Option<ResMut<CCarCtrl>>,
    script_support: (
        ResMut<CPads>,
        ResMut<CStats>,
        Option<Res<CWeaponInfoStore>>,
        Option<ResMut<CMenuManager>>,
        Option<Res<CCutsceneMgr>>,
        Res<CGameState>,
        ResMut<ScriptGameplayBridge>,
        ResMut<ScriptPickupIndex>,
        ResMut<ScriptRadarBlipIndex>,
        ResMut<ScriptTaxiLightState>,
    ),
) {
    const HIGHLIGHT_MARKER_PREFIX: u32 = 0x4840_0000;
    let (
        water_level,
        weather,
        mut population,
        mut fire_manager,
        mut explosions,
        mut projectiles,
        mut shadows,
        mut particles,
        particle_systems,
        mut special_fx,
    ) = runtime_environment;
    let (
        mut pads,
        mut stats,
        weapon_info,
        mut menu,
        cutscene,
        game_state,
        mut bridge,
        mut pickup_index,
        mut radar_blips,
        mut taxi_lights,
    ) = script_support;
    let Some(mut pools) = pools else {
        bevy::log::warn_once!("script_dynamic_effects_waiting_pools");
        return;
    };
    if let Some(shadows) = shadows.as_mut() {
        // re3 empties the transient rendered-shadow list once per frame.
        shadows.script_shadows.clear();
        shadows.static_shadows.clear();
    }
    apply_script_player_clothing_effects(&mut scripts, &mut world_players);
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    let mut active_highlight_markers = HashSet::new();
    for effect in effects {
        if super::ScriptActorDeletion::apply_script_actor_deletion(
            &effect, &mut script_entities, &mut pools, car_ctrl.as_deref_mut(),
        ) { continue; }
        match effect {
            ScriptCommandEffect::MissionCleanupProcessed => {
                if !bridge.playing_intro && scripts.countdown_to_make_player_unsafe == 0 {
                    pads.get_pad_mut(0)
                        .set_enable_player_controls(crate::core::Pad::PLAYERCONTROL_PLAYERINFO);
                }
                // re3 `CMissionCleanup::Process` touches only handles stored in
                // its fixed cleanup array. The VM already materialises those
                // exact entries as Mark*AsNoLongerNeeded effects before this
                // marker. Sweeping every live MISSION_* entity here also
                // captures persistent main-script objects that were never in
                // the finished mission's cleanup list; after their 20 second
                // TEMP_OBJECT lifetime expires, SCM keeps using a dead pool
                // handle (for example rotating/sliding world-script objects).
            }
            ScriptCommandEffect::EnablePlayerControlCamera => {
                pads.get_pad_mut(0)
                    .set_enable_player_controls(PLAYERCONTROL_CAMERA);
            }
            ScriptCommandEffect::ShakePad {
                pad,
                duration,
                frequency,
            } => {
                pads.get_pad_mut(pad).start_shake(
                    duration,
                    frequency,
                    menu.as_ref()
                        .is_none_or(|menu| menu.prefs_use_vibration != 0),
                    cutscene
                        .as_ref()
                        .is_some_and(|cutscene| cutscene.is_running()),
                    game_state.playing_intro,
                );
            }
            ScriptCommandEffect::HighlightArea {
                script_ip,
                x1,
                y1,
                x2,
                y2,
                z,
            } => {
                let Some(special_fx) = special_fx.as_mut() else {
                    leftover.push(ScriptCommandEffect::HighlightArea {
                        script_ip,
                        x1,
                        y1,
                        x2,
                        y2,
                        z,
                    });
                    continue;
                };
                // CTheScripts::HighlightImportantArea normalises both axes,
                // grounds the MAP_Z_LOW_LIMIT sentinel at the rectangle
                // centre and passes the larger half-extent to
                // CShadows::RenderIndicatorShadow.
                let inf_x = x1.min(x2);
                let sup_x = x1.max(x2);
                let inf_y = y1.min(y2);
                let sup_y = y1.max(y2);
                let center_x = (inf_x + sup_x) * 0.5;
                let center_y = (inf_y + sup_y) * 0.5;
                let center_z = if z <= MAP_Z_LOW_LIMIT {
                    file_loader.world.find_ground_z_for_coord(
                        center_x,
                        center_y,
                        &file_loader.model_info,
                    )
                } else {
                    z
                };
                let size = (sup_x - center_x).max(sup_y - center_y);
                for (variant, (scale, rotate_rate)) in [(1.0_f32, 1_i16), (0.93, 2), (0.86, -1)]
                    .into_iter()
                    .enumerate()
                {
                    let marker_size = size * scale;
                    let mut position = CVector::new(center_x, center_y, center_z);
                    if let Some(ground_z) = file_loader.world.find_ground_z_for_3d_coord(
                        center_x,
                        center_y,
                        center_z + 1.0,
                        &file_loader.model_info,
                    ) && ground_z != 0.0
                    {
                        // C3dMarkers::PlaceMarker grounds a newly-created
                        // cylinder slightly below the surface by 5% of its
                        // own scale. PlaceMarkerSet repeats this for all three
                        // nested variants.
                        position.z = ground_z - 0.05 * marker_size;
                    }
                    let identifier = HIGHLIGHT_MARKER_PREFIX
                        | (script_ip.wrapping_mul(3).wrapping_add(variant as u32) & 0x000F_FFFF);
                    active_highlight_markers.insert(identifier);
                    special_fx.place_marker(
                        identifier,
                        MARKERTYPE_CYLINDER,
                        position,
                        marker_size,
                        0,
                        128,
                        255,
                        128,
                        2048,
                        0.2,
                        rotate_rate,
                        timer.time_in_milliseconds(),
                        world_players.focused().map_or(position, |player| player.position()),
                        timer.time_step(),
                    );
                }
            }
            ScriptCommandEffect::DrawShadow {
                texture,
                position,
                front,
                side,
                intensity,
                color,
            } => {
                if let Some(shadows) = shadows.as_mut() {
                    shadows.script_shadows.push(ScriptShadow {
                        texture,
                        position,
                        front,
                        side,
                        intensity,
                        color,
                        corners: None,
                    });
                } else {
                    leftover.push(ScriptCommandEffect::DrawShadow {
                        texture,
                        position,
                        front,
                        side,
                        intensity,
                        color,
                    });
                }
            }
            ScriptCommandEffect::CreateRandomCarForCarPark {
                model_id,
                position,
                heading_degrees,
            } => {
                let zone_level = file_loader
                    .world
                    .zones
                    .get_level_from_position([position.x, position.y, position.z])
                    as i8;
                let loader = &mut *file_loader;
                let model_info = loader.model_info.get_model_info(model_id.max(0) as usize);
                let Some(_handle) = script_entities.insert_random_car_for_car_park(
                    &mut pools,
                    model_id,
                    position,
                    heading_degrees,
                    timer.time_in_milliseconds(),
                    model_info,
                    handling_manager.as_deref(),
                    &mut loader.path_find,
                    zone_level,
                    &mut general,
                    taxi_lights.all_enabled,
                ) else {
                    leftover.push(ScriptCommandEffect::CreateRandomCarForCarPark {
                        model_id,
                        position,
                        heading_degrees,
                    });
                    continue;
                };
                if let Some(car_ctrl) = car_ctrl.as_mut() {
                    car_ctrl.num_random_cars += 1;
                }
            }
            ScriptCommandEffect::StartBoatFoamAnimation => {
                if let Some(special_fx) = special_fx.as_mut() {
                    special_fx.start_boat_foam_animation(timer.time_in_milliseconds());
                } else {
                    leftover.push(ScriptCommandEffect::StartBoatFoamAnimation);
                }
            }
            ScriptCommandEffect::UpdateBoatFoamAnimation { handle } => {
                let (Some(special_fx), Some(particles), Some(object)) = (
                    special_fx.as_mut(),
                    particles.as_mut(),
                    script_entities.object_get(handle),
                ) else {
                    leftover.push(ScriptCommandEffect::UpdateBoatFoamAnimation { handle });
                    continue;
                };
                let Some(camera) = camera.as_deref() else {
                    bevy::log::warn_once!("script_boat_foam_waiting_original_camera");
                    leftover.push(ScriptCommandEffect::UpdateBoatFoamAnimation { handle });
                    continue;
                };
                let mut particles = particles.emitter_with_context(
                    particle_systems.as_deref(), Some(camera.position));
                for intent in special_fx.update_boat_foam_animation(
                    &object.object.physical.entity.matrix,
                    camera.orientation,
                    timer.previous_time_in_milliseconds(),
                    timer.time_in_milliseconds(),
                    &mut general,
                ) {
                    particles.add_particle(
                        intent.particle_type,
                        intent.position,
                        CVector::ZERO,
                        None,
                        intent.size,
                        1,
                        0,
                        0,
                        0,
                    );
                }
            }
            ScriptCommandEffect::GiveRemoteControlledCar {
                player_index,
                mut position,
                rotation_radians,
                model_id,
            } => {
                let Some(_) = world_players.player(player_index) else {
                    leftover.push(ScriptCommandEffect::GiveRemoteControlledCar {
                        player_index,
                        position,
                        rotation_radians,
                        model_id,
                    });
                    continue;
                };
                let previous_handle = world_players.relation(player_index).and_then(|relation| {
                    script_entities.vehicle_handle_for_entity(relation.vehicle)
                });
                position.z = file_loader
                    .world
                    .find_ground_z_for_3d_coord(
                        position.x,
                        position.y,
                        position.z + 2.0,
                        &file_loader.model_info,
                    )
                    .unwrap_or(0.0);
                let model = file_loader
                    .model_info
                    .get_model_info(model_id.max(0) as usize);
                let request = ScriptCreateCarRequest {
                    model_id,
                    position,
                    time_ms: timer.time_in_milliseconds(),
                    clear_space_for_mission_entity: true,
                };
                let Some(handle) = script_entities.insert_script_car(
                    &mut pools,
                    request,
                    model,
                    handling_manager.as_deref(),
                    &mut general,
                    &file_loader,
                    taxi_lights.all_enabled,
                ) else {
                    leftover.push(ScriptCommandEffect::GiveRemoteControlledCar {
                        player_index,
                        position,
                        rotation_radians,
                        model_id,
                    });
                    continue;
                };
                if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut vehicle = vehicle.base_mut();
                    vehicle.physical.entity.set_heading(rotation_radians);
                    vehicle.set_status(eEntityStatus::STATUS_PLAYER_REMOTE);
                    vehicle.flags.is_locked = true;
                    vehicle.drive.engine_on = true;
                    vehicle.auto_pilot.car_mission = eCarMission::MISSION_NONE;
                    vehicle.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                    vehicle.auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS;
                    vehicle.auto_pilot.cruise_speed = 9;
                    vehicle.auto_pilot.max_traffic_speed = 9.0;
                    vehicle.auto_pilot.current_lane = 0;
                    vehicle.auto_pilot.next_lane = 0;
                }
                if let Some(mut previous) =
                    previous_handle.and_then(|handle| script_entities.vehicles.get_mut(&script_entities.index, &handle))
                {
                    previous
                        .base_mut()
                        .set_status(eEntityStatus::STATUS_PLAYER_DISABLED);
                }
                remote.give_player_remote_controlled_car(handle, previous_handle);
                if let Some(car_ctrl) = car_ctrl.as_mut() {
                    car_ctrl.num_mission_cars += 1;
                }
                if let Some(camera) = camera.as_mut() {
                    camera.script_mode = CameraScriptMode::FollowRemoteVehicle;
                    camera.script_target = Some(crate::core::Camera::CameraScriptTarget::Vehicle(ScriptHandle(handle)));
                }
            }
            ScriptCommandEffect::CreateCollectable { position } => {
                let Some(pickups) = pickups.as_mut() else {
                    leftover.push(ScriptCommandEffect::CreateCollectable { position });
                    continue;
                };
                let Some((model_id, _)) = file_loader.model_info.get_model_info_by_name("package1")
                else {
                    leftover.push(ScriptCommandEffect::CreateCollectable { position });
                    continue;
                };
                if let Some(actual_handle) = pickups.generate_new_one(
                    position,
                    model_id as i32,
                    i32::from(PICKUP_COLLECTABLE1),
                    0,
                    timer.time_in_milliseconds(),
                ) {
                    let _ = materialize_pickup_object(
                        actual_handle,
                        pickups,
                        &mut script_entities.commands,
                        &mut script_entities.index,
                            &mut pools,
                            &mut general,
                            &file_loader,
                    );
                }
            }
            ScriptCommandEffect::CreateFloatingPackage { handle, position } => {
                let Some(pickups) = pickups.as_mut() else {
                    leftover.push(ScriptCommandEffect::CreateFloatingPackage { handle, position });
                    continue;
                };
                let Some((model_id, _)) = file_loader
                    .model_info
                    .get_model_info_by_name("floatpackge1")
                else {
                    leftover.push(ScriptCommandEffect::CreateFloatingPackage { handle, position });
                    continue;
                };
                if let Some(actual_handle) = pickups.generate_new_one(
                    position,
                    model_id as i32,
                    i32::from(PICKUP_FLOATINGPACKAGE),
                    0,
                    timer.time_in_milliseconds(),
                ) {
                    pickup_index.0.insert(handle, actual_handle);
                    let _ = materialize_pickup_object(
                        actual_handle,
                        pickups,
                        &mut script_entities.commands,
                        &mut script_entities.index,
                        &mut pools,
                        &mut general,
                        &file_loader,
                    );
                }
            }
            ScriptCommandEffect::CreatePickup {
                handle,
                model_id,
                pickup_type,
                quantity,
                position,
            } => {
                if let Some(pickups) = pickups.as_mut() {
                    if let Some(actual_handle) = pickups.generate_new_one(
                        position,
                        model_id,
                        pickup_type,
                        quantity,
                        timer.time_in_milliseconds(),
                    ) {
                        pickup_index.0.insert(handle, actual_handle);
                        let _ = materialize_pickup_object(
                            actual_handle,
                            pickups,
                            &mut script_entities.commands,
                            &mut script_entities.index,
                            &mut pools,
                            &mut general,
                            &file_loader,
                        );
                    }
                } else {
                    leftover.push(ScriptCommandEffect::CreatePickup {
                        handle,
                        model_id,
                        pickup_type,
                        quantity,
                        position,
                    });
                }
            }
            ScriptCommandEffect::CreateUntrackedPickup {
                model_id,
                pickup_type,
                quantity,
                position,
            } => {
                if let Some(pickups) = pickups.as_mut() {
                    if let Some(actual_handle) = pickups.generate_new_one(
                        position,
                        model_id,
                        pickup_type,
                        quantity,
                        timer.time_in_milliseconds(),
                    ) {
                        let _ = materialize_pickup_object(
                            actual_handle,
                            pickups,
                            &mut script_entities.commands,
                            &mut script_entities.index,
                            &mut pools,
                            &mut general,
                            &file_loader,
                        );
                    }
                } else {
                    leftover.push(ScriptCommandEffect::CreateUntrackedPickup {
                        model_id,
                        pickup_type,
                        quantity,
                        position,
                    });
                }
            }
            ScriptCommandEffect::CreatePickupBlip {
                handle,
                pickup_handle,
            } => {
                let Some(pickups) = pickups.as_ref() else {
                    leftover.push(ScriptCommandEffect::CreatePickupBlip {
                        handle,
                        pickup_handle,
                    });
                    continue;
                };
                let Some(actual_pickup) = pickup_index.0.get(&pickup_handle).copied() else {
                    leftover.push(ScriptCommandEffect::CreatePickupBlip {
                        handle,
                        pickup_handle,
                    });
                    continue;
                };
                let Some(pickup_slot) = pickups.actual_pickup_index(actual_pickup) else {
                    leftover.push(ScriptCommandEffect::CreatePickupBlip {
                        handle,
                        pickup_handle,
                    });
                    continue;
                };
                let Some(object_pool_index) = pickups.pickups[pickup_slot].object_pool_index else {
                    leftover.push(ScriptCommandEffect::CreatePickupBlip {
                        handle,
                        pickup_handle,
                    });
                    continue;
                };
                let actual_blip = file_loader.radar.set_entity_blip(
                    crate::core::Radar::eBlipType::BLIP_OBJECT,
                    object_pool_index,
                    6,
                    crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BOTH,
                );
                file_loader.radar.change_blip_scale(actual_blip, 3);
                radar_blips.0.insert(handle, actual_blip);
            }
            ScriptCommandEffect::CreateSpritePickupBlip {
                handle,
                pickup_handle,
                sprite,
            } => {
                let Some(pickups) = pickups.as_ref() else {
                    leftover.push(ScriptCommandEffect::CreateSpritePickupBlip {
                        handle,
                        pickup_handle,
                        sprite,
                    });
                    continue;
                };
                let Some(actual_pickup) = pickup_index.0.get(&pickup_handle).copied() else {
                    leftover.push(ScriptCommandEffect::CreateSpritePickupBlip {
                        handle,
                        pickup_handle,
                        sprite,
                    });
                    continue;
                };
                let Some(pickup_slot) = pickups.actual_pickup_index(actual_pickup) else {
                    leftover.push(ScriptCommandEffect::CreateSpritePickupBlip {
                        handle,
                        pickup_handle,
                        sprite,
                    });
                    continue;
                };
                let Some(object_pool_index) = pickups.pickups[pickup_slot].object_pool_index else {
                    leftover.push(ScriptCommandEffect::CreateSpritePickupBlip {
                        handle,
                        pickup_handle,
                        sprite,
                    });
                    continue;
                };
                let actual_blip = file_loader.radar.set_entity_blip(
                    crate::core::Radar::eBlipType::BLIP_OBJECT,
                    object_pool_index,
                    6,
                    crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BOTH,
                );
                file_loader.radar.set_blip_sprite(
                    actual_blip,
                    crate::core::Radar::eRadarSprite::from_re3(sprite),
                );
                radar_blips.0.insert(handle, actual_blip);
            }
            ScriptCommandEffect::RemovePickup { handle } => {
                if let Some(pickups) = pickups.as_mut() {
                    if let Some(actual_handle) = pickup_index.0.remove(&handle) {
                        remove_materialized_pickup(
                            actual_handle,
                            pickups,
                            &mut script_entities.commands,
                            &mut script_entities.index,
                            &mut pools,
                        );
                    }
                } else {
                    leftover.push(ScriptCommandEffect::RemovePickup { handle });
                }
            }
            ScriptCommandEffect::UndressChar { handle, model_name } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.physical.entity.flags.is_visible = false;
                    leftover.push(ScriptCommandEffect::RequestSpecialModel {
                        model_id: entity.model_id,
                        name: model_name,
                    });
                } else {
                    leftover.push(ScriptCommandEffect::UndressChar { handle, model_name });
                }
            }
            ScriptCommandEffect::DressChar { handle } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.physical.entity.flags.is_visible = true;
                } else {
                    leftover.push(ScriptCommandEffect::DressChar { handle });
                }
            }
            ScriptCommandEffect::RemoveCharElegantly { handle } => {
                let Some(ped) = script_entities.peds.get(&script_entities.index, &handle) else {
                    // re3 accepts a null pool lookup here without side effects.
                    continue;
                };
                if ped.ped.char_created_by != MISSION_CHAR {
                    continue;
                }
                if script_entities.relation_for_char(handle).is_some() {
                    super::ScriptActorDeletion::delete_script_ped(
                        &mut script_entities, &mut pools, handle, car_ctrl.as_deref_mut(),
                    );
                } else if let Some(ped) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    // re3 releases an on-foot mission ped back to population and lets
                    // its normal fade-out lifecycle remove it.
                    ped.ped.char_created_by = RANDOM_CHAR;
                    ped.ped.flags.responds_to_threats = true;
                    ped.ped.flags.script_objective_completed = false;
                    ped.ped.leader = None;
                    ped.ped.flags.fade_out = true;
                }
            }
            ScriptCommandEffect::CarGotoCoordinates {
                handle,
                mut position,
                needs_ground,
                accurate,
                anti_reverse_timer_ms,
            } => {
                let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) else {
                    leftover.push(ScriptCommandEffect::CarGotoCoordinates {
                        handle,
                        position,
                        needs_ground,
                        accurate,
                        anti_reverse_timer_ms,
                    });
                    continue;
                };
                let mut base = vehicle.base_mut();
                if needs_ground {
                    position.z = file_loader.world.find_ground_z_for_coord(
                        position.x,
                        position.y,
                        &file_loader.model_info,
                    );
                }
                let Some(col_model) = file_loader
                    .model_info
                    .col_model(base.physical.entity.model_index.max(0) as usize)
                else {
                    leftover.push(ScriptCommandEffect::CarGotoCoordinates {
                        handle,
                        position,
                        needs_ground: false,
                        accurate,
                        anti_reverse_timer_ms,
                    });
                    continue;
                };
                position.z += -col_model.bounding_box.min.z;
                let goes_straight = join_car_with_road_system_goto_coors(
                    &mut file_loader.path_find,
                    &mut base,
                    position,
                    &mut general,
                );
                base.auto_pilot.car_mission = match (accurate, goes_straight) {
                    (true, true) => eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE,
                    (true, false) => eCarMission::MISSION_GOTOCOORDS_ACCURATE,
                    (false, true) => eCarMission::MISSION_GOTOCOORDS_STRAIGHT,
                    (false, false) => eCarMission::MISSION_GOTOCOORDS,
                };
                base.set_status(eEntityStatus::STATUS_PHYSICS);
                base.drive.engine_on = true;
                base.auto_pilot.cruise_speed = base.auto_pilot.cruise_speed.max(6);
                base.auto_pilot.anti_reverse_timer = anti_reverse_timer_ms;
            }
            ScriptCommandEffect::ApplyBrakesToPlayersCar {
                player_index,
                enabled,
            } => {
                pads.get_pad_mut(player_index).apply_brakes = enabled;
            }
            ScriptCommandEffect::TurnCarToFaceCoord { handle, x, y } => {
                if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let entity = &mut vehicle.base_mut().physical.entity;
                    let position = entity.get_position();
                    let mut heading = CGeneral::get_atan_of_xy(position.x - x, position.y - y)
                        + std::f32::consts::FRAC_PI_2;
                    if heading > std::f32::consts::TAU {
                        heading -= std::f32::consts::TAU;
                    }
                    entity.set_heading(heading);
                } else {
                    leftover.push(ScriptCommandEffect::TurnCarToFaceCoord { handle, x, y });
                }
            }
            ScriptCommandEffect::CreateObject {
                handle,
                model_id,
                position,
                add_model_offset,
            } => {
                let model_base_offset = if add_model_offset {
                    file_loader
                        .model_info
                        .get_model_info(model_id.max(0) as usize)
                        .and_then(|model| model.base().col_model.as_ref())
                        .map(|col| -col.bounding_box.min.z)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                if let Some(object) = script_entities.build_script_object(
                    &mut pools,
                    Some(handle),
                    model_id,
                    position,
                    model_base_offset,
                    &mut general,
                    &file_loader,
                ) {
                    script_entities.object_spawn(object);
                }
            }
            ScriptCommandEffect::SetObjectCoordinates { handle, position } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    entity.object.teleport(position);
                } else {
                    leftover.push(ScriptCommandEffect::SetObjectCoordinates { handle, position });
                }
            }
            ScriptCommandEffect::PlaceObjectRelativeToCar {
                object_handle,
                vehicle_handle,
                offset,
                time_step,
            } => {
                let Some(vehicle_physical) = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                    .map(|vehicle| vehicle.base().physical.clone())
                else {
                    leftover.push(ScriptCommandEffect::PlaceObjectRelativeToCar {
                        object_handle,
                        vehicle_handle,
                        offset,
                        time_step,
                    });
                    continue;
                };
                let Some(object) = script_entities.object_get_mut(object_handle) else {
                    leftover.push(ScriptCommandEffect::PlaceObjectRelativeToCar {
                        object_handle,
                        vehicle_handle,
                        offset,
                        time_step,
                    });
                    continue;
                };
                crate::entities::Physical::CPhysical::place_physical_relative_to_other_physical(
                    &vehicle_physical,
                    &mut object.object.physical,
                    offset,
                    time_step,
                );
            }
            ScriptCommandEffect::SetObjectVelocity {
                handle,
                velocity,
                add,
            } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    if add {
                        entity.object.physical.move_speed.x += velocity.x;
                        entity.object.physical.move_speed.y += velocity.y;
                        entity.object.physical.move_speed.z += velocity.z;
                    } else {
                        entity.object.physical.move_speed = velocity;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetObjectVelocity {
                        handle,
                        velocity,
                        add,
                    });
                }
            }
            ScriptCommandEffect::SetObjectCollision { handle, enabled } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    entity.object.physical.entity.flags.uses_collision = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetObjectCollision { handle, enabled });
                }
            }
            ScriptCommandEffect::SetObjectDynamic { handle, dynamic } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    entity.object.physical.entity.set_is_static(!dynamic);
                } else {
                    leftover.push(ScriptCommandEffect::SetObjectDynamic { handle, dynamic });
                }
            }
            ScriptCommandEffect::SetObjectDrawLast { handle, draw_last } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    entity.object.physical.entity.flags.draw_last = draw_last;
                } else {
                    leftover.push(ScriptCommandEffect::SetObjectDrawLast { handle, draw_last });
                }
            }
            ScriptCommandEffect::MakeObjectTargettable { handle } => {
                if script_entities.object_get(handle).is_none() {
                    leftover.push(ScriptCommandEffect::MakeObjectTargettable { handle });
                    continue;
                }
                if let Some(mut player) = world_players.focused_mut()
                    && let Some(slot) = player.targettable_objects.iter_mut().find(|target| {
                        **target == -1 || script_entities.object_get(**target).is_none()
                    })
                {
                    *slot = handle;
                }
            }
            ScriptCommandEffect::SetObjectHeading {
                handle,
                heading_degrees,
            } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    entity
                        .object
                        .physical
                        .entity
                        .set_heading(heading_degrees.to_radians());
                    entity.object.object_matrix = entity.object.physical.entity.matrix;
                } else {
                    leftover.push(ScriptCommandEffect::SetObjectHeading {
                        handle,
                        heading_degrees,
                    });
                }
            }
            ScriptCommandEffect::SetObjectRotation { handle, rotation } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    entity.object.physical.entity.matrix.set_rotate(
                        rotation.x.to_radians(),
                        rotation.y.to_radians(),
                        rotation.z.to_radians(),
                    );
                    entity.object.physical.entity.matrix.position =
                        entity.object.physical.entity.get_position();
                    entity.object.object_matrix = entity.object.physical.entity.matrix;
                } else {
                    leftover.push(ScriptCommandEffect::SetObjectRotation { handle, rotation });
                }
            }
            ScriptCommandEffect::SetCharWaitState {
                handle,
                wait_state,
                time_ms,
                now_ms,
            } => {
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    apply_ped_wait_state(&mut entity.ped, entity.locomotion, wait_state, time_ms, now_ms);
                } else {
                    leftover.push(ScriptCommandEffect::SetCharWaitState {
                        handle,
                        wait_state,
                        time_ms,
                        now_ms,
                    });
                }
            }
            ScriptCommandEffect::SetCharObjective {
                handle,
                objective,
                target_char,
                target_car,
                position,
                radius,
                route,
            } => {
                if !apply_script_char_objective(
                    &mut script_entities,
                    &mut world_players,
                    handle,
                    objective,
                    target_char,
                    target_car,
                    position,
                    radius,
                    route,
                    &ped_routes,
                    timer.time_in_milliseconds(),
                ) {
                    leftover.push(ScriptCommandEffect::SetCharObjective {
                        handle,
                        objective,
                        target_char,
                        target_car,
                        position,
                        radius,
                        route,
                    });
                }
            }
            ScriptCommandEffect::CharSetIdle { handle } => {
                // re3 Script.cpp COMMAND_CHAR_SET_IDLE:
                //   ped->bScriptObjectiveCompleted = false;
                //   ped->SetObjective(OBJECTIVE_WAIT_ON_FOOT);
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.flags.script_objective_completed = false;
                    entity.ped.set_objective(&entity.vitality, 
                        eObjective::OBJECTIVE_WAIT_ON_FOOT,
                        timer.time_in_milliseconds(),
                    );
                } else {
                    leftover.push(ScriptCommandEffect::CharSetIdle { handle });
                }
            }
            ScriptCommandEffect::CharWanderDir { handle, path } => {
                // re3 Script.cpp COMMAND_CHAR_WANDER_DIR:
                //   ped->ClearAll(); ped->SetWanderPath(path);
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    apply_script_ped_wander_path(
                        &mut entity.ped,
                        &entity.vitality,
                        path,
                        &file_loader,
                        &mut general,
                        timer.time_in_milliseconds(),
                    );
                } else {
                    leftover.push(ScriptCommandEffect::CharWanderDir { handle, path });
                }
            }
            ScriptCommandEffect::CarWanderRandomly { handle } => {
                // re3 Script.cpp COMMAND_CAR_WANDER_RANDOMLY:
                //   CCarCtrl::JoinCarWithRoadSystem(car);
                //   m_nCarMission = MISSION_CRUISE; bEngineOn = true;
                //   cruise speed = Max(6, ...); anti-reverse timer = now.
                if let Some(mut vehicle_entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = vehicle_entity.base_mut();
                    join_car_with_road_system(&mut file_loader.path_find, &mut base, &mut general);
                    base.auto_pilot.car_mission = eCarMission::MISSION_CRUISE;
                    base.drive.engine_on = true;
                    base.auto_pilot.cruise_speed = base.auto_pilot.cruise_speed.max(6);
                    base.auto_pilot.anti_reverse_timer = timer.time_in_milliseconds();
                } else {
                    leftover.push(ScriptCommandEffect::CarWanderRandomly { handle });
                }
            }
            ScriptCommandEffect::ActivateSaveMenu => {
                // re3 Script4.cpp COMMAND_ACTIVATE_SAVE_MENU:
                //   FrontEndMenuManager.m_bSaveMenuActive = true;
                if let Some(menu) = menu.as_mut() {
                    menu.save_menu_active = true;
                } else {
                    leftover.push(ScriptCommandEffect::ActivateSaveMenu);
                }
            }
            ScriptCommandEffect::SetAllOccupantsOfCarLeaveCar { handle } => {
                if !set_all_occupants_of_car_leave_car(
                    &mut script_entities,
                    &mut world_players,
                    handle,
                    timer.time_in_milliseconds(),
                ) {
                    leftover.push(ScriptCommandEffect::SetAllOccupantsOfCarLeaveCar { handle });
                }
            }
            ScriptCommandEffect::AddRoutePoint { route, position } => {
                let _ = ped_routes.add_route_point(route as i16, position);
            }
            ScriptCommandEffect::RemoveRoute { route } => {
                ped_routes.remove_route(route as i16);
            }
            ScriptCommandEffect::ClearCharThreatSearch { handle } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.threat_entity = None;
                    entity.ped.fear_flags = 0;
                } else {
                    leftover.push(ScriptCommandEffect::ClearCharThreatSearch { handle });
                }
            }
            ScriptCommandEffect::SetCharThreatSearch { handle, threats } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    // re3: CPed::m_fearFlags |= ScriptParams[1].
                    entity.ped.fear_flags |= threats;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharThreatSearch { handle, threats });
                }
            }
            ScriptCommandEffect::SetCharCantBeDraggedOut { handle, enabled } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.flags.dont_drag_me_out_car = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharCantBeDraggedOut { handle, enabled });
                }
            }
            ScriptCommandEffect::SetTaxiLights { handle, enabled } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    car.taxi_light = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetTaxiLights { handle, enabled });
                }
            }
            ScriptCommandEffect::SetAllTaxiLights(enabled) => {
                taxi_lights.all_enabled = enabled;
            }
            ScriptCommandEffect::FlashCar { handle, enabled } => {
                if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    vehicle.base_mut().physical.entity.flags.has_blip = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::FlashCar { handle, enabled });
                }
            }
            ScriptCommandEffect::FlashChar { handle, enabled } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.physical.entity.flags.has_blip = enabled;
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    player.ped.physical.entity.flags.has_blip = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::FlashChar { handle, enabled });
                }
            }
            ScriptCommandEffect::FlashObject { handle, enabled } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    entity.object.physical.entity.flags.has_blip = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::FlashObject { handle, enabled });
                }
            }
            ScriptCommandEffect::ArmCarWithBomb { handle, bomb_type } => {
                let player_slot = world_players
                    .focused()
                    .map(|player| CPoolHandle(player.pool_handle).index());
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    car.bomb_type = script_bomb_type(bomb_type);
                    car.bomb_rigger = player_slot;
                } else {
                    leftover.push(ScriptCommandEffect::ArmCarWithBomb { handle, bomb_type });
                }
            }
            ScriptCommandEffect::ExplodeCar { handle } => {
                let Some(vehicle_entity) = script_entities.vehicle_entity(handle) else {
                    leftover.push(ScriptCommandEffect::ExplodeCar { handle });
                    continue;
                };
                let occupant_handles = script_entities
                    .script_occupant_handles(vehicle_entity)
                    .map(|(ped_handle, _)| ped_handle)
                    .collect::<Vec<_>>();
                let occupant_players = (0..world_players.slot_count())
                    .filter(|index| {
                        world_players
                            .relation(*index as i32)
                            .is_some_and(|relation| relation.vehicle == vehicle_entity)
                    })
                    .collect::<Vec<_>>();
                let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) else {
                    leftover.push(ScriptCommandEffect::ExplodeCar { handle });
                    continue;
                };
                if !vehicle.base().vitality.can_be_damaged {
                    continue;
                }
                let position = vehicle.base().physical.entity.get_position();
                match &mut vehicle {
                    crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } => {
                        car.damage.fuck_car_completely();
                        car.bomb_type = crate::vehicles::Automobile::eBombType::CARBOMB_NONE;
                        car.taxi_light = false;
                    }
                    crate::control::ScriptEntities::ScriptVehicleMut::Boat { .. }
                    | crate::control::ScriptEntities::ScriptVehicleMut::Heli { .. }
                    | crate::control::ScriptEntities::ScriptVehicleMut::Plane { .. } => {}
                }
                let mut base = vehicle.base_mut();
                base.physical.move_speed.z += 0.13;
                base.set_status(eEntityStatus::STATUS_WRECKED);
                base.vitality.time_of_death = timer.time_in_milliseconds();
                base.vitality.health = 0.0;
                base.vitality.bomb_timer = 0;
                base.drive.engine_on = false;
                base.appearance.lights_on = false;
                base.audio.siren_or_alarm = false;
                base.flags.is_law_enforcer = false;

                for ped_handle in occupant_handles {
                    if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &ped_handle) {
                        if entity.ped.ped_state == PedState::PED_DRIVING {
                            entity.ped.set_dead(entity.vitality, 
                                ped_handle,
                                timer.time_in_milliseconds(),
                                &mut events,
                            );
                        } else {
                            entity.ped.set_die(entity.vitality, Some(crate::animation::AnimationId::ANIM_STD_KO_FRONT), 4.0, 0.0, timer.time_in_milliseconds());
                        }
                    }
                }
                for player_index in occupant_players {
                    if let Some(player) = world_players.player_mut(player_index as i32) {
                        if player.ped.ped_state == PedState::PED_DRIVING {
                            player.ped.set_dead(player.vitality, 
                                player.pool_handle,
                                timer.time_in_milliseconds(),
                                &mut events,
                            );
                        } else {
                            if player.can_be_damaged { player.ped.set_die(player.vitality, Some(crate::animation::AnimationId::ANIM_STD_KO_FRONT), 4.0, 0.0, timer.time_in_milliseconds()); }
                        }
                    }
                }
                if let Some(explosions) = explosions.as_mut() {
                    explosions.AddExplosion(
                        Some(crate::entities::EntityReference::EntityReference::Vehicle(handle)),
                        None,
                        eExplosionType::EXPLOSION_CAR,
                        position,
                        0,
                        timer.time_in_milliseconds(),
                    );
                }
            }
            ScriptCommandEffect::CreateSingleParticle {
                particle_type,
                position,
                direction,
                size,
            } => {
                if let Some(particles) = particles.as_mut() {
                    particles.emitter_with_context(particle_systems.as_deref(),
                        camera.as_ref().map(|camera| camera.position)).add_particle(
                        particle_type,
                        position,
                        direction,
                        None,
                        size,
                        0,
                        0,
                        0,
                        0,
                    );
                }
            }
            ScriptCommandEffect::AddExplosion {
                position,
                explosion_type,
            } => {
                if let (Some(explosions), Some(explosion_type)) =
                    (explosions.as_mut(), script_explosion_type(explosion_type))
                {
                    explosions.AddExplosion(
                        None,
                        None,
                        explosion_type,
                        position,
                        0,
                        timer.time_in_milliseconds(),
                    );
                }
            }
            ScriptCommandEffect::SetCharPersonality {
                handle,
                personality,
            } => {
                if let (Some(entity), Some(&ped_stats)) = (
                    script_entities.peds.get_mut(&script_entities.index, &handle),
                    usize::try_from(personality)
                        .ok()
                        .and_then(|index| PED_STATS_TYPES.get(index)),
                ) {
                    entity.ped.ped_stats = ped_stats;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharPersonality {
                        handle,
                        personality,
                    });
                }
            }
            ScriptCommandEffect::MarkCharAsNoLongerNeeded { handle } => {
                let relation = script_entities.relation_for_char(handle);
                let vehicle_handle = relation.and_then(|relation| {
                    script_entities.vehicle_handle_for_entity(relation.vehicle)
                });
                let vehicle_slot = vehicle_handle.map(|handle| CPoolHandle(handle).index());
                let is_vehicle_driver =
                    relation.is_some_and(|relation| relation.seat == VehicleSeat::Driver);

                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    let ped = &mut entity.ped;
                    let vitality = &mut *entity.vitality;
                    if ped.char_created_by != MISSION_CHAR {
                        continue;
                    }
                    ped.char_created_by = RANDOM_CHAR;
                    if ped.ped_type == ePedType::PEDTYPE_PROSTITUTE {
                        ped.objective_timer = timer.time_in_milliseconds().saturating_add(30_000);
                    }
                    let flee_state = matches!(
                        ped.ped_state,
                        PedState::PED_FLEE_ENTITY | PedState::PED_FLEE_POS
                    )
                    .then_some((ped.ped_state, ped.move_state));
                    ped.clear_objective(vitality, is_vehicle_driver, vehicle_slot);
                    ped.flags.responds_to_threats = true;
                    ped.flags.script_objective_completed = false;
                    ped.leader = None;
                    if ped.is_ped_in_control(vitality, ) {
                        let direction = i32::from(general.get_random_number() & 7);
                        apply_script_ped_wander_path(
                            ped, vitality,
                            direction,
                            &file_loader,
                            &mut general,
                            timer.time_in_milliseconds(),
                        );
                    }
                    if let Some((state, move_state)) = flee_state {
                        ped.ped_state = state;
                        ped.move_state = move_state;
                    }
                }
                if is_vehicle_driver {
                    if let Some(vehicle_handle) = vehicle_handle
                        && let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                            script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
                    {
                        join_car_with_road_system(
                            &file_loader.path_find, &mut car.vehicle, &mut general,
                        );
                        car.vehicle.core.auto_pilot.car_mission = eCarMission::MISSION_CRUISE;
                    }
                }
            }
            ScriptCommandEffect::MarkCarAsNoLongerNeeded { handle } => {
                let vehicle_entity = script_entities.vehicle_entity(handle);
                let mut released = false;
                if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = vehicle.base_mut();
                    if base.vehicle_created_by != MISSION_VEHICLE {
                        continue;
                    }
                    base.flags.is_locked = false;
                    base.vehicle_created_by = RANDOM_VEHICLE;
                    if let Some(car_ctrl) = car_ctrl.as_mut() {
                        car_ctrl.num_random_cars = car_ctrl.num_random_cars.saturating_add(1);
                        car_ctrl.num_mission_cars = car_ctrl.num_mission_cars.saturating_sub(1);
                    }
                    released = true;
                }
                if released && let Some(vehicle_entity) = vehicle_entity {
                    // CTheScripts::CleanUpThisVehicle ->
                    // CCarCtrl::RemoveFromInterestingVehicleList.
                    script_entities
                        .commands
                        .entity(vehicle_entity)
                        .remove::<VehicleOfInterest>();
                }
            }
            ScriptCommandEffect::MarkObjectAsNoLongerNeeded { handle } => {
                if let Some(entity) = script_entities.object_get_mut(handle) {
                    if entity.object.object_created_by != MISSION_OBJECT {
                        continue;
                    }
                    entity.object.object_created_by = TEMP_OBJECT;
                    entity.object.end_of_life_time =
                        timer.time_in_milliseconds().saturating_add(20_000);
                    entity.object.ref_model_index = -1;
                    entity.object.flags.use_vehicle_colours = false;
                }
            }
            ScriptCommandEffect::ExplodeCharHead { handle, nasty_game } => {
                let mut apply = |ped: &mut CPed, vitality: &mut crate::peds::PedVitality::PedVitality| {
                    if ped.ped_state == PedState::PED_DRIVING {
                        ped.set_dead(vitality, handle, timer.time_in_milliseconds(), &mut events);
                        if !ped.is_player() {
                            ped.physical.entity.flags.remove_from_world = true;
                        }
                    } else if nasty_game && ped.is_ped_in_control(vitality, ) {
                        stats.heads_popped = stats.heads_popped.saturating_add(1);
                        ped.set_die(vitality, Some(crate::animation::AnimationId::ANIM_STD_KO_FRONT), 4.0, 0.0, timer.time_in_milliseconds());
                        ped.flags.body_part_just_came_off = true;
                        ped.ped_state_timer = timer.time_in_milliseconds().saturating_add(150);
                    } else {
                        ped.set_die(vitality, Some(crate::animation::AnimationId::ANIM_STD_KO_FRONT), 4.0, 0.0, timer.time_in_milliseconds());
                    }
                };
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    apply(&mut entity.ped, entity.vitality);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    apply(player.ped, player.vitality);
                } else {
                    leftover.push(ScriptCommandEffect::ExplodeCharHead { handle, nasty_game });
                }
            }
            ScriptCommandEffect::ClearAreaOfChars { min, max } => {
                let removable: Vec<i32> = script_entities
                    .peds
                    .iter()
                    .filter_map(|(&handle, entity)| {
                        let ped = &entity.ped;
                        let position = ped.physical.entity.get_position();
                        (!ped.is_player()
                            && script_entities.relation_for_char(handle).is_none()
                            && ped.char_created_by != MISSION_CHAR
                            && position.x >= min.x
                            && position.x <= max.x
                            && position.y >= min.y
                            && position.y <= max.y
                            && position.z >= min.z
                            && position.z <= max.z)
                            .then_some(handle)
                    })
                    .collect();
                for handle in removable {
                    if let Some(ped_type) = script_entities.peds.get(&script_entities.index, &handle)
                        .map(|entity| entity.ped.ped_type)
                    {
                        if let Some(population) = population.as_mut() {
                            remove_ped_from_population_counts(population, ped_type);
                        }
                    }
                    script_entities.remove_char(&mut pools, handle);
                }
            }
            ScriptCommandEffect::ClearAreaOfCars { min, max } => {
                let removable: Vec<i32> = script_entities
                    .vehicles
                    .iter()
                    .filter_map(|(&handle, vehicle)| {
                        let base = vehicle.base();
                        let position = base.physical.entity.get_position();
                        (position.x >= min.x
                            && position.x <= max.x
                            && position.y >= min.y
                            && position.y <= max.y
                            && position.z >= min.z
                            && position.z <= max.z
                            && !base.flags.is_locked
                            && script_vehicle_base_can_be_deleted(&script_entities, handle, &base))
                        .then_some(handle)
                    })
                    .collect();
                for handle in removable {
                    remove_script_vehicle_and_occupants(
                        &mut script_entities,
                        &mut pools,
                        handle,
                        population.as_deref_mut(),
                        car_ctrl.as_deref_mut(),
                    );
                }
            }
            ScriptCommandEffect::ClearArea {
                position,
                radius,
                remove_projectiles_and_tidy_shadows,
            } => {
                clear_exciting_stuff_from_area_runtime(
                    &mut script_entities,
                    &mut pools,
                    population.as_deref_mut(),
                    car_ctrl.as_deref_mut(),
                    fire_manager.as_deref_mut(),
                    explosions.as_deref_mut(),
                    projectiles.as_deref_mut(),
                    shadows.as_deref_mut(),
                    position,
                    radius,
                    remove_projectiles_and_tidy_shadows,
                );
            }
            ScriptCommandEffect::SetCharOnlyDamagedByPlayer { handle, enabled } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.physical.entity.flags.only_damaged_by_player = enabled;
                } else {
                    leftover
                        .push(ScriptCommandEffect::SetCharOnlyDamagedByPlayer { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCharProofs {
                handle,
                bullet,
                fire,
                explosion,
                collision,
                melee,
            } => {
                let flags = if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    Some(&mut entity.ped.physical.entity.flags)
                } else {
                    world_players
                        .iter_mut()
                        .find(|player| player.pool_handle == handle)
                        .map(|player| &mut player.ped.physical.entity.flags)
                };
                if let Some(flags) = flags {
                    flags.bullet_proof = bullet;
                    flags.fire_proof = fire;
                    flags.explosion_proof = explosion;
                    flags.collision_proof = collision;
                    flags.melee_proof = melee;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharProofs {
                        handle,
                        bullet,
                        fire,
                        explosion,
                        collision,
                        melee,
                    });
                }
            }
            ScriptCommandEffect::SetCharUsePedNodeSeek { handle, enabled } => {
                let ped = if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    Some(entity.ped)
                } else {
                    world_players
                        .iter_mut()
                        .find(|player| player.pool_handle == handle)
                        .map(|player| player.ped)
                };
                if let Some(ped) = ped {
                    if enabled {
                        ped.next_path_node = None;
                    }
                    ped.flags.use_ped_node_seek = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharUsePedNodeSeek { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCharStaysInCurrentLevel { handle, enabled } => {
                let ped = if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    Some(entity.ped)
                } else {
                    world_players
                        .iter_mut()
                        .find(|player| player.pool_handle == handle)
                        .map(|player| player.ped)
                };
                if let Some(ped) = ped {
                    let position = ped.physical.entity.get_position();
                    ped.physical.zone_level = if enabled {
                        file_loader
                            .world
                            .zones
                            .get_level_from_position([position.x, position.y, position.z])
                            as i8
                    } else {
                        crate::collision::ColModel::LEVEL_GENERIC as i8
                    };
                } else {
                    leftover
                        .push(ScriptCommandEffect::SetCharStaysInCurrentLevel { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCharIgnoreLevelTransitions { handle, enabled } => {
                let ped = if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    Some(entity.ped)
                } else {
                    world_players
                        .iter_mut()
                        .find(|player| player.pool_handle == handle)
                        .map(|player| player.ped)
                };
                if let Some(ped) = ped {
                    let position = ped.physical.entity.get_position();
                    ped.physical.zone_level = if enabled {
                        ELevelName::Ignore as i8
                    } else {
                        file_loader
                            .world
                            .zones
                            .get_level_from_position([position.x, position.y, position.z])
                            as i8
                    };
                } else {
                    leftover.push(ScriptCommandEffect::SetCharIgnoreLevelTransitions {
                        handle,
                        enabled,
                    });
                }
            }
            ScriptCommandEffect::SetCharsChatting {
                first,
                second,
                duration_ms,
            } => {
                let first_exists = script_entities.peds.contains_key(&script_entities.index, &first)
                    || world_players
                        .iter()
                        .any(|player| player.pool_handle == first);
                let second_exists = script_entities.peds.contains_key(&script_entities.index, &second)
                    || world_players
                        .iter()
                        .any(|player| player.pool_handle == second);
                if !first_exists || !second_exists {
                    leftover.push(ScriptCommandEffect::SetCharsChatting {
                        first,
                        second,
                        duration_ms,
                    });
                    continue;
                }
                let now_ms = timer.time_in_milliseconds();
                let second_slot = CPoolHandle(second).index();
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &first) {
                    set_script_ped_chat(&mut entity.ped, second_slot, duration_ms, now_ms);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == first)
                {
                    set_script_ped_chat(player.ped, second_slot, duration_ms, now_ms);
                }
                let first_slot = CPoolHandle(first).index();
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &second) {
                    set_script_ped_chat(&mut entity.ped, first_slot, duration_ms, now_ms);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == second)
                {
                    set_script_ped_chat(player.ped, first_slot, duration_ms, now_ms);
                }
            }
            ScriptCommandEffect::SetZonePedDensity {
                label,
                day_density,
                night_density,
            } => {
                let _ = file_loader.world.zones.set_zone_ped_info(
                    &label,
                    true,
                    day_density,
                    [-1; 9],
                    -1,
                );
                let _ = file_loader.world.zones.set_zone_ped_info(
                    &label,
                    false,
                    night_density,
                    [-1; 9],
                    -1,
                );
            }
            ScriptCommandEffect::SetCharBleeding { handle, enabled } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.flags.ped_is_bleeding = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharBleeding { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCharRunning { handle, enabled } => {
                if !set_script_char_running(&mut script_entities, &mut world_players, handle, enabled) {
                    leftover.push(ScriptCommandEffect::SetCharRunning { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCharLookAtChar { source, target } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &source) {
                    entity.ped.look_target = Some(target.max(0) as usize);
                    entity.ped.flags.is_looking = true;
                    entity.ped.flags.keep_trying_to_look = true;
                    entity.ped.look_timer = timer.time_in_milliseconds().saturating_add(60_000);
                } else {
                    leftover.push(ScriptCommandEffect::SetCharLookAtChar { source, target });
                }
            }
            ScriptCommandEffect::SetCharLookAtPlayer {
                source,
                player_index,
            } => {
                let target = world_players
                    .player(player_index)
                    .map(|player| player.pool_handle);
                if let (Some(entity), Some(target)) =
                    (script_entities.peds.get_mut(&script_entities.index, &source), target)
                {
                    entity.ped.look_target = Some(target.max(0) as usize);
                    entity.ped.flags.is_looking = true;
                    entity.ped.flags.keep_trying_to_look = true;
                    entity.ped.look_timer = timer.time_in_milliseconds().saturating_add(60_000);
                } else {
                    leftover.push(ScriptCommandEffect::SetCharLookAtPlayer {
                        source,
                        player_index,
                    });
                }
            }
            ScriptCommandEffect::SetPlayerLookAtChar {
                player_index,
                target,
            } => {
                if let Some(player) = world_players.player_mut(player_index) {
                    player.ped.look_target = Some(target.max(0) as usize);
                    player.ped.flags.is_looking = true;
                    player.ped.flags.keep_trying_to_look = true;
                    player.ped.look_timer = timer.time_in_milliseconds().saturating_add(60_000);
                } else {
                    leftover.push(ScriptCommandEffect::SetPlayerLookAtChar {
                        player_index,
                        target,
                    });
                }
            }
            ScriptCommandEffect::StopCharLooking(handle) => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.look_target = None;
                    entity.ped.flags.is_looking = false;
                    entity.ped.flags.keep_trying_to_look = false;
                } else {
                    leftover.push(ScriptCommandEffect::StopCharLooking(handle));
                }
            }
            ScriptCommandEffect::StopPlayerLooking(player_index) => {
                if let Some(player) = world_players.player_mut(player_index) {
                    player.ped.look_target = None;
                    player.ped.flags.is_looking = false;
                    player.ped.flags.keep_trying_to_look = false;
                } else {
                    leftover.push(ScriptCommandEffect::StopPlayerLooking(player_index));
                }
            }
            ScriptCommandEffect::SetCharAnimGroup { handle, anim_group } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.anim_group = anim_group;
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    player.ped.anim_group = anim_group;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharAnimGroup { handle, anim_group });
                }
            }
            ScriptCommandEffect::SetPlayerAnimGroup {
                player_index,
                anim_group,
            } => {
                if let Some(player) = world_players.player_mut(player_index) {
                    player.ped.anim_group = anim_group;
                } else {
                    leftover.push(ScriptCommandEffect::SetPlayerAnimGroup {
                        player_index,
                        anim_group,
                    });
                }
            }
            ScriptCommandEffect::SetCharStayInSamePlace { handle, enabled } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.flags.kinda_stay_in_same_place = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharStayInSamePlace { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCharRespondsToThreats { handle, enabled } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.flags.responds_to_threats = enabled;
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    player.ped.flags.responds_to_threats = enabled;
                } else {
                    leftover
                        .push(ScriptCommandEffect::SetCharRespondsToThreats { handle, enabled });
                }
            }
            ScriptCommandEffect::SetPlayerRespondsToThreats {
                player_index,
                enabled,
            } => {
                if let Some(player) = world_players.player_mut(player_index) {
                    player.ped.flags.responds_to_threats = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetPlayerRespondsToThreats {
                        player_index,
                        enabled,
                    });
                }
            }
            ScriptCommandEffect::SetCharVisible { handle, visible } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.physical.entity.flags.is_visible = visible;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharVisible { handle, visible });
                }
            }
            ScriptCommandEffect::SetCharSay { handle, sound_id } => {
                let Some(sound) = script_char_say_sound(sound_id) else {
                    continue;
                };
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    queue_ped_script_say(&mut entity.ped, sound);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    queue_ped_script_say(player.ped, sound);
                } else {
                    leftover.push(ScriptCommandEffect::SetCharSay { handle, sound_id });
                }
            }
            ScriptCommandEffect::SetCharHealth { handle, health } => {
                let script_in_vehicle = script_entities.relation_for_char(handle).is_some();
                let player_in_vehicle = world_players
                    .vehicle_relation_by_pool_handle(handle)
                    .is_some();
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    if health != 0.0 {
                        entity.vitality.health = health;
                    } else if script_in_vehicle {
                        entity
                            .ped
                            .set_dead(entity.vitality, handle, timer.time_in_milliseconds(), &mut events);
                        entity.ped.physical.entity.flags.remove_from_world = true;
                    } else {
                        entity.ped.set_die(entity.vitality, Some(crate::animation::AnimationId::ANIM_STD_KO_FRONT), 4.0, 0.0, timer.time_in_milliseconds());
                    }
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    if health != 0.0 {
                        player.vitality.health = health;
                    } else if player_in_vehicle {
                        player
                            .ped
                            .set_dead(player.vitality, handle, timer.time_in_milliseconds(), &mut events);
                    } else {
                        if player.can_be_damaged { player.ped.set_die(player.vitality, Some(crate::animation::AnimationId::ANIM_STD_KO_FRONT), 4.0, 0.0, timer.time_in_milliseconds()); }
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetCharHealth { handle, health });
                }
            }
            ScriptCommandEffect::SetCharAccuracy { handle, accuracy } => {
                let accuracy = accuracy as u8;
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.ped.wep_accuracy = accuracy;
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    player.ped.wep_accuracy = accuracy;
                } else {
                    leftover.push(ScriptCommandEffect::SetCharAccuracy {
                        handle,
                        accuracy: i32::from(accuracy),
                    });
                }
            }
            ScriptCommandEffect::AddArmourToChar { handle, armour } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    entity.vitality.armour = (entity.vitality.armour + armour).min(100.0);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    player.vitality.armour = (player.vitality.armour + armour).min(100.0);
                } else {
                    leftover.push(ScriptCommandEffect::AddArmourToChar { handle, armour });
                }
            }
            ScriptCommandEffect::SetCharAmmo {
                handle,
                weapon_type,
                ammo,
            } => {
                let Some(info) = weapon_info.as_deref() else {
                    bridge.weapon_data_deferrals = bridge.weapon_data_deferrals.saturating_add(1);
                    leftover.push(ScriptCommandEffect::SetCharAmmo { handle, weapon_type, ammo });
                    continue;
                };
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    set_ped_ammo(&mut entity.ped, weapon_type, ammo, info);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    set_ped_ammo(player.ped, weapon_type, ammo, info);
                } else {
                    leftover.push(ScriptCommandEffect::SetCharAmmo {
                        handle,
                        weapon_type,
                        ammo,
                    });
                }
            }
            ScriptCommandEffect::SetCurrentCharWeapon {
                handle,
                weapon_type,
            } => {
                if let Some(type_) = script_weapon_type(weapon_type) {
                    if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                        if (type_ as usize) < entity.ped.weapons.len()
                            && entity.ped.weapons[type_ as usize].weapon_type == type_
                        {
                            entity.ped.current_weapon = type_;
                        }
                    } else if let Some(player) = world_players
                        .iter_mut()
                        .find(|player| player.pool_handle == handle)
                    {
                        if (type_ as usize) < player.ped.weapons.len()
                            && player.ped.weapons[type_ as usize].weapon_type == type_
                        {
                            player.ped.current_weapon = type_;
                        }
                    } else {
                        leftover.push(ScriptCommandEffect::SetCurrentCharWeapon {
                            handle,
                            weapon_type,
                        });
                    }
                }
            }
            ScriptCommandEffect::AddAmmoToChar {
                handle,
                weapon_type,
                ammo,
            } => {
                let Some(info) = weapon_info.as_deref() else {
                    bridge.weapon_data_deferrals = bridge.weapon_data_deferrals.saturating_add(1);
                    leftover.push(ScriptCommandEffect::AddAmmoToChar { handle, weapon_type, ammo });
                    continue;
                };
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    add_ped_ammo(&mut entity.ped, weapon_type, ammo, info);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    add_ped_ammo(player.ped, weapon_type, ammo, info);
                } else {
                    leftover.push(ScriptCommandEffect::AddAmmoToChar {
                        handle,
                        weapon_type,
                        ammo,
                    });
                }
            }
            ScriptCommandEffect::GiveWeaponToChar {
                handle,
                weapon_type,
                ammo,
            } => {
                let Some(info) = weapon_info.as_deref() else {
                    bridge.weapon_data_deferrals = bridge.weapon_data_deferrals.saturating_add(1);
                    leftover.push(ScriptCommandEffect::GiveWeaponToChar { handle, weapon_type, ammo });
                    continue;
                };
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    if let Some(type_) = crate::peds::Ped::give_ped_weapon_with_info(&mut entity.ped, weapon_type, ammo, info) {
                        entity.ped.current_weapon = type_;
                    }
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    if let Some(type_) = crate::peds::Ped::give_ped_weapon_with_info(player.ped, weapon_type, ammo, info) {
                        player.ped.current_weapon = type_;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::GiveWeaponToChar {
                        handle,
                        weapon_type,
                        ammo,
                    });
                }
            }
            ScriptCommandEffect::SetCharCoordinates {
                handle,
                mut position,
                needs_ground,
            } => {
                if !script_entities.peds.contains_key(&script_entities.index, &handle) {
                    leftover.push(ScriptCommandEffect::SetCharCoordinates {
                        handle,
                        position,
                        needs_ground,
                    });
                    continue;
                }
                if needs_ground {
                    position.z = file_loader.world.find_ground_z_for_coord(
                        position.x,
                        position.y,
                        &file_loader.model_info,
                    );
                }
                let vehicle_handle =
                    script_entities
                        .relation_for_char(handle)
                        .and_then(|relation| {
                            script_entities.vehicle_handle_for_entity(relation.vehicle)
                        });
                if let Some(vehicle_handle) = vehicle_handle {
                    let Some(model_index) = script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
                        .map(|vehicle| vehicle.base().physical.entity.model_index.max(0) as usize)
                    else {
                        continue;
                    };
                    if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) {
                        let model_info = file_loader.model_info.get_model_info(model_index);
                        position.z += match &mut vehicle {
                            crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } => {
                                script_automobile_base_z(model_info, &car)
                            }
                            _ => script_vehicle_base_z(model_info),
                        };
                        vehicle.teleport(position);
                        vehicle.base_mut().physical.entity.flags.is_static = false;
                    }
                    let zone_level = mission_zone_level(&file_loader, position);
                    sync_mission_vehicle_zone_level(
                        &mut script_entities,
                        vehicle_handle,
                        zone_level,
                    );
                    // COMMAND_SET_CHAR_COORDINATES moves m_pMyVehicle when the
                    // character is seated; moving only the ped leaves the ECS
                    // seat relation attached to a car at the old location.
                    script_entities.clear_space_for_mission_entity_except(
                        &mut pools,
                        position,
                        model_index,
                        &file_loader,
                        Some(vehicle_handle),
                    );
                } else if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    position.z += crate::peds::PedPlacement::PED_FEET_OFFSET;
                    // CPed::Teleport keeps the current move speed but clears
                    // the standing/action state and state timer.
                    entity.ped.physical.entity.set_position(position);
                    entity.ped.flags.is_standing = false;
                    entity.ped.ped_state_timer = 0;
                    entity.ped.action_x = 0.0;
                    entity.ped.action_y = 0.0;
                    entity.ped.colliding_entity = None;
                    entity.ped.physical.zone_level = mission_zone_level(&file_loader, position);
                    let model_index = entity.ped.physical.entity.model_index.max(0) as usize;
                    script_entities.clear_space_for_mission_entity(
                        &mut pools,
                        position,
                        model_index,
                        &file_loader,
                    );
                }
            }
            ScriptCommandEffect::SetCarCoordinates {
                handle,
                mut position,
                needs_ground,
            } => {
                let Some(model_index) = script_entities.vehicles.get(&script_entities.index, &handle)
                    .map(|entity| entity.base().physical.entity.model_index.max(0) as usize)
                else {
                    leftover.push(ScriptCommandEffect::SetCarCoordinates {
                        handle,
                        position,
                        needs_ground,
                    });
                    continue;
                };
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    if needs_ground {
                        position.z = file_loader.world.find_ground_z_for_coord(
                            position.x,
                            position.y,
                            &file_loader.model_info,
                        );
                    }
                    let model_info = file_loader.model_info.get_model_info(model_index);
                    position.z += match &mut entity {
                        crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } => {
                            script_automobile_base_z(model_info, &car)
                        }
                        _ => script_vehicle_base_z(model_info),
                    };
                    entity.teleport(position);
                    entity.base_mut().physical.entity.flags.is_static = false;
                }
                let zone_level = mission_zone_level(&file_loader, position);
                sync_mission_vehicle_zone_level(&mut script_entities, handle, zone_level);
                // re3 COMMAND_SET_CAR_COORDINATES calls Teleport, clears the
                // destination around the mission vehicle, then rebuilds the
                // road route for its current mission. Leaving stale route-node
                // indices after a far teleport sends cars through unrelated
                // geometry and lets them fall through bridges.
                script_entities.clear_space_for_mission_entity_except(
                    &mut pools,
                    position,
                    model_index,
                    &file_loader,
                    Some(handle),
                );
                let target_car_position = script_entities.vehicles.get(&script_entities.index, &handle)
                    .and_then(|entity| entity.base().auto_pilot.target_car)
                    .and_then(|target_slot| {
                        script_entities.vehicles.values().find_map(|vehicle| {
                            (CPoolHandle(vehicle.handle()).index() == target_slot)
                                .then(|| vehicle.base().physical.entity.get_position())
                        })
                    });
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = entity.base_mut();
                    let mission = base.auto_pilot.car_mission;
                    match mission {
                        eCarMission::MISSION_CRUISE => {
                            join_car_with_road_system(&file_loader.path_find, &mut base, &mut general);
                        }
                        eCarMission::MISSION_RAMPLAYER_FARAWAY
                        | eCarMission::MISSION_RAMPLAYER_CLOSE
                        | eCarMission::MISSION_BLOCKPLAYER_FARAWAY
                        | eCarMission::MISSION_BLOCKPLAYER_CLOSE
                        | eCarMission::MISSION_BLOCKPLAYER_HANDBRAKESTOP => {
                            if let Some(player) = world_players.focused() {
                                let _ = join_car_with_road_system_goto_coors(
                                    &mut file_loader.path_find,
                                    &mut base,
                                    player.position(),
                                    &mut general,
                                );
                            }
                        }
                        eCarMission::MISSION_GOTOCOORDS
                        | eCarMission::MISSION_GOTOCOORDS_STRAIGHT
                        | eCarMission::MISSION_GOTOCOORDS_ACCURATE
                        | eCarMission::MISSION_GOTO_COORDS_STRAIGHT_ACCURATE => {
                            let target = base.auto_pilot.destination_coors;
                            let _ = join_car_with_road_system_goto_coors(
                                &mut file_loader.path_find,
                                &mut base,
                                target,
                                &mut general,
                            );
                        }
                        eCarMission::MISSION_RAMCAR_FARAWAY
                        | eCarMission::MISSION_RAMCAR_CLOSE
                        | eCarMission::MISSION_BLOCKCAR_FARAWAY
                        | eCarMission::MISSION_BLOCKCAR_CLOSE
                        | eCarMission::MISSION_BLOCKCAR_HANDBRAKESTOP => {
                            if let Some(target) = target_car_position {
                                let _ = join_car_with_road_system_goto_coors(
                                    &mut file_loader.path_find,
                                    &mut base,
                                    target,
                                    &mut general,
                                );
                            }
                        }
                        _ => {}
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetCarCoordinates {
                        handle,
                        position,
                        needs_ground,
                    });
                }
            }
            ScriptCommandEffect::SetPlayerCoordinates {
                index,
                mut position,
                needs_ground,
            } => {
                let vehicle_handle = world_players.relation(index).and_then(|relation| {
                    script_entities.vehicle_handle_for_entity(relation.vehicle)
                });
                let Some(vehicle_handle) = vehicle_handle else {
                    leftover.push(ScriptCommandEffect::SetPlayerCoordinates {
                        index,
                        position,
                        needs_ground,
                    });
                    continue;
                };
                if needs_ground {
                    position.z = file_loader.world.find_ground_z_for_coord(
                        position.x,
                        position.y,
                        &file_loader.model_info,
                    );
                }
                let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle) else {
                    leftover.push(ScriptCommandEffect::SetPlayerCoordinates {
                        index,
                        position,
                        needs_ground,
                    });
                    continue;
                };
                let model_index = vehicle.base().physical.entity.model_index.max(0) as usize;
                let model_info = file_loader.model_info.get_model_info(model_index);
                position.z += match &mut vehicle {
                    crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } => {
                        script_automobile_base_z(model_info, &car)
                    }
                    _ => script_vehicle_base_z(model_info),
                };
                vehicle.teleport(position);
                if let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = vehicle {
                    let col = file_loader.model_info.col_model(model_index);
                    let (bbox_min, bbox_max) = col
                        .map(|c| {
                            (
                                CVector::new(
                                    c.bounding_box.min.x,
                                    c.bounding_box.min.y,
                                    c.bounding_box.min.z,
                                ),
                                CVector::new(
                                    c.bounding_box.max.x,
                                    c.bounding_box.max.y,
                                    c.bounding_box.max.z,
                                ),
                            )
                        })
                        .unwrap_or((CVector::new(-1.0, -2.0, -0.8), CVector::new(1.0, 2.0, 0.8)));
                    car.place_on_road_properly(bbox_min, bbox_max, |probe, low_z| {
                        file_loader
                            .world
                            .process_vertical_line_buildings_only_with_entity(
                                [probe.x, probe.y, probe.z],
                                low_z,
                                &file_loader.model_info,
                            )
                            .map(|(point, entity)| (point.point.z, entity))
                    });
                }
                let zone_level = mission_zone_level(&file_loader, position);
                sync_mission_vehicle_zone_level(&mut script_entities, vehicle_handle, zone_level);
                if let Some(mut player) = world_players.player_mut(index) {
                    player.set_position(position);
                }
                bridge.player_position = position;
                bridge.player_spawned = true;
            }
            ScriptCommandEffect::CarSetIdle { handle } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().auto_pilot.car_mission =
                        crate::control::AutoPilot::eCarMission::MISSION_NONE;
                }
            }
            ScriptCommandEffect::SetCarHealth { handle, health } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().vitality.health = health;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarHealth { handle, health });
                }
            }
            ScriptCommandEffect::SetCarAmmo { handle, ammo } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().ammo_in_clip = ammo.clamp(0, 255) as u8;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarAmmo { handle, ammo });
                }
            }
            ScriptCommandEffect::AddAmmoToCar { handle, ammo } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = entity.base_mut();
                    base.ammo_in_clip = i32::from(base.ammo_in_clip)
                        .saturating_add(ammo)
                        .clamp(0, 255) as u8;
                } else {
                    leftover.push(ScriptCommandEffect::AddAmmoToCar { handle, ammo });
                }
            }
            ScriptCommandEffect::SetCarOnlyDamagedByPlayer { handle, enabled } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity
                        .base_mut()
                        .physical
                        .entity
                        .flags
                        .only_damaged_by_player = enabled;
                } else {
                    leftover
                        .push(ScriptCommandEffect::SetCarOnlyDamagedByPlayer { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarCanBeDamaged { handle, enabled } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().vitality.can_be_damaged = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarCanBeDamaged { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarLock { handle, lock } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().entry.door_lock = script_car_lock(lock);
                    entity.base_mut().flags.is_locked = lock != 1;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarLock { handle, lock });
                }
            }
            ScriptCommandEffect::SetCarColour {
                handle,
                colour1,
                colour2,
            } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().appearance.current_colour1 = colour1.clamp(0, u8::MAX as i32) as u8;
                    entity.base_mut().appearance.current_colour2 = colour2.clamp(0, u8::MAX as i32) as u8;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarColour {
                        handle,
                        colour1,
                        colour2,
                    });
                }
            }
            ScriptCommandEffect::SetCarHeading {
                handle,
                heading_degrees,
            } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity
                        .base_mut()
                        .physical
                        .entity
                        .set_heading(heading_degrees.to_radians());
                    if let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = entity {
                        let model_index = car.vehicle.core.physical.entity.model_index.max(0) as usize;
                        let col = file_loader.model_info.col_model(model_index);
                        let (bbox_min, bbox_max) = col
                            .map(|c| {
                                (
                                    CVector::new(
                                        c.bounding_box.min.x,
                                        c.bounding_box.min.y,
                                        c.bounding_box.min.z,
                                    ),
                                    CVector::new(
                                        c.bounding_box.max.x,
                                        c.bounding_box.max.y,
                                        c.bounding_box.max.z,
                                    ),
                                )
                            })
                            .unwrap_or((
                                CVector::new(-1.0, -2.0, -0.8),
                                CVector::new(1.0, 2.0, 0.8),
                            ));
                        car.place_on_road_properly(bbox_min, bbox_max, |probe, low_z| {
                            file_loader
                                .world
                                .process_vertical_line_buildings_only_with_entity(
                                    [probe.x, probe.y, probe.z],
                                    low_z,
                                    &file_loader.model_info,
                                )
                                .map(|(point, entity)| (point.point.z, entity))
                        });
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetCarHeading {
                        handle,
                        heading_degrees,
                    });
                }
            }
            ScriptCommandEffect::SetCharHeading {
                handle,
                heading_degrees,
            } => {
                if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    let rad = heading_degrees.to_radians();
                    entity.locomotion.rotation_cur = rad;
                    entity.locomotion.rotation_dest = rad;
                    entity.ped.physical.entity.set_heading(rad);
                } else {
                    leftover.push(ScriptCommandEffect::SetCharHeading {
                        handle,
                        heading_degrees,
                    });
                }
            }
            ScriptCommandEffect::SetCarVisible { handle, visible } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().physical.entity.flags.is_visible = visible;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarVisible { handle, visible });
                }
            }
            ScriptCommandEffect::SetCarStrong { handle, strong } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().vitality.take_less_damage = strong;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarStrong { handle, strong });
                }
            }
            ScriptCommandEffect::SetCarMoreResistantToDamage { handle, enabled } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    car.more_resistant_to_damage = enabled;
                } else if !script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    leftover
                        .push(ScriptCommandEffect::SetCarMoreResistantToDamage { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarStatus { handle, status } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    if let Some(status) = script_entity_status(status) {
                        entity.base_mut().set_status(status);
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetCarStatus { handle, status });
                }
            }
            ScriptCommandEffect::SetCarBigWheels { handle, enabled } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    car.big_wheels = enabled;
                } else if script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    // re3 asserts that the target is an automobile; retain the existing type.
                } else {
                    leftover.push(ScriptCommandEffect::SetCarBigWheels { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarCanRespray { handle, enabled } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    // re3 stores the inverse (`bFixedColour`).
                    car.fixed_colour = !enabled;
                } else if script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    // re3 asserts that the target is an automobile.
                } else {
                    leftover.push(ScriptCommandEffect::SetCarCanRespray { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarComedyControls { handle, enabled } => {
                if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    vehicle.base_mut().drive.comedy_controls = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarComedyControls { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarHeavy { handle, enabled } => {
                if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    vehicle.base_mut().physical.flags.is_heavy = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarHeavy { handle, enabled });
                }
            }
            ScriptCommandEffect::AnchorBoat { handle, anchored } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Boat { mut boat, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    boat.is_anchored = anchored;
                } else if script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    // re3 asserts that the target is a boat; retain the existing type.
                } else {
                    leftover.push(ScriptCommandEffect::AnchorBoat { handle, anchored });
                }
            }
            ScriptCommandEffect::SwitchCarSiren { handle, enabled } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().audio.siren_or_alarm = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SwitchCarSiren { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarProofs {
                handle,
                bullet,
                fire,
                explosion,
                collision,
                melee,
            } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let flags = &mut entity.base_mut().physical.entity.flags;
                    flags.bullet_proof = bullet;
                    flags.fire_proof = fire;
                    flags.explosion_proof = explosion;
                    flags.collision_proof = collision;
                    flags.melee_proof = melee;
                } else {
                    leftover.push(ScriptCommandEffect::SetCarProofs {
                        handle,
                        bullet,
                        fire,
                        explosion,
                        collision,
                        melee,
                    });
                }
            }
            ScriptCommandEffect::SetCarWaterTight { handle, enabled } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    car.water_tight = enabled;
                } else if !script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    leftover.push(ScriptCommandEffect::SetCarWaterTight { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarAvoidLevelTransitions { handle, enabled } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().auto_pilot.stay_in_current_level = enabled;
                } else {
                    leftover
                        .push(ScriptCommandEffect::SetCarAvoidLevelTransitions { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarStaysInCurrentLevel { handle, enabled } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = entity.base_mut();
                    let position = base.physical.entity.get_position();
                    base.physical.zone_level = if enabled {
                        file_loader
                            .world
                            .zones
                            .get_level_from_position([position.x, position.y, position.z])
                            as i8
                    } else {
                        crate::collision::ColModel::LEVEL_GENERIC as i8
                    };
                } else {
                    leftover
                        .push(ScriptCommandEffect::SetCarStaysInCurrentLevel { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarIgnoreLevelTransitions { handle, enabled } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = entity.base_mut();
                    let position = base.physical.entity.get_position();
                    base.physical.zone_level = if enabled {
                        ELevelName::Ignore as i8
                    } else {
                        file_loader
                            .world
                            .zones
                            .get_level_from_position([position.x, position.y, position.z])
                            as i8
                    };
                } else {
                    leftover.push(ScriptCommandEffect::SetCarIgnoreLevelTransitions {
                        handle,
                        enabled,
                    });
                }
            }
            ScriptCommandEffect::SetGenerateCarsAroundCamera(enabled) => {
                if let Some(car_ctrl) = car_ctrl.as_mut() {
                    car_ctrl.cars_generated_around_camera = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetGenerateCarsAroundCamera(enabled));
                }
            }
            ScriptCommandEffect::SetJamesCarOnPathToPlayer {
                handle,
                player_position,
            } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = entity.base_mut();
                    join_car_with_road_system_goto_coors(
                        &mut file_loader.path_find,
                        &mut base,
                        player_position,
                        &mut general,
                    );
                } else {
                    leftover.push(ScriptCommandEffect::SetJamesCarOnPathToPlayer {
                        handle,
                        player_position,
                    });
                }
            }
            ScriptCommandEffect::BlowUpRemoteCar => {
                remote.request_blow_up();
            }
            ScriptCommandEffect::SetAllCarsCanBeDamaged(enabled) => {
                for mut entity in script_entities.vehicles.values_mut() {
                    entity.base_mut().vitality.can_be_damaged = enabled;
                }
            }
            ScriptCommandEffect::SetUpsideDownCarNotDamaged { handle, enabled } => {
                if let Some(entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    if let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = entity {
                        car.not_damaged_upside_down = enabled;
                    }
                } else {
                    leftover
                        .push(ScriptCommandEffect::SetUpsideDownCarNotDamaged { handle, enabled });
                }
            }
            ScriptCommandEffect::SetCarCruiseSpeed { handle, speed } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().auto_pilot.cruise_speed = speed.max(0.0).min(255.0) as u8;
                }
            }
            ScriptCommandEffect::BoatGotoCoordinates {
                handle,
                mut position,
                needs_water_level,
                anti_reverse_timer_ms,
            } => {
                if needs_water_level {
                    if let Some(level) = water_level.as_ref().and_then(|water| {
                        water.get_water_level(
                            position.x,
                            position.y,
                            position.z,
                            false,
                            timer.time_in_milliseconds(),
                            weather.as_ref().map_or(0.0, |weather| weather.wind),
                        )
                    }) {
                        position.z = level.level;
                    }
                }
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Boat { mut boat, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    let base = &mut boat.vehicle;
                    base.auto_pilot.car_mission =
                        crate::control::AutoPilot::eCarMission::MISSION_GOTOCOORDS_ASTHECROWSWIMS;
                    base.auto_pilot.destination_coors = position;
                    base.auto_pilot.anti_reverse_timer = anti_reverse_timer_ms;
                    base.set_status(crate::entities::Entity::eEntityStatus::STATUS_PHYSICS);
                    base.drive.engine_on = true;
                    base.auto_pilot.cruise_speed = base.auto_pilot.cruise_speed.max(6);
                } else if script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    // re3 asserts that this opcode's target is a boat.
                } else {
                    leftover.push(ScriptCommandEffect::BoatGotoCoordinates {
                        handle,
                        position,
                        needs_water_level,
                        anti_reverse_timer_ms,
                    });
                }
            }
            ScriptCommandEffect::BoatStop { handle } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Boat { mut boat, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    let base = &mut boat.vehicle;
                    base.auto_pilot.car_mission =
                        crate::control::AutoPilot::eCarMission::MISSION_NONE;
                    base.set_status(crate::entities::Entity::eEntityStatus::STATUS_PHYSICS);
                    base.drive.engine_on = false;
                    base.auto_pilot.cruise_speed = 0;
                } else if script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    // re3 asserts that this opcode's target is a boat.
                } else {
                    leftover.push(ScriptCommandEffect::BoatStop { handle });
                }
            }
            ScriptCommandEffect::SetBoatCruiseSpeed { handle, speed } => {
                if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Boat { boat, .. }) =
                    script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    boat.vehicle.core.auto_pilot.cruise_speed = speed.max(0.0).min(255.0) as u8;
                } else if script_entities.vehicles.contains_key(&script_entities.index, &handle) {
                    // re3 asserts that this opcode's target is a boat.
                } else {
                    leftover.push(ScriptCommandEffect::SetBoatCruiseSpeed { handle, speed });
                }
            }
            ScriptCommandEffect::SetCarDrivingStyle { handle, style } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    entity.base_mut().auto_pilot.driving_style = match style {
                        1 => crate::control::AutoPilot::eCarDrivingStyle::DRIVINGSTYLE_SLOW_DOWN_FOR_CARS,
                        2 => crate::control::AutoPilot::eCarDrivingStyle::DRIVINGSTYLE_AVOID_CARS,
                        3 => crate::control::AutoPilot::eCarDrivingStyle::DRIVINGSTYLE_PLOUGH_THROUGH,
                        4 => crate::control::AutoPilot::eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS_IGNORE_LIGHTS,
                        _ => crate::control::AutoPilot::eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS,
                    };
                }
            }
            ScriptCommandEffect::SetCarMission {
                handle,
                mission,
                anti_reverse_timer_ms,
            } => {
                if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = entity.base_mut();
                    base.auto_pilot.car_mission = script_car_mission(mission);
                    base.auto_pilot.anti_reverse_timer = anti_reverse_timer_ms;
                }
            }
            ScriptCommandEffect::SetCarTarget {
                handle,
                target,
                ram,
            } => {
                if !script_entities.vehicles.contains_key(&script_entities.index, &target) {
                    leftover.push(ScriptCommandEffect::SetCarTarget {
                        handle,
                        target,
                        ram,
                    });
                } else if let Some(mut entity) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    // re3 CCarAI::TellCarTo{Ram,Block}OtherCar retains a pool reference,
                    // activates the engine and ensures a minimum cruise speed of six.
                    let mut base = entity.base_mut();
                    base.auto_pilot.target_car = Some(CPoolHandle(target).index());
                    base.auto_pilot.car_mission = if ram {
                        crate::control::AutoPilot::eCarMission::MISSION_RAMCAR_FARAWAY
                    } else {
                        crate::control::AutoPilot::eCarMission::MISSION_BLOCKCAR_FARAWAY
                    };
                    base.drive.engine_on = true;
                    base.auto_pilot.cruise_speed = base.auto_pilot.cruise_speed.max(6);
                } else {
                    leftover.push(ScriptCommandEffect::SetCarTarget {
                        handle,
                        target,
                        ram,
                    });
                }
            }
            ScriptCommandEffect::WarpCharIntoCar {
                char_handle,
                vehicle_handle,
            } => {
                if !script_entities.warp_char_into_car(char_handle, vehicle_handle) {
                    leftover.push(ScriptCommandEffect::WarpCharIntoCar {
                        char_handle,
                        vehicle_handle,
                    });
                }
            }
            ScriptCommandEffect::WarpCharFromCarToCoord {
                handle,
                mut position,
                needs_ground,
            } => {
                if needs_ground {
                    position.z = file_loader.world.find_ground_z_for_coord(
                        position.x,
                        position.y,
                        &file_loader.model_info,
                    );
                }
                let script_ped_model = script_entities.peds.get(&script_entities.index, &handle)
                    .map(|entity| entity.ped.physical.entity.model_index);
                let player_model = world_players
                    .iter()
                    .find(|player| player.pool_handle == handle)
                    .map(|player| player.ped.physical.entity.model_index);
                let Some(model_index) = script_ped_model.or(player_model) else {
                    leftover.push(ScriptCommandEffect::WarpCharFromCarToCoord {
                        handle,
                        position,
                        needs_ground: false,
                    });
                    continue;
                };
                if let Some(col_model) = file_loader
                    .model_info
                    .col_model(model_index.max(0) as usize)
                {
                    position.z += -col_model.bounding_box.min.z;
                }

                let player_index = (0..world_players.slot_count()).find(|index| {
                    world_players
                        .player(*index as i32)
                        .is_some_and(|player| player.pool_handle == handle)
                });
                let ped_entity = script_entities.ped_entity(handle).or_else(|| {
                    player_index.and_then(|index| world_players.player_entity(index as i32))
                });
                let relation = script_entities.relation_for_char(handle).or_else(|| {
                    player_index.and_then(|index| world_players.relation(index as i32))
                });
                let vehicle_handle = relation.and_then(|relation| {
                    script_entities.vehicle_handle_for_entity(relation.vehicle)
                });
                let vehicle_is_bus = vehicle_handle
                    .and_then(|vehicle_handle| script_entities.vehicles.get(&script_entities.index, &vehicle_handle))
                    .is_some_and(|vehicle| vehicle.base().flags.is_bus);
                if script_entities.relation_for_char(handle).is_some() {
                    let _ = script_entities.clear_char_relation(handle);
                } else if let Some(index) = player_index {
                    let _ = world_players.clear_relation(index as i32);
                }
                if let Some(ped_entity) = ped_entity {
                    script_entities
                        .commands
                        .entity(ped_entity)
                        .remove::<CPedVehicleAnimation>();
                }
                if let Some(vehicle_handle) = vehicle_handle
                    && let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
                {
                    let mut base = vehicle.base_mut();
                    if relation.is_some_and(|relation| relation.seat == VehicleSeat::Driver) {
                        base.set_status(eEntityStatus::STATUS_ABANDONED);
                        base.drive.engine_on = false;
                        base.auto_pilot.cruise_speed = 0;
                    }
                    base.physical.move_speed = CVector::new(0.0, 0.0, -0.00001);
                    base.physical.turn_speed = CVector::ZERO;
                }

                let initialise_ped = |ped: &mut CPed| {
                    if vehicle_is_bus {
                        ped.flags.render_ped_in_car = true;
                    }
                    ped.ped_state = PedState::PED_IDLE;
                    ped.last_ped_state = PedState::PED_NONE;
                    ped.physical.entity.flags.uses_collision = true;
                    ped.physical.move_speed = CVector::ZERO;
                    ped.move_state = eMoveState::PEDMOVE_NONE;
                    if let Some(weapon_info) = weapon_info.as_ref() {
                        let weapon = ped.current_weapon as usize;
                        ped.weapon_model_id = weapon_info
                            .weapon_info
                            .get(weapon)
                            .map_or(-1, |info| info.model_id);
                    }
                    ped.physical.entity.set_position(position);
                };
                if let Some(mut entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    initialise_ped(&mut entity.ped);
                } else if let Some(player) = world_players
                    .iter_mut()
                    .find(|player| player.pool_handle == handle)
                {
                    initialise_ped(player.ped);
                }
            }
            ScriptCommandEffect::WarpPlayerIntoCar {
                player_index,
                vehicle_handle,
            } => {
                let Some(player_entity) = world_players.player_entity(player_index) else {
                    leftover.push(ScriptCommandEffect::WarpPlayerIntoCar {
                        player_index,
                        vehicle_handle,
                    });
                    continue;
                };
                let Some(mut player) = world_players.player_mut(player_index) else {
                    leftover.push(ScriptCommandEffect::WarpPlayerIntoCar {
                        player_index,
                        vehicle_handle,
                    });
                    continue;
                };
                let warped = script_entities.warp_player_into_car(
                    player_entity,
                    &mut player,
                    vehicle_handle,
                    VehicleSeat::Driver,
                    &file_loader,
                );
                bevy::log::debug!(
                    "script WarpPlayerIntoCar: handle={} ok={} pos=({:.2},{:.2},{:.2})",
                    vehicle_handle,
                    warped,
                    player.position().x,
                    player.position().y,
                    player.position().z
                );
                if !warped {
                    leftover.push(ScriptCommandEffect::WarpPlayerIntoCar {
                        player_index,
                        vehicle_handle,
                    });
                }
            }
            ScriptCommandEffect::WarpPlayerFromCarToCoord {
                player_index,
                position,
                needs_ground: _,
            } => {
                let player_entity = world_players.player_entity(player_index);
                let relation = world_players.relation(player_index);
                let vehicle_handle = relation.and_then(|relation| {
                    script_entities.vehicle_handle_for_entity(relation.vehicle)
                });
                let _ = world_players.clear_relation(player_index);
                if let Some(player_entity) = player_entity {
                    script_entities
                        .commands
                        .entity(player_entity)
                        .remove::<CPedVehicleAnimation>();
                }
                if relation.is_some_and(|relation| relation.seat == VehicleSeat::Driver)
                    && let Some(vehicle_handle) = vehicle_handle
                    && let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &vehicle_handle)
                {
                    let mut base = vehicle.base_mut();
                    base.set_status(eEntityStatus::STATUS_ABANDONED);
                    base.drive.engine_on = false;
                    base.auto_pilot.cruise_speed = 0;
                }
                let Some(mut player) = world_players.player_mut(player_index) else {
                    leftover.push(ScriptCommandEffect::WarpPlayerFromCarToCoord {
                        player_index,
                        position,
                        needs_ground: false,
                    });
                    continue;
                };
                player.ped.ped_state = PedState::PED_IDLE;
                player.ped.physical.entity.flags.uses_collision = true;
                player.ped.physical.move_speed = rust_gta_3_core::core::Coordinates::CVector::ZERO;
                player.ped.move_state = eMoveState::PEDMOVE_NONE;
                player.set_position(position);
            }
            other => leftover.push(other),
        }
    }
    if let Some(special_fx) = special_fx.as_mut() {
        for marker in &mut special_fx.markers {
            if marker.identifier & 0xFFF0_0000 == HIGHLIGHT_MARKER_PREFIX
                && !active_highlight_markers.contains(&marker.identifier)
            {
                *marker = C3dMarkerState::default();
            }
        }
    }
    scripts.pending_effects = leftover;
}

const SCRIPT_SPHERE_MARKER_PREFIX: u32 = 0x5350_0000;

/// re3 `CTheScripts::DrawScriptSpheres` — refreshes each active cylinder every
/// script frame. Marker identifiers use the fixed sphere slot because the C++
/// pointer-derived identifier is not observable by SCM.
pub(crate) fn draw_script_spheres(
    scripts: Res<CTheScripts>,
    mut special_fx: ResMut<CSpecialFX>,
    timer: Res<CTimer>,
    players: CWorldPlayersRead,
    file_loader: Res<CFileLoader>,
    collision: Res<crate::collision::Collision::CCollision>,
    mut resident_level: Local<Option<rust_gta_3_core::core::GameState::ELevelName>>,
) {
    // C3dMarker's initial ground projection belongs to the resident COL.
    // Recreate only when that receiver set changes during cooperative loading.
    if *resident_level != Some(collision.collision_in_memory) {
        for marker in &mut special_fx.markers {
            if marker.identifier & 0xFFFF_0000 == SCRIPT_SPHERE_MARKER_PREFIX {
                *marker = C3dMarkerState::default();
            }
        }
        *resident_level = Some(collision.collision_in_memory);
    }
    // C++ `PlaceMarkerSet` emits three nested cylinder markers (1.0, .93,
    // .86) with independent rotation rates.  Keep the same identifier space
    // so the pulse/rotation state is preserved by CSpecialFX.
    let mut active = [false; crate::control::Script::MAX_NUM_SCRIPT_SPHERES * 3];
    for (slot, sphere) in scripts.script_spheres.iter().enumerate() {
        if !sphere.in_use || slot >= active.len() {
            continue;
        }
        let position = CVector::new(sphere.center[0], sphere.center[1], sphere.center[2]);
        for (variant, (scale, rotate_rate)) in [(1.0_f32, 1_i16), (0.93, 2), (0.86, -1)]
            .into_iter()
            .enumerate()
        {
            let marker_slot = slot * 3 + variant;
            active[marker_slot] = true;
            let mut grounded_position = position;
            if let Some(z) = file_loader.world.find_ground_z_for_3d_coord(
                position.x, position.y, position.z + 1.0, &file_loader.model_info,
            ) && z != 0.0 {
                grounded_position.z = z - 0.05 * sphere.radius * scale;
            }
            special_fx.place_marker(
                SCRIPT_SPHERE_MARKER_PREFIX | marker_slot as u32,
                MARKERTYPE_CYLINDER,
                grounded_position,
                sphere.radius * scale,
                0,
                128,
                255,
                128,
                2048,
                0.1,
                rotate_rate,
                timer.time_in_milliseconds(),
                players.focused().map_or(position, |player| player.position()),
                timer.time_step(),
            );
        }
    }
    for marker in &mut special_fx.markers {
        if marker.identifier & 0xFFFF_0000 != SCRIPT_SPHERE_MARKER_PREFIX {
            continue;
        }
        let slot = (marker.identifier & 0xFFFF) as usize;
        if !active.get(slot).copied().unwrap_or(false) {
            *marker = C3dMarkerState::default();
        }
    }
}

pub(crate) fn apply_script_object_motion(
    request: ScriptObjectMotionRequest,
    object_entity: &mut ScriptObjectMut<'_>,
    vehicles: &ScriptVehicleStore,
    peds: &ScriptPedStore,
    world_players: &CWorldPlayers,
    file_loader: &CFileLoader,
) -> Option<bool> {
    match request {
        ScriptObjectMotionRequest::Rotate {
            handle,
            heading_target_degrees,
            rotate_by_degrees,
            check_collision,
        } => {
            if object_entity.handle != handle {
                return None;
            }
            let forward = object_entity.object.physical.entity.get_forward();
            let mut heading = (-forward.x).atan2(forward.y).to_degrees();
            if heading < 0.0 {
                heading += 360.0;
            }
            if heading_target_degrees == heading {
                return Some(true);
            }
            let mut clockwise = heading_target_degrees - heading;
            if clockwise < 0.0 {
                clockwise += 360.0;
            }
            let mut counterclockwise = heading - heading_target_degrees;
            if counterclockwise < 0.0 {
                counterclockwise += 360.0;
            }
            let new_heading = if clockwise < counterclockwise {
                if rotate_by_degrees < clockwise {
                    heading + rotate_by_degrees
                } else {
                    heading_target_degrees
                }
            } else if rotate_by_degrees < counterclockwise {
                heading - rotate_by_degrees
            } else {
                heading_target_degrees
            };

            let position = object_entity.object.physical.entity.get_position();
            let model_id = object_entity.object.physical.entity.model_index.max(0) as usize;
            let bounds = file_loader
                .model_info
                .col_model(model_id)?
                .bounding_box
                .clone();
            let mut candidate = CMatrix::unity();
            candidate.set_rotate_z(new_heading.to_radians());
            candidate.position = position;
            if check_collision
                && script_object_box_hits_vehicle_or_ped(
                    &bounds,
                    &candidate,
                    position,
                    vehicles,
                    peds,
                    world_players,
                    file_loader,
                )
            {
                return Some(true);
            }
            object_entity
                .object
                .physical
                .entity
                .set_heading(new_heading.to_radians());
            Some(new_heading == heading_target_degrees)
        }
        ScriptObjectMotionRequest::Slide {
            handle,
            target,
            slide_by,
            check_collision,
        } => {
            if object_entity.handle != handle {
                return None;
            }
            let position = object_entity.object.physical.entity.get_position();
            if target == position {
                return Some(true);
            }
            let new_position = CVector::new(
                slide_axis(position.x, target.x, slide_by.x),
                slide_axis(position.y, target.y, slide_by.y),
                slide_axis(position.z, target.z, slide_by.z),
            );
            let model_id = object_entity.object.physical.entity.model_index.max(0) as usize;
            let bounds = file_loader
                .model_info
                .col_model(model_id)?
                .bounding_box
                .clone();
            let mut candidate = object_entity.object.physical.entity.matrix;
            candidate.position = new_position;
            if check_collision
                && script_object_box_hits_vehicle_or_ped(
                    &bounds,
                    &candidate,
                    new_position,
                    vehicles,
                    peds,
                    world_players,
                    file_loader,
                )
            {
                return Some(true);
            }
            object_entity.object.teleport(new_position);
            Some(new_position == target)
        }
    }
}

pub(crate) fn slide_axis(position: f32, target: f32, step: f32) -> f32 {
    let difference = position - target;
    if difference < 0.0 {
        if -difference < step {
            target
        } else {
            position + step
        }
    } else if difference < step {
        target
    } else {
        position - step
    }
}

pub(crate) fn script_object_box_hits_vehicle_or_ped(
    bounding_box: &CColBox,
    object_matrix: &CMatrix,
    object_position: CVector,
    vehicles: &ScriptVehicleStore,
    peds: &ScriptPedStore,
    world_players: &CWorldPlayers,
    file_loader: &CFileLoader,
) -> bool {
    let intersects = |entity: &crate::entities::Entity::CEntity| {
        let Some(col_model) = file_loader
            .model_info
            .col_model(entity.model_index.max(0) as usize)
        else {
            return false;
        };
        let world_center = entity.matrix.transform_point(CVector::new(
            col_model.bounding_sphere.center.x,
            col_model.bounding_sphere.center.y,
            col_model.bounding_sphere.center.z,
        ));
        let delta = CVector::new(
            world_center.x - object_position.x,
            world_center.y - object_position.y,
            world_center.z - object_position.z,
        );
        let local_center = CColVector::new(
            delta.x * object_matrix.right.x
                + delta.y * object_matrix.right.y
                + delta.z * object_matrix.right.z,
            delta.x * object_matrix.forward.x
                + delta.y * object_matrix.forward.y
                + delta.z * object_matrix.forward.z,
            delta.x * object_matrix.up.x
                + delta.y * object_matrix.up.y
                + delta.z * object_matrix.up.z,
        );
        CCollision::test_sphere_box(
            &CColSphere::new(
                col_model.bounding_sphere.radius,
                local_center,
                col_model.bounding_sphere.surface,
                col_model.bounding_sphere.piece,
            ),
            bounding_box,
        )
    };

    vehicles
        .values()
        .any(|vehicle| intersects(&vehicle.base().physical.entity))
        || peds
            .values()
            .any(|ped| intersects(&ped.ped.physical.entity))
        || world_players
            .iter()
            .any(|player| intersects(&player.ped.physical.entity))
}

pub(crate) fn set_script_snapshot_bounds(
    snapshot: &mut ScriptEntitySnapshot,
    matrix: &CMatrix,
    model_id: usize,
    file_loader: &CFileLoader,
) {
    let Some(col_model) = file_loader.model_info.col_model(model_id) else {
        return;
    };
    let local = &col_model.bounding_sphere.center;
    snapshot.bound_center = matrix.transform_point(CVector::new(local.x, local.y, local.z));
    snapshot.bound_radius = col_model.bounding_sphere.radius;
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_script_char_objective(
    script_entities: &mut ScriptEntityWorld,
    world_players: &mut dyn WorldPlayerAccess,
    handle: i32,
    objective: i32,
    target_char: Option<i32>,
    target_car: Option<i32>,
    position: Option<CVector>,
    radius: f32,
    route: Option<(i32, i32)>,
    routes: &CPedRoutes,
    now_ms: u32,
) -> bool {
    let current_vehicle = script_entities
        .current_vehicle_handle_for_char(handle)
        .map(|vehicle| CPoolHandle(vehicle).index())
        .or_else(|| {
            world_players
                .vehicle_relation_by_pool_handle(handle)
                .and_then(|relation| {
                    script_entities
                        .vehicle_handle_for_entity(relation.vehicle)
                        .map(|vehicle| CPoolHandle(vehicle).index())
                })
        });
    let (ped, vitality) = if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &handle) {
        (&mut *entity.ped, entity.vitality)
    } else if let Some(player) = world_players.find_by_pool_handle_mut(handle) {
        // `GET_PLAYER_CHAR` returns the real player-pool handle. Player peds
        // live in CWorldPlayers rather than ScriptEntityWorld, but all
        // SET_CHAR_OBJ_* opcodes address both through the same CPed pool in
        // re3. Route the handle to its actual ECS owner before applying the
        // common CPed objective transition.
        (player.ped, player.vitality)
    } else {
        return false;
    };
    apply_ped_objective(
        ped, vitality,
        objective,
        target_char,
        target_car,
        position,
        radius,
        route,
        routes,
        now_ms,
        current_vehicle,
    );
    true
}

/// Runtime counterpart of re3 `CWorld::ClearExcitingStuffFromArea` used by
/// `CGameLogic::Update` before a hospital/police resurrection.  Mission-owned
/// entities and the player are deliberately left alone; ordinary population
/// peds/cars are removed from the CPools and their render handles disappear on
/// the next registry refresh instead of being carried into the restart scene.
pub(crate) fn clear_exciting_stuff_from_area_runtime(
    entities: &mut ScriptEntityWorld,
    pools: &mut CPools,
    mut population: Option<&mut CPopulation>,
    mut car_ctrl: Option<&mut CCarCtrl>,
    fire_manager: Option<&mut CFireManager>,
    explosions: Option<&mut CExplosionPool>,
    projectiles: Option<&mut CProjectileInfoPool>,
    shadows: Option<&mut CShadows>,
    center: CVector,
    radius: f32,
    remove_projectiles_and_tidy_shadows: bool,
) {
    let radius_sqr = radius * radius;
    let ped_handles: Vec<i32> = entities
        .peds
        .iter()
        .filter_map(|(&handle, entity)| {
            let ped = &entity.ped;
            let pos = ped.physical.entity.get_position();
            let dx = pos.x - center.x;
            let dy = pos.y - center.y;
            (ped.char_created_by != MISSION_CHAR
                && entities.relation_for_char(handle).is_none()
                && dx * dx + dy * dy < radius_sqr)
                .then_some(handle)
        })
        .collect();
    for handle in ped_handles {
        if let Some(ped_type) = entities.peds.get(&entities.index, &handle).map(|entity| entity.ped.ped_type) {
            if let Some(population) = population.as_deref_mut() {
                remove_ped_from_population_counts(population, ped_type);
            }
        }
        entities.remove_char(pools, handle);
    }

    let vehicle_handles: Vec<i32> = entities
        .vehicles
        .iter()
        .filter_map(|(&handle, vehicle)| {
            let base = vehicle.base();
            let pos = base.physical.entity.get_position();
            let dx = pos.x - center.x;
            let dy = pos.y - center.y;
            (!base.flags.is_locked
                && dx * dx + dy * dy < radius_sqr
                && script_vehicle_base_can_be_deleted(entities, handle, &base))
            .then_some(handle)
        })
        .collect();
    for handle in vehicle_handles {
        remove_script_vehicle_and_occupants(
            entities,
            pools,
            handle,
            population.as_deref_mut(),
            car_ctrl.as_deref_mut(),
        );
    }

    let temp_objects: Vec<i32> = entities
        .objects
        .values()
        .filter_map(|entity| {
            let position = entity.object.physical.entity.get_position();
            let delta = CVector::new(
                position.x - center.x,
                position.y - center.y,
                position.z - center.z,
            );
            (entity.object.object_created_by == TEMP_OBJECT
                && delta.magnitude_sqr() < radius_sqr)
                .then_some(entity.handle)
        })
        .collect();
    for handle in temp_objects {
        entities.object_remove(Some(pools), handle);
    }

    if let Some(fire_manager) = fire_manager {
        fire_manager.ExtinguishPoint(center, radius);
    }
    for mut vehicle in entities.vehicles.values_mut() {
        let position = vehicle.base().physical.entity.get_position();
        let delta = CVector::new(
            position.x - center.x,
            position.y - center.y,
            position.z - center.z,
        );
        if delta.magnitude_sqr() < radius_sqr {
            vehicle.base_mut().vitality.car_fire = None;
        }
    }
    if let Some(explosions) = explosions {
        explosions.RemoveAllExplosionsInArea(center, radius);
    }
    if remove_projectiles_and_tidy_shadows {
        if let Some(projectiles) = projectiles {
            projectiles.RemoveAllProjectiles();
        }
        if let Some(shadows) = shadows {
            shadows.TidyUpShadows();
        }
    }
}
