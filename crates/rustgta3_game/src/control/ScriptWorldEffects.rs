//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Split from control/Script.rs (verbatim, parity-preserving).

#[allow(unused_imports)]
use super::Script::{
    ScriptEntityWorld, ScriptGameplayBridge, ScriptVehicleEntity, script_key_to_string,
};
#[allow(unused_imports)]
use super::ScriptTypes::{
    CTheScripts, MAP_Z_LOW_LIMIT, ScriptCommandEffect, ScriptHandle, ScriptInvisibilityTarget,
    ScriptStatsEffect,
};
#[allow(unused_imports)]
use bevy::input::{ButtonInput, mouse::MouseButton};
#[allow(unused_imports)]
use bevy::prelude::{KeyCode, error};
#[allow(unused_imports)]
use rustgta3_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral};
pub(crate) const SCM_REQUEST_MODEL_FLAGS: u8 = crate::core::Streaming::STREAMFLAGS_DEPENDENCY
    | crate::core::Streaming::STREAMFLAGS_NOFADE
    | crate::core::Streaming::STREAMFLAGS_SCRIPTOWNED;

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
use crate::core::References::CReferences;
#[allow(unused_imports)]
use crate::core::Stats::CStats;
#[allow(unused_imports)]
use crate::core::Streaming::{CStreaming, STREAMFLAGS_DEPENDENCY};
#[allow(unused_imports)]
use crate::core::World::EEntityType;
#[allow(unused_imports)]
use crate::entities::Entity::script_entity_status;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_DEADDODO;
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::script_vehicle_base_z;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::{
    choose_vehicle_optional_components, choose_vehicle_optional_components_from_clump, eVehicleType,
};
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
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
#[allow(unused_imports)]
use rustgta3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use rustgta3_assets::streaming::CdStream::CdStream;
#[allow(unused_imports)]
use rustgta3_core::core::GameState::{CGameState, ELevelName};

#[allow(unused_imports)]
use crate::audio::soundlist::*;
#[allow(unused_imports)]
use crate::control::CarCtrl::CCarCtrl;
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
use rustgta3_core::core::Timer::CTimer;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

pub(crate) fn refresh_script_streaming_snapshots(
    scripts: &mut CTheScripts,
    streaming: &CStreaming,
) {
    scripts.loaded_vehicle_models = streaming.vehicles_loaded;
    for (model_id, loaded) in scripts.model_loaded.iter_mut().enumerate() {
        *loaded = streaming.has_model_loaded(model_id);
    }
    for (slot, loaded) in scripts.special_char_loaded.iter_mut().enumerate() {
        *loaded = streaming.has_model_loaded(crate::modelinfo::ModelIndices::MI_SPECIAL01 + slot);
    }
}

pub(crate) fn request_scm_model(
    streaming: &mut CStreaming,
    file_loader: Option<&mut CFileLoader>,
    model_id: i32,
) {
    let model_id = model_id.max(0) as usize;
    if let Some(file_loader) = file_loader {
        streaming.request_model_with_adapter(model_id, SCM_REQUEST_MODEL_FLAGS, &*file_loader);
        // re3 CStreaming::RequestModel forces an already-resident simple model
        // fully opaque when STREAMFLAGS_NOFADE is requested.
        if streaming.has_model_loaded(model_id) {
            file_loader.model_info.set_simple_alpha(model_id, 255);
        }
    } else {
        streaming.request_model(model_id, SCM_REQUEST_MODEL_FLAGS);
    }
}

fn freeze_cutscene_model_snapshot(
    scripts: &mut CTheScripts,
    file_loader: &CFileLoader,
    handle: i32,
    model_id: i32,
) {
    let Some(snapshot) = cutscene_model_snapshot(file_loader, model_id.max(0) as usize) else {
        bevy::log::warn_once!(handle, model_id, "cutscene_create_waiting_original_model_snapshot");
        return;
    };
    scripts
        .pending_cutscene_model_snapshots
        .retain(|(stored_handle, _)| *stored_handle != handle);
    scripts
        .pending_cutscene_model_snapshots
        .push((handle, snapshot));
}

fn take_cutscene_model_snapshot(
    scripts: &mut CTheScripts,
    handle: i32,
) -> Option<crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot> {
    let index = scripts
        .pending_cutscene_model_snapshots
        .iter()
        .position(|(stored_handle, _)| *stored_handle == handle)?;
    Some(
        scripts
            .pending_cutscene_model_snapshots
            .swap_remove(index)
            .1,
    )
}

/// Apply command-boundary side effects that re3 performs before the following
/// SCM opcode. Streaming mutations are committed immediately, and cutscene
/// object model identity is frozen at CREATE_CUTSCENE_* just like the RW
/// CreateInstance performed by CEntity::SetModelIndex.
///
/// Requests are committed immediately so following SCM predicates observe the
/// same CStreaming state as re3. Blocking load commands themselves are turned
/// into cooperative barriers, so this boundary no longer needs direct access
/// to a CD channel.
pub(crate) fn flush_immediate_script_streaming_effect(
    scripts: &mut CTheScripts,
    streaming: &mut CStreaming,
    file_loader: &mut CFileLoader,
) {
    let Some(effect) = scripts.pending_effects.pop() else {
        return;
    };
    match effect {
        ScriptCommandEffect::SetStreamingEnabled(enabled) => {
            streaming.disable_streaming = !enabled;
        }
        ScriptCommandEffect::RequestSpecialModel { model_id, name } => {
            let end = name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name.len());
            let model_name = String::from_utf8_lossy(&name[..end]).to_ascii_lowercase();
            let flags = crate::core::Streaming::STREAMFLAGS_DEPENDENCY
                | crate::core::Streaming::STREAMFLAGS_SCRIPTOWNED;
            let _ = streaming.request_special_model(
                model_id.max(0) as usize,
                &model_name,
                flags,
                file_loader,
            );
        }
        ScriptCommandEffect::MarkModelAsNoLongerNeeded { model_id } => {
            streaming.set_mission_doesnt_require_model(model_id.max(0) as usize);
        }
        ScriptCommandEffect::UnloadSpecialCharacter { char_id } => {
            streaming.set_mission_doesnt_require_special_char(char_id.max(0) as usize);
        }
        ScriptCommandEffect::RequestSpecialCharacter { slot, name } => {
            let end = name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name.len());
            let model_name = String::from_utf8_lossy(&name[..end]).to_ascii_lowercase();
            let flags = crate::core::Streaming::STREAMFLAGS_DEPENDENCY
                | crate::core::Streaming::STREAMFLAGS_SCRIPTOWNED;
            let _ = streaming.request_special_char(slot, &model_name, flags, file_loader);
        }
        ScriptCommandEffect::RequestModel { model_id } => {
            request_scm_model(streaming, Some(file_loader), model_id);
        }
        ScriptCommandEffect::CreateCutsceneObject { handle, model_id } => {
            freeze_cutscene_model_snapshot(scripts, file_loader, handle, model_id);
            scripts
                .pending_effects
                .push(ScriptCommandEffect::CreateCutsceneObject { handle, model_id });
            return;
        }
        ScriptCommandEffect::CreateCutsceneHead {
            handle,
            source_object_handle,
            model_id,
        } => {
            freeze_cutscene_model_snapshot(scripts, file_loader, handle, model_id);
            scripts
                .pending_effects
                .push(ScriptCommandEffect::CreateCutsceneHead {
                    handle,
                    source_object_handle,
                    model_id,
                });
            return;
        }
        ScriptCommandEffect::LoadAllModelsNow => {
            // Script4.cpp wraps this synchronous CStreaming call in
            // CTimer::Stop/Update. Preserve that VM/game-time barrier without
            // blocking Bevy render extraction: Streaming drains the already
            // requested models over subsequent frames.
            file_loader.begin_load_all_models_streaming(streaming);
        }
        ScriptCommandEffect::LoadScene { position } => {
            // re3 blocks here inside CStreaming::LoadScene. In Bevy that also
            // blocks ECS/render extraction and creates a severe visible hitch.
            // Prepare the identical scene request set now; the Streaming set
            // drains it cooperatively while gameplay time is code-paused.
            file_loader.begin_load_scene_streaming(streaming, [position.x, position.y, position.z]);
        }
        other => {
            scripts.pending_effects.push(other);
            return;
        }
    }
    refresh_script_streaming_snapshots(scripts, streaming);
}

/// Drain `pending_effects` into bridge + optional world resources.
/// Without a CD channel LOAD_ALL_MODELS_NOW remains pending rather than being
/// reported as complete.
pub fn apply_script_effects_to_world(
    scripts: &mut CTheScripts,
    bridge: &mut ScriptGameplayBridge,
    game_state: Option<&mut CGameState>,
    stats: Option<&mut CStats>,
    cutscene: Option<&mut CCutsceneMgr>,
    streaming: Option<&mut CStreaming>,
    camera: Option<&mut CCamera>,
    weather: Option<&mut CWeather>,
    gangs: Option<&mut CGangs>,
    world_players: Option<&mut dyn WorldPlayerAccess>,
    general: Option<&mut CGeneral>,
    car_generators: Option<&mut CTheCarGenerators>,
    file_loader: Option<&mut CFileLoader>,
    time_ms: u32,
) {
    apply_script_effects_to_world_with_streaming(
        scripts,
        bridge,
        game_state,
        stats,
        cutscene,
        streaming,
        camera,
        weather,
        gangs,
        None,
        None,
        world_players,
        general,
        car_generators,
        file_loader,
        None,
        None,
        time_ms,
        None,
    );
}

/// Production apply path with re3's synchronous CD streaming dependency.
pub fn apply_script_effects_to_world_with_streaming(
    scripts: &mut CTheScripts,
    bridge: &mut ScriptGameplayBridge,
    mut game_state: Option<&mut CGameState>,
    mut stats: Option<&mut CStats>,
    mut cutscene: Option<&mut CCutsceneMgr>,
    mut streaming: Option<&mut CStreaming>,
    mut camera: Option<&mut CCamera>,
    mut weather: Option<&mut CWeather>,
    mut gangs: Option<&mut CGangs>,
    mut ped_types: Option<&mut CPedTypeRuntime>,
    mut script_entities: Option<&mut ScriptEntityWorld<'_, '_>>,
    mut world_players: Option<&mut dyn WorldPlayerAccess>,
    mut general: Option<&mut CGeneral>,
    mut car_generators: Option<&mut CTheCarGenerators>,
    mut file_loader: Option<&mut CFileLoader>,
    mut cd_stream: Option<&mut CdStream>,
    mut particles: Option<&mut CParticleObjectRuntime>,
    time_ms: u32,
    weapon_info: Option<&CWeaponInfoStore>,
) {
    if scripts.pending_effects.is_empty() {
        return;
    }
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::MissionCleanupProcessed => {
                // re3 `CMissionCleanup::Process` resets mission-global state
                // before releasing its owned entities. Keep the marker alive
                // for the population/HUD/dynamic passes, which own the rest.
                scripts.store_vehicle_index = -1;
                scripts.store_vehicle_was_random = true;
                scripts.upside_down_cars.init();
                scripts.stuck_cars.init();
                if let Some(camera) = camera.as_mut() {
                    camera.restore();
                    camera.wide_screen_on = false;
                }
                bridge.widescreen = false;
                if let Some(weather) = weather.as_mut() {
                    weather.release_weather();
                }
                if let Some(streaming) = streaming.as_mut() {
                    streaming.disable_streaming = false;
                    for slot in 0..crate::modelinfo::ModelIndices::NUM_OF_SPECIAL_CHARS {
                        streaming.set_mission_doesnt_require_special_char(slot);
                    }
                    // re3 releases MI_CUTOBJ01..MI_CUTOBJ05 here.
                    for model_id in 185..=189 {
                        streaming.set_mission_doesnt_require_model(model_id);
                    }
                }
                if let Some(players) = world_players.as_mut()
                    && let Some(mut player) = players.focused_mut()
                {
                    player.wanted.m_fCrimeSensitivity = 1.0;
                    player.wanted.m_bIgnoredByCops = false;
                    player.wanted.m_bIgnoredByEveryone = false;
                    // `MakePlayerSafe(false)` is gated this way in re3.
                    if !bridge.playing_intro && scripts.countdown_to_make_player_unsafe == 0 {
                        player.can_be_damaged = true;
                        let flags = &mut player.ped.physical.entity.flags;
                        flags.bullet_proof = false;
                        flags.fire_proof = false;
                        flags.collision_proof = false;
                        flags.melee_proof = false;
                        flags.only_damaged_by_player = false;
                        flags.explosion_proof = false;
                    }
                }
                leftover.push(ScriptCommandEffect::MissionCleanupProcessed);
            }
            ScriptCommandEffect::SetTotalNumberOfMissions(total) => {
                if let Some(stats) = stats.as_mut() {
                    stats.total_number_missions = total;
                }
            }
            ScriptCommandEffect::SetProgressTotal(total) => {
                if let Some(stats) = stats.as_mut() {
                    stats.total_progress_in_game = total;
                }
            }
            ScriptCommandEffect::RegisterFastestTime { index, time } => {
                if let Some(stats) = stats.as_mut() {
                    stats.register_fastest_time(index, time);
                }
            }
            ScriptCommandEffect::RegisterHighestScore { index, score } => {
                if let Some(stats) = stats.as_mut() {
                    stats.register_highest_score(index, score);
                }
            }
            ScriptCommandEffect::RegisterElBurroTime(time) => {
                if let Some(stats) = stats.as_mut() {
                    stats.register_el_burro_time(time);
                }
            }
            ScriptCommandEffect::Register4x4Time { race, time } => {
                if let Some(stats) = stats.as_mut() {
                    stats.register_4x4_time(race, time);
                }
            }
            ScriptCommandEffect::RegisterLifeSaved => {
                if let Some(stats) = stats.as_mut() {
                    stats.lives_saved_with_ambulance += 1;
                }
            }
            ScriptCommandEffect::RegisterCriminalCaught => {
                if let Some(stats) = stats.as_mut() {
                    stats.criminals_caught += 1;
                }
            }
            ScriptCommandEffect::RegisterAmbulanceLevel(level) => {
                if let Some(stats) = stats.as_mut() {
                    stats.register_ambulance_level(level);
                }
            }
            ScriptCommandEffect::RegisterFireExtinguished => {
                if let Some(stats) = stats.as_mut() {
                    stats.fires_extinguished += 1;
                }
            }
            ScriptCommandEffect::RegisterLongestDodoFlight(time) => {
                if let Some(stats) = stats.as_mut() {
                    stats.register_longest_dodo_flight(time);
                }
            }
            ScriptCommandEffect::RegisterDefuseBombTime(time) => {
                if let Some(stats) = stats.as_mut() {
                    stats.register_defuse_bomb_time(time);
                }
            }
            ScriptCommandEffect::SetTotalKillFrenzies(total) => {
                if let Some(stats) = stats.as_mut() {
                    stats.total_number_kill_frenzies = total;
                }
            }
            ScriptCommandEffect::Stats(update) => {
                if let Some(stats) = stats.as_mut() {
                    match update {
                        ScriptStatsEffect::AddProgress(value) => stats.progress_made += value,
                        ScriptStatsEffect::IndustrialPassed => stats.industrial_passed = 1,
                        ScriptStatsEffect::CommercialPassed => stats.commercial_passed = 1,
                        ScriptStatsEffect::SuburbanPassed => stats.suburban_passed = 1,
                        ScriptStatsEffect::RegisterJumpDistance(value) => {
                            stats.maximum_jump_distance = stats.maximum_jump_distance.max(value);
                        }
                        ScriptStatsEffect::RegisterJumpHeight(value) => {
                            stats.maximum_jump_height = stats.maximum_jump_height.max(value);
                        }
                        ScriptStatsEffect::RegisterJumpFlips(value) => {
                            stats.maximum_jump_flips = stats.maximum_jump_flips.max(value);
                        }
                        ScriptStatsEffect::RegisterJumpSpins(value) => {
                            stats.maximum_jump_spins = stats.maximum_jump_spins.max(value);
                        }
                        ScriptStatsEffect::RegisterJumpStunt(value) => {
                            stats.best_stunt_jump = stats.best_stunt_jump.max(value);
                        }
                        ScriptStatsEffect::RegisterUniqueJump => {
                            stats.number_of_unique_jumps_found += 1;
                        }
                        ScriptStatsEffect::SetUniqueJumpTotal(value) => {
                            stats.total_number_of_unique_jumps = value;
                        }
                        ScriptStatsEffect::RegisterTaxiPassenger => {
                            stats.passengers_dropped_off_with_taxi += 1;
                        }
                        ScriptStatsEffect::AddTaxiMoney(value) => {
                            stats.money_made_with_taxi += value;
                        }
                        ScriptStatsEffect::RegisterMissionGiven => stats.missions_given += 1,
                        ScriptStatsEffect::RegisterMissionPassed(name) => {
                            stats.last_mission_passed_name = name;
                            stats.missions_passed += 1;
                            stats.total_legitimate_kills += stats.kills_since_last_checkpoint;
                            stats.kills_since_last_checkpoint = 0;
                        }
                    }
                } else {
                    leftover.push(ScriptCommandEffect::Stats(update));
                }
            }
            ScriptCommandEffect::RegisterKillFrenzyPassed => {
                if let Some(stats) = stats.as_mut() {
                    stats.number_kill_frenzies_passed += 1;
                }
            }
            ScriptCommandEffect::LoadCutscene { name } => {
                // re3 Script3.cpp COMMAND_LOAD_CUTSCENE → CCutsceneMgr::LoadCutsceneData
                let cut_name = script_key_to_string(&name);
                info!("SCM LOAD_CUTSCENE '{cut_name}'");
                bridge.cutscene_name = cut_name.clone();
                bridge.cutscene_requested = true;
                // re3 LoadCutsceneData hides player and disables pad when ped exists.
                if bridge.player_spawned {
                    bridge.player_visible = false;
                    bridge.player_control_enabled = false;
                }
                match load_cutscene_runtime_data(&cut_name) {
                    Ok(data) => {
                        if let Some(camera_entry) = data.camera_entry.as_ref() {
                            if let Some(camera) = camera.as_mut() {
                                match camera.load_path_splines(&camera_entry.bytes) {
                                    Ok(spline_count) => info!(
                                        "SCM LOAD_CUTSCENE '{cut_name}' loaded {spline_count} camera splines from CUTS.IMG"
                                    ),
                                    Err(error) => error!(
                                        "SCM LOAD_CUTSCENE '{cut_name}' camera spline parse failed: {error}"
                                    ),
                                }
                            }
                        }
                        if let Some(cutscene) = cutscene.as_mut() {
                            if data.hierarchies.is_empty() {
                                cutscene.load_cutscene_data(&cut_name, false);
                            } else {
                                info!(
                                    "SCM LOAD_CUTSCENE '{cut_name}' loaded {} hierarchies from CUTS.IMG",
                                    data.hierarchies.len()
                                );
                                cutscene.load_cutscene_data_with_hierarchies(
                                    &cut_name,
                                    &data.hierarchies,
                                );
                            }
                        }
                    }
                    Err(load_error) => {
                        error!("SCM LOAD_CUTSCENE '{cut_name}' failed: {load_error}");
                        bridge.cutscene_requested = false;
                        // The hide/pad-disable above happens *before* the asset
                        // load, so a failed cutscene must undo it. re3 hides the
                        // player inside LoadCutsceneData only after the asset
                        // opened; without this restore a mission whose CUTS.IMG
                        // entry is missing left the player permanently invisible
                        // with input disabled behind a black fade.
                        if bridge.player_spawned {
                            bridge.player_visible = true;
                            bridge.player_control_enabled = true;
                        }
                        bridge.cutscene_started = false;
                        bridge.widescreen = false;
                        if let Some(cutscene) = cutscene.as_mut() {
                            cutscene.abort_failed_load();
                        }
                    }
                }
            }
            ScriptCommandEffect::MakePlayerSafeForCutscene { player_index } => {
                let Some(players) = world_players.as_mut() else {
                    leftover.push(ScriptCommandEffect::MakePlayerSafeForCutscene { player_index });
                    continue;
                };
                let Some(mut player) = players.player_mut(player_index) else {
                    leftover.push(ScriptCommandEffect::MakePlayerSafeForCutscene { player_index });
                    continue;
                };
                bridge.player_control_enabled = false;
                player.wanted.m_bIgnoredByEveryone = true;
                player.can_be_damaged = false;
                player.adrenaline_active = false;
                player.adrenaline_time = 0;
                let flags = &mut player.ped.physical.entity.flags;
                flags.bullet_proof = true;
                flags.fire_proof = true;
                flags.collision_proof = true;
                flags.melee_proof = true;
                flags.only_damaged_by_player = true;
                flags.explosion_proof = true;
                if let Some(cutscene) = cutscene.as_mut() {
                    cutscene.start_cutscene_processing();
                }
            }
            ScriptCommandEffect::StartCutscene => {
                if !bridge.cutscene_requested {
                    error!(
                        "SCM START_CUTSCENE ignored because original cutscene data did not load"
                    );
                    continue;
                }
                // re3: CCutsceneMgr::ms_cutsceneLoadStatus = 1 (LoadingAudio);
                // Update() then SetupCutsceneToStart. Player hidden during cutscene
                // is done in LoadCutsceneData in re3 after player exists.
                info!("SCM START_CUTSCENE name='{}'", bridge.cutscene_name);
                bridge.cutscene_started = true;
                bridge.widescreen = true;
                if bridge.player_spawned {
                    bridge.player_visible = false;
                    bridge.player_control_enabled = false;
                }
                if let Some(cutscene) = cutscene.as_mut() {
                    cutscene.cutscene_load_status =
                        crate::animation::CutsceneMgr::CutsceneLoadStatus::LoadingAudio;
                    cutscene.start_cutscene_processing();
                }
            }
            ScriptCommandEffect::ClearCutscene => {
                // re3 DeleteCutsceneData: restore visibility, pad and camera.
                if let Some(cutscene) = cutscene.as_mut() {
                    restore_after_cutscene_clear(
                        bridge,
                        game_state.as_deref_mut(),
                        cutscene,
                        camera.as_deref_mut(),
                    );
                } else {
                    bridge.cutscene_started = false;
                    bridge.cutscene_requested = false;
                    bridge.widescreen = false;
                    bridge.playing_intro = false;
                    if let Some(game_state) = game_state.as_mut() {
                        game_state.playing_intro = false;
                    }
                    if bridge.player_spawned {
                        bridge.player_visible = true;
                        bridge.player_control_enabled = true;
                    }
                    if let Some(camera) = camera.as_mut() {
                        camera.restore();
                    }
                }
            }
            ScriptCommandEffect::SetCutsceneOffset { position } => {
                if let Some(cutscene) = cutscene.as_mut() {
                    cutscene.set_cutscene_offset(crate::math::Vector::CVector::new(
                        position.x, position.y, position.z,
                    ));
                }
            }
            ScriptCommandEffect::CreateCutsceneObject { handle, model_id } => {
                let captured_snapshot = take_cutscene_model_snapshot(scripts, handle);
                let rainy = weather.as_deref().is_some_and(|weather| {
                    weather.old_weather_type == crate::renderer::Weather::WEATHER_RAINY
                        || weather.new_weather_type == crate::renderer::Weather::WEATHER_RAINY
                });
                let optional_components = general.as_deref_mut().map_or([-1; 2], |general| {
                    if let Some(snapshot) = captured_snapshot.as_ref()
                        && let Some(rules) = snapshot.vehicle_component_rules
                    {
                        choose_vehicle_optional_components_from_clump(
                            snapshot.clump_summary.as_ref(),
                            rules,
                            general,
                            rainy,
                        )
                    } else {
                        choose_vehicle_optional_components(
                            file_loader.as_deref().and_then(|file_loader| {
                                file_loader
                                    .model_info
                                    .get_model_info(model_id.max(0) as usize)
                            }),
                            general,
                            rainy,
                        )
                    }
                });
                if let Some(cutscene) = cutscene.as_mut() {
                    let created = cutscene.create_cutscene_object(model_id);
                    if let Some(created) = created {
                        cutscene.set_cutscene_optional_components(created, optional_components);
                        let snapshot = captured_snapshot.or_else(|| {
                            file_loader.as_deref().and_then(|file_loader| {
                                cutscene_model_snapshot(file_loader, model_id.max(0) as usize)
                            })
                        });
                        if let Some(snapshot) = snapshot {
                            cutscene.set_cutscene_model_snapshot(created, snapshot);
                        }
                    }
                    if created != Some(handle.max(0) as usize) {
                        warn!(
                            "SCM CREATE_CUTSCENE_OBJECT handle mismatch script={} created={:?}",
                            handle, created
                        );
                    }
                }
                // re3 RequestModel for cutscene object so DFF/TXD land before draw.
                if let (Some(streaming), Some(file_loader)) =
                    (streaming.as_mut(), file_loader.as_mut())
                {
                    streaming.request_model_with_adapter(
                        model_id.max(0) as usize,
                        crate::core::Streaming::STREAMFLAGS_PRIORITY
                            | crate::core::Streaming::STREAMFLAGS_DONT_REMOVE,
                        *file_loader,
                    );
                }
            }
            ScriptCommandEffect::SetCutsceneAnim {
                object_handle,
                anim_name,
            } => {
                if let Some(cutscene) = cutscene.as_mut() {
                    let anim = script_key_to_string(&anim_name);
                    cutscene.set_cutscene_anim(&anim, object_handle.max(0) as usize);
                }
            }
            ScriptCommandEffect::CreateCutsceneHead {
                handle,
                source_object_handle,
                model_id,
            } => {
                let captured_snapshot = take_cutscene_model_snapshot(scripts, handle);
                if let Some(cutscene) = cutscene.as_mut() {
                    let source_position = cutscene
                        .get_cutscene_object(source_object_handle.max(0) as usize)
                        .map(|object| object.position)
                        .unwrap_or(crate::math::Vector::CVector::ZERO);
                    let created = cutscene.add_cutscene_head(
                        model_id,
                        source_object_handle.max(0) as usize,
                        source_position,
                    );
                    if let Some(created) = created {
                        let snapshot = captured_snapshot.or_else(|| {
                            file_loader.as_deref().and_then(|file_loader| {
                                cutscene_model_snapshot(file_loader, model_id.max(0) as usize)
                            })
                        });
                        if let Some(snapshot) = snapshot {
                            cutscene.set_cutscene_model_snapshot(created, snapshot);
                        }
                    }
                    if created != Some(handle.max(0) as usize) {
                        warn!(
                            "SCM CREATE_CUTSCENE_HEAD handle mismatch script={} created={:?}",
                            handle, created
                        );
                    }
                }
            }
            ScriptCommandEffect::SetCutsceneHeadAnim {
                object_handle,
                anim_name,
            } => {
                if let Some(cutscene) = cutscene.as_mut() {
                    let object_id = object_handle.max(0) as usize;
                    let anim = script_key_to_string(&anim_name);
                    cutscene.set_head_anim(&anim, object_id);
                    let head_name = cutscene
                        .get_cutscene_object(object_id)
                        .and_then(|object| object.head_animation_name.clone());
                    if let Some(head_name) = head_name {
                        match load_cutscene_head_animation(&head_name) {
                            Ok(animation) => {
                                info!(
                                    "SCM SET_CUTSCENE_HEAD_ANIM '{}' decoded {} HAnim frames",
                                    head_name,
                                    animation.keyframes.len()
                                );
                                cutscene.set_head_animation_data(object_id, animation);
                            }
                            Err(error) => {
                                error!("SCM SET_CUTSCENE_HEAD_ANIM '{}' failed: {error}", head_name)
                            }
                        }
                    }
                }
            }
            ScriptCommandEffect::SetStreamingEnabled(enabled) => {
                if let Some(streaming) = streaming.as_mut() {
                    streaming.disable_streaming = !enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetStreamingEnabled(enabled));
                }
            }
            ScriptCommandEffect::RequestSpecialModel { model_id, name } => {
                let (Some(streaming), Some(file_loader)) =
                    (streaming.as_mut(), file_loader.as_mut())
                else {
                    leftover.push(ScriptCommandEffect::RequestSpecialModel { model_id, name });
                    continue;
                };
                let end = name
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(name.len());
                let model_name = String::from_utf8_lossy(&name[..end]).to_ascii_lowercase();
                let flags = crate::core::Streaming::STREAMFLAGS_DEPENDENCY
                    | crate::core::Streaming::STREAMFLAGS_SCRIPTOWNED;
                let _ = streaming.request_special_model(
                    model_id.max(0) as usize,
                    &model_name,
                    flags,
                    *file_loader,
                );
            }
            ScriptCommandEffect::MarkModelAsNoLongerNeeded { model_id } => {
                if let Some(streaming) = streaming.as_mut() {
                    streaming.set_mission_doesnt_require_model(model_id.max(0) as usize);
                } else {
                    leftover.push(ScriptCommandEffect::MarkModelAsNoLongerNeeded { model_id });
                }
            }
            ScriptCommandEffect::UnloadSpecialCharacter { char_id } => {
                if let Some(streaming) = streaming.as_mut() {
                    streaming.set_mission_doesnt_require_special_char(char_id.max(0) as usize);
                } else {
                    leftover.push(ScriptCommandEffect::UnloadSpecialCharacter { char_id });
                }
            }
            ScriptCommandEffect::RequestSpecialCharacter { slot, name } => {
                let (Some(streaming), Some(file_loader)) =
                    (streaming.as_mut(), file_loader.as_mut())
                else {
                    leftover.push(ScriptCommandEffect::RequestSpecialCharacter { slot, name });
                    continue;
                };
                let end = name
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(name.len());
                let model_name = String::from_utf8_lossy(&name[..end]).to_ascii_lowercase();
                let flags = crate::core::Streaming::STREAMFLAGS_DEPENDENCY
                    | crate::core::Streaming::STREAMFLAGS_SCRIPTOWNED;
                let _ = streaming.request_special_char(slot, &model_name, flags, *file_loader);
            }
            ScriptCommandEffect::RequestModel { model_id } => {
                if let Some(streaming) = streaming.as_deref_mut() {
                    request_scm_model(streaming, file_loader.as_deref_mut(), model_id);
                }
            }
            ScriptCommandEffect::LoadAllModelsNow => {
                // Script4.cpp COMMAND_LOAD_ALL_MODELS_NOW calls the synchronous
                // CStreaming drain at this exact point in the command stream.
                if let (Some(streaming), Some(file_loader), Some(cd_stream)) = (
                    streaming.as_deref_mut(),
                    file_loader.as_deref_mut(),
                    cd_stream.as_deref_mut(),
                ) {
                    if let Err(error) =
                        file_loader.load_all_requested_models(streaming, cd_stream, false)
                    {
                        error!("SCM LOAD_ALL_MODELS_NOW failed: {error}");
                    }
                } else {
                    // Keep the side effect pending when a harness omitted the
                    // production streaming dependencies; never pretend it loaded.
                    leftover.push(ScriptCommandEffect::LoadAllModelsNow);
                }
            }
            ScriptCommandEffect::CreatePlayer {
                index,
                position,
                needs_ground,
            } => {
                // re3 Script.cpp COMMAND_CREATE_PLAYER:
                // RequestSpecialModel; LoadAllRequestedModels;
                // SetupPlayerPed; Deactivate; SetPosition; Reactivate.
                let mut pos = position;
                if needs_ground && let Some(loader) = file_loader.as_ref() {
                    pos.z = loader
                        .world
                        .find_ground_z_for_coord(pos.x, pos.y, &loader.model_info)
                        + 1.0;
                }
                info!(
                    "SCM CREATE_PLAYER index={} pos=({:.2},{:.2},{:.2}) (SetupPlayerPed)",
                    index, pos.x, pos.y, pos.z
                );
                // re3 RequestSpecialModel(MI_PLAYER, "player", DONT_REMOVE|DEPENDENCY).
                // Actual special-dir load + LoadAllRequestedModels runs in
                // ensure_player_special_model_loaded (needs FileLoader + CdStream).
                if let Some(streaming) = streaming.as_mut() {
                    let _ = streaming.request_model(
                        crate::modelinfo::ModelIndices::MI_PLAYER,
                        crate::core::Streaming::STREAMFLAGS_DONT_REMOVE
                            | crate::core::Streaming::STREAMFLAGS_DEPENDENCY,
                    );
                }
                if let (Some(players), Some(general_state)) =
                    (world_players.as_deref_mut(), general.as_mut())
                {
                    CPlayerPed::setup_player_ped(
                        players,
                        index,
                        general_state,
                        ped_types.as_deref(),
                    );
                    CPlayerPed::set_active_in_world(players, index, false);
                    if let Some(mut player) = players.player_mut(index) {
                        player.set_position(pos);
                        player.set_heading_degrees(bridge.player_heading_degrees);
                        if let Some(slot) =
                            scripts.player_char_handles.get_mut(index.max(0) as usize)
                        {
                            *slot = player.pool_handle;
                        }
                    }
                    CPlayerPed::set_active_in_world(players, index, true);
                }
                bridge.player_spawned = true;
                bridge.player_index = index;
                bridge.player_position = pos;
                if let Some(camera) = camera.as_mut() {
                    if camera.script_mode == CameraScriptMode::FollowPlayer {
                        snap_follow_camera_resource(
                            camera,
                            bridge.player_position,
                            bridge.player_heading_degrees,
                        );
                    }
                }
                bridge.player_visible = true;
                bridge.player_control_enabled = true;
            }
            ScriptCommandEffect::SetPlayerHeading {
                player_index,
                heading_degrees,
            } => {
                bridge.player_heading_degrees = heading_degrees;
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        player.set_heading_degrees(heading_degrees);
                    }
                }
                if bridge.player_spawned {
                    if let Some(camera) = camera.as_mut() {
                        if camera.script_mode == CameraScriptMode::FollowPlayer {
                            snap_follow_camera_resource(
                                camera,
                                bridge.player_position,
                                bridge.player_heading_degrees,
                            );
                        }
                    }
                }
            }
            effect @ ScriptCommandEffect::CreatePickup { .. }
            | effect @ ScriptCommandEffect::RemovePickup { .. } => {
                // CPickups owns the fixed slot/generation state and is applied by
                // the dynamic entity phase after this global script bridge.
                leftover.push(effect);
            }
            ScriptCommandEffect::SetPlayerCoordinates {
                index,
                position,
                needs_ground,
            } => {
                if world_players
                    .as_deref()
                    .is_some_and(|players| players.vehicle_relation(index).is_some())
                {
                    // Re3 teleports `m_pMyVehicle` here. The dynamic-entity
                    // phase owns ScriptEntityWorld and performs that virtual
                    // CVehicle::Teleport boundary later in this frame.
                    leftover.push(ScriptCommandEffect::SetPlayerCoordinates {
                        index,
                        position,
                        needs_ground,
                    });
                    continue;
                }
                let mut position = position;
                if needs_ground && let Some(loader) = file_loader.as_ref() {
                    position.z = loader.world.find_ground_z_for_coord(
                        position.x,
                        position.y,
                        &loader.model_info,
                    );
                }
                position.z += crate::peds::PedPlacement::PED_FEET_OFFSET;
                bridge.player_position = position;
                bridge.player_spawned = true;
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(index) {
                        player.set_position(position);
                        // re3 `CPlayerPed::Teleport` clears the previous
                        // physical flight/fall state.  Leaving the old
                        // vertical velocity and `bIsInTheAir` bit alive after
                        // an SCM SET_PLAYER_COORDINATES made the intro player
                        // float above the next collision surface indefinitely.
                        player.ped.physical.move_speed = CVector::ZERO;
                        player.ped.physical.turn_speed = CVector::ZERO;
                        player.ped.flags.is_in_the_air = false;
                        player.ped.flags.is_standing = true;
                        player.ped.flags.is_landing = false;
                        player.ped.ped_state = PedState::PED_IDLE;
                        player.ped.move_state = eMoveState::PEDMOVE_STILL;
                    }
                }
            }
            ScriptCommandEffect::LoadScene { position } => {
                info!(
                    "SCM LOAD_SCENE pos=({:.2},{:.2},{:.2})",
                    position.x, position.y, position.z
                );
                // Script4.cpp COMMAND_LOAD_SCENE stops the timer and calls the
                // synchronous CStreaming::LoadScene before the next opcode.
                // Starting the fly-by while this was merely a view hint left
                // roads/buildings absent for most of BET on slower machines.
                if let (Some(streaming), Some(file_loader), Some(cd_stream)) = (
                    streaming.as_deref_mut(),
                    file_loader.as_deref_mut(),
                    cd_stream.as_deref_mut(),
                ) {
                    if let Err(error) = file_loader.load_scene_streaming(
                        streaming,
                        cd_stream,
                        [position.x, position.y, position.z],
                    ) {
                        error!("SCM LOAD_SCENE failed: {error}");
                    }
                } else {
                    leftover.push(ScriptCommandEffect::LoadScene { position });
                }
            }
            ScriptCommandEffect::DrawLight { .. } => {}
            ScriptCommandEffect::StartChaseScene(start_time) => {
                leftover.push(ScriptCommandEffect::StartChaseScene(start_time));
            }
            ScriptCommandEffect::StopChaseScene => {
                leftover.push(ScriptCommandEffect::StopChaseScene);
            }
            ScriptCommandEffect::RemoveCarFromChase(index) => {
                leftover.push(ScriptCommandEffect::RemoveCarFromChase(index));
            }
            ScriptCommandEffect::FlashHudObject(item) => {
                leftover.push(ScriptCommandEffect::FlashHudObject(item));
            }
            ScriptCommandEffect::ClearSmallPrints => {
                leftover.push(ScriptCommandEffect::ClearSmallPrints);
            }
            ScriptCommandEffect::LoadSplashScreen(name) => {
                let splash_name = String::from_utf8_lossy(&name)
                    .trim_end_matches('\0')
                    .to_ascii_lowercase();
                bridge.splash_screen_name = splash_name.clone();

                // re3 `COMMAND_LOAD_SPLASH_SCREEN` calls `LoadSplash`, which
                // owns a dedicated `splash` TXD slot and replaces that
                // dictionary when a different named texture is requested.
                // Keep the decoded TXD in the normal renderer cache path so
                // `DoFade` can composite the actual mission transition image.
                if !splash_name.is_empty()
                    && let Some(file_loader) = file_loader.as_deref_mut()
                {
                    let slot = file_loader.txd_store.add_txd_slot("splash");
                    let already_loaded = file_loader
                        .txd_store
                        .slot(slot)
                        .and_then(|slot| slot.texture_summary.as_ref())
                        .is_some_and(|summary| {
                            summary
                                .textures
                                .iter()
                                .any(|texture| texture.name.eq_ignore_ascii_case(&splash_name))
                        });
                    if !already_loaded {
                        file_loader.txd_store.remove_txd(slot);
                        match OriginalGtaInstall::from_config_file() {
                            Ok(install) => {
                                let mut file_mgr = install.file_mgr();
                                let path = format!("TXD\\{splash_name}.txd");
                                if file_loader.try_load_txd_slot_from_disk(
                                    &mut file_mgr,
                                    slot,
                                    Some(&path),
                                ) {
                                    if file_loader.txd_store.get_num_refs(slot) == 0 {
                                        file_loader.txd_store.add_ref(slot);
                                    }
                                    info!("SCM LOAD_SPLASH_SCREEN '{splash_name}' loaded");
                                } else {
                                    error!(
                                        "SCM LOAD_SPLASH_SCREEN '{splash_name}' could not load {path}"
                                    );
                                }
                            }
                            Err(error) => error!(
                                "SCM LOAD_SPLASH_SCREEN '{splash_name}' could not resolve GTA install: {error}"
                            ),
                        }
                    }
                }
            }
            ScriptCommandEffect::AddMovingParticleEffect {
                object_type,
                position,
                target,
                size,
                life_time,
                color,
                remove,
            } => {
                if let Some(particles) = particles.as_mut() {
                    particles.add_object(
                        object_type,
                        position,
                        target,
                        size,
                        life_time,
                        crate::objects::ParticleObject::CRGBA::new(
                            color[0], color[1], color[2], color[3],
                        ),
                        remove,
                        time_ms,
                    );
                } else {
                    leftover.push(ScriptCommandEffect::AddMovingParticleEffect {
                        object_type,
                        position,
                        target,
                        size,
                        life_time,
                        color,
                        remove,
                    });
                }
            }
            ScriptCommandEffect::RemoveParticleEffectsInArea { min, max } => {
                if let Some(particles) = particles.as_mut() {
                    particles.remove_effects_in_area(min, max);
                } else {
                    leftover.push(ScriptCommandEffect::RemoveParticleEffectsInArea { min, max });
                }
            }
            ScriptCommandEffect::SetNearClip(near_clip) => {
                if let Some(camera) = camera.as_mut() {
                    camera.near_clip_script = near_clip;
                }
            }
            ScriptCommandEffect::SetMusicDoesFade(does_fade) => {
                if let Some(camera) = camera.as_mut() {
                    camera.ignore_fading_stuff_for_music = !does_fade;
                }
            }
            ScriptCommandEffect::SetIntroIsPlaying { playing } => {
                bridge.playing_intro = playing;
                if let Some(game_state) = game_state.as_mut() {
                    game_state.playing_intro = playing;
                }
                if playing {
                    if let Some(streaming) = streaming.as_mut() {
                        streaming.remove_current_zones_models();
                    }
                } else {
                    // The audio owner applies the native ChangeMusicMode call
                    // after this world's opcode side effects, including while
                    // the cooperative bridge streaming barrier is paused.
                    leftover.push(ScriptCommandEffect::RestoreGameMusicAfterIntro);
                    // re3 Script6.cpp COMMAND_SET_INTRO_IS_PLAYING(false)
                    // requests *only* bridgefukb with STREAMFLAGS_DEPENDENCY,
                    // then blocks in LoadAllRequestedModels(false). Keep that
                    // exact request set, but use the cooperative streaming
                    // barrier so Bevy can keep rendering while game time/SCM
                    // remain stopped at this opcode boundary.
                    let (Some(streaming), Some(file_loader)) =
                        (streaming.as_mut(), file_loader.as_mut())
                    else {
                        leftover.push(ScriptCommandEffect::SetIntroIsPlaying { playing });
                        continue;
                    };
                    if let Some((bridge_model_id, _)) =
                        file_loader.model_info.get_model_info_by_name("bridgefukb")
                    {
                        streaming.request_model_with_adapter(
                            bridge_model_id,
                            STREAMFLAGS_DEPENDENCY,
                            &mut **file_loader,
                        );
                        file_loader.begin_load_all_models_streaming(streaming);
                    } else {
                        error!(
                            "SET_INTRO_IS_PLAYING(false): bridgefukb is missing from the loaded GTA model info"
                        );
                    }
                }
            }
            ScriptCommandEffect::DoFade { time_ms, direction } => {
                // re3: TheCamera.Fade(timeMs/1000.0f, direction)
                info!("SCM DO_FADE time={time_ms} dir={direction}");
                if let Some(camera) = camera.as_mut() {
                    camera.fade(time_ms as f32 / 1000.0, direction);
                }
            }
            ScriptCommandEffect::SetFadingColour { r, g, b } => {
                if let Some(camera) = camera.as_mut() {
                    camera.set_fade_colour(r, g, b);
                }
            }
            ScriptCommandEffect::SetFixedCameraPosition {
                position,
                orientation,
            } => {
                if let Some(camera) = camera.as_mut() {
                    camera.SetCamPositionForFixedMode(position, orientation);
                }
            }
            ScriptCommandEffect::PointCameraAtPoint {
                position,
                switch_style: _,
            } => {
                if let Some(camera) = camera.as_mut() {
                    camera.take_control_no_entity(position);
                }
            }
            ScriptCommandEffect::PointCameraAtPlayer {
                player_index: _,
                mode: _,
                switch_style: _,
            } => {
                if let Some(camera) = camera.as_mut() {
                    camera.script_mode = CameraScriptMode::FollowPlayer;
                }
            }
            ScriptCommandEffect::PointCameraAtChar {
                handle,
                position,
                mode,
                switch_style,
            } => {
                if let Some(camera) = camera.as_deref_mut() {
                    camera.take_control_script_entity(
                        crate::core::Camera::CameraScriptTarget::Char(ScriptHandle(handle)),
                        position,
                        mode,
                        switch_style,
                    );
                }
            }
            ScriptCommandEffect::PointCameraAtCar {
                handle,
                position,
                mode,
                switch_style,
            } => {
                if let Some(camera) = camera.as_deref_mut() {
                    camera.take_control_script_entity(
                        crate::core::Camera::CameraScriptTarget::Vehicle(ScriptHandle(handle)),
                        position,
                        mode,
                        switch_style,
                    );
                }
            }
            ScriptCommandEffect::RestoreCamera | ScriptCommandEffect::RestoreCameraJumpcut => {
                if let Some(camera) = camera.as_mut() {
                    if matches!(effect, ScriptCommandEffect::RestoreCameraJumpcut) {
                        camera.restore_with_jump_cut();
                    } else {
                        camera.restore();
                    }
                }
            }
            ScriptCommandEffect::SetCameraBehindPlayer => {
                if let Some(camera) = camera.as_mut() {
                    camera.set_camera_directly_behind_for_follow_ped(
                        bridge.player_heading_degrees.to_radians(),
                    );
                }
            }
            ScriptCommandEffect::SetCameraInFrontOfPlayer => {
                if let Some(camera) = camera.as_mut() {
                    camera.set_camera_directly_in_front_for_follow_ped(
                        bridge.player_heading_degrees.to_radians(),
                    );
                }
            }
            ScriptCommandEffect::ShakeCamera { strength, position } => {
                if let Some(camera) = camera.as_mut() {
                    if let Some(position) = position {
                        camera.cam_shake(strength, position, time_ms);
                    } else {
                        camera.cam_shake_no_position(strength, time_ms);
                    }
                } else {
                    leftover.push(ScriptCommandEffect::ShakeCamera { strength, position });
                }
            }
            ScriptCommandEffect::SetMotionBlur(blur_type) => {
                if let Some(camera) = camera.as_mut() {
                    camera.set_motion_blur(0, 0, 0, 0, blur_type);
                }
            }
            ScriptCommandEffect::ForceWeather(weather_type) => {
                if let Some(weather) = weather.as_mut() {
                    weather.force_weather(weather_type);
                }
            }
            ScriptCommandEffect::ForceWeatherNow(weather_type) => {
                if let Some(weather) = weather.as_mut() {
                    weather.force_weather_now(weather_type);
                }
            }
            ScriptCommandEffect::ReleaseWeather => {
                if let Some(weather) = weather.as_mut() {
                    weather.release_weather();
                }
            }
            ScriptCommandEffect::SetScriptsForceRain(enabled) => {
                if let Some(weather) = weather.as_mut() {
                    weather.scripts_force_rain = enabled;
                } else {
                    leftover.push(ScriptCommandEffect::SetScriptsForceRain(enabled));
                }
            }
            ScriptCommandEffect::StoreWeather => {
                if let Some(weather) = weather.as_mut() {
                    weather.store_weather_state();
                } else {
                    leftover.push(ScriptCommandEffect::StoreWeather);
                }
            }
            ScriptCommandEffect::RestoreWeather => {
                if let Some(weather) = weather.as_mut() {
                    weather.restore_weather_state();
                } else {
                    leftover.push(ScriptCommandEffect::RestoreWeather);
                }
            }
            ScriptCommandEffect::SetGangCarModel { gang, model } => {
                if let Some(gangs) = gangs.as_mut() {
                    gangs.set_gang_vehicle_model(gang, model);
                }
            }
            ScriptCommandEffect::SetGangWeapons {
                gang,
                weapon1,
                weapon2,
            } => {
                if let Some(gangs) = gangs.as_mut() {
                    gangs.set_gang_weapons(gang, weapon1, weapon2);
                }
            }
            ScriptCommandEffect::SetThreatForPedType {
                ped_type,
                threat,
                add,
            } => {
                if let Some(ped_types) = ped_types.as_mut() {
                    if add {
                        ped_types.AddThreat(ped_type as usize, threat as u32);
                    } else {
                        ped_types.RemoveThreat(ped_type as usize, threat as u32);
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetThreatForPedType {
                        ped_type,
                        threat,
                        add,
                    });
                }
            }
            ScriptCommandEffect::SetWantedLevel {
                player_index,
                level,
                no_drop,
            } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        if no_drop {
                            if level > player.wanted.get_wanted_level() {
                                player.wanted.set_wanted_level_at(level, time_ms);
                            }
                        } else {
                            player.wanted.set_wanted_level_at(level, time_ms);
                        }
                    }
                }
            }
            ScriptCommandEffect::SetMaximumWantedLevel(level) => {
                if let Some(players) = world_players.as_mut() {
                    players.set_maximum_wanted_level(level);
                } else {
                    leftover.push(ScriptCommandEffect::SetMaximumWantedLevel(level));
                }
            }
            ScriptCommandEffect::ClearWantedLevel { player_index } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        player.wanted.set_wanted_level_at(0, time_ms);
                    }
                }
            }
            ScriptCommandEffect::SetWantedMultiplier(multiplier) => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.focused_mut() {
                        player.wanted.m_fCrimeSensitivity = multiplier;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetWantedMultiplier(multiplier));
                }
            }
            ScriptCommandEffect::SetSpecialPoliceRequired { kind, required } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.focused_mut() {
                        match kind {
                            0 => player.wanted.m_bSwatRequired = required,
                            1 => player.wanted.m_bFbiRequired = required,
                            2 => player.wanted.m_bArmyRequired = required,
                            _ => {}
                        }
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetSpecialPoliceRequired { kind, required });
                }
            }
            ScriptCommandEffect::CreateCarGenerator {
                position,
                angle_degrees,
                model_index,
                color1,
                color2,
                force_spawn,
                alarm,
                doorlock,
                min_delay,
                max_delay,
            } => {
                if let Some(gens) = car_generators.as_mut() {
                    let _idx = gens.create_car_generator(
                        position,
                        angle_degrees,
                        model_index,
                        color1,
                        color2,
                        force_spawn,
                        alarm,
                        doorlock,
                        min_delay,
                        max_delay,
                    );
                } else {
                    leftover.push(ScriptCommandEffect::CreateCarGenerator {
                        position,
                        angle_degrees,
                        model_index,
                        color1,
                        color2,
                        force_spawn,
                        alarm,
                        doorlock,
                        min_delay,
                        max_delay,
                    });
                }
            }
            ScriptCommandEffect::SwitchCarGenerator { index, count } => {
                if let Some(gens) = car_generators.as_mut() {
                    gens.switch_car_generator(index, count, time_ms);
                } else {
                    leftover.push(ScriptCommandEffect::SwitchCarGenerator { index, count });
                }
            }
            ScriptCommandEffect::SwapNearestBuildingModel {
                position,
                radius,
                old_model,
                new_model,
            } => {
                let (Some(file_loader), Some(_), Some(game_state)) =
                    (file_loader.as_mut(), streaming.as_mut(), game_state.as_deref())
                else {
                    warn_once!("scm_building_swap_waiting_world_streaming_level");
                    leftover.push(effect);
                    continue;
                };
                super::ScriptWorldSwap::apply_building_swap(
                    scripts,
                    file_loader,
                    game_state.curr_level as i32,
                    super::ScriptWorldSwap::BuildingSwapRequest {
                        position,
                        radius,
                        old_model,
                        new_model,
                    },
                );
            }
            ScriptCommandEffect::SetVisibilityOfClosestObjectOfType {
                position,
                radius,
                model_id,
                visible,
            } => {
                if let Some(file_loader) = file_loader.as_mut() {
                    let mut pos = [position.x, position.y, position.z];
                    if pos[2] <= MAP_Z_LOW_LIMIT {
                        pos[2] = file_loader.world.find_ground_z_for_coord(
                            pos[0],
                            pos[1],
                            &file_loader.model_info,
                        );
                    }
                    let model_id = model_id.max(0) as usize;
                    let world_candidate = file_loader
                        .world
                        .find_nearest_entity_of_model(model_id, pos, radius)
                        .and_then(|index| {
                            let entity = file_loader.world.entities.get(index)?;
                            let dx = entity.position[0] - pos[0];
                            let dy = entity.position[1] - pos[1];
                            let dz = entity.position[2] - pos[2];
                            Some((dx * dx + dy * dy + dz * dz, ScriptInvisibilityTarget::WorldEntity(index as i32)))
                        });
                    let radius_sq = radius * radius;
                    let dynamic_candidate = script_entities
                        .as_deref()
                        .into_iter()
                        .flat_map(|entities| entities.objects.values())
                        .filter_map(|object| {
                            let entity = &object.object.physical.entity;
                            if entity.model_index.max(0) as usize != model_id {
                                return None;
                            }
                            let point = entity.get_position();
                            let dx = point.x - pos[0];
                            let dy = point.y - pos[1];
                            let dz = point.z - pos[2];
                            (dx * dx + dy * dy < radius_sq)
                                .then_some((dx * dx + dy * dy + dz * dz, object.handle))
                        })
                        .filter(|(distance_sq, _)| *distance_sq < (2.0 * radius).powi(2))
                        .min_by(|left, right| left.0.total_cmp(&right.0))
                        .map(|(distance_sq, handle)| {
                            (distance_sq, ScriptInvisibilityTarget::ScriptObject(handle))
                        });
                    let target = match (world_candidate, dynamic_candidate) {
                        (Some(world), Some(_)) if !visible => Some(world),
                        (Some(world), Some(object)) => Some(if world.0 <= object.0 { world } else { object }),
                          (world, object) => world.or(object),
                    };
                    if let Some((_, target)) = target {
                        match target {
                            ScriptInvisibilityTarget::WorldEntity(index) => {
                                if let Ok(index) = usize::try_from(index)
                                    && file_loader.world.set_entity_visible(index, visible)
                                {
                                    scripts.add_to_invisibility_swap_array(target, visible);
                                }
                            }
                            ScriptInvisibilityTarget::ScriptObject(handle) => {
                                if let Some(entities) = script_entities.as_deref_mut()
                                    && let Some(object) = entities.object_get_mut(handle)
                                {
                                    object.object.physical.entity.flags.is_visible = visible;
                                    scripts.add_to_invisibility_swap_array(target, visible);
                                }
                            }
                            ScriptInvisibilityTarget::ObjectPoolSlot(_) => {}
                        }
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetVisibilityOfClosestObjectOfType {
                        position,
                        radius,
                        model_id,
                        visible,
                    });
                }
            }
            ScriptCommandEffect::SetPoliceIgnorePlayer {
                player_index,
                ignored,
            } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        player.wanted.m_bIgnoredByCops = ignored;
                    }
                }
            }
            ScriptCommandEffect::SetEveryoneIgnorePlayer {
                player_index,
                ignored,
            } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        player.wanted.m_bIgnoredByEveryone = ignored;
                    }
                }
            }
            ScriptCommandEffect::SetPlayerControl {
                player_index: _,
                enabled,
            } => {
                bridge.player_control_enabled = enabled;
            }
            ScriptCommandEffect::SetPlayerHealth {
                player_index,
                health,
            } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(player) = players.player_mut(player_index) {
                        player.vitality.health = health;
                    }
                }
            }
            ScriptCommandEffect::AddArmourToPlayer {
                player_index,
                armour,
            } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(player) = players.player_mut(player_index) {
                        player.vitality.armour = (player.vitality.armour + armour).min(100.0);
                    }
                }
            }
            ScriptCommandEffect::SetPlayerAmmo {
                player_index,
                weapon_type,
                ammo,
            } => {
                let Some(info) = weapon_info else {
                    bridge.weapon_data_deferrals = bridge.weapon_data_deferrals.saturating_add(1);
                    leftover.push(ScriptCommandEffect::SetPlayerAmmo { player_index, weapon_type, ammo });
                    continue;
                };
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        set_ped_ammo(&mut player.ped, weapon_type, ammo, info);
                    }
                }
            }
            ScriptCommandEffect::SetCurrentPlayerWeapon {
                player_index,
                weapon_type,
            } => {
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        if let Some(slot) = player.ped.weapons.iter().rposition(|weapon| {
                            weapon.weapon_type as i32 == weapon_type
                        }) {
                            if let Some(slot_type) = script_weapon_type(slot as i32) {
                                player.selected_wep_slot = slot_type;
                            }
                        }
                    }
                }
            }
            ScriptCommandEffect::AddAmmoToPlayer {
                player_index,
                weapon_type,
                ammo,
            } => {
                let Some(info) = weapon_info else {
                    bridge.weapon_data_deferrals = bridge.weapon_data_deferrals.saturating_add(1);
                    leftover.push(ScriptCommandEffect::AddAmmoToPlayer { player_index, weapon_type, ammo });
                    continue;
                };
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        add_ped_ammo(&mut player.ped, weapon_type, ammo, info);
                    }
                }
            }
            ScriptCommandEffect::GiveWeaponToPlayer {
                player_index,
                weapon_type,
                ammo,
            } => {
                let Some(info) = weapon_info else {
                    bridge.weapon_data_deferrals = bridge.weapon_data_deferrals.saturating_add(1);
                    leftover.push(ScriptCommandEffect::GiveWeaponToPlayer { player_index, weapon_type, ammo });
                    continue;
                };
                if let Some(players) = world_players.as_mut() {
                    if let Some(mut player) = players.player_mut(player_index) {
                        if let Some(type_) = crate::peds::Ped::give_ped_weapon_with_info(&mut player.ped, weapon_type, ammo, info) {
                            player.selected_wep_slot = type_;
                        }
                    }
                }
            }
            ScriptCommandEffect::RemoveAllPlayerWeapons { player_index } => {
                if let Some(player) = world_players.as_mut()
                    .and_then(|players| players.player_mut(player_index))
                {
                    player.ped.clear_weapons();
                } else {
                    bevy::log::warn_once!("script_clear_weapons_waiting_player");
                    leftover.push(ScriptCommandEffect::RemoveAllPlayerWeapons { player_index });
                }
            }
            ScriptCommandEffect::SwitchWidescreen { enabled } => {
                bridge.widescreen = enabled;
                // re3 Script3.cpp COMMAND_SWITCH_WIDESCREEN calls
                // TheCamera.SetWideScreenOn/Off directly.  Keep the bridge
                // mirror for script probes, but make the camera authoritative
                // for HUD suppression and the cinematic borders just like C++.
                if let Some(camera) = camera.as_mut() {
                    camera.wide_screen_on = enabled;
                }
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

/// re3 CREATE_PLAYER path: RequestSpecialModel("player") + LoadAllRequestedModels.
/// Runs once player ped exists until MI_PLAYER has clump/atomic geometry.
fn player_special_model_name(name: Option<&str>) -> &str {
    match name {
        Some(name) if !name.is_empty() && !name.eq_ignore_ascii_case("null") => name,
        _ => "player",
    }
}

pub(crate) fn ensure_player_special_model_loaded(
    world_players: CWorldPlayers,
    bridge: Res<ScriptGameplayBridge>,
    mut streaming: ResMut<CStreaming>,
    mut file_loader: ResMut<CFileLoader>,
    before_rw: Option<ResMut<CGameBeforeRw>>,
) {
    if !bridge.player_spawned {
        return;
    }
    if !world_players.iter().any(|p| p.active_in_world) {
        return;
    }
    let mi = crate::modelinfo::ModelIndices::MI_PLAYER;
    // Already have renderable geometry?
    let has_geometry = file_loader
        .model_info
        .get_model_info(mi)
        .is_some_and(|model| {
            model
                .base()
                .clump_summary
                .as_ref()
                .is_some_and(|s| !s.geometries.is_empty())
                || file_loader
                    .model_info
                    .simple_model_info(mi)
                    .is_some_and(|simple| !simple.atomic_bindings.is_empty())
        });
    if has_geometry {
        return;
    }
    let Some(mut before_rw) = before_rw else {
        return;
    };
    let flags = crate::core::Streaming::STREAMFLAGS_DONT_REMOVE
        | crate::core::Streaming::STREAMFLAGS_DEPENDENCY;
    let model_name = file_loader
        .model_info
        .get_model_info(mi)
        .map(|model| model.base().name.as_str());
    // `CModelInfo::Initialise` leaves MI_PLAYER named "null" until the
    // CREATE_PLAYER special-model request replaces it. C++ does not use that
    // placeholder as a DFF name: Script.cpp always requests literal "player".
    // Once UNDRESS_CHAR swaps MI_PLAYER to e.g. "playerp", however, preserve
    // that current special-model identity instead of forcing Claude back in.
    let model_name = player_special_model_name(model_name).to_owned();
    // re3 CStreaming::RequestSpecialModel(MI_PLAYER, model_name, flags)
    if streaming.extra_objects_dir.find_item(&model_name).is_none() {
        error!(
            "ensure_player_special_model_loaded: original GTA extra objects DIR has no '{model_name}' DFF"
        );
        return;
    }
    let _ = streaming.request_special_model(mi, &model_name, flags, &mut *file_loader);
    if let Err(error) =
        file_loader.load_all_requested_models(&mut streaming, &mut before_rw.cd_stream, false)
    {
        warn!("ensure_player_special_model_loaded: LoadAllRequestedModels failed: {error}");
        return;
    }
    let loaded = file_loader
        .model_info
        .get_model_info(mi)
        .is_some_and(|model| model.base().clump_summary.is_some())
        || file_loader
            .model_info
            .simple_model_info(mi)
            .is_some_and(|simple| !simple.atomic_bindings.is_empty());
    if loaded {
        info!("ensure_player_special_model_loaded: MI_PLAYER '{model_name}' geometry ready");
    }
}

#[cfg(test)]
mod player_special_model_tests {
    use super::player_special_model_name;

    #[test]
    fn create_player_placeholder_resolves_to_default_player_model() {
        assert_eq!(player_special_model_name(None), "player");
        assert_eq!(player_special_model_name(Some("")), "player");
        assert_eq!(player_special_model_name(Some("null")), "player");
        assert_eq!(player_special_model_name(Some("NULL")), "player");
    }

    #[test]
    fn scripted_player_special_model_is_not_clobbered() {
        assert_eq!(player_special_model_name(Some("player")), "player");
        assert_eq!(player_special_model_name(Some("playerp")), "playerp");
    }
}

#[cfg(test)]
mod cutscene_model_snapshot_tests {
    use super::{freeze_cutscene_model_snapshot, take_cutscene_model_snapshot};
    use crate::control::ScriptTypes::CTheScripts;
    use crate::core::FileLoader::CFileLoader;
    use rustgta3_formats::formats::Script::ScmFile;

    #[test]
    fn reused_special_slot_keeps_the_model_identity_from_create_time() {
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        let mut file_loader = CFileLoader::default();
        let colt1_txd = file_loader.txd_store.add_txd_slot("colt1");
        {
            let model = file_loader.model_info.add_clump_model(188);
            model.base.set_model_name("colt1");
            model.base.txd_slot = colt1_txd as i16;
        }

        freeze_cutscene_model_snapshot(&mut scripts, &file_loader, 8, 188);

        let colt2_txd = file_loader.txd_store.add_txd_slot("colt2");
        {
            let model = file_loader
                .model_info
                .get_model_info_mut(188)
                .expect("special cutscene slot");
            model.base_mut().set_model_name("colt2");
            model.base_mut().txd_slot = colt2_txd as i16;
        }

        let snapshot =
            take_cutscene_model_snapshot(&mut scripts, 8).expect("create-time model snapshot");
        assert_eq!(snapshot.model_name, "colt1");
        assert_eq!(snapshot.txd_slot, colt1_txd as i16);
        assert!(scripts.pending_cutscene_model_snapshots.is_empty());
    }
}
