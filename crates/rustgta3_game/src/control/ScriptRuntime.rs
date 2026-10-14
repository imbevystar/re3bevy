//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Split from control/Script.rs (verbatim, parity-preserving).

#[allow(unused_imports)]
use super::Script::{
    ScriptEntityIndex, ScriptEntityWorld, ScriptGameplayBridge, ScriptObjectEntity,
    ScriptPedEntity, ScriptPedStore, ScriptPickupIndex, ScriptTaxiLightState, ScriptVehicleEntity,
    ScriptVehicleStore, apply_script_effects_to_world,
    apply_script_effects_to_world_with_streaming, apply_script_object_motion,
    flush_immediate_script_streaming_effect, refresh_script_streaming_snapshots,
    script_char_entity_snapshot, script_object_entity_snapshot, script_object_snapshot,
    script_ped_snapshot_from_entity, script_vehicle_entity_snapshot, script_vehicle_is_stopped,
    script_vehicle_snapshot_from_entity, set_script_snapshot_bounds, spawn_script_object,
};
#[allow(unused_imports)]
use super::ScriptCommands::{
    COMMAND_CLEAR_CUTSCENE, COMMAND_DRESS_CHAR, COMMAND_LOAD_ALL_MODELS_NOW, COMMAND_LOAD_CUTSCENE,
    COMMAND_LOAD_SCENE, COMMAND_MISSION_HAS_FINISHED, COMMAND_START_CUTSCENE, COMMAND_UNDRESS_CHAR,
};
#[allow(unused_imports)]
use super::ScriptTypes::{
    CTheScripts, ScriptAreaEntityBounds, ScriptCommandEffect, ScriptEntityMutation,
    ScriptEntityMutationResult, ScriptEntityRuntimeAccess, ScriptEntitySnapshot,
    ScriptGarageVehicleContainment, ScriptHandle, ScriptMissionEntityBounds,
    ScriptObjectMotionRequest, ScriptPathNodeSnapshot, ScriptPlayerSnapshot, ScriptStatsEffect,
    ScriptWorldStateSnapshot, ScriptZoneBounds,
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
use crate::audio::soundlist::*;
#[allow(unused_imports)]
use crate::collision::ColModel::{CColBox, CColSphere, CColVector};
#[allow(unused_imports)]
use crate::collision::Collision::CCollision;
#[allow(unused_imports)]
use crate::control::AutoPilot::{
    eCarDrivingStyle, eCarMission, eCarTempAction, script_car_mission,
};
#[allow(unused_imports)]
use crate::control::CarCtrl::CCarCtrl;
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
use crate::control::Phones::CPhoneInfo;
#[allow(unused_imports)]
use crate::control::Pickups::{
    CPacManPickups, PICKUP_COLLECTABLE1, PICKUP_FLOATINGPACKAGE, remove_pacman_object,
    reset_vehicle_pacman_load, spawn_pacman_objects,
};
#[allow(unused_imports)]
use crate::control::Pickups::{CPickups, PICKUP_IN_SHOP, PICKUP_IN_SHOP_OUT_OF_STOCK};
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
use crate::core::FileLoader::CFileLoader;
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
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPoolHandle, CPools};
#[allow(unused_imports)]
use crate::core::References::CReferences;
#[allow(unused_imports)]
use crate::core::Stats::CStats;
#[allow(unused_imports)]
use crate::core::Streaming::{CStreaming, STREAMFLAGS_DEPENDENCY};
#[allow(unused_imports)]
use crate::core::World::EEntityType;
#[allow(unused_imports)]
use crate::entities::Entity::{eEntityStatus, script_entity_status};
#[allow(unused_imports)]
use crate::entities::Physical::{GTA_PHYSICS_HZ, physical_collision_records};
#[allow(unused_imports)]
use crate::math::Matrix::CMatrix;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_DEADDODO;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_PICKUP_BONUS;
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::script_vehicle_base_z;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::{choose_vehicle_optional_components, eVehicleType};
#[allow(unused_imports)]
use crate::objects::Object::TEMP_OBJECT;
#[allow(unused_imports)]
use crate::objects::Object::{CObject, MISSION_OBJECT};
#[allow(unused_imports)]
use crate::objects::ObjectData::CObjectInfo;
#[allow(unused_imports)]
use crate::objects::ParticleObject::CParticleObjectRuntime;
#[allow(unused_imports)]
use crate::peds::Gangs::CGangs;
#[allow(unused_imports)]
use crate::peds::Ped::{
    CPed, MISSION_CHAR, PedState, apply_ped_objective, eMoveState, eObjective, ped_weapon_ammo_snapshot,
    script_ped_can_see_entity, script_ped_can_see_player,
};
#[allow(unused_imports)]
use crate::peds::Ped::{ped_seat_world_position, player_seat_index};
#[allow(unused_imports)]
use crate::peds::PedType::script_ped_type;
#[allow(unused_imports)]
use crate::peds::PedType::{CPedTypeRuntime, PED_STATS_TYPES, ePedType};
#[allow(unused_imports)]
use crate::peds::PedRoutes::CPedRoutes;
#[allow(unused_imports)]
use crate::peds::PlayerPed::{CWorldPlayers, WorldPlayerAccess};
#[allow(unused_imports)]
use crate::peds::Population::{CPopulation, remove_ped_from_population_counts};
#[allow(unused_imports)]
use crate::peds::VehicleRelations::VehicleSeat;
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
use crate::vehicles::Automobile::CAutomobile;
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
use crate::vehicles::Vehicle::CVehicle;
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
use crate::weapons::WeaponType::WEAPONTYPE_TOTAL_INVENTORY_WEAPONS;
#[allow(unused_imports)]
use crate::weapons::WeaponType::script_weapon_type;
#[allow(unused_imports)]
use bevy::ecs::system::SystemParam;
#[allow(unused_imports)]
use bevy::input::{ButtonInput, mouse::MouseButton};
#[allow(unused_imports)]
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
#[allow(unused_imports)]
use bevy::prelude::{KeyCode, error};
#[allow(unused_imports)]
use rustgta3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use rustgta3_core::core::GameState::{CGameState, ELevelName};
#[allow(unused_imports)]
use rustgta3_core::core::Timer::CTimer;
#[allow(unused_imports)]
use rustgta3_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};
#[derive(SystemParam)]
pub(crate) struct ActiveScriptResources<'w, 's> {
    scripts: ResMut<'w, CTheScripts>,
    mission_debug: Option<ResMut<'w, super::ScriptDebug::GtaMissionDebugSwitcher>>,
    fire_manager: Res<'w, CFireManager>,
    general: ResMut<'w, CGeneral>,
    clock: ResMut<'w, CClock>,
    timer: ResMut<'w, CTimer>,
    ped_routes: Res<'w, CPedRoutes>,
    bridge: Res<'w, ScriptGameplayBridge>,
    world_players: CWorldPlayers<'w, 's>,
    script_entities: ScriptEntityWorld<'w, 's>,
    taxi_lights: ResMut<'w, ScriptTaxiLightState>,
    commands: Commands<'w, 's>,
    pickup_index: Res<'w, ScriptPickupIndex>,
    pools: Option<ResMut<'w, CPools>>,
    file_loader: ResMut<'w, CFileLoader>,
    handling_manager: Option<Res<'w, cHandlingDataMgr>>,
    pads: Res<'w, CPads>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    player_info: Option<Res<'w, CPlayerInfo>>,
    camera: Option<ResMut<'w, CCamera>>,
    cutscene: Option<Res<'w, CCutsceneMgr>>,
    dmaudio: Option<Res<'w, cDMAudio>>,
    replay: Option<Res<'w, CReplay>>,
    darkel: Option<Res<'w, CDarkel>>,
    ped_types: Option<Res<'w, CPedTypeRuntime>>,
    pickups: Option<ResMut<'w, CPickups>>,
    pacman_pickups: Option<Res<'w, CPacManPickups>>,
    garages: Option<Res<'w, CGarages>>,
    game_state: Res<'w, CGameState>,
    credits: Option<Res<'w, CCredits>>,
    streaming: ResMut<'w, CStreaming>,
    collision: Option<Res<'w, CCollision>>,
    remote: Res<'w, CRemote>,
    car_ctrl: Option<Res<'w, CCarCtrl>>,
    planes: Option<Res<'w, CPlaneRuntime>>,
    helis: Option<Res<'w, CHeliRuntime>>,
    projectiles: Option<Res<'w, CProjectileInfoPool>>,
    explosions: Option<Res<'w, CExplosionPool>>,
    menu: Option<Res<'w, CMenuManager>>,
    cranes: Option<Res<'w, CCranes>>,
    phone_info: Option<Res<'w, CPhoneInfo>>,
    threat_reaction_range: ResMut<'w, crate::peds::PedNearby::PedThreatReactionRange>,
}

/// Command-local visibility overlay for entities spawned through deferred
/// `Commands`. It is owned by one SCM execution system invocation, never by a
/// Resource, and is discarded after commands are flushed into the ECS world.
pub(crate) struct SamePassOverlay<T> {
    pub(crate) entries: Vec<(i32, T)>,
}

#[derive(Debug, Clone, Copy)]
struct CollectiveOccupant {
    seat_order: u8,
    ped_handle: i32,
}

fn collective_seat_order(seat: VehicleSeat) -> u8 {
    match seat {
        VehicleSeat::Driver => 0,
        VehicleSeat::Passenger(index) => index.saturating_add(1),
    }
}

pub(crate) struct ActiveScriptEntityRuntime<'a, 'w, 's> {
    script_entities: &'a mut ScriptEntityWorld<'w, 's>,
    fire_manager: &'a CFireManager,
    taxi_lights: &'a mut ScriptTaxiLightState,
    commands: &'a mut Commands<'w, 's>,
    pools: &'a mut Option<ResMut<'w, CPools>>,
    file_loader: &'a CFileLoader,
    handling_manager: Option<&'a cHandlingDataMgr>,
    ped_types: Option<&'a CPedTypeRuntime>,
    garages: Option<&'a CGarages>,
    world_players: &'a CWorldPlayers<'w, 's>,
    live_pos: CVector,
    player_spawned: bool,
    player_index: i32,
    pool_handle: i32,
    player_char_handles: &'a [i32],
    player_current_vehicle_handle: Option<i32>,
    live_stopped: bool,
    script_time_step: f32,
    pending_peds: &'a mut SamePassOverlay<ScriptPedEntity>,
    pending_vehicles: &'a mut SamePassOverlay<ScriptVehicleEntity>,
    /// Same-SCM-pass overlay for deferred `InVehicle` component inserts.
    /// This is transient execution state only; authoritative runtime ownership
    /// remains the ped's ECS `InVehicle` component after Commands are applied.
    pending_relations: &'a mut SamePassOverlay<(i32, VehicleSeat)>,
    ped_routes: &'a CPedRoutes,
    now_ms: u32,
    pending_objects: &'a mut SamePassOverlay<ScriptObjectEntity>,
    threat_reaction_range: &'a mut crate::peds::PedNearby::PedThreatReactionRange,
    pending_static_world_promotions: &'a mut Vec<usize>,
}

impl ScriptEntityRuntimeAccess for ActiveScriptEntityRuntime<'_, '_, '_> {
    fn supports_mutation(&self, _request: &ScriptEntityMutation) -> bool {
        true
    }

    fn fire_slot_is_free(&mut self, slot: usize) -> Option<bool> {
        Some(
            self.fire_manager
                .fires
                .get(slot)
                .is_some_and(|fire| !fire.active && !fire.is_script_fire),
        )
    }

    fn script_fire_is_extinguished(&mut self, slot: i32) -> Option<bool> {
        Some(self.fire_manager.is_script_fire_extinguished(slot))
    }

    fn mutate(
        &mut self,
        request: ScriptEntityMutation,
        general: Option<&mut CGeneral>,
    ) -> Option<ScriptEntityMutationResult> {
        match request {
            ScriptEntityMutation::ObjectMotion(request) => {
                let handle = match request {
                    ScriptObjectMotionRequest::Rotate { handle, .. }
                    | ScriptObjectMotionRequest::Slide { handle, .. } => handle,
                };
                let bevy_entity = self.script_entities.index.object(handle)?;
                if let Ok(mut object) = self.script_entities.objects.get_entity_mut(bevy_entity) {
                    let completed = apply_script_object_motion(
                        request,
                        &mut object,
                        &self.script_entities.vehicles,
                        &self.script_entities.peds,
                        self.world_players,
                        self.file_loader,
                    )?;
                    return Some(ScriptEntityMutationResult::ObjectMotion(completed));
                }

                // A Commands spawn is not query-visible until the schedule
                // flushes. Keep only that newly-created component in this
                // pass-local cache so later SCM commands see the synchronous
                // CWorld state without introducing a Resource-owned world.
                let mut object = self.pending_objects.get_mut(&handle)?.as_mut();
                let completed = apply_script_object_motion(
                    request,
                    &mut object,
                    &self.script_entities.vehicles,
                    &self.script_entities.peds,
                    self.world_players,
                    self.file_loader,
                )?;
                self.commands.entity(bevy_entity).insert(object.object.clone());
                Some(ScriptEntityMutationResult::ObjectMotion(completed))
            }
            ScriptEntityMutation::CreateChar(mut request) => {
                let relation = if let Some(vehicle_handle) = request.vehicle_handle {
                    let seat = match request.passenger_seat {
                        None => VehicleSeat::Driver,
                        Some(seat) if seat >= 0 => {
                            VehicleSeat::Passenger(u8::try_from(seat).unwrap_or(u8::MAX))
                        }
                        Some(_) => {
                            // re3 `CVehicle::AddPassenger(ped)` picks the first
                            // empty seat below `m_nNumMaxPassengers`. Deferred
                            // Commands are not query-visible during this SCM
                            // pass, so include the transient overlay as well as
                            // already-authoritative ECS relations.
                            let max_passengers = self
                                .pending_vehicles
                                .get(&vehicle_handle).map(ScriptVehicleEntity::as_ref)
                                .or_else(|| self.script_entities.vehicles.get(&self.script_entities.index, &vehicle_handle))
                                .map(|vehicle| usize::from(vehicle.base().entry.num_max_passengers))?;
                            let vehicle_entity =
                                self.script_entities.vehicle_entity(vehicle_handle)?;
                            let mut occupied =
                                [false; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS];
                            for (_, relation_seat) in
                                self.script_entities.relation_occupants(vehicle_entity)
                            {
                                if let VehicleSeat::Passenger(index) = relation_seat
                                    && let Some(slot) = occupied.get_mut(usize::from(index))
                                {
                                    *slot = true;
                                }
                            }
                            for &(pending_vehicle_handle, relation_seat) in
                                self.pending_relations.values()
                            {
                                if pending_vehicle_handle != vehicle_handle {
                                    continue;
                                }
                                if let VehicleSeat::Passenger(index) = relation_seat
                                    && let Some(slot) = occupied.get_mut(usize::from(index))
                                {
                                    *slot = true;
                                }
                            }
                            let passenger = (0..max_passengers.min(occupied.len()))
                                .find(|&index| !occupied[index])
                                .and_then(|index| u8::try_from(index).ok())
                                // AddPassenger can fail when every authored
                                // passenger slot is occupied. Keep the ped's
                                // in-vehicle relation without fabricating a
                                // valid reverse occupant slot.
                                .unwrap_or(u8::MAX);
                            request.passenger_seat = Some(i32::from(passenger));
                            VehicleSeat::Passenger(passenger)
                        }
                    };
                    Some((vehicle_handle, seat))
                } else {
                    None
                };
                let pools = self.pools.as_deref_mut()?;
                let pending_vehicle = request
                    .vehicle_handle
                    .and_then(|handle| self.pending_vehicles.get_mut(&handle));
                let entity = self.script_entities.insert_script_char_entity(
                    pools,
                    request,
                    general?,
                    self.ped_types,
                    self.file_loader,
                    pending_vehicle,
                )?;
                let handle = entity.handle;
                if let Some(relation) = relation {
                    self.pending_relations.insert(handle, relation);
                }
                self.pending_peds.insert(handle, entity);
                Some(ScriptEntityMutationResult::Created(handle))
            }
            ScriptEntityMutation::CreateCar(request) => {
                let pools = self.pools.as_deref_mut()?;
                let model_info = self
                    .file_loader
                    .model_info
                    .get_model_info(request.model_id.max(0) as usize);
                let entity = self.script_entities.insert_script_car_entity(
                    pools,
                    request,
                    model_info,
                    self.handling_manager,
                    general?,
                    self.file_loader,
                    self.taxi_lights.all_enabled,
                )?;
                let handle = entity.handle();
                self.pending_vehicles.insert(handle, entity);
                Some(ScriptEntityMutationResult::Created(handle))
            }
            ScriptEntityMutation::SetAllTaxiLights(enabled) => {
                self.taxi_lights.all_enabled = enabled;
                Some(ScriptEntityMutationResult::Applied)
            }
            ScriptEntityMutation::CreateObject(request) => {
                let model_base_offset = if request.add_model_offset {
                    self.file_loader
                        .model_info
                        .get_model_info(request.model_id.max(0) as usize)
                        .and_then(|model| model.base().col_model.as_ref())
                        .map(|col| -col.bounding_box.min.z)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                let pools = self.pools.as_deref_mut()?;
                let object = self.script_entities.build_script_object(
                    pools,
                    None,
                    request.model_id,
                    request.position,
                    model_base_offset,
                    general?,
                    self.file_loader,
                )?;
                let handle = object.handle;
                self.pending_objects.insert(handle, object.clone());
                spawn_script_object(self.commands, &mut self.script_entities.index, object);
                Some(ScriptEntityMutationResult::Created(handle))
            }
        }
    }

    fn char_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        if handle == 0 {
            return None;
        }
        if let Some(entity) = self.pending_peds.get(&handle) {
            let current_vehicle = self
                .pending_relations
                .get(&handle)
                .map(|(vehicle_handle, _)| *vehicle_handle);
            let mut snapshot = script_ped_snapshot_from_entity(entity, current_vehicle);
            set_script_snapshot_bounds(
                &mut snapshot,
                &entity.ped.physical.entity.matrix,
                entity.ped.physical.entity.model_index.max(0) as usize,
                self.file_loader,
            );
            snapshot.can_see_player = script_ped_can_see_player(
                entity.ped.physical.entity.get_position(),
                entity.ped.physical.entity.get_forward(),
                self.live_pos,
                self.file_loader,
            );
            return Some(snapshot);
        }
        if let Some(snapshot) = script_char_entity_snapshot(
            self.script_entities,
            handle,
            self.live_pos,
            self.file_loader,
        ) {
            return Some(snapshot);
        }
        if !self.player_spawned {
            return None;
        }
        let is_player_char = self
            .player_char_handles
            .iter()
            .any(|&slot| slot != 0 && slot == handle)
            || handle == self.pool_handle
            || handle == self.player_index.saturating_add(1);
        if !is_player_char {
            return None;
        }

        let (current_weapon, weapon_ammo) = self
            .world_players
            .player(self.player_index)
            .map(|player| {
                (
                    player.ped.current_weapon as i32,
                    ped_weapon_ammo_snapshot(&player.ped),
                )
            })
            .unwrap_or((0, [0; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS]));
        let mut snapshot = ScriptEntitySnapshot {
            position: self.live_pos,
            bound_center: self.live_pos,
            bound_radius: 0.8,
            heading_degrees: self
                .world_players
                .player(self.player_index)
                .map(|player| player.heading_degrees())
                .unwrap_or(0.0),
            up_z: self
                .world_players
                .player(self.player_index)
                .map(|player| player.ped.physical.entity.get_up().z)
                .unwrap_or(1.0),
            model_index: self
                .world_players
                .player(self.player_index)
                .map(|player| i32::from(player.ped.physical.entity.model_index))
                .unwrap_or(0),
            entity_status: self
                .world_players
                .player(self.player_index)
                .map(|player| player.ped.physical.entity.status as i32)
                .unwrap_or(eEntityStatus::STATUS_PLAYER as i32),
            alive: true,
            current_vehicle: self.player_current_vehicle_handle,
            current_vehicle_model: None,
            vehicle_colour1: None,
            vehicle_colour2: None,
            vehicle_is_boat: false,
            vehicle_in_water: false,
            automobile_drive_wheels_on_ground: None,
            stopped: self.live_stopped,
            is_shooting: self
                .world_players
                .player(self.player_index)
                .is_some_and(|player| player.ped.flags.is_shooting),
            collision_records: self
                .world_players
                .player(self.player_index)
                .map(|player| physical_collision_records(&player.ped.physical))
                .unwrap_or([None; crate::entities::Physical::PHYSICAL_MAX_COLLISIONRECORDS]),
            speed: self
                .world_players
                .player(self.player_index)
                .map(|player| player.ped.physical.move_speed.magnitude())
                .unwrap_or(0.0),
            health: self
                .world_players
                .player(self.player_index)
                .map(|player| player.vitality.health)
                .unwrap_or(100.0),
            current_weapon,
            weapon_ammo,
            script_objective_completed: self
                .world_players
                .player(self.player_index)
                .is_some_and(|player| player.ped.flags.script_objective_completed),
            leader_slot: self
                .world_players
                .player(self.player_index)
                .and_then(|player| player.ped.leader),
            ped_type: self
                .world_players
                .player(self.player_index)
                .map(|player| player.ped.ped_type as i32),
            vehicle_bomb_type: None,
            vehicle_damaged: false,
            last_weapon_damage: self
                .world_players
                .player(self.player_index)
                .map_or(-1, |player| i32::from(player.vitality.last_wep_dam)),
            vehicle_num_passengers: 0,
            vehicle_num_max_passengers: 0,
            vehicle_passenger_handles: [None; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS],
            vehicle_closed_doors: 0,
            can_see_player: false,
        };
        if let Some(player) = self.world_players.player(self.player_index) {
            snapshot.can_see_player = script_ped_can_see_player(
                player.position(),
                player.ped.physical.entity.get_forward(),
                self.live_pos,
                self.file_loader,
            );
        }
        Some(snapshot)
    }

    fn char_can_see_char(&mut self, observer: i32, target: i32) -> Option<bool> {
        let observer_pose = if let Some(ped) = self.pending_peds.get(&observer) {
            Some((
                ped.ped.physical.entity.get_position(),
                ped.ped.physical.entity.get_forward(),
            ))
        } else if let Some(ped) = self
            .script_entities
            .peds
            .get(&self.script_entities.index, &observer)
        {
            Some((
                ped.ped.physical.entity.get_position(),
                ped.ped.physical.entity.get_forward(),
            ))
        } else {
            self.world_players
                .find_by_pool_handle(observer)
                .map(|ped| (ped.position(), ped.ped.physical.entity.get_forward()))
        }?;

        let target_position = if let Some(ped) = self.pending_peds.get(&target) {
            Some(ped.ped.physical.entity.get_position())
        } else if let Some(ped) = self
            .script_entities
            .peds
            .get(&self.script_entities.index, &target)
        {
            Some(ped.ped.physical.entity.get_position())
        } else {
            self.world_players
                .find_by_pool_handle(target)
                .map(|ped| ped.position())
        }?;

        Some(script_ped_can_see_entity(
            observer_pose.0,
            observer_pose.1,
            target_position,
            self.file_loader,
        ))
    }

    fn char_can_see_dead_char(&mut self, observer: i32, requested_ped_type: i32) -> Option<bool> {
        let observer_data = if let Some(ped) = self.pending_peds.get(&observer) {
            let count = usize::from(ped.ped.num_near_peds).min(ped.ped.near_peds.len());
            Some((
                ped.ped.physical.entity.get_position(),
                ped.ped.physical.entity.get_forward(),
                ped.ped.near_peds[..count]
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            ))
        } else if let Some(ped) = self
            .script_entities
            .peds
            .get(&self.script_entities.index, &observer)
        {
            let count = usize::from(ped.ped.num_near_peds).min(ped.ped.near_peds.len());
            Some((
                ped.ped.physical.entity.get_position(),
                ped.ped.physical.entity.get_forward(),
                ped.ped.near_peds[..count]
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            ))
        } else {
            self.world_players
                .find_by_pool_handle(observer)
                .map(|ped| {
                    let count = usize::from(ped.ped.num_near_peds).min(ped.ped.near_peds.len());
                    (
                        ped.position(),
                        ped.ped.physical.entity.get_forward(),
                        ped.ped.near_peds[..count]
                            .iter()
                            .flatten()
                            .copied()
                            .collect::<Vec<_>>(),
                    )
                })
        }?;

        for target_handle in observer_data.2 {
            let target_data = if let Some(ped) = self.pending_peds.get(&target_handle) {
                Some((
                    ped.ped.ped_type as i32,
                    ped.vitality.health,
                    ped.ped.physical.entity.get_position(),
                ))
            } else if let Some(ped) = self
                .script_entities
                .peds
                .get(&self.script_entities.index, &target_handle)
            {
                Some((
                    ped.ped.ped_type as i32,
                    ped.vitality.health,
                    ped.ped.physical.entity.get_position(),
                ))
            } else {
                self.world_players
                    .find_by_pool_handle(target_handle)
                    .map(|ped| {
                        (
                            ped.ped.ped_type as i32,
                            ped.vitality.health,
                            ped.position(),
                        )
                    })
            };
            let Some((ped_type, health, target_position)) = target_data else {
                continue;
            };
            if health <= 0.0
                && ped_type == requested_ped_type
                && script_ped_can_see_entity(
                    observer_data.0,
                    observer_data.1,
                    target_position,
                    self.file_loader,
                )
            {
                return Some(true);
            }
        }
        Some(false)
    }

    fn set_threat_reaction_range_multiplier(&mut self, multiplier: f32) -> Option<()> {
        self.threat_reaction_range.multiplier = multiplier as u16;
        Some(())
    }

    fn set_char_objective_now(
        &mut self,
        handle: i32,
        objective: i32,
        target_char: Option<i32>,
        target_car: Option<i32>,
        position: Option<CVector>,
        radius: f32,
        route: Option<(i32, i32)>,
    ) -> Option<()> {
        let pending_vehicle = self
            .pending_relations
            .entries
            .iter()
            .find_map(|(ped_handle, (vehicle_handle, _))| {
                (*ped_handle == handle).then_some(CPoolHandle(*vehicle_handle).index())
            });
        if let Some(ped) = self.pending_peds.get_mut(&handle) {
            apply_ped_objective(
                &mut ped.ped,
                &ped.vitality,
                objective,
                target_char,
                target_car,
                position,
                radius,
                route,
                self.ped_routes,
                self.now_ms,
                pending_vehicle,
            );
            return Some(());
        }

        let current_vehicle = pending_vehicle.or_else(|| {
            self.script_entities
                .current_vehicle_handle_for_char(handle)
                .map(|vehicle| CPoolHandle(vehicle).index())
        });
        let mut ped = self
            .script_entities
            .peds
            .get_mut(&self.script_entities.index, &handle)?;
        apply_ped_objective(
            &mut ped.ped,
            &ped.vitality,
            objective,
            target_char,
            target_car,
            position,
            radius,
            route,
            self.ped_routes,
            self.now_ms,
            current_vehicle,
        );
        Some(())
    }

    fn set_car_target_now(&mut self, handle: i32, target: i32, ram: bool) -> Option<()> {
        let target_entity = self.script_entities.vehicle_entity(target)?;
        let target_pool_index = CPoolHandle(target).index();
        if let Some(vehicle) = self.pending_vehicles.get_mut(&handle) {
            let base = vehicle.base_mut();
            let result = if ram {
                crate::control::CarAI::CCarAI::TellCarToRamOtherCar(
                    &mut base.auto_pilot,
                    target_entity,
                    target_pool_index,
                )
            } else {
                crate::control::CarAI::CCarAI::TellCarToBlockOtherCar(
                    &mut base.auto_pilot,
                    target_entity,
                    target_pool_index,
                )
            };
            base.drive.engine_on = result.engine_on;
            return Some(());
        }

        let mut vehicle = self
            .script_entities
            .vehicles
            .get_mut(&self.script_entities.index, &handle)?;
        let mut base = vehicle.base_mut();
        let result = if ram {
            crate::control::CarAI::CCarAI::TellCarToRamOtherCar(
                &mut base.auto_pilot,
                target_entity,
                target_pool_index,
            )
        } else {
            crate::control::CarAI::CCarAI::TellCarToBlockOtherCar(
                &mut base.auto_pilot,
                target_entity,
                target_pool_index,
            )
        };
        base.drive.engine_on = result.engine_on;
        Some(())
    }

    fn collective_peds_in_area(&mut self, center: CVector, radius: f32) -> Option<Vec<i32>> {
        if radius < 0.0 {
            return Some(Vec::new());
        }
        let radius_sq = radius * radius;
        let mut candidates = Vec::new();
        for ped in self.script_entities.peds.values() {
            if self
                .script_entities
                .relation_for_char(ped.handle)
                .is_some()
            {
                continue;
            }
            let position = ped.ped.physical.entity.get_position();
            let dx = position.x - center.x;
            let dy = position.y - center.y;
            if dx * dx + dy * dy < radius_sq {
                candidates.push((position, vec![ped.handle]));
            }
        }
        for (handle, ped) in &self.pending_peds.entries {
            if self
                .pending_relations
                .entries
                .iter()
                .any(|(ped_handle, _)| ped_handle == handle)
            {
                continue;
            }
            let position = ped.ped.physical.entity.get_position();
            let dx = position.x - center.x;
            let dy = position.y - center.y;
            if dx * dx + dy * dy < radius_sq {
                candidates.push((position, vec![*handle]));
            }
        }
        for (handle, vehicle) in self.script_entities.vehicles.iter() {
            let position = vehicle.base().physical.entity.get_position();
            let dx = position.x - center.x;
            let dy = position.y - center.y;
            if dx * dx + dy * dy >= radius_sq {
                continue;
            }
            let Some(entity) = self.script_entities.vehicle_entity(*handle) else {
                continue;
            };
            let mut occupants: Vec<CollectiveOccupant> = self
                .script_entities
                .script_occupant_handles(entity)
                .map(|(ped_handle, seat)| CollectiveOccupant {
                    seat_order: collective_seat_order(seat),
                    ped_handle,
                })
                .collect();
            occupants.sort_by_key(|occupant| occupant.seat_order);
            let occupants = occupants
                .into_iter()
                .map(|occupant| occupant.ped_handle)
                .collect();
            candidates.push((position, occupants));
        }
        for (handle, vehicle) in &self.pending_vehicles.entries {
            let position = vehicle.base().physical.entity.get_position();
            let dx = position.x - center.x;
            let dy = position.y - center.y;
            if dx * dx + dy * dy >= radius_sq {
                continue;
            }
            let mut occupants: Vec<CollectiveOccupant> = self
                .pending_relations
                .entries
                .iter()
                .filter_map(|(ped_handle, (vehicle_handle, seat))| {
                    if *vehicle_handle != *handle {
                        return None;
                    }
                    Some(CollectiveOccupant {
                        seat_order: collective_seat_order(*seat),
                        ped_handle: *ped_handle,
                    })
                })
                .collect();
            occupants.sort_by_key(|occupant| occupant.seat_order);
            let occupants = occupants
                .into_iter()
                .map(|occupant| occupant.ped_handle)
                .collect();
            candidates.push((position, occupants));
        }

        // re3 FindObjectsInRange stores at most 64 CWorld entities before
        // expanding eligible vehicle occupants. ECS query order is not yet
        // the native sector-list order; the parity audit records this gap.
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for (_, members) in candidates.into_iter().take(64) {
            for handle in members {
                let ped = self
                    .pending_peds
                    .get(&handle)
                    .map(ScriptPedEntity::as_ref)
                    .or_else(|| {
                        self.script_entities
                            .peds
                            .get(&self.script_entities.index, &handle)
                    });
                let Some(ped) = ped else {
                    continue;
                };
                if ped.ped.char_created_by == MISSION_CHAR
                    || ped.ped.ped_type == ePedType::PEDTYPE_COP
                    || !seen.insert(handle)
                {
                    continue;
                }
                result.push(handle);
            }
        }
        Some(result)
    }

    fn collective_peds_in_vehicle(&mut self, vehicle: i32) -> Option<Vec<i32>> {
        let entity = self.script_entities.vehicle_entity(vehicle);
        let mut occupants: Vec<CollectiveOccupant> = entity
            .into_iter()
            .flat_map(|entity| {
                self.script_entities
                    .script_occupant_handles(entity)
                    .map(|(ped_handle, seat)| CollectiveOccupant {
                        seat_order: collective_seat_order(seat),
                        ped_handle,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        occupants.extend(
            self.pending_relations
                .entries
                .iter()
                .filter_map(|(ped_handle, (vehicle_handle, seat))| {
                    if *vehicle_handle != vehicle {
                        return None;
                    }
                    Some(CollectiveOccupant {
                        seat_order: collective_seat_order(*seat),
                        ped_handle: *ped_handle,
                    })
                }),
        );
        occupants.sort_by_key(|occupant| occupant.seat_order);
        let mut seen = HashSet::new();
        occupants.retain(|occupant| {
            let handle = occupant.ped_handle;
            if !seen.insert(handle) {
                return false;
            }
            self.pending_peds
                .get(&handle)
                .map(ScriptPedEntity::as_ref)
                .or_else(|| {
                    self.script_entities
                        .peds
                        .get(&self.script_entities.index, &handle)
                })
                .is_some_and(|ped| {
                    ped.ped.char_created_by != MISSION_CHAR
                        && ped.ped.ped_type != ePedType::PEDTYPE_COP
                })
        });
        Some(
            occupants
                .into_iter()
                .map(|occupant| occupant.ped_handle)
                .collect(),
        )
    }

    fn closest_object_of_type(
        &mut self,
        position: CVector,
        range: f32,
        model_index: i32,
        mut general: Option<&mut CGeneral>,
    ) -> Option<i32> {
        let Ok(model_index) = usize::try_from(model_index) else {
            return Some(-1);
        };
        if range <= 0.0 {
            return Some(-1);
        }

        let world_candidates = self
            .file_loader
            .world
            .find_closest_object_command_candidates(model_index, position, range);
        if let Some(static_index) = closest_static_object_candidate(
            self.file_loader,
            &world_candidates,
            model_index,
            position,
            range,
        ) {
            let Some(world_entity) = self.file_loader.world.entities.get(static_index) else {
                return Some(-1);
            };
            if world_entity.entity_type == crate::core::World::EEntityType::Building {
                bevy::log::warn_once!(target: "rustgta3::script_object_lookup", model_index,
                    "closest object command matched a building that has no CObject pool identity");
                return Some(-1);
            }

            let Some(object_col) = self.file_loader.model_info.col_model(world_entity.model_index)
            else {
                bevy::log::warn_once!(target: "rustgta3::script_object_lookup", model_index,
                    "static object promotion requires authored collision data");
                return Some(-1);
            };
            let Some(object_matrix) = self.file_loader.world.matrix_for_static_entity(static_index)
            else {
                bevy::log::warn_once!(target: "rustgta3::script_object_lookup", static_index,
                    "static object promotion could not resolve its world matrix");
                return Some(-1);
            };
            for vehicle in self.script_entities.vehicles.values() {
                let vehicle = vehicle.base();
                let Ok(vehicle_model_index) =
                    usize::try_from(vehicle.physical.entity.model_index)
                else {
                    bevy::log::warn_once!(target: "rustgta3::script_object_lookup", vehicle_model_index = vehicle.physical.entity.model_index,
                        "safe static object promotion requires a valid vehicle model index");
                    return Some(-1);
                };
                let Some(vehicle_col) = self
                    .file_loader
                    .model_info
                    .col_model(vehicle_model_index)
                else {
                    bevy::log::warn_once!(target: "rustgta3::script_object_lookup", vehicle_model_index,
                        "safe static object promotion requires vehicle collision data");
                    return Some(-1);
                };
                if !CCollision::process_col_models_spheres(
                    &object_matrix,
                    object_col,
                    &vehicle.physical.entity.matrix,
                    vehicle_col,
                )
                .is_empty()
                {
                    return Some(-1);
                }
            }

            let Some(pools) = self.pools.as_deref_mut() else {
                bevy::log::warn_once!(target: "rustgta3::script_object_lookup", model_index,
                    "static object promotion requires the canonical object pool");
                return Some(-1);
            };
            let Some(pool_handle) = pools
                .object_pool
                .new_entry(CPoolEntry::new(CPoolEntryKind::Object))
            else {
                bevy::log::warn_once!(target: "rustgta3::script_object_lookup", model_index,
                    "static object promotion could not allocate a CObject pool slot");
                return Some(-1);
            };
            let Some(general) = general.as_deref_mut() else {
                pools.object_pool.delete_by_slot(pool_handle.index());
                bevy::log::warn_once!(target: "rustgta3::script_object_lookup", model_index,
                    "static object promotion requires the game random generator");
                return Some(-1);
            };
            let model_id = match i32::try_from(world_entity.model_index) {
                Ok(model_id) => model_id,
                Err(_) => {
                    pools.object_pool.delete_by_slot(pool_handle.index());
                    return Some(-1);
                }
            };
            let mut object = CObject::new_with_model(model_id, false, general);
            object.physical.entity.matrix = object_matrix;
            object.object_matrix = object_matrix;
            object.physical.entity.set_position(object_matrix.position);
            object.physical.entity.level = world_entity.level.clamp(0, 3) as u16;
            object.physical.entity.flags.is_visible = world_entity.is_visible;
            object.physical.entity.flags.uses_collision = true;
            object.initialize_world_state(self.file_loader);
            object.object_created_by = MISSION_OBJECT;
            if let Some(model_name) = self
                .file_loader
                .model_info
                .get_model_info(world_entity.model_index)
                .map(|model| model.base().name.as_str())
            {
                if matches!(
                    model_name.to_ascii_lowercase().as_str(),
                    "glassfx1" | "glassfx2" | "glassfx3" | "glassfx4" | "glassfx55"
                        | "glassfxsub1" | "glassfxsub2" | "glassfx_composh"
                ) {
                    object.physical.entity.flags.is_visible = false;
                }
                if model_name.eq_ignore_ascii_case("bouy") {
                    object.physical.entity.set_is_static(false);
                    object.physical.set_move_speed(0.0, 0.0, -0.001);
                    object.physical.entity.flags.touching_water = true;
                }
            }

            let handle = pool_handle.0;
            let promoted = ScriptObjectEntity { handle, object };
            self.pending_objects.insert(handle, promoted.clone());
            spawn_script_object(
                self.commands,
                &mut self.script_entities.index,
                promoted,
            );
            self.pending_static_world_promotions.push(static_index);
            return Some(handle);
        }

        let range_sq = range * range;
        let mut eligible_matches = 0usize;
        let mut closest = None;
        let mut minimum_distance = 2.0 * range;
        for object in self
            .script_entities
            .objects
            .values()
            .chain(self.pending_objects.values().map(ScriptObjectEntity::as_ref))
        {
            if usize::try_from(object.object.physical.entity.model_index).ok() != Some(model_index) {
                continue;
            }
            let object_position = object.object.physical.entity.get_position();
            let dx = object_position.x - position.x;
            let dy = object_position.y - position.y;
            if dx * dx + dy * dy >= range_sq {
                continue;
            }
            if eligible_matches == 16 {
                break;
            }
            eligible_matches += 1;

            let dz = object_position.z - position.z;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            if distance < minimum_distance {
                minimum_distance = distance;
                closest = Some(object.handle);
            }
        }

        let Some(handle) = closest else {
            return Some(-1);
        };
        if let Some(pending) = self.pending_objects.get_mut(&handle) {
            pending.object.object_created_by = MISSION_OBJECT;
        } else if let Some(live) = self.script_entities.object_get_mut(handle) {
            live.object.object_created_by = MISSION_OBJECT;
        } else {
            bevy::log::warn_once!(target: "rustgta3::script_object_lookup", handle,
                "closest object lost its canonical pool handle before mission ownership could be set");
            return Some(-1);
        }
        Some(handle)
    }

    fn vehicle_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        if let Some(vehicle) = self.pending_vehicles.get(&handle) {
            let vehicle_entity = self.script_entities.vehicle_entity(handle)?;
            let mut snapshot = script_vehicle_snapshot_from_entity(vehicle.as_ref(), |seat_index| {
                let seat = VehicleSeat::Passenger(u8::try_from(seat_index).ok()?);
                self.pending_relations
                    .iter()
                    .find_map(|(&ped_handle, &(vehicle_handle, relation_seat))| {
                        (vehicle_handle == handle && relation_seat == seat).then_some(ped_handle)
                    })
                    .or_else(|| {
                        self.script_entities
                            .script_occupant_handles(vehicle_entity)
                            .find_map(|(ped_handle, relation_seat)| {
                                (relation_seat == seat).then_some(ped_handle)
                            })
                    })
                    .or_else(|| {
                        (0..self.world_players.slot_count()).find_map(|player_index| {
                            let relation = self.world_players.relation(player_index as i32)?;
                            (relation.vehicle == vehicle_entity && relation.seat == seat)
                                .then(|| {
                                    self.world_players
                                        .player(player_index as i32)
                                        .map(|player| player.pool_handle)
                                })
                                .flatten()
                        })
                    })
            });
            let base = vehicle.base();
            snapshot.stopped =
                script_vehicle_is_stopped(base.physical.distance_travelled, self.script_time_step);
            set_script_snapshot_bounds(
                &mut snapshot,
                &base.physical.entity.matrix,
                base.physical.entity.model_index.max(0) as usize,
                self.file_loader,
            );
            return Some(snapshot);
        }
        script_vehicle_entity_snapshot(
            self.script_entities,
            handle,
            self.script_time_step,
            self.world_players,
            self.file_loader,
        )
    }

    fn object_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        if let Some(object) = self.pending_objects.get(&handle) {
            return Some(script_object_entity_snapshot(&object.as_ref(), self.file_loader));
        }
        let object = self.script_entities.objects.get(&self.script_entities.index, &handle)?;
        Some(script_object_entity_snapshot(&object, self.file_loader))
    }

    fn any_entity_intersects_cube(
        &mut self,
        min: CVector,
        max: CVector,
        entity_types: [bool; 5],
    ) -> Option<bool> {
        if self.file_loader.world.any_entity_intersects_cube(
            min,
            max,
            &self.file_loader.model_info,
            entity_types,
        ) {
            return Some(true);
        }

        if entity_types[1] {
            for vehicle in self
                .script_entities
                .vehicles
                .values()
                .chain(self.pending_vehicles.values().map(ScriptVehicleEntity::as_ref))
            {
                let base = vehicle.base();
                if script_entity_sphere_intersects_cube(
                    base.physical.entity.get_position(),
                    script_model_bound_radius(
                        self.file_loader,
                        base.physical.entity.model_index,
                        1.2,
                    ),
                    min,
                    max,
                ) {
                    return Some(true);
                }
            }
        }
        if entity_types[2] {
            for entity in self
                .script_entities
                .peds
                .values()
                .chain(self.pending_peds.values().map(|ped| ped.as_ref()))
            {
                if script_entity_sphere_intersects_cube(
                    entity.ped.physical.entity.get_position(),
                    script_model_bound_radius(
                        self.file_loader,
                        entity.ped.physical.entity.model_index,
                        0.8,
                    ),
                    min,
                    max,
                ) {
                    return Some(true);
                }
            }
            if let Some(player) = self.world_players.player(self.player_index)
                && script_entity_sphere_intersects_cube(
                    player.position(),
                    script_model_bound_radius(
                        self.file_loader,
                        player.ped.physical.entity.model_index,
                        0.8,
                    ),
                    min,
                    max,
                )
            {
                return Some(true);
            }
        }
        if entity_types[3] {
            for entity in self
                .script_entities
                .objects
                .values()
                .chain(self.pending_objects.values().map(ScriptObjectEntity::as_ref))
            {
                if script_entity_sphere_intersects_cube(
                    entity.object.physical.entity.get_position(),
                    script_model_bound_radius(
                        self.file_loader,
                        entity.object.physical.entity.model_index,
                        0.8,
                    ),
                    min,
                    max,
                ) {
                    return Some(true);
                }
            }
        }
        Some(false)
    }

    fn any_mission_entity_intersects_cube(&mut self, min: CVector, max: CVector) -> Option<bool> {
        for entity in self
            .script_entities
            .peds
            .values()
            .chain(self.pending_peds.values().map(|ped| ped.as_ref()))
        {
            if entity.ped.char_created_by == MISSION_CHAR
                && script_entity_sphere_intersects_cube(
                    entity.ped.physical.entity.get_position(),
                    script_model_bound_radius(
                        self.file_loader,
                        entity.ped.physical.entity.model_index,
                        0.8,
                    ),
                    min,
                    max,
                )
            {
                return Some(true);
            }
        }
        for vehicle in self
            .script_entities
            .vehicles
            .values()
            .chain(self.pending_vehicles.values().map(ScriptVehicleEntity::as_ref))
        {
            let base = vehicle.base();
            if base.vehicle_created_by == MISSION_VEHICLE
                && script_entity_sphere_intersects_cube(
                    base.physical.entity.get_position(),
                    script_model_bound_radius(
                        self.file_loader,
                        base.physical.entity.model_index,
                        1.2,
                    ),
                    min,
                    max,
                )
            {
                return Some(true);
            }
        }
        for entity in self
            .script_entities
            .objects
            .values()
            .chain(self.pending_objects.values().map(ScriptObjectEntity::as_ref))
        {
            if entity.object.object_created_by == MISSION_OBJECT
                && script_entity_sphere_intersects_cube(
                    entity.object.physical.entity.get_position(),
                    script_model_bound_radius(
                        self.file_loader,
                        entity.object.physical.entity.model_index,
                        0.8,
                    ),
                    min,
                    max,
                )
            {
                return Some(true);
            }
        }
        if let Some(player) = self.world_players.player(self.player_index)
            && player.ped.char_created_by == MISSION_CHAR
            && script_entity_sphere_intersects_cube(
                player.position(),
                script_model_bound_radius(
                    self.file_loader,
                    player.ped.physical.entity.model_index,
                    0.8,
                ),
                min,
                max,
            )
        {
            return Some(true);
        }
        Some(false)
    }

    fn garage_contains_vehicle(&mut self, garage_handle: i32, vehicle_handle: i32) -> Option<bool> {
        let garage_index = usize::try_from(garage_handle).ok()?;
        let garage = self.garages?.garages.get(garage_index)?;
        let vehicle = self
            .pending_vehicles
            .get(&vehicle_handle).map(ScriptVehicleEntity::as_ref)
            .or_else(|| self.script_entities.vehicles.get(&self.script_entities.index, &vehicle_handle))?;
        Some(garage_contains_vehicle(
            garage,
            &vehicle.base(),
            self.file_loader,
        ))
    }
}

fn script_model_bound_radius(file_loader: &CFileLoader, model_index: i16, fallback: f32) -> f32 {
    file_loader
        .model_info
        .col_model(model_index.max(0) as usize)
        .map_or(fallback, |col| col.bounding_sphere.radius)
}

fn closest_static_object_candidate(
    file_loader: &CFileLoader,
    candidates: &[usize],
    model_index: usize,
    position: CVector,
    range: f32,
) -> Option<usize> {
    let mut closest = None;
    let mut minimum_distance = 2.0 * range;
    for &entity_index in candidates {
        let Some(entity) = file_loader.world.entities.get(entity_index) else {
            continue;
        };
        if entity.model_index != model_index {
            continue;
        }
        let dx = entity.position[0] - position.x;
        let dy = entity.position[1] - position.y;
        let dz = entity.position[2] - position.z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        if distance < minimum_distance {
            minimum_distance = distance;
            closest = Some(entity_index);
        }
    }
    closest
}

fn script_entity_sphere_intersects_cube(
    position: CVector,
    radius: f32,
    min: CVector,
    max: CVector,
) -> bool {
    position.x + radius >= min.x
        && position.x - radius <= max.x
        && position.y + radius >= min.y
        && position.y - radius <= max.y
        && position.z + radius >= min.z
        && position.z - radius <= max.z
}

pub(crate) fn process_active_scripts(resources: ActiveScriptResources) {
    let ActiveScriptResources {
        mut scripts,
        mut mission_debug,
        fire_manager,
        mut general,
        mut clock,
        mut timer,
        ped_routes,
        bridge,
        world_players,
        mut script_entities,
        mut taxi_lights,
        mut commands,
        pickup_index,
        mut pools,
        mut file_loader,
        handling_manager,
        pads,
        keys,
        mouse,
        player_info,
        mut camera,
        cutscene,
        dmaudio,
        replay,
        darkel,
        ped_types,
        mut pickups,
        pacman_pickups,
        garages,
        game_state,
        credits,
        mut streaming,
        collision,
        remote,
        car_ctrl,
        planes,
        helis,
        projectiles,
        explosions,
        menu,
        cranes,
        phone_info,
        mut threat_reaction_range,
    } = resources;

    if mission_debug.as_ref().is_some_and(|switcher| switcher.is_waiting_for_effect_barrier()) {
        return;
    }

    // re3 does not return from LOAD_SCENE / LOAD_ALL_MODELS_NOW (and the
    // equivalent collision/chase loads) until CStreaming has finished.  Our
    // streaming work is cooperative, so the opcode itself has already
    // advanced m_nIp before the load completes.  Do not let the VM execute the
    // following opcode on the next rendered frame while that synthetic
    // blocking call is still in flight; the Streaming set will keep pumping
    // the barrier and release the code pause when residency is complete.
    if file_loader.script_streaming_barrier_in_progress() {
        if !timer.get_is_code_paused() {
            timer.set_code_pause(true);
            file_loader.set_script_streaming_owns_code_pause(true);
        }
        return;
    }

    scripts.credits_done = credits
        .as_ref()
        .map_or(true, |credits| credits.are_credits_done());
    scripts.nasty_game = game_state.nasty_game;
    scripts.french_game = game_state.french_game;
    scripts.german_game = game_state.german_game;
    scripts.player_in_remote_mode = remote.in_remote_mode;
    if let Some(collision) = collision.as_ref() {
        scripts.collision_in_memory = collision.collision_in_memory as i32;
    }
    if let Some(phone_info) = phone_info.as_ref() {
        phone_info.sync_script_mirror(&mut scripts);
    }
    refresh_script_streaming_snapshots(&mut scripts, &streaming);
    scripts.num_random_cars = car_ctrl
        .as_ref()
        .map_or(scripts.num_random_cars, |car_ctrl| car_ctrl.num_random_cars);
    if let Some(planes) = planes.as_ref() {
        scripts.drug_run_completed = planes.cesna_mission_status == CesnaMissionStatus::Landed;
        scripts.drug_plane_shot_down = planes.cesna_mission_status == CesnaMissionStatus::Destroyed;
        scripts.drug_plane_position = planes.drug_plane_coordinates();
        scripts.drop_off_plane_shot_down =
            planes.drop_off_mission_status == CesnaMissionStatus::Destroyed;
        scripts.drop_off_plane_position = planes.drop_off_plane_coordinates();
    }
    if let Some(helis) = helis.as_ref() {
        scripts.catalina_heli_shot_down = helis.catalina_has_been_shot_down;
        scripts.catalina_heli_handle = helis.catalina_handle().unwrap_or(-1);
    }
    if let Some(pacman) = pacman_pickups.as_ref() {
        scripts.pacman_pills_eaten = pacman.pills_eaten_in_race;
    }
    scripts.projectile_snapshots = projectiles
        .as_ref()
        .map(|pool| {
            pool.info
                .iter()
                .filter(|projectile| projectile.in_use)
                .map(|projectile| (projectile.weapon_type as u8, projectile.pos))
                .collect()
        })
        .unwrap_or_default();
    scripts.explosion_snapshots = explosions
        .as_ref()
        .map(|pool| {
            pool.explosions
                .iter()
                .filter_map(|explosion| {
                    explosion
                        .as_ref()
                        .map(|explosion| (explosion.explosion_type as i32, explosion.position))
                })
                .collect()
        })
        .unwrap_or_default();
    // Navigation-zone definitions do not move during a game. C++ opcodes
    // query CTheZones in place; rebuilding and reallocating the complete
    // mirror every frame only adds VM bridge overhead.
    if scripts.zone_bounds.len() != file_loader.world.zones.zones.len() {
        scripts.zone_bounds = file_loader
            .world
            .zones
            .zones
            .iter()
            .map(|zone| {
                let mut label = [0_u8; crate::control::Script::KEY_LENGTH_IN_SCRIPT];
                let bytes = zone.name.as_bytes();
                let count = bytes.len().min(label.len());
                label[..count].copy_from_slice(&bytes[..count]);
                ScriptZoneBounds {
                    label,
                    min: CVector::new(zone.min[0], zone.min[1], zone.min[2]),
                    max: CVector::new(zone.max[0], zone.max[1], zone.max[2]),
                }
            })
            .collect();
    }
    // CPickups::IsPickUpPickedUp consumes entries from the fixed collected ring.
    // Translate real generation handles back to the script-visible handles before
    // this frame's synchronous HAS_PICKUP_BEEN_COLLECTED command executes.
    if let Some(pickups) = pickups.as_mut() {
        for (&script_handle, &actual_handle) in &pickup_index.0 {
            if pickups.is_pickup_picked_up(actual_handle)
                && !scripts.collected_pickup_handles.contains(&script_handle)
            {
                scripts.collected_pickup_handles.push(script_handle);
            }
        }
    }
    if let Some(player_info) = player_info.as_ref() {
        scripts.player_wb_state = player_info.m_WBState as i8;
        scripts.collected_packages = player_info.m_nCollectedPackages;
    }
    if let Some(darkel) = darkel.as_ref() {
        scripts.kill_frenzy_status = darkel.status;
        scripts.models_killed_by_player = darkel.registered_kills;
    }
    if let Some(ped_types) = ped_types.as_ref() {
        for (snapshot, ped_type) in scripts
            .ped_type_threats
            .iter_mut()
            .zip(ped_types.ms_apPedType.iter())
        {
            *snapshot = ped_type.m_threats;
        }
    }
    if let Some(garages) = garages.as_ref() {
        scripts.garages_created = garages.num_garages as i32;
        scripts.crushed_car_id = garages.crushed_car_id;
        for (index, garage) in garages.garages.iter().enumerate() {
            scripts.garage_states[index] = garage.garage_state as u8;
            scripts.garage_respray_happened[index] = garage.respray_happened;
            scripts.garage_collection_masks[index] = garages
                .import_export_collection_mask(index as i32)
                .unwrap_or(0);
        }
    }
    if let Some(cutscene) = cutscene.as_ref() {
        scripts.cutscene_time_ms = cutscene.get_cutscene_time_in_milliseconds();
    }
    // re3 HasCutsceneFinished reads TheCamera.GetPositionAlongSpline() == 1.
    scripts.cutscene_finished = camera
        .as_ref()
        .is_some_and(|c| c.cutscene_finished || (c.position_along_spline - 1.0).abs() < 1e-4);
    if let Some(dmaudio) = dmaudio.as_ref() {
        scripts.mission_audio_loading_status = dmaudio.GetMissionAudioLoadingStatus();
        scripts.mission_audio_finished = dmaudio.IsMissionAudioSampleFinished();
    } else {
        // re3 GetMissionAudioLoadingStatus returns LOADED when audio is not initialised.
        scripts.mission_audio_loading_status = 1;
        scripts.mission_audio_finished = true;
    }
    let replay_playing_back = replay
        .as_ref()
        .is_some_and(|replay| replay.mode == ReplayMode::Playback);

    // re3 Process reads live CWorld / CPad / TheCamera. Prefer live ped position
    // from CWorldPlayers after CREATE_PLAYER; fall back to bridge snapshot.
    let player_spawned = bridge.player_spawned;
    let player_index = bridge.player_index;
    let live_pos = world_players
        .player(player_index)
        .map(|p| p.position())
        .unwrap_or(bridge.player_position);
    let pool_handle = world_players
        .player(player_index)
        .map(|p| p.pool_handle)
        .unwrap_or(player_index.saturating_add(1));
    let player_current_vehicle_handle = world_players
        .vehicle_relation(player_index)
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle));
    scripts.pacman_pills_carried = player_current_vehicle_handle
        .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
        .map_or(0, |vehicle| {
            i32::from(vehicle.base().pac_man_pickups_carried)
        });
    let live_stopped = if let Some(handle) = player_current_vehicle_handle {
        script_entities.vehicles.get(&script_entities.index, &handle)
            .is_some_and(|vehicle| {
                script_vehicle_is_stopped(
                    vehicle.base().physical.distance_travelled,
                    timer.time_step(),
                )
            })
    } else {
        world_players.player(player_index).is_none_or(|player| {
            use crate::animation::AnimationId::{
                ANIM_STD_JUMP_GLIDE, ANIM_STD_JUMP_LAUNCH, ANIM_STD_RUNSTOP1, ANIM_STD_RUNSTOP2,
            };
            let has_stop_or_jump_transition =
                player.animation.movement_anim_clump
                    .associations
                    .iter()
                    .any(|association| {
                        matches!(
                            association.anim_id,
                            ANIM_STD_RUNSTOP1
                                | ANIM_STD_RUNSTOP2
                                | ANIM_STD_JUMP_LAUNCH
                                | ANIM_STD_JUMP_GLIDE
                        )
                    });
            !has_stop_or_jump_transition
                && matches!(
                    player.ped.move_state,
                    eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL
                )
        })
    };
    let player_char_handles = scripts.player_char_handles;
    let pad0 = pads.pads[0];
    let pad1 = pads.pads[1];
    let now_ms = timer.time_in_milliseconds();

    // CRunningScript::Process MESSAGE_WAIT skip path: Cross clears the wake
    // time, expires current message heads, and returns without executing the
    // next command until the following frame.
    if pad0.cross_just_down() {
        let active = scripts.active_scripts.clone();
        for script_index in active {
            let script = &mut scripts.scripts[script_index];
            if script.wake_time > now_ms && script.skip_wake_time {
                script.wake_time = 0;
                script.skip_message_wait_this_frame = true;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SkipMessageWait);
            }
        }
    }

    let mut player_probe = |index: i32| -> Option<ScriptPlayerSnapshot> {
        if !player_spawned {
            return None;
        }
        if index != 0 && index != player_index {
            return None;
        }
        let (current_weapon, weapon_ammo) = world_players
            .player(index)
            .map(|player| {
                (
                    player.ped.current_weapon as i32,
                    ped_weapon_ammo_snapshot(&player.ped),
                )
            })
            .unwrap_or((0, [0; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS]));
        Some(ScriptPlayerSnapshot {
            ped_handle: pool_handle,
            position: live_pos,
            current_vehicle: player_current_vehicle_handle,
            current_vehicle_model: None,
            stopped: live_stopped,
            is_shooting: world_players
                .player(index)
                .is_some_and(|player| player.ped.flags.is_shooting),
            horn_pressed: pads.get_pad(index).get_horn(),
            is_in_control: world_players
                .player(index)
                .is_some_and(|player| player.ped.is_ped_in_control(&player.vitality, )),
            is_driving: world_players
                .player(index)
                .is_some_and(|player| player.ped.ped_state == PedState::PED_DRIVING),
            is_making_call: world_players
                .player(index)
                .is_some_and(|player| player.ped.ped_state == PedState::PED_MAKE_CALL),
            health: world_players
                .player(index)
                .map(|player| player.vitality.health)
                .unwrap_or(100.0),
            wanted_level: world_players
                .player(index)
                .map(|player| player.wanted.get_wanted_level())
                .unwrap_or(0),
            money: player_info.as_ref().map(|info| info.m_nMoney).unwrap_or(0),
            current_weapon,
            weapon_ammo,
        })
    };

    let handling_manager_ref = handling_manager.as_deref();
    let script_time_step = timer.time_step();

    // Camera visibility is a read-only snapshot for the VM pass. Keeping that
    // snapshot separate from the mutable effect sink avoids aliasing the same
    // ResMut while SCM is executing multiple callbacks.
    let camera_visibility: Option<CCamera> = camera.as_ref().map(|camera| (**camera).clone());
    let mut camera_probe = |pos: rustgta3_core::core::Coordinates::CVector, radius: f32| -> bool {
        camera_visibility
            .as_ref()
            .is_none_or(|camera| camera.is_sphere_visible(pos, radius))
    };

    let mut flush_immediate_camera_effect = |scripts: &mut CTheScripts| {
        let effect = match scripts.pending_effects.last() {
            Some(ScriptCommandEffect::DoFade { .. })
            | Some(ScriptCommandEffect::SetFadingColour { .. }) => scripts.pending_effects.pop(),
            _ => None,
        };
        let Some(effect) = effect else {
            return;
        };
        let Some(camera) = camera.as_deref_mut() else {
            scripts.pending_effects.push(effect);
            return;
        };
        match effect {
            ScriptCommandEffect::DoFade { time_ms, direction } => {
                camera.fade(time_ms as f32 / 1_000.0, direction);
            }
            ScriptCommandEffect::SetFadingColour { r, g, b } => {
                camera.set_fade_colour(r, g, b);
            }
            _ => unreachable!(),
        }
    };

    // re3 Script.cpp COMMAND_IS_BUTTON_PRESSED has a PC-only intro rule:
    // pad 0 / button 12 (Start) also observes LMB, Enter and Space just-down.
    // Keep this at the synchronous SCM pad boundary so the same physical press
    // can both FinishCutscene in CCutsceneMgr::Update and select the authored
    // intro-skip branch later in the same CGame::Process frame.
    let intro_keyboard_mouse_skip = game_state.playing_intro
        && (mouse.just_pressed(MouseButton::Left)
            || keys.just_pressed(KeyCode::Enter)
            || keys.just_pressed(KeyCode::NumpadEnter)
            || keys.just_pressed(KeyCode::Space));
    let mut pad_probe = |pad: i32, button: i32| -> i16 {
        if pad == 0 && button == 12 && intro_keyboard_mouse_skip {
            return 255;
        }
        let p = if pad <= 0 { &pad0 } else { &pad1 };
        p.get_pad_state_button(button)
    };

    let mut pending_peds = SamePassOverlay::default();
    let mut pending_vehicles = SamePassOverlay::default();
    let mut pending_relations = SamePassOverlay::default();
    let mut pending_objects = SamePassOverlay::default();
    let mut pending_static_world_promotions = Vec::new();

    let mut world_state_probe = (menu.is_some() && cranes.is_some()).then(|| {
        let menu = menu.as_deref();
        let cranes = cranes.as_deref();
        move || ScriptWorldStateSnapshot {
            menu_active: menu.is_some_and(|menu| menu.get_is_menu_active()),
            military_crane_all_cars_collected: cranes
                .is_some_and(|cranes| cranes.have_all_cars_been_collected_by_military_crane()),
        }
    });

    let cleanup_script = mission_debug.as_ref().and_then(|switcher| switcher.cleanup_script());
    let process_result = scripts.process_active_scripts_with_selected_executor(
        now_ms,
        timer.time_step_in_milliseconds() as f32,
        replay_playing_back,
        cleanup_script,
        |scripts, script_index| {
            let effect_start = scripts.pending_effects.len();
            let is_mission = super::ScriptDebug::is_loaded_mission_thread(&scripts.scripts[script_index]);
            let command_result = {
                let file_loader_ref: &CFileLoader = &file_loader;
                let mut ground_probe = |x: f32, y: f32| -> f32 {
                    file_loader_ref
                        .world
                        .find_ground_z_for_coord(x, y, &file_loader_ref.model_info)
                };
                let mut ground_3d_probe = |x: f32, y: f32, z: f32| -> f32 {
                    file_loader_ref
                        .world
                        .find_ground_z_for_3d_coord(x, y, z, &file_loader_ref.model_info)
                        .unwrap_or(0.0)
                };
                let mut path_node_probe = |position: CVector,
                                           path_type: u8,
                                           ignore_disabled: bool,
                                           ignore_between_levels: bool|
                 -> Option<ScriptPathNodeSnapshot> {
                    let node_index = file_loader_ref.path_find.find_node_closest_to_coors(
                        position,
                        path_type,
                        999_999.9,
                        ignore_disabled,
                        ignore_between_levels,
                    );
                    let node_index = usize::try_from(node_index).ok()?;
                    let node = file_loader_ref.path_find.path_nodes.get(node_index)?;
                    Some(ScriptPathNodeSnapshot {
                        position: node.pos,
                        heading_degrees: file_loader_ref
                            .path_find
                            .find_node_orientation_for_car_placement(node_index),
                    })
                };
                let mut entity_runtime = ActiveScriptEntityRuntime {
                    script_entities: &mut script_entities,
                    fire_manager: &fire_manager,
                    taxi_lights: &mut taxi_lights,
                    commands: &mut commands,
                    pools: &mut pools,
                    file_loader: file_loader_ref,
                    handling_manager: handling_manager_ref,
                    ped_types: ped_types.as_deref(),
                    garages: garages.as_deref(),
                    world_players: &world_players,
                    player_spawned,
                    player_index,
                    pool_handle,
                    player_char_handles: &player_char_handles,
                    live_pos,
                    live_stopped,
                    player_current_vehicle_handle,
                    script_time_step,
                    pending_peds: &mut pending_peds,
                    pending_vehicles: &mut pending_vehicles,
                    pending_relations: &mut pending_relations,
                    ped_routes: &ped_routes,
                    now_ms,
                    pending_objects: &mut pending_objects,
                    threat_reaction_range: &mut threat_reaction_range,
                    pending_static_world_promotions: &mut pending_static_world_promotions,
                };
                scripts.process_one_command_for_script_with_entity_runtime_context(
                    script_index,
                    now_ms,
                    timer.time_step(),
                    Some(&mut general),
                    Some(&mut clock),
                    Some(&mut camera_probe),
                    Some(&mut ground_probe),
                    Some(&mut ground_3d_probe),
                    Some(&mut path_node_probe),
                    Some(&mut entity_runtime),
                    Some(&mut player_probe),
                    Some(&mut pad_probe),
                    world_state_probe
                        .as_mut()
                        .map(|probe| probe as &mut dyn FnMut() -> ScriptWorldStateSnapshot),
                )
            };

            for entity_index in pending_static_world_promotions.drain(..) {
                let CFileLoader {
                    world, model_info, ..
                } = &mut *file_loader;
                if !world.retire_promoted_object_dummy(entity_index, model_info) {
                    bevy::log::warn_once!(target: "rustgta3::script_object_lookup", entity_index,
                        "promoted static object did not leave its CWorld sector lists");
                }
            }
            let outcome = command_result.map_err(|error| {
                let script = &scripts.scripts[script_index];
                error!(target: "rustgta3::scm_command_fault", script_index,
                    script_name = ?script.script_name, ip = script.ip,
                    mission_index = ?scripts.last_loaded_mission_index,
                    error = ?error, "SCM command failed at original instruction boundary");
                error
            })?;

            if let Some(switcher) = mission_debug.as_deref_mut() {
                switcher.filter_cleanup_effects(&mut scripts.pending_effects, effect_start);
                switcher.observe_entry_teleport(is_mission, scripts.last_loaded_mission_index,
                    &scripts.pending_effects[effect_start..]);
                switcher.observe_effects(is_mission, &scripts.pending_effects[effect_start..]);
                switcher.observe_termination(scripts.last_loaded_mission_index,
                    &scripts.scripts[script_index], &outcome);
            }

            // re3 executes these CStreaming calls directly inside the opcode.
            // The command-local probes above are dropped before this mutable
            // access, so the next SCM command observes the real residency state.
            flush_immediate_script_streaming_effect(scripts, &mut streaming, &mut file_loader);
            flush_immediate_camera_effect(scripts);
            Ok(outcome)
        },
    );

    // re3's blocking SCM streaming opcodes wrap their CStreaming call in
    // CTimer::Stop/Update, so gameplay simulation does not advance while they
    // wait. Our load is cooperative: own code-pause only if nobody else
    // already owns it, then let the Streaming set continue servicing CD work.
    if file_loader.script_streaming_barrier_in_progress() && !timer.get_is_code_paused() {
        timer.set_code_pause(true);
        file_loader.set_script_streaming_owns_code_pause(true);
    }

    match process_result {
        Ok(_) => scripts.last_vm_error = None,
        Err(error) => {
            if scripts.last_vm_error != Some(error) {
                scripts.vm_fault_recovery_pending = true;
                scripts.last_vm_error = Some(error);
            }
            error!("CTheScripts::Process failed: {error:?}");
        }
    }
}

/// Canonical resources used by SCM asset effects and their streaming barrier.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ScriptEffectStreaming<'w> {
    file_loader: Option<ResMut<'w, CFileLoader>>,
    before_rw: Option<ResMut<'w, CGameBeforeRw>>,
    particles: Option<ResMut<'w, CParticleObjectRuntime>>,
    weapon_info: Res<'w, CWeaponInfoStore>,
    timer: ResMut<'w, CTimer>,
}

pub(crate) fn apply_script_runtime_effects(
    scripts: Option<ResMut<CTheScripts>>,
    mut bridge: ResMut<ScriptGameplayBridge>,
    mut game_state: ResMut<CGameState>,
    mut stats: Option<ResMut<CStats>>,
    mut cutscene: Option<ResMut<CCutsceneMgr>>,
    mut streaming: Option<ResMut<CStreaming>>,
    mut camera: Option<ResMut<CCamera>>,
    mut weather: Option<ResMut<CWeather>>,
    mut gangs: Option<ResMut<CGangs>>,
    mut ped_types: Option<ResMut<CPedTypeRuntime>>,
    mut world_players: CWorldPlayers,
    mut script_entities: ScriptEntityWorld,
    mut general: ResMut<CGeneral>,
    mut car_generators: Option<ResMut<CTheCarGenerators>>,
    streaming_runtime: ScriptEffectStreaming,
) {
    let ScriptEffectStreaming {
        mut file_loader,
        mut before_rw,
        mut particles,
        weapon_info,
        mut timer,
    } = streaming_runtime;
    let Some(mut scripts) = scripts else {
        return;
    };
    if let Some(camera) = camera.as_mut() {
        camera.update_fade(timer.time_step_in_seconds());
        scripts.camera_fading = camera.get_fading();
    }
    let time_ms = timer.time_in_milliseconds();
    apply_script_effects_to_world_with_streaming(
        &mut scripts,
        &mut bridge,
        Some(&mut game_state),
        stats.as_deref_mut(),
        cutscene.as_deref_mut(),
        streaming.as_deref_mut(),
        camera.as_deref_mut(),
        weather.as_deref_mut(),
        gangs.as_deref_mut(),
        ped_types.as_deref_mut(),
        Some(&mut script_entities),
        Some(&mut world_players as &mut dyn WorldPlayerAccess),
        Some(&mut *general),
        car_generators.as_deref_mut(),
        file_loader.as_deref_mut(),
        before_rw
            .as_deref_mut()
            .map(|before_rw| &mut before_rw.cd_stream),
        particles.as_deref_mut(),
        time_ms,
        Some(&weapon_info),
    );
    if let Some(file_loader) = file_loader.as_deref_mut()
        && file_loader.script_streaming_barrier_in_progress()
        && !timer.get_is_code_paused()
    {
        timer.set_code_pause(true);
        file_loader.set_script_streaming_owns_code_pause(true);
    }

    // A Rust VM error is recoverable, unlike re3's script_assert/crash path.
    // Successful opcodes before the fault have already applied their direct
    // world side effects, so explicitly unwind the transient global modes that
    // would otherwise poison every later frame (cutscene camera, disabled
    // streaming and cutscene-only world processing). Do this once per distinct
    // fault; retrying the same failing opcode may still become valid later.
    if scripts.vm_fault_recovery_pending {
        scripts.vm_fault_recovery_pending = false;

        if let Some(streaming) = streaming.as_deref_mut() {
            streaming.disable_streaming = false;
        }
        if let Some(file_loader) = file_loader.as_deref_mut() {
            file_loader.world.process_cutscene_only = false;
            file_loader.renderer.loading_priority = false;
            if !file_loader.script_streaming_barrier_in_progress()
                && file_loader.script_streaming_owns_code_pause()
            {
                timer.set_code_pause(false);
                file_loader.set_script_streaming_owns_code_pause(false);
            }
        }

        let cutscene_transient_active = bridge.cutscene_requested
            || bridge.cutscene_started
            || bridge.widescreen
            || bridge.playing_intro
            || cutscene.as_deref().is_some_and(|cutscene| {
                cutscene.loaded || cutscene.running || cutscene.is_cutscene_processing()
            })
            || camera.as_deref().is_some_and(|camera| {
                camera.script_mode == CameraScriptMode::FlyBy || camera.wide_screen_on
            });

        if cutscene_transient_active {
            if let Some(cutscene) = cutscene.as_deref_mut() {
                let was_loaded = cutscene.loaded;
                restore_after_cutscene_clear(
                    &mut bridge,
                    Some(&mut game_state),
                    cutscene,
                    camera.as_deref_mut(),
                );
                // delete_cutscene_data intentionally no-ops before the loaded
                // flag is set. A VM can fail in precisely that half-loaded
                // window, so clear those manager fields explicitly as well.
                if !was_loaded {
                    cutscene.abort_failed_load();
                }
            } else {
                bridge.cutscene_started = false;
                bridge.cutscene_requested = false;
                bridge.widescreen = false;
                if bridge.player_spawned {
                    bridge.player_visible = true;
                    bridge.player_control_enabled = true;
                }
                if let Some(camera) = camera.as_deref_mut() {
                    camera.restore_with_jump_cut();
                    camera.wide_screen_on = false;
                    camera.cutscene_finished = false;
                }
            }

            bridge.playing_intro = false;
            game_state.playing_intro = false;
            if let Some(mut player) = world_players.focused_mut() {
                player.wanted.m_bIgnoredByCops = false;
                player.wanted.m_bIgnoredByEveryone = false;
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
    }
}

pub(crate) fn apply_script_timer_effects(
    mut scripts: ResMut<CTheScripts>,
    mut timer: ResMut<CTimer>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    for effect in effects {
        match effect {
            ScriptCommandEffect::SetTimeScale(scale) => timer.set_time_scale(scale),
            other => scripts.pending_effects.push(other),
        }
    }
}
