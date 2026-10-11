//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Split from control/Script.rs (verbatim, parity-preserving).

#[allow(unused_imports)]
use super::Script::{
    ScriptEntityWorld, ScriptGameplayBridge, ScriptRadarBlipIndex, ScriptTaxiLightState,
    ScriptVehicleEntity, clear_exciting_stuff_from_area_runtime, script_key_string,
};
#[allow(unused_imports)]
use super::ScriptTypes::{CTheScripts, ScriptCommandEffect, ScriptCreateCarRequest};
#[allow(unused_imports)]
use crate::audio::soundlist::*;
#[allow(unused_imports)]
use crate::control::CarCtrl::CCarCtrl;
#[allow(unused_imports)]
use crate::control::Garages::EGarageType;
#[allow(unused_imports)]
use crate::control::Pickups::{CPickups, PICKUP_IN_SHOP, PICKUP_IN_SHOP_OUT_OF_STOCK};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Pools::CPoolHandle;
#[allow(unused_imports)]
// FAZ8a bridge consumed by FAZ8d: ScriptEntityIndex, ScriptEntityWorld,
// ScriptObjectEntity, ScriptPedEntity, ScriptVehicleEntity, spawn_script_object
// and remove_script_object now live in this module.
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPools};
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
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
use crate::weapons::WeaponType::{WEAPONTYPE_TOTAL_INVENTORY_WEAPONS, eWeaponType};
#[allow(unused_imports)]
use bevy::input::{ButtonInput, mouse::MouseButton};
#[allow(unused_imports)]
use bevy::prelude::*;
#[allow(unused_imports)]
use bevy::prelude::{KeyCode, error};
#[allow(unused_imports)]
use re3bevy_core::core::Timer::CTimer;
#[allow(unused_imports)]
use re3bevy_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

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
    apply_script_ped_wander_path, give_ped_weapon, set_ped_ammo, set_script_ped_chat,
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
use bevy::ecs::system::SystemParam;
#[allow(unused_imports)]
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
#[allow(unused_imports)]
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use re3bevy_assets::streaming::CdStream::CdStream;
#[allow(unused_imports)]
use re3bevy_core::core::GameState::{CGameState, ELevelName};

pub(crate) fn apply_script_pacman_effects(
    mut scripts: ResMut<CTheScripts>,
    mut pacman: Option<ResMut<CPacManPickups>>,
    mut script_entities: ScriptEntityWorld,
    mut commands: Commands,
    mut pools: Option<ResMut<CPools>>,
    mut general: ResMut<CGeneral>,
    world_players: CWorldPlayers,
    file_loader: Res<CFileLoader>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        let Some(pacman) = pacman.as_mut() else {
            leftover.push(effect);
            continue;
        };
        match effect {
            ScriptCommandEffect::StartPacManRace(race) => {
                if let Some(spawns) = pacman.start_pac_man_race(race)
                    && let Some(pools) = pools.as_mut()
                {
                    spawn_pacman_objects(
                        &spawns,
                        pacman,
                        &mut commands,
                        &mut script_entities.index,
                        pools,
                        &mut general,
                        &file_loader,
                    );
                }
            }
            ScriptCommandEffect::StartPacManRecord => {
                if let Some(position) = world_players.focused().map(|player| player.position()) {
                    pacman.start_pac_man_record(position);
                }
            }
            ScriptCommandEffect::ClearPacMan => {
                let handles = pacman.clear_pm_pickups();
                for handle in handles {
                    remove_pacman_object(
                        handle,
                        &mut commands,
                        &mut script_entities.index,
                        pools.as_deref_mut(),
                    );
                }
            }
            ScriptCommandEffect::StartPacManScramble {
                position,
                scramble_mult,
                count,
            } => {
                let spawns = pacman.start_pac_man_scramble(
                    position,
                    scramble_mult,
                    count,
                    || general.get_random_number(),
                    |x, y| {
                        let (point, entity_index) =
                            file_loader.world.process_vertical_line_static_with_entity(
                                [x, y, 1000.0],
                                -1000.0,
                                &file_loader.model_info,
                            )?;
                        let road = file_loader.world.entities.get(entity_index)?;
                        (road.entity_type == EEntityType::Building
                            && road.treadable_pool_slot.is_some())
                        .then_some(point.point.z)
                    },
                );
                if let Some(pools) = pools.as_mut() {
                    spawn_pacman_objects(
                        &spawns,
                        pacman,
                        &mut commands,
                        &mut script_entities.index,
                        pools,
                        &mut general,
                        &file_loader,
                    );
                }
            }
            ScriptCommandEffect::ResetPacManPillsCarried => {
                if let Some(handle) =
                    focused_player_vehicle_handle(&world_players, &script_entities)
                    && let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle)
                {
                    reset_vehicle_pacman_load(&mut vehicle.base_mut());
                }
            }
            ScriptCommandEffect::ResetPacManPillsEaten => {
                pacman.pills_eaten_in_race = 0;
            }
            ScriptCommandEffect::AddPowerPill(position) => {
                pacman.generate_one_pm_pickup(position);
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

pub(crate) fn apply_script_weapon_area_effects(
    mut scripts: ResMut<CTheScripts>,
    mut projectiles: Option<ResMut<CProjectileInfoPool>>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::DestroyProjectilesInArea { min, max } => {
                if let Some(projectiles) = projectiles.as_mut() {
                    projectiles.IsProjectileInRange(min.x, max.x, min.y, max.y, min.z, max.z, true);
                } else {
                    leftover.push(ScriptCommandEffect::DestroyProjectilesInArea { min, max });
                }
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

pub(crate) fn apply_script_aircraft_effects(
    mut scripts: ResMut<CTheScripts>,
    mut planes: Option<ResMut<CPlaneRuntime>>,
    mut helis: Option<ResMut<CHeliRuntime>>,
    mut script_entities: ScriptEntityWorld,
    mut pools: Option<ResMut<CPools>>,
    mut general: ResMut<CGeneral>,
    timer: Res<CTimer>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    for effect in effects {
        match effect {
            ScriptCommandEffect::StartDrugRun => {
                if let (Some(planes), Some(pools)) = (planes.as_mut(), pools.as_mut()) {
                    if planes.cesna_mission_status == CesnaMissionStatus::Flying {
                        let handle = {
                            script_entities
                                .vehicles
                                .iter()
                                .find_map(|(&handle, vehicle)| match vehicle {
                                    crate::control::ScriptEntities::ScriptVehicleRef::Plane { plane, .. }
                                        if plane.is_drug_run_cesna =>
                                    {
                                        Some(handle)
                                    }
                                    _ => None,
                                })
                        };
                        if let Some(handle) = handle {
                            script_entities.remove_car(pools, handle);
                        }
                    }
                    planes.create_incoming_cesna(timer.time_in_milliseconds());
                    let _ = script_entities.insert_plane(
                        pools,
                        MI_DEADDODO,
                        0,
                        timer.time_in_milliseconds(),
                        &mut general,
                        true,
                        false,
                    );
                    scripts.drug_plane_position = planes.drug_plane_coordinates();
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StartDrugRun);
                }
            }
            ScriptCommandEffect::StartDrugDropOff => {
                if let (Some(planes), Some(pools)) = (planes.as_mut(), pools.as_mut()) {
                    if planes.drop_off_mission_status == CesnaMissionStatus::Flying {
                        let handle = {
                            script_entities
                                .vehicles
                                .iter()
                                .find_map(|(&handle, vehicle)| match vehicle {
                                    crate::control::ScriptEntities::ScriptVehicleRef::Plane { plane, .. }
                                        if plane.is_drop_off_cesna =>
                                    {
                                        Some(handle)
                                    }
                                    _ => None,
                                })
                        };
                        if let Some(handle) = handle {
                            script_entities.remove_car(pools, handle);
                        }
                    }
                    planes.create_drop_off_cesna(timer.time_in_milliseconds());
                    let _ = script_entities.insert_plane(
                        pools,
                        MI_DEADDODO,
                        0,
                        timer.time_in_milliseconds(),
                        &mut general,
                        false,
                        true,
                    );
                    scripts.drop_off_plane_position = planes.drop_off_plane_coordinates();
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StartDrugDropOff);
                }
            }
            ScriptCommandEffect::StartCatalinaHeli => {
                if let Some(helis) = helis.as_mut() {
                    helis.start_catalina_fly_by();
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StartCatalinaHeli);
                }
            }
            ScriptCommandEffect::SwitchHelicopter(enabled) => {
                if let Some(helis) = helis.as_mut() {
                    helis.script_heli_on = enabled;
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SwitchHelicopter(enabled));
                }
            }
            ScriptCommandEffect::CatalinaHeliTakeOff => {
                let applied = helis
                    .as_ref()
                    .and_then(|helis| helis.catalina_handle())
                    .and_then(|handle| script_entities.vehicles.get_mut(&script_entities.index, &handle))
                    .and_then(|vehicle| match vehicle {
                        crate::control::ScriptEntities::ScriptVehicleMut::Heli { heli, .. } => Some(heli),
                        _ => None,
                    })
                    .is_some_and(|mut heli| {
                        heli.path_state = 8;
                        true
                    });
                if !applied {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::CatalinaHeliTakeOff);
                }
            }
            ScriptCommandEffect::RemoveCatalinaHeli => {
                if let Some(helis) = helis.as_mut() {
                    if let Some(handle) = helis.clear_catalina() {
                        if let Some(pools) = pools.as_mut() {
                            script_entities.remove_car(pools, handle);
                        } else {
                            script_entities.despawn_vehicle(handle);
                        }
                    }
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::RemoveCatalinaHeli);
                }
            }
            ScriptCommandEffect::CatalinaHeliFlyAway => {
                let applied = helis
                    .as_ref()
                    .and_then(|helis| helis.catalina_handle())
                    .and_then(|handle| script_entities.vehicles.get_mut(&script_entities.index, &handle))
                    .and_then(|vehicle| match vehicle {
                        crate::control::ScriptEntities::ScriptVehicleMut::Heli { heli, .. } => Some(heli),
                        _ => None,
                    })
                    .is_some_and(|mut heli| {
                        heli.path_state = 12;
                        true
                    });
                if !applied {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::CatalinaHeliFlyAway);
                }
            }
            other => scripts.pending_effects.push(other),
        }
    }
}

pub(crate) fn apply_script_population_effects(
    mut scripts: ResMut<CTheScripts>,
    mut population: Option<ResMut<CPopulation>>,
    mut car_ctrl: Option<ResMut<CCarCtrl>>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    for effect in effects {
        match effect {
            ScriptCommandEffect::MissionCleanupProcessed => {
                if let Some(population) = population.as_mut() {
                    population.all_random_peds_this_type = -1;
                    population.ped_density_multiplier = 1.0;
                }
                if let Some(car_ctrl) = car_ctrl.as_mut() {
                    car_ctrl.car_density_multiplier = 1.0;
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::MissionCleanupProcessed);
            }
            ScriptCommandEffect::SetPedDensityMultiplier(multiplier) => {
                if let Some(population) = population.as_mut() {
                    population.ped_density_multiplier = multiplier;
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetPedDensityMultiplier(multiplier));
                }
            }
            ScriptCommandEffect::ForceRandomPedType(ped_type) => {
                if let Some(population) = population.as_mut() {
                    population.all_random_peds_this_type = ped_type;
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::ForceRandomPedType(ped_type));
                }
            }
            ScriptCommandEffect::SetCarDensityMultiplier(multiplier) => {
                if let Some(car_ctrl) = car_ctrl.as_mut() {
                    car_ctrl.car_density_multiplier = multiplier;
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetCarDensityMultiplier(multiplier));
                }
            }
            other => scripts.pending_effects.push(other),
        }
    }
}

pub(crate) fn prepare_gta_frame(
    mut font: Option<ResMut<CFont>>,
    mut point_lights: Option<ResMut<CPointLights>>,
) {
    if let Some(font) = font.as_mut() {
        font.init_per_frame();
    }
    if let Some(point_lights) = point_lights.as_mut() {
        // re3 `Idle`: `CPointLights::InitPerFrame` runs immediately before
        // `CGame::Process`, so script/fire lights never leak into later frames.
        point_lights.init_per_frame();
    }
}

pub(crate) fn apply_script_hud_effects(
    mut scripts: ResMut<CTheScripts>,
    mut hud: ResMut<CHud>,
    mut messages: ResMut<CMessages>,
    mut onscreen_timer: ResMut<COnscreenTimer>,
    mut pager: Option<ResMut<CPager>>,
    text: Option<Res<CText>>,
    timer: Res<CTimer>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    for effect in effects {
        match effect {
            ScriptCommandEffect::MissionCleanupProcessed => {
                hud.item_to_flash = -1;
                hud.set_help_message(&[], false);
                onscreen_timer.disabled = false;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::MissionCleanupProcessed);
            }
            ScriptCommandEffect::FlashHudObject(item) => hud.item_to_flash = item as i16,
            ScriptCommandEffect::ClearSmallPrints => messages.clear_small_messages_only(),
            ScriptCommandEffect::AddPagerMessage {
                key,
                speed,
                priority,
                unused,
                numbers,
            } => {
                if let (Some(pager), Some(text)) = (pager.as_mut(), text.as_ref()) {
                    let text = text.get_key_bytes(&key);
                    if let Some(numbers) = numbers {
                        pager.add_message_with_number(
                            text,
                            numbers,
                            speed,
                            priority,
                            unused,
                            timer.time_in_milliseconds(),
                        );
                    } else {
                        pager.add_message(
                            text,
                            speed,
                            priority,
                            unused,
                            timer.time_in_milliseconds(),
                        );
                    }
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::AddPagerMessage {
                            key,
                            speed,
                            priority,
                            unused,
                            numbers,
                        });
                }
            }
            ScriptCommandEffect::AddOnscreenClock {
                variable_offset,
                key,
            } => {
                onscreen_timer.AddClock(
                    variable_offset,
                    key.map(script_key_string).unwrap_or_default(),
                );
            }
            ScriptCommandEffect::ClearOnscreenClock { variable_offset } => {
                onscreen_timer.ClearClock(variable_offset);
            }
            ScriptCommandEffect::AddOnscreenCounter {
                variable_offset,
                counter_type,
                key,
            } => {
                onscreen_timer.AddCounter(
                    variable_offset,
                    counter_type as u16,
                    key.map(script_key_string).unwrap_or_default(),
                );
            }
            ScriptCommandEffect::ClearOnscreenCounter { variable_offset } => {
                onscreen_timer.ClearCounter(variable_offset);
            }
            ScriptCommandEffect::FreezeOnscreenTimer(disabled) => {
                onscreen_timer.disabled = disabled;
            }
            other => scripts.pending_effects.push(other),
        }
    }
}

pub(crate) fn apply_script_chase_effects(
    mut scripts: ResMut<CTheScripts>,
    mut chase: ResMut<CRecordDataForChase>,
    mut timer: ResMut<CTimer>,
    install: Option<Res<OriginalGtaInstall>>,
    mut file_loader: ResMut<CFileLoader>,
    mut streaming: Option<ResMut<CStreaming>>,
    mut pools: Option<ResMut<CPools>>,
    mut script_entities: ScriptEntityWorld,
    handling_manager: Res<cHandlingDataMgr>,
    mut general: ResMut<CGeneral>,
    taxi_lights: Res<ScriptTaxiLightState>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    for effect in effects {
        match effect {
            ScriptCommandEffect::StartChaseScene(start_time) => {
                let Some(install) = install.as_deref() else {
                    error!("START_CHASE_SCENE ignored because original GTA install is unavailable");
                    continue;
                };
                let (Some(streaming), Some(pools)) =
                    (streaming.as_deref_mut(), pools.as_deref_mut())
                else {
                    error!("START_CHASE_SCENE ignored because streaming or pools are unavailable");
                    continue;
                };
                if !chase.streaming_prepared {
                    for handle in chase.clean_up_chase_scene() {
                        script_entities.remove_car(pools, handle);
                    }
                    // Script4.cpp COMMAND_START_CHASE_SCENE performs
                    // CStreaming::DeleteAllRwObjects before the blocking
                    // chase setup. Missing this left old map/LOD mirrors alive
                    // across the intro/chase transition.
                    file_loader.delete_all_world_rw_objects();
                    chase.streaming_prepared = true;
                }
                file_loader.request_models_for_runtime(
                    streaming,
                    CHASE_CARS.iter().map(|(_, model, ..)| *model),
                    STREAMFLAGS_DEPENDENCY,
                );
                file_loader.begin_load_all_models_streaming(streaming);
                if file_loader.script_streaming_barrier_in_progress() {
                    if !timer.get_is_code_paused() {
                        timer.set_code_pause(true);
                        file_loader.set_script_streaming_owns_code_pause(true);
                    }
                    // C++ does not return from START_CHASE_SCENE until these
                    // cars are resident. Keep the effect pending so creation
                    // happens only after the cooperative barrier has drained.
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StartChaseScene(start_time));
                    continue;
                }
                chase.start_chase_scene(start_time, timer.time_in_milliseconds());
                for &(index, model_id, position, angle, colour1, colour2) in &CHASE_CARS {
                    if !streaming.has_model_loaded(model_id) {
                        warn!("START_CHASE_SCENE model {model_id} was not loaded for slot {index}");
                        continue;
                    }
                    if let Err(error) = chase.load_recording(install, index) {
                        warn!("START_CHASE_SCENE chase{index}.dat failed: {error}");
                        continue;
                    }
                    let request = ScriptCreateCarRequest {
                        model_id: model_id as i32,
                        position,
                        time_ms: timer.time_in_milliseconds(),
                        clear_space_for_mission_entity: true,
                    };
                    let model_info = file_loader.model_info.get_model_info(model_id);
                    let Some(handle) = script_entities.insert_script_car(
                        pools,
                        request,
                        model_info,
                        Some(&handling_manager),
                        &mut general,
                        &file_loader,
                        taxi_lights.all_enabled,
                    ) else {
                        warn!("START_CHASE_SCENE vehicle pool is full at slot {index}");
                        continue;
                    };
                    if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                        script_entities.vehicles.get_mut(&script_entities.index, &handle)
                    {
                        let mut matrix = CMatrix::unity();
                        matrix.set_rotate_z_only(angle.to_radians());
                        matrix.position = position;
                        car.vehicle.core.physical.entity.set_matrix(matrix);
                        car.vehicle
                            .set_status(eEntityStatus::STATUS_PLAYER_PLAYBACKFROMBUFFER);
                        // Chase DAT owns the transform. Running normal world
                        // collision on all recorded cars only rolls that pose
                        // back before the next recording sample overwrites it.
                        car.vehicle.core.physical.entity.flags.uses_collision = false;
                        car.vehicle.appearance.current_colour1 = colour1;
                        car.vehicle.appearance.current_colour2 = colour2;
                        car.vehicle.core.flags.is_locked = false;
                        car.vehicle.entry.door_lock = eCarLock::CARLOCK_UNLOCKED;
                    }
                    chase.car_handles[index] = Some(handle);
                    chase.chase_car_present[index] = true;
                }
            }
            ScriptCommandEffect::StopChaseScene => {
                if let Some(pools) = pools.as_deref_mut() {
                    for handle in chase.clean_up_chase_scene() {
                        script_entities.remove_car(pools, handle);
                    }
                }
            }
            ScriptCommandEffect::RemoveCarFromChase(index) => {
                if let (Some(handle), Some(pools)) =
                    (chase.remove_car_from_chase(index), pools.as_deref_mut())
                {
                    script_entities.remove_car(pools, handle);
                }
            }
            other => scripts.pending_effects.push(other),
        }
    }
}

pub(crate) fn process_chase_scene(
    mut chase: ResMut<CRecordDataForChase>,
    mut script_entities: ScriptEntityWorld,
    timer: Res<CTimer>,
) {
    let lerp = |from: CVector, to: CVector, amount: f32| {
        CVector::new(
            from.x + (to.x - from.x) * amount,
            from.y + (to.y - from.y) * amount,
            from.z + (to.z - from.z) * amount,
        )
    };
    if chase.status != 3 {
        return;
    }
    chase.time_multiplier += timer.time_step_in_seconds().max(0.0);
    for index in 0..crate::control::Record::NUM_CHASE_CARS {
        let (Some(handle), Some((frame, next, blend))) =
            (chase.car_handles[index], chase.playback_sample(index))
        else {
            continue;
        };
        let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. }) =
            script_entities.vehicles.get_mut(&script_entities.index, &handle)
        else {
            continue;
        };
        let mut matrix = CMatrix::unity();
        matrix.right = frame.right;
        matrix.forward = frame.forward;
        matrix.up = frame.right.cross(frame.forward);
        matrix.position = frame.position;
        if let Some(next) = next {
            matrix.right = lerp(matrix.right, next.right, blend);
            matrix.forward = lerp(matrix.forward, next.forward, blend);
            matrix.up = lerp(matrix.up, next.right.cross(next.forward), blend);
            matrix.position = lerp(matrix.position, next.position, blend);
        } else if index == 0 {
            matrix.position.z += 0.2;
        }
        car.vehicle.core.physical.entity.set_matrix(matrix);
        car.vehicle.core.physical.move_speed = if next.is_some() {
            frame.velocity
        } else {
            CVector::ZERO
        };
        car.vehicle.core.physical.turn_speed = CVector::ZERO;
        car.vehicle.drive.steer_angle = frame.steer_angle;
        car.vehicle.drive.gas_pedal = if next.is_some() { frame.gas_pedal } else { 0.0 };
        car.vehicle.drive.brake_pedal = if next.is_some() {
            frame.brake_pedal
        } else {
            0.0
        };
        car.vehicle.drive.is_handbrake_on = next.is_some() && frame.handbrake;
        car.vehicle.vitality.health = car.vehicle.vitality.health.min(500.0);
    }
}

pub(crate) fn apply_script_light_effects(
    mut scripts: ResMut<CTheScripts>,
    mut lights: Option<ResMut<crate::renderer::PointLights::CPointLights>>,
    mut coronas: Option<ResMut<CCoronas>>,
    camera: Option<Res<CCamera>>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    for effect in effects {
        match effect {
            ScriptCommandEffect::DrawLight { position, color } => {
                if let (Some(lights), Some(camera)) = (lights.as_mut(), camera.as_ref()) {
                    lights.add_light(
                        crate::renderer::PointLights::LIGHT_POINT,
                        position,
                        CVector::ZERO,
                        12.0,
                        color[0] as f32 / 255.0,
                        color[1] as f32 / 255.0,
                        color[2] as f32 / 255.0,
                        0,
                        true,
                        camera.position,
                    );
                }
            }
            ScriptCommandEffect::DrawCorona {
                id,
                position,
                size,
                texture_type,
                flare_type,
                colour,
            } => {
                if let Some(coronas) = coronas.as_mut() {
                    coronas.register_corona(
                        id,
                        colour[0],
                        colour[1],
                        colour[2],
                        255,
                        position,
                        size,
                        150.0,
                        texture_type as u8,
                        flare_type as i8,
                        1,
                        0,
                        0,
                        0.0,
                    );
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::DrawCorona {
                            id,
                            position,
                            size,
                            texture_type,
                            flare_type,
                            colour,
                        });
                }
            }
            other => scripts.pending_effects.push(other),
        }
    }
}

pub(crate) fn apply_script_credit_effects(
    mut scripts: ResMut<CTheScripts>,
    mut credits: Option<ResMut<CCredits>>,
    timer: Res<CTimer>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::StartCredits => {
                if let Some(credits) = credits.as_mut() {
                    credits.start(timer.time_in_milliseconds());
                    scripts.credits_done = false;
                } else {
                    leftover.push(ScriptCommandEffect::StartCredits);
                }
            }
            ScriptCommandEffect::StopCredits => {
                if let Some(credits) = credits.as_mut() {
                    credits.stop();
                    scripts.credits_done = true;
                } else {
                    leftover.push(ScriptCommandEffect::StopCredits);
                }
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

pub(crate) fn apply_script_restart_effects(
    mut scripts: ResMut<CTheScripts>,
    mut restart: Option<ResMut<CRestart>>,
    mut garages: Option<ResMut<CGarages>>,
    mut cranes: Option<ResMut<CCranes>>,
    mut rubbish: Option<ResMut<CRubbish>>,
    mut radar_blips: ResMut<ScriptRadarBlipIndex>,
    mut file_loader: ResMut<CFileLoader>,
    mut game_state: ResMut<CGameState>,
    mut streaming: ResMut<CStreaming>,
    mut before_rw: ResMut<CGameBeforeRw>,
    mut collision: ResMut<CCollision>,
    mut timer: ResMut<CTimer>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::AddRestartPoint {
                police,
                mut position,
                heading,
                needs_ground,
            } => {
                let Some(restart) = restart.as_mut() else {
                    leftover.push(ScriptCommandEffect::AddRestartPoint {
                        police,
                        position,
                        heading,
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
                if police {
                    restart.add_police_restart_point(position, heading);
                } else {
                    restart.add_hospital_restart_point(position, heading);
                }
            }
            ScriptCommandEffect::OverrideNextRestart {
                mut position,
                heading,
                needs_ground,
            } => {
                let Some(restart) = restart.as_mut() else {
                    leftover.push(ScriptCommandEffect::OverrideNextRestart {
                        position,
                        heading,
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
                restart.override_next_restart(position, heading);
            }
            ScriptCommandEffect::CancelOverrideRestart => {
                if let Some(restart) = restart.as_mut() {
                    restart.cancel_override_restart();
                } else {
                    leftover.push(ScriptCommandEffect::CancelOverrideRestart);
                }
            }
            ScriptCommandEffect::OverrideHospitalLevel(level) => {
                if let Some(restart) = restart.as_mut() {
                    restart.override_hospital_level = script_level(level);
                } else {
                    leftover.push(ScriptCommandEffect::OverrideHospitalLevel(level));
                }
            }
            ScriptCommandEffect::OverridePoliceLevel(level) => {
                if let Some(restart) = restart.as_mut() {
                    restart.override_police_station_level = script_level(level);
                } else {
                    leftover.push(ScriptCommandEffect::OverridePoliceLevel(level));
                }
            }
            ScriptCommandEffect::AddGarage {
                handle,
                min,
                max,
                garage_type,
                target_model,
            } => {
                if let Some(garages) = garages.as_mut() {
                    let actual = garages.add_one(
                        min,
                        max,
                        EGarageType::from_re3(garage_type as u8),
                        target_model,
                    );
                    debug_assert_eq!(actual, handle);
                } else {
                    leftover.push(ScriptCommandEffect::AddGarage {
                        handle,
                        min,
                        max,
                        garage_type,
                        target_model,
                    });
                }
            }
            ScriptCommandEffect::SetGarageCameraFollowsPlayer { handle } => {
                if garages
                    .as_mut()
                    .is_none_or(|garages| !garages.set_leave_camera_for_this_garage(handle))
                {
                    leftover.push(ScriptCommandEffect::SetGarageCameraFollowsPlayer { handle });
                }
            }
            ScriptCommandEffect::SetRotatingGarageDoor { handle } => {
                if garages
                    .as_mut()
                    .is_none_or(|garages| !garages.set_garage_door_to_rotate(handle))
                {
                    leftover.push(ScriptCommandEffect::SetRotatingGarageDoor { handle });
                }
            }
            ScriptCommandEffect::ActivateCrane {
                pickup_min,
                pickup_max,
                dropoff,
                heading_radians,
                crane_position,
                is_crusher,
                is_military,
            } => {
                if cranes.as_mut().is_none_or(|cranes| {
                    !cranes.activate_crane(
                        &file_loader.world,
                        pickup_min,
                        pickup_max,
                        dropoff,
                        heading_radians,
                        is_crusher,
                        is_military,
                        crane_position,
                    )
                }) {
                    leftover.push(ScriptCommandEffect::ActivateCrane {
                        pickup_min,
                        pickup_max,
                        dropoff,
                        heading_radians,
                        crane_position,
                        is_crusher,
                        is_military,
                    });
                }
            }
            ScriptCommandEffect::DeactivateCrane { position } => {
                if cranes
                    .as_mut()
                    .is_none_or(|cranes| !cranes.deactivate_crane(&file_loader.world, position))
                {
                    leftover.push(ScriptCommandEffect::DeactivateCrane { position });
                }
            }
            ScriptCommandEffect::ChangeGarageType {
                handle,
                garage_type,
                target_model,
            } => {
                if garages.as_mut().is_none_or(|garages| {
                    !garages.change_garage_type(handle, garage_type, target_model)
                }) {
                    leftover.push(ScriptCommandEffect::ChangeGarageType {
                        handle,
                        garage_type,
                        target_model,
                    });
                }
            }
            ScriptCommandEffect::SetTargetCarForMissionGarage {
                handle,
                vehicle_handle,
            } => {
                let target_vehicle_slot = vehicle_handle.map(|handle| CPoolHandle(handle).index());
                if garages.as_mut().is_none_or(|garages| {
                    !garages.set_target_car_for_mission_garage(handle, target_vehicle_slot)
                }) {
                    leftover.push(ScriptCommandEffect::SetTargetCarForMissionGarage {
                        handle,
                        vehicle_handle,
                    });
                }
            }
            ScriptCommandEffect::OpenGarage { handle } => {
                if garages
                    .as_mut()
                    .is_none_or(|garages| !garages.open_garage(handle))
                {
                    leftover.push(ScriptCommandEffect::OpenGarage { handle });
                }
            }
            ScriptCommandEffect::CloseGarage { handle } => {
                if garages
                    .as_mut()
                    .is_none_or(|garages| !garages.close_garage(handle))
                {
                    leftover.push(ScriptCommandEffect::CloseGarage { handle });
                }
            }
            ScriptCommandEffect::SetGarageActive { handle, active } => {
                let applied = garages.as_mut().is_some_and(|garages| {
                    if active {
                        garages.activate_garage(handle)
                    } else {
                        garages.deactivate_garage(handle)
                    }
                });
                if !applied {
                    leftover.push(ScriptCommandEffect::SetGarageActive { handle, active });
                }
            }
            ScriptCommandEffect::ClearGarageResprayHappened { handle } => {
                if garages
                    .as_mut()
                    .and_then(|garages| garages.take_respray_happened(handle))
                    .is_none()
                {
                    leftover.push(ScriptCommandEffect::ClearGarageResprayHappened { handle });
                }
            }
            ScriptCommandEffect::LoadCollision { level } => {
                let new_level = script_level(level);
                let old_level = collision.collision_in_memory;
                game_state.curr_level = new_level;
                // Script6.cpp COMMAND_LOAD_COLLISION sets CGame::currLevel;
                // CTheZones::m_CurrLevel remains position-derived.
                file_loader.remove_unused_big_buildings(&mut streaming, level);
                file_loader.remove_unused_buildings(&mut streaming, level);
                if old_level != new_level {
                    file_loader
                        .model_info
                        .remove_col_models_from_other_levels(level);
                    let dat_file = game_state.dat_file.clone();
                    let load_result = if new_level == ELevelName::Generic {
                        Ok(Vec::new())
                    } else {
                        file_loader.load_collision_from_dat_file(
                            &mut before_rw.file_mgr,
                            &dat_file,
                            level,
                        )
                    };
                    match load_result {
                        Ok(_) => {
                            collision.collision_in_memory = new_level;
                            collision.last_load_error = None;
                        }
                        Err(error) => collision.last_load_error = Some(error.to_string()),
                    }
                }
                // COMMAND_LOAD_COLLISION uses RequestIslands, not the broader
                // RequestBigBuildings path used by LOAD_COLLISION_WITH_SCREEN.
                file_loader.request_islands(&mut streaming, level);
                file_loader.begin_load_all_models_streaming(&streaming);
                if file_loader.script_streaming_barrier_in_progress() {
                    if !timer.get_is_code_paused() {
                        timer.set_code_pause(true);
                        file_loader.set_script_streaming_owns_code_pause(true);
                    }
                    // Keep the opcode's domain side effect pending until its
                    // C++ LoadAllRequestedModels barrier is genuinely done.
                    leftover.push(ScriptCommandEffect::LoadCollision { level });
                    continue;
                }
            }
            ScriptCommandEffect::SetWorldProcessingEnabled(enabled) => {
                file_loader.world.process_cutscene_only = !enabled;
            }
            ScriptCommandEffect::SetRubbishVisible(visible) => {
                if let Some(rubbish) = rubbish.as_mut() {
                    rubbish.set_visibility(visible);
                } else {
                    leftover.push(ScriptCommandEffect::SetRubbishVisible(visible));
                }
            }
            ScriptCommandEffect::CreateCoordinateBlip {
                handle,
                position,
                sprite,
                contact_point,
                colour,
                display,
                scale,
            } => {
                let blip_type = if contact_point {
                    crate::core::Radar::eBlipType::BLIP_CONTACT_POINT
                } else {
                    crate::core::Radar::eBlipType::BLIP_COORD
                };
                let display = match display {
                    1 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_MARKER_ONLY,
                    2 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BLIP_ONLY,
                    3 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BOTH,
                    _ => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_NEITHER,
                };
                let actual = file_loader
                    .radar
                    .set_coord_blip(blip_type, position, colour, display);
                file_loader
                    .radar
                    .set_blip_sprite(actual, crate::core::Radar::eRadarSprite::from_re3(sprite));
                file_loader.radar.change_blip_scale(actual, scale);
                radar_blips.0.insert(handle, actual);
            }
            ScriptCommandEffect::CreateEntityBlip {
                handle,
                entity_handle,
                entity_type,
                colour,
                display,
                sprite,
                scale,
            } => {
                let blip_type = match entity_type {
                    1 => crate::core::Radar::eBlipType::BLIP_CAR,
                    2 => crate::core::Radar::eBlipType::BLIP_CHAR,
                    _ => crate::core::Radar::eBlipType::BLIP_OBJECT,
                };
                let display = match display {
                    1 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_MARKER_ONLY,
                    2 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BLIP_ONLY,
                    3 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BOTH,
                    _ => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_NEITHER,
                };
                let actual =
                    file_loader
                        .radar
                        .set_entity_blip(blip_type, entity_handle, colour, display);
                file_loader
                    .radar
                    .set_blip_sprite(actual, crate::core::Radar::eRadarSprite::from_re3(sprite));
                file_loader.radar.change_blip_scale(actual, scale);
                radar_blips.0.insert(handle, actual);
            }
            ScriptCommandEffect::RemoveBlip { handle } => {
                let actual = radar_blips.0.remove(&handle).unwrap_or(handle);
                file_loader.radar.clear_blip(actual);
            }
            ScriptCommandEffect::ChangeBlipColour { handle, colour } => {
                let actual = radar_blips.0.get(&handle).copied().unwrap_or(handle);
                file_loader.radar.change_blip_colour(actual, colour);
            }
            ScriptCommandEffect::ChangeBlipBrightness { handle, bright } => {
                let actual = radar_blips.0.get(&handle).copied().unwrap_or(handle);
                file_loader.radar.change_blip_brightness(actual, bright);
            }
            ScriptCommandEffect::ChangeBlipScale { handle, scale } => {
                let actual = radar_blips.0.get(&handle).copied().unwrap_or(handle);
                file_loader.radar.change_blip_scale(actual, scale);
            }
            ScriptCommandEffect::ChangeBlipDisplay { handle, display } => {
                let actual = radar_blips.0.get(&handle).copied().unwrap_or(handle);
                let display = match display {
                    1 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_MARKER_ONLY,
                    2 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BLIP_ONLY,
                    3 => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_BOTH,
                    _ => crate::core::Radar::eBlipDisplay::BLIP_DISPLAY_NEITHER,
                };
                file_loader.radar.change_blip_display(actual, display);
            }
            ScriptCommandEffect::SetZonePedInfo {
                label,
                day,
                ped_density,
                gang_density,
                cop_density,
            } => {
                let _ = file_loader.world.zones.set_zone_ped_info(
                    &label,
                    day,
                    ped_density,
                    gang_density,
                    cop_density,
                );
            }
            ScriptCommandEffect::SetZoneCarInfo {
                label,
                day,
                car_density,
                gang_counts,
                cop_count,
                car_counts,
            } => {
                let _ = file_loader.world.zones.set_zone_car_info(
                    &label,
                    day,
                    car_density,
                    gang_counts,
                    cop_count,
                    car_counts,
                );
            }
            ScriptCommandEffect::SetZoneGroup {
                label,
                day,
                ped_group,
            } => {
                let _ = file_loader
                    .world
                    .zones
                    .set_zone_group(&label, day, ped_group);
            }
            ScriptCommandEffect::SwitchRoadsInArea {
                min,
                max,
                pedestrian,
                disable,
            } => {
                if pedestrian {
                    file_loader.path_find.switch_ped_roads_off_in_area(
                        min.x, max.x, min.y, max.y, min.z, max.z, disable,
                    );
                } else {
                    file_loader.path_find.switch_roads_off_in_area(
                        min.x, max.x, min.y, max.y, min.z, max.z, disable,
                    );
                }
            }
            ScriptCommandEffect::SwitchRoadsInAngledArea {
                p1,
                p2,
                length,
                pedestrian,
                enable,
            } => {
                file_loader.path_find.switch_roads_in_angled_area(
                    p1,
                    p2,
                    length,
                    if pedestrian { 0 } else { 1 },
                    enable,
                );
            }
            ScriptCommandEffect::MarkRoadsBetweenLevels {
                min,
                max,
                pedestrian,
            } => {
                if pedestrian {
                    file_loader.path_find.ped_mark_roads_between_levels_in_area(
                        min.x, max.x, min.y, max.y, min.z, max.z,
                    );
                } else {
                    file_loader.path_find.mark_roads_between_levels_in_area(
                        min.x, max.x, min.y, max.y, min.z, max.z,
                    );
                }
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

pub(crate) fn script_level(level: i32) -> ELevelName {
    match level {
        1 => ELevelName::Industrial,
        2 => ELevelName::Commercial,
        3 => ELevelName::Suburban,
        -1 => ELevelName::Ignore,
        _ => ELevelName::Generic,
    }
}

#[derive(SystemParam)]
pub(crate) struct GameLogicResources<'w, 's> {
    player_info: Option<ResMut<'w, CPlayerInfo>>,
    players: CWorldPlayers<'w, 's>,
    script_entities: ScriptEntityWorld<'w, 's>,
    pools: Option<ResMut<'w, CPools>>,
    car_ctrl: Option<ResMut<'w, CCarCtrl>>,
    darkel: Option<ResMut<'w, CDarkel>>,
    population: Option<ResMut<'w, CPopulation>>,
    weapon_info: Option<Res<'w, CWeaponInfoStore>>,
    messages: Option<ResMut<'w, CMessages>>,
    text: Option<Res<'w, CText>>,
    stats: Option<ResMut<'w, CStats>>,
    clock: ResMut<'w, CClock>,
    pickups: Option<ResMut<'w, CPickups>>,
    restart: Option<ResMut<'w, CRestart>>,
    garages: Option<ResMut<'w, CGarages>>,
    references: ResMut<'w, CReferences>,
    cleared_references: MessageWriter<'w, crate::entities::EntityReference::EntityReferencesCleared>,
    fire_manager: ResMut<'w, CFireManager>,
    explosions: ResMut<'w, CExplosionPool>,
    projectiles: ResMut<'w, CProjectileInfoPool>,
    shadows: ResMut<'w, CShadows>,
    camera: Option<ResMut<'w, CCamera>>,
    cutscene: Option<Res<'w, CCutsceneMgr>>,
    file_loader: ResMut<'w, CFileLoader>,
    streaming: ResMut<'w, CStreaming>,
    timer: ResMut<'w, CTimer>,
}

pub(crate) fn update_game_logic(resources: GameLogicResources) {
    let GameLogicResources {
        mut player_info,
        mut players,
        mut script_entities,
        mut pools,
        mut car_ctrl,
        mut darkel,
        mut population,
        weapon_info,
        mut messages,
        text,
        mut stats,
        mut clock,
        mut pickups,
        mut restart,
        mut garages,
        mut references,
        mut cleared_references,
        mut fire_manager,
        mut explosions,
        mut projectiles,
        mut shadows,
        mut camera,
        cutscene,
        mut file_loader,
        mut streaming,
        mut timer,
    } = resources;
    if cutscene
        .as_ref()
        .is_some_and(|cutscene| cutscene.is_cutscene_processing())
    {
        return;
    }
    let focused_index = players.player_in_focus() as i32;
    let focused_relation = players.relation(focused_index);
    let focused_player_entity = players.player_entity(focused_index);
    let missing_player_info = player_info.is_none();
    let missing_player = players.player(focused_index).is_none();
    let missing_darkel = darkel.is_none();
    let missing_population = population.is_none();
    let missing_weapon_info = weapon_info.is_none();
    let missing_messages = messages.is_none();
    let missing_text = text.is_none();
    let missing_stats = stats.is_none();
    let (
        Some(player_info),
        Some(mut player),
        Some(darkel),
        Some(population),
        Some(weapon_info),
        Some(messages),
        Some(text),
        Some(stats),
    ) = (
        player_info.as_mut(),
        players.focused_mut(),
        darkel.as_mut(),
        population.as_mut(),
        weapon_info.as_ref(),
        messages.as_mut(),
        text.as_ref(),
        stats.as_mut(),
    )
    else {
        bevy::log::warn_once!(
            target: "re3bevy::game_logic",
            missing_player_info, missing_player, missing_darkel, missing_population,
            missing_weapon_info, missing_messages, missing_text, missing_stats,
            "CGameLogic::Update skipped because required gameplay resources are missing"
        );
        return;
    };

    // The PLAYING switch arm does not fall through after KillPlayer/ArrestPlayer.
    if player_info.m_WBState == crate::core::PlayerInfo::eWastedBustedState::WBSTATE_PLAYING {
        match player.ped.ped_state {
            PedState::PED_DEAD => {
                player.adrenaline_active = false;
                player.adrenaline_time = 0;
                player_info.kill_player(
                    timer.time_in_milliseconds(),
                    &mut player,
                    focused_relation.is_some(),
                    darkel,
                    population,
                    weapon_info,
                    messages,
                    text,
                    stats,
                );
            }
            PedState::PED_ARRESTED => {
                player.adrenaline_active = false;
                player.adrenaline_time = 0;
                player_info.arrest_player(
                    timer.time_in_milliseconds(),
                    &mut player,
                    focused_relation.is_some(),
                    darkel,
                    population,
                    weapon_info,
                    messages,
                    text,
                    stats,
                );
            }
            _ => {}
        }
        return;
    }

    let wanted_level = player.wanted.get_wanted_level();
    let intents = CGameLogic::update_wasted_busted_state(
        player_info,
        timer.time_in_milliseconds(),
        timer.previous_time_in_milliseconds(),
        wanted_level,
    );
    for intent in intents {
        match intent {
            GameLogicIntent::FadeOut { colour, seconds } => {
                if let Some(camera) = camera.as_mut() {
                    camera.set_fade_colour(colour[0] as i32, colour[1] as i32, colour[2] as i32);
                    camera.fade(seconds, 0);
                }
            }
            GameLogicIntent::Resurrect {
                kind,
                clear_weapons,
                ..
            } => {
                let Some(restart) = restart.as_mut() else {
                    bevy::log::warn_once!(
                        target: "re3bevy::game_logic",
                        "CGameLogic resurrection skipped because CRestart is missing"
                    );
                    return;
                };
                let old_position = player.position();
                let node_index = file_loader.path_find.find_node_closest_to_coors(
                    old_position,
                    crate::control::PathFind::PATH_PED,
                    999_999.9,
                    false,
                    false,
                );
                let fallback = usize::try_from(node_index)
                    .ok()
                    .and_then(|index| file_loader.path_find.path_nodes.get(index))
                    .map(|node| node.pos);
                let level_at = |position: CVector| {
                    script_level(
                        file_loader
                            .world
                            .zones
                            .get_level_from_position([position.x, position.y, position.z]),
                    )
                };
                let (position, heading) = match kind {
                    ResurrectionKind::Hospital => restart
                        .find_closest_hospital_restart_point_with_level(
                            old_position,
                            level_at,
                            fallback,
                        ),
                    ResurrectionKind::Police | ResurrectionKind::FailedCriticalMission => restart
                        .find_closest_police_restart_point_with_level(
                            old_position,
                            level_at,
                            fallback,
                        ),
                };
                restart.override_hospital_level = ELevelName::Generic;
                restart.override_police_station_level = ELevelName::Generic;
                if !matches!(kind, ResurrectionKind::FailedCriticalMission) {
                    if let Some(pickups) = pickups.as_mut() {
                        CGameLogic::pass_time(
                            crate::control::GameLogic::RESURRECTION_TIME_ADVANCE_MINUTES,
                            timer.time_in_milliseconds(),
                            &mut clock,
                            stats,
                            pickups,
                        );
                    }
                }
                if clear_weapons {
                    player.ped.clear_weapons();
                    player.selected_wep_slot = eWeaponType::WEAPONTYPE_UNARMED;
                }
                messages.clear_messages();
                let mut previous_vehicle_handle = None;
                if let Some(relation) = focused_relation {
                    cleared_references.write(crate::entities::EntityReference::EntityReferencesCleared(relation.vehicle));
                    fire_manager.clear_actor_references(relation.vehicle,
                        script_entities.vehicle_handle_for_entity(relation.vehicle)
                            .map(crate::core::Fire::ScriptFireTarget::Vehicle));
                    if let Some(handle) =
                        script_entities.vehicle_handle_for_entity(relation.vehicle)
                    {
                        previous_vehicle_handle = Some(handle);
                        crate::peds::PedObstacle::clear_obstacle_references(
                            &mut script_entities.commands, relation.vehicle,
                        );
                        if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                            let mut base = vehicle.base_mut();
                            if matches!(relation.seat, VehicleSeat::Driver) {
                                if base.physical.entity.status != eEntityStatus::STATUS_WRECKED {
                                    base.set_status(eEntityStatus::STATUS_ABANDONED);
                                }
                            }
                        }
                    }
                }
                if let Some(player_entity) = focused_player_entity {
                    cleared_references.write(crate::entities::EntityReference::EntityReferencesCleared(player_entity));
                    fire_manager.clear_actor_references(player_entity,
                        Some(crate::core::Fire::ScriptFireTarget::Char(player.pool_handle)));
                    crate::peds::PedObstacle::clear_obstacle_references(
                        &mut script_entities.commands, player_entity,
                    );
                    script_entities.clear_relation_for_ped_entity(player_entity);
                    script_entities
                        .commands
                        .entity(player_entity)
                        .remove::<CPedVehicleAnimation>();
                }
                if let Some(fire) = player.vitality.fire {
                    fire_manager.extinguish_slot(fire);
                }
                if let Some(handle) =
                    previous_vehicle_handle.and_then(|handle| u32::try_from(handle).ok())
                {
                    references.resolve_references(handle);
                }
                if let Ok(handle) = u32::try_from(player.pool_handle) {
                    references.resolve_references(handle);
                }
                if let Some(pools) = pools.as_deref_mut() {
                    clear_exciting_stuff_from_area_runtime(
                        &mut script_entities,
                        pools,
                        Some(&mut *population),
                        car_ctrl.as_mut().map(|value| &mut **value),
                        Some(&mut *fire_manager),
                        Some(&mut *explosions),
                        Some(&mut *projectiles),
                        Some(&mut *shadows),
                        old_position,
                        4_000.0,
                        true,
                    );
                }
                CGameLogic::restore_player_stuff_during_resurrection(&mut player, position, heading);
                timer.set_time_scale(1.0);
                let resurrection_position = player.position();
                if let Some(pools) = pools.as_deref_mut() {
                    clear_exciting_stuff_from_area_runtime(
                        &mut script_entities,
                        pools,
                        Some(&mut *population),
                        car_ctrl.as_mut().map(|value| &mut **value),
                        Some(&mut *fire_manager),
                        Some(&mut *explosions),
                        Some(&mut *projectiles),
                        Some(&mut *shadows),
                        resurrection_position,
                        4_000.0,
                        true,
                    );
                }
                // re3 CGameLogic::SortOutStreamingAndMemory. GTA III's
                // RemoveUnusedModelsInLoadedList step is empty; flush stale
                // requests and start the same CStreaming::LoadScene barrier
                // at the resurrected player position.
                streaming.flush_request_list();
                file_loader.begin_load_scene_streaming(
                    &mut streaming,
                    [
                        resurrection_position.x,
                        resurrection_position.y,
                        resurrection_position.z,
                    ],
                );
                if file_loader.script_streaming_barrier_in_progress() && !timer.get_is_code_paused()
                {
                    timer.set_code_pause(true);
                    file_loader.set_script_streaming_owns_code_pause(true);
                }
                if let Some(garages) = garages.as_mut() {
                    garages.player_arrested_or_died();
                }
                stats.kills_since_last_checkpoint = 0;
                if let Some(camera) = camera.as_mut() {
                    camera.cam_directly_in_front = true;
                    camera.cam_directly_behind = false;
                    camera.set_motion_blur(0, 0, 0, 0, 0);
                    let (colour, should_fade) = match kind {
                        ResurrectionKind::Hospital => {
                            ([200, 200, 200], restart.fade_in_after_next_death)
                        }
                        ResurrectionKind::Police => ([0, 0, 0], restart.fade_in_after_next_arrest),
                        ResurrectionKind::FailedCriticalMission => ([0, 0, 0], true),
                    };
                    if should_fade {
                        camera.set_fade_colour(colour[0], colour[1], colour[2]);
                        camera.fade(4.0, 1);
                    }
                }
                match kind {
                    ResurrectionKind::Hospital if !restart.fade_in_after_next_death => {
                        restart.fade_in_after_next_death = true;
                    }
                    ResurrectionKind::Police if !restart.fade_in_after_next_arrest => {
                        restart.fade_in_after_next_arrest = true;
                    }
                    _ => {}
                }
            }
        }
    }
}

pub(crate) fn undress_script_player(
    world_players: &mut dyn WorldPlayerAccess,
    handle: i32,
) -> Option<i32> {
    for index in 0..crate::peds::PlayerPed::MAX_PLAYERS {
        let Some(mut player) = world_players.player_mut(index as i32) else {
            continue;
        };
        if player.pool_handle != handle {
            continue;
        }
        player.ped.physical.entity.flags.is_visible = false;
        player.active_in_world = false;
        return Some(crate::modelinfo::ModelIndices::MI_PLAYER as i32);
    }
    None
}

pub(crate) fn dress_script_player(world_players: &mut dyn WorldPlayerAccess, handle: i32) -> bool {
    for index in 0..crate::peds::PlayerPed::MAX_PLAYERS {
        let Some(mut player) = world_players.player_mut(index as i32) else {
            continue;
        };
        if player.pool_handle != handle {
            continue;
        }
        player.ped.physical.entity.flags.is_visible = true;
        player.active_in_world = true;
        return true;
    }
    false
}

/// Apply the `CPlayerPed` branch of re3 `UNDRESS_CHAR` / `DRESS_CHAR`.
/// Non-player peds stay queued for the CPools-owned dynamic entity pass.
pub fn apply_script_player_clothing_effects(
    scripts: &mut CTheScripts,
    world_players: &mut dyn WorldPlayerAccess,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::with_capacity(effects.len());
    for effect in effects {
        match effect {
            ScriptCommandEffect::UndressChar { handle, model_name } => {
                if let Some(model_id) = undress_script_player(world_players, handle) {
                    leftover.push(ScriptCommandEffect::RequestSpecialModel {
                        model_id,
                        name: model_name,
                    });
                } else {
                    leftover.push(ScriptCommandEffect::UndressChar { handle, model_name });
                }
            }
            ScriptCommandEffect::DressChar { handle } => {
                if !dress_script_player(world_players, handle) {
                    leftover.push(ScriptCommandEffect::DressChar { handle });
                }
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}
