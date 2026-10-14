//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! SCM opcode helper methods (verbatim split from control/Script.rs).

#[allow(unused_imports)]
use super::Script::{
    CRunningScript, ScriptEntityWorld, ScriptGameplayBridge, locate_char_mode, locate_player_mode,
    point_in_area_2d, point_in_area_3d,
};
#[allow(unused_imports)]
use super::ScriptCommands::*;
#[allow(unused_imports)]
use super::ScriptTypes::{
    ANDOR_NONE, ANDS_1, ANDS_8, ARGUMENT_END, ARGUMENT_FLOAT, ARGUMENT_GLOBALVAR, ARGUMENT_INT8,
    ARGUMENT_INT16, ARGUMENT_INT32, ARGUMENT_LOCALVAR, CTheScripts, LocateMode, MAP_Z_LOW_LIMIT,
    MAX_STACK_DEPTH, NUM_LOCAL_VARS, NUM_TIMERS, ORS_1, ORS_8, ScriptBinaryOp, ScriptCommandEffect,
    ScriptCommandOutcome, ScriptEntityReadProbeRuntime, ScriptEntityRuntimeAccess,
    ScriptEntitySnapshot, ScriptPathNodeSnapshot, ScriptPlayerSnapshot, ScriptVarPointer,
    ScriptVmError, ScriptWorldStateSnapshot, VAR_GLOBAL, VAR_LOCAL,
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
use rust_gta_3_core::core::Timer::CTimer;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

impl CRunningScript {
    pub fn new() -> Self {
        let mut script = Self {
            next: None,
            prev: None,
            script_name: [0; 8],
            ip: 0,
            stack: [0; MAX_STACK_DEPTH],
            stack_pointer: 0,
            local_variables: [0; NUM_LOCAL_VARS + NUM_TIMERS],
            cond_result: false,
            is_mission_script: false,
            skip_wake_time: false,
            wake_time: 0,
            and_or_state: 0,
            not_flag: false,
            deatharrest_enabled: true,
            deatharrest_executed: false,
            mission_flag: false,
            skip_message_wait_this_frame: false,
        };
        script.init();
        script
    }

    pub fn init(&mut self) {
        self.script_name = [0; 8];
        self.script_name[..6].copy_from_slice(b"noname");
        self.next = None;
        self.prev = None;
        self.set_ip(0);
        self.stack = [0; MAX_STACK_DEPTH];
        self.stack_pointer = 0;
        self.wake_time = 0;
        self.cond_result = false;
        self.is_mission_script = false;
        self.skip_wake_time = false;
        self.local_variables = [0; NUM_LOCAL_VARS + NUM_TIMERS];
        self.and_or_state = 0;
        self.not_flag = false;
        self.deatharrest_enabled = true;
        self.deatharrest_executed = false;
        self.mission_flag = false;
        self.skip_message_wait_this_frame = false;
    }

    pub fn set_ip(&mut self, ip: u32) {
        self.ip = ip;
    }

    pub fn update_timers(&mut self, time_step: f32) {
        self.local_variables[NUM_LOCAL_VARS] =
            self.local_variables[NUM_LOCAL_VARS].saturating_add(time_step as i32);
        self.local_variables[NUM_LOCAL_VARS + 1] =
            self.local_variables[NUM_LOCAL_VARS + 1].saturating_add(time_step as i32);
    }

    pub fn collect_parameters(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        total: i16,
    ) -> Result<[i32; 32], ScriptVmError> {
        for i in 0..total as usize {
            scripts.script_params[i] = self.collect_one_parameter(scripts, ip)?;
        }
        Ok(scripts.script_params)
    }

    pub fn collect_next_parameter_without_increasing_pc(
        &self,
        scripts: &CTheScripts,
        ip: u32,
    ) -> Result<i32, ScriptVmError> {
        let mut ip = ip;
        self.collect_one_parameter_readonly(scripts, &mut ip)
    }

    pub fn store_parameters(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        number: i16,
    ) -> Result<(), ScriptVmError> {
        for i in 0..number as usize {
            match scripts.read_1_byte_from_script(ip)? {
                ARGUMENT_GLOBALVAR => {
                    let var_index = scripts.read_2_bytes_from_script(ip)? as usize;
                    scripts.write_i32_at_script_space(var_index, scripts.script_params[i])?;
                }
                ARGUMENT_LOCALVAR => {
                    let var_index = scripts.read_2_bytes_from_script(ip)? as usize;
                    let local = self
                        .local_variables
                        .get_mut(var_index)
                        .ok_or(ScriptVmError::LocalVariableOutOfRange(var_index))?;
                    *local = scripts.script_params[i];
                }
                arg => return Err(ScriptVmError::InvalidArgumentType(arg)),
            }
        }
        Ok(())
    }

    pub fn get_pointer_to_script_variable(
        &self,
        scripts: &CTheScripts,
        ip: &mut u32,
        var_type: i16,
    ) -> Result<ScriptVarPointer, ScriptVmError> {
        match scripts.read_1_byte_from_script(ip)? {
            ARGUMENT_GLOBALVAR if var_type == VAR_GLOBAL => Ok(ScriptVarPointer::Global(
                scripts.read_2_bytes_from_script(ip)? as usize,
            )),
            ARGUMENT_LOCALVAR if var_type == VAR_LOCAL => Ok(ScriptVarPointer::Local(
                scripts.read_2_bytes_from_script(ip)? as usize,
            )),
            _ => Err(ScriptVmError::InvalidVariableType),
        }
    }

    pub fn update_compare_flag(&mut self, result: bool) {
        let mut result = result;
        if self.not_flag {
            result = !result;
        }
        if self.and_or_state == ANDOR_NONE {
            self.cond_result = result;
            return;
        }
        if (ANDS_1..=ANDS_8).contains(&self.and_or_state) {
            self.cond_result &= result;
            if self.and_or_state == ANDS_1 {
                self.and_or_state = ANDOR_NONE;
                return;
            }
        } else if (ORS_1..=ORS_8).contains(&self.and_or_state) {
            self.cond_result |= result;
            if self.and_or_state == ORS_1 {
                self.and_or_state = ANDOR_NONE;
                return;
            }
        } else {
            return;
        }
        self.and_or_state -= 1;
    }

    pub(crate) fn push_char_objective(
        &mut self,
        scripts: &mut CTheScripts,
        handle: i32,
        objective: i32,
        target_char: Option<i32>,
        target_car: Option<i32>,
        position: Option<CVector>,
        radius: f32,
        route: Option<(i32, i32)>,
    ) {
        scripts
            .pending_effects
            .push(ScriptCommandEffect::SetCharObjective {
                handle,
                objective,
                target_char,
                target_car,
                position,
                radius,
                route,
            });
    }

    pub(crate) fn push_collective_objective(
        &mut self,
        scripts: &mut CTheScripts,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
        collective_index: i32,
        objective: i32,
        target_char: Option<i32>,
        target_car: Option<i32>,
        position: Option<CVector>,
        radius: f32,
        route: Option<(i32, i32)>,
    ) -> Result<(), ScriptVmError> {
        let runtime = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingCollectiveProbe)?;
        for index in 0..scripts.collectives.len() {
            let Some(member) = scripts.collectives.get(index) else {
                continue;
            };
            if member.col_index != collective_index {
                continue;
            }
            let handle = member.ped_index;
            if runtime.char_snapshot(handle).is_none() {
                if let Some(member) = scripts.collectives.get_mut(index) {
                    member.col_index = -1;
                    member.ped_index = 0;
                }
                continue;
            }
            if runtime
                .set_char_objective_now(
                    handle,
                    objective,
                    target_char,
                    target_car,
                    position,
                    radius,
                    route,
                )
                .is_none()
            {
                // Lightweight VM probes may not own mutable live actors.
                // Production applies synchronously so the next SCM command
                // observes the changed CPed objective in the same pass.
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharObjective {
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
        Ok(())
    }

    pub(crate) fn print_big_with_numbers_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        count: usize,
        queue: bool,
    ) -> Result<(), ScriptVmError> {
        let key = scripts.read_key_from_script(ip)?;
        let params = self.collect_parameters(scripts, ip, (count + 2) as i16)?;
        let mut numbers = [-1; 6];
        numbers[..count].copy_from_slice(&params[..count]);
        scripts
            .pending_effects
            .push(ScriptCommandEffect::PrintBigWithNumbers {
                key,
                time_ms: params[count],
                style: params[count + 1] - 1,
                numbers,
                queue,
            });
        Ok(())
    }

    pub(crate) fn print_with_numbers_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        count: usize,
        mode: u8,
    ) -> Result<(), ScriptVmError> {
        let key = scripts.read_key_from_script(ip)?;
        let params = self.collect_parameters(scripts, ip, (count + 2) as i16)?;
        let mut numbers = [-1; 6];
        numbers[..count].copy_from_slice(&params[..count]);
        scripts
            .pending_effects
            .push(ScriptCommandEffect::PrintWithNumbers {
                key,
                time_ms: params[count],
                flag: params[count + 1],
                numbers,
                mode,
            });
        Ok(())
    }

    pub fn process_one_command(
        &mut self,
        scripts: &mut CTheScripts,
        time_ms: u32,
        time_step: f32,
        general: Option<&mut CGeneral>,
        clock: Option<&mut CClock>,
        camera_probe: Option<&mut dyn FnMut(CVector, f32) -> bool>,
        ground_probe: Option<&mut dyn FnMut(f32, f32) -> f32>,
        path_node_probe: Option<
            &mut dyn FnMut(CVector, u8, bool, bool) -> Option<ScriptPathNodeSnapshot>,
        >,
        player_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        char_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        vehicle_entity_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        object_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        pad_probe: Option<&mut dyn FnMut(i32, i32) -> i16>,
    ) -> Result<ScriptCommandOutcome, ScriptVmError> {
        self.process_one_command_with_3d_ground(
            scripts,
            time_ms,
            time_step,
            general,
            clock,
            camera_probe,
            ground_probe,
            None,
            path_node_probe,
            player_probe,
            char_probe,
            vehicle_entity_probe,
            object_probe,
            pad_probe,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_one_command_with_3d_ground(
        &mut self,
        scripts: &mut CTheScripts,
        time_ms: u32,
        time_step: f32,
        general: Option<&mut CGeneral>,
        clock: Option<&mut CClock>,
        camera_probe: Option<&mut dyn FnMut(CVector, f32) -> bool>,
        ground_probe: Option<&mut dyn FnMut(f32, f32) -> f32>,
        ground_3d_probe: Option<&mut dyn FnMut(f32, f32, f32) -> f32>,
        path_node_probe: Option<
            &mut dyn FnMut(CVector, u8, bool, bool) -> Option<ScriptPathNodeSnapshot>,
        >,
        player_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        char_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        vehicle_entity_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        object_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        pad_probe: Option<&mut dyn FnMut(i32, i32) -> i16>,
        world_state_probe: Option<&mut dyn FnMut() -> ScriptWorldStateSnapshot>,
    ) -> Result<ScriptCommandOutcome, ScriptVmError> {
        let mut entity_runtime = ScriptEntityReadProbeRuntime {
            char_probe,
            vehicle_probe: vehicle_entity_probe,
            object_probe,
        };
        self.process_one_command_with_3d_ground_runtime(
            scripts,
            time_ms,
            time_step,
            general,
            clock,
            camera_probe,
            ground_probe,
            ground_3d_probe,
            path_node_probe,
            Some(&mut entity_runtime),
            player_probe,
            pad_probe,
            world_state_probe,
        )
    }

    pub(crate) fn process_entity_area_2d_effects(
        &mut self,
        scripts: &mut CTheScripts,
        script_ip: u32,
        position: CVector,
        params: &[i32],
    ) {
        let x1 = Self::script_param_as_f32(params[0]);
        let y1 = Self::script_param_as_f32(params[1]);
        let x2 = Self::script_param_as_f32(params[2]);
        let y2 = Self::script_param_as_f32(params[3]);
        self.update_compare_flag(point_in_area_2d(position, x1, y1, x2, y2));
        if params[4] != 0 {
            scripts
                .pending_effects
                .push(ScriptCommandEffect::HighlightArea {
                    script_ip,
                    x1,
                    y1,
                    x2,
                    y2,
                    z: MAP_Z_LOW_LIMIT,
                });
        }
        if scripts.dbg_flag {
            scripts
                .pending_effects
                .push(ScriptCommandEffect::DebugSquare { x1, y1, x2, y2 });
        }
    }

    pub(crate) fn process_entity_area_3d_effects(
        &mut self,
        scripts: &mut CTheScripts,
        script_ip: u32,
        position: CVector,
        params: &[i32],
    ) {
        let x1 = Self::script_param_as_f32(params[0]);
        let y1 = Self::script_param_as_f32(params[1]);
        let z1 = Self::script_param_as_f32(params[2]);
        let x2 = Self::script_param_as_f32(params[3]);
        let y2 = Self::script_param_as_f32(params[4]);
        let z2 = Self::script_param_as_f32(params[5]);
        self.update_compare_flag(point_in_area_3d(position, x1, y1, z1, x2, y2, z2));
        if params[6] != 0 {
            scripts
                .pending_effects
                .push(ScriptCommandEffect::HighlightArea {
                    script_ip,
                    x1,
                    y1,
                    x2,
                    y2,
                    z: (z1 + z2) * 0.5,
                });
        }
        if scripts.dbg_flag {
            scripts
                .pending_effects
                .push(ScriptCommandEffect::DebugCube {
                    x1,
                    y1,
                    z1,
                    x2,
                    y2,
                    z2,
                });
        }
    }

    pub(crate) fn player_in_area_check_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        player_probe: &mut Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_IS_PLAYER_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_IN_AREA_IN_CAR_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_IN_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 8 } else { 6 })?;
        let player = player_probe
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
        .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
        let mut position = player.position;
        let mut stopped = player.stopped;
        if let Some(handle) = player.current_vehicle {
            let vehicle = entity_runtime
                .as_deref_mut()
                .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                .vehicle_snapshot(handle)
                .ok_or(ScriptVmError::MissingVehicle(handle))?;
            position = vehicle.position;
            stopped = vehicle.stopped;
        }
        let stopped_required = matches!(
            command,
            COMMAND_IS_PLAYER_STOPPED_IN_AREA_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_IN_CAR_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_IN_CAR_3D
        );
        let mode = if matches!(
            command,
            COMMAND_IS_PLAYER_IN_AREA_ON_FOOT_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_2D
                | COMMAND_IS_PLAYER_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_3D
        ) {
            LocateMode::OnFoot
        } else if matches!(
            command,
            COMMAND_IS_PLAYER_IN_AREA_IN_CAR_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_IN_CAR_2D
                | COMMAND_IS_PLAYER_IN_AREA_IN_CAR_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_AREA_IN_CAR_3D
        ) {
            LocateMode::InCar
        } else {
            LocateMode::AnyMeans
        };
        self.process_boundary_area_result(
            scripts,
            *ip,
            position,
            player.current_vehicle.is_some(),
            stopped,
            stopped_required,
            mode,
            is_3d,
            &params[1..],
        );
        Ok(())
    }

    /// Direct port of re3 `CRunningScript::PlayerInAngledAreaCheckCommand`.
    /// The two points form the first side and `side2_length` extends the
    /// perpendicular second side; it is not an axis-aligned bounding box.
    pub(crate) fn player_in_angled_area_check_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        player_probe: &mut Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_IS_PLAYER_IN_ANGLED_AREA_3D
                | COMMAND_IS_PLAYER_IN_ANGLED_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_IN_ANGLED_AREA_IN_CAR_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 9 } else { 7 })?;
        let player = player_probe
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
        .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
        let mut position = player.position;
        let mut stopped = player.stopped;
        if let Some(handle) = player.current_vehicle {
            let vehicle = entity_runtime
                .as_deref_mut()
                .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                .vehicle_snapshot(handle)
                .ok_or(ScriptVmError::MissingVehicle(handle))?;
            position = vehicle.position;
            stopped = vehicle.stopped;
        }

        let stopped_required = matches!(
            command,
            COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_3D
        );
        let mode = if matches!(
            command,
            COMMAND_IS_PLAYER_IN_ANGLED_AREA_ON_FOOT_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_2D
                | COMMAND_IS_PLAYER_IN_ANGLED_AREA_ON_FOOT_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_3D
        ) {
            LocateMode::OnFoot
        } else if matches!(
            command,
            COMMAND_IS_PLAYER_IN_ANGLED_AREA_IN_CAR_2D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_2D
                | COMMAND_IS_PLAYER_IN_ANGLED_AREA_IN_CAR_3D
                | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_3D
        ) {
            LocateMode::InCar
        } else {
            LocateMode::AnyMeans
        };

        let inf_x = Self::script_param_as_f32(params[1]);
        let inf_y = Self::script_param_as_f32(params[2]);
        let (mut inf_z, sup_x, sup_y, mut sup_z, side2_length, debug) = if is_3d {
            (
                Self::script_param_as_f32(params[3]),
                Self::script_param_as_f32(params[4]),
                Self::script_param_as_f32(params[5]),
                Self::script_param_as_f32(params[6]),
                Self::script_param_as_f32(params[7]),
                params[8] != 0,
            )
        } else {
            (
                MAP_Z_LOW_LIMIT,
                Self::script_param_as_f32(params[3]),
                Self::script_param_as_f32(params[4]),
                MAP_Z_LOW_LIMIT,
                Self::script_param_as_f32(params[5]),
                params[6] != 0,
            )
        };
        if is_3d && inf_z > sup_z {
            std::mem::swap(&mut inf_z, &mut sup_z);
        }

        let mut angle = CGeneral::get_radian_angle_between_points(inf_x, inf_y, sup_x, sup_y)
            + std::f32::consts::FRAC_PI_2;
        while angle < 0.0 {
            angle += std::f32::consts::TAU;
        }
        while angle > std::f32::consts::TAU {
            angle -= std::f32::consts::TAU;
        }
        let rotated_sup_x = sup_x + side2_length * angle.sin();
        let rotated_sup_y = sup_y - side2_length * angle.cos();
        let rotated_inf_x = inf_x + side2_length * angle.sin();
        let rotated_inf_y = inf_y - side2_length * angle.cos();
        let side1_x = sup_x - inf_x;
        let side1_y = sup_y - inf_y;
        let side1_len = (side1_x * side1_x + side1_y * side1_y).sqrt();
        let side2_x = rotated_inf_x - inf_x;
        let side2_y = rotated_inf_y - inf_y;
        let side2_len = (side2_x * side2_x + side2_y * side2_y).sqrt();
        let x = position.x - inf_x;
        let y = position.y - inf_y;
        let along1 = if side1_len > 0.0 {
            x * side1_x / side1_len + y * side1_y / side1_len
        } else {
            f32::INFINITY
        };
        let along2 = if side2_len > 0.0 {
            x * side2_x / side2_len + y * side2_y / side2_len
        } else {
            f32::INFINITY
        };
        let in_area = along1 >= 0.0
            && along1 <= side1_len
            && along2 >= 0.0
            && along2 <= side2_len
            && (!is_3d || (position.z >= inf_z && position.z <= sup_z));
        let mode_matches = match mode {
            LocateMode::AnyMeans => true,
            LocateMode::OnFoot => player.current_vehicle.is_none(),
            LocateMode::InCar => player.current_vehicle.is_some(),
        };
        self.update_compare_flag((!stopped_required || stopped) && in_area && mode_matches);

        // The current renderer exposes axis-aligned debug highlight intents;
        // retain the exact angled predicate and bound only the diagnostic draw.
        let min_x = inf_x.min(sup_x).min(rotated_inf_x).min(rotated_sup_x);
        let max_x = inf_x.max(sup_x).max(rotated_inf_x).max(rotated_sup_x);
        let min_y = inf_y.min(sup_y).min(rotated_inf_y).min(rotated_sup_y);
        let max_y = inf_y.max(sup_y).max(rotated_inf_y).max(rotated_sup_y);
        if debug {
            scripts
                .pending_effects
                .push(ScriptCommandEffect::HighlightArea {
                    script_ip: *ip,
                    x1: min_x,
                    y1: min_y,
                    x2: max_x,
                    y2: max_y,
                    z: if is_3d {
                        (inf_z + sup_z) * 0.5
                    } else {
                        MAP_Z_LOW_LIMIT
                    },
                });
        }
        if scripts.dbg_flag {
            if is_3d {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DebugCube {
                        x1: min_x,
                        y1: min_y,
                        z1: inf_z,
                        x2: max_x,
                        y2: max_y,
                        z2: sup_z,
                    });
            } else {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DebugSquare {
                        x1: min_x,
                        y1: min_y,
                        x2: max_x,
                        y2: max_y,
                    });
            }
        }
        Ok(())
    }

    pub(crate) fn char_in_area_check_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_IS_CHAR_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_CHAR_IN_AREA_IN_CAR_3D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_3D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_IN_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 8 } else { 6 })?;
        let ped = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingCharProbe)?
            .char_snapshot(params[0])
            .ok_or(ScriptVmError::MissingChar(params[0]))?;
        let mut position = ped.position;
        let mut stopped = ped.stopped;
        if let Some(handle) = ped.current_vehicle {
            let vehicle = entity_runtime
                .as_deref_mut()
                .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                .vehicle_snapshot(handle)
                .ok_or(ScriptVmError::MissingVehicle(handle))?;
            position = vehicle.position;
            stopped = vehicle.stopped;
        }
        let stopped_required = matches!(
            command,
            COMMAND_IS_CHAR_STOPPED_IN_AREA_2D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_ON_FOOT_2D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_IN_CAR_2D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_3D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_IN_CAR_3D
        );
        let mode = if matches!(
            command,
            COMMAND_IS_CHAR_IN_AREA_ON_FOOT_2D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_ON_FOOT_2D
                | COMMAND_IS_CHAR_IN_AREA_ON_FOOT_3D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_ON_FOOT_3D
        ) {
            LocateMode::OnFoot
        } else if matches!(
            command,
            COMMAND_IS_CHAR_IN_AREA_IN_CAR_2D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_IN_CAR_2D
                | COMMAND_IS_CHAR_IN_AREA_IN_CAR_3D
                | COMMAND_IS_CHAR_STOPPED_IN_AREA_IN_CAR_3D
        ) {
            LocateMode::InCar
        } else {
            LocateMode::AnyMeans
        };
        self.process_boundary_area_result(
            scripts,
            *ip,
            position,
            ped.current_vehicle.is_some(),
            stopped,
            stopped_required,
            mode,
            is_3d,
            &params[1..],
        );
        Ok(())
    }

    pub(crate) fn car_in_area_check_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = command == COMMAND_IS_CAR_STOPPED_IN_AREA_3D;
        let params = self.collect_parameters(scripts, ip, if is_3d { 8 } else { 6 })?;
        let vehicle = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
            .vehicle_snapshot(params[0])
            .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
        self.process_boundary_area_result(
            scripts,
            *ip,
            vehicle.position,
            true,
            vehicle.stopped,
            true,
            LocateMode::AnyMeans,
            is_3d,
            &params[1..],
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn process_boundary_area_result(
        &mut self,
        scripts: &mut CTheScripts,
        script_ip: u32,
        position: CVector,
        actor_in_vehicle: bool,
        actor_stopped: bool,
        stopped_required: bool,
        mode: LocateMode,
        is_3d: bool,
        params: &[i32],
    ) {
        let mut x1 = Self::script_param_as_f32(params[0]);
        let mut y1 = Self::script_param_as_f32(params[1]);
        let (mut z1, mut x2, mut y2, mut z2, debug) = if is_3d {
            (
                Self::script_param_as_f32(params[2]),
                Self::script_param_as_f32(params[3]),
                Self::script_param_as_f32(params[4]),
                Self::script_param_as_f32(params[5]),
                params[6] != 0,
            )
        } else {
            (
                MAP_Z_LOW_LIMIT,
                Self::script_param_as_f32(params[2]),
                Self::script_param_as_f32(params[3]),
                MAP_Z_LOW_LIMIT,
                params[4] != 0,
            )
        };
        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y1 > y2 {
            std::mem::swap(&mut y1, &mut y2);
        }
        if is_3d && z1 > z2 {
            std::mem::swap(&mut z1, &mut z2);
        }
        let in_area = if is_3d {
            point_in_area_3d(position, x1, y1, z1, x2, y2, z2)
        } else {
            point_in_area_2d(position, x1, y1, x2, y2)
        };
        let mode_matches = match mode {
            LocateMode::AnyMeans => true,
            LocateMode::OnFoot => !actor_in_vehicle,
            LocateMode::InCar => actor_in_vehicle,
        };
        self.update_compare_flag((!stopped_required || actor_stopped) && in_area && mode_matches);

        if debug {
            scripts
                .pending_effects
                .push(ScriptCommandEffect::HighlightArea {
                    script_ip,
                    x1,
                    y1,
                    x2,
                    y2,
                    z: if is_3d {
                        (z1 + z2) * 0.5
                    } else {
                        MAP_Z_LOW_LIMIT
                    },
                });
        }
        if scripts.dbg_flag {
            if is_3d {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DebugCube {
                        x1,
                        y1,
                        z1,
                        x2,
                        y2,
                        z2,
                    });
            } else {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DebugSquare { x1, y1, x2, y2 });
            }
        }
    }

    pub(crate) fn locate_player_area_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        player_probe: &mut Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_LOCATE_PLAYER_ANY_MEANS_3D
                | COMMAND_LOCATE_PLAYER_ON_FOOT_3D
                | COMMAND_LOCATE_PLAYER_IN_CAR_3D
                | COMMAND_LOCATE_STOPPED_PLAYER_ANY_MEANS_3D
                | COMMAND_LOCATE_STOPPED_PLAYER_ON_FOOT_3D
                | COMMAND_LOCATE_STOPPED_PLAYER_IN_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 8 } else { 6 })?;
        let snapshot = player_probe
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
        .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
        let stopped_required = matches!(
            command,
            COMMAND_LOCATE_STOPPED_PLAYER_ANY_MEANS_2D
                | COMMAND_LOCATE_STOPPED_PLAYER_ANY_MEANS_3D
                | COMMAND_LOCATE_STOPPED_PLAYER_IN_CAR_2D
                | COMMAND_LOCATE_STOPPED_PLAYER_IN_CAR_3D
                | COMMAND_LOCATE_STOPPED_PLAYER_ON_FOOT_2D
                | COMMAND_LOCATE_STOPPED_PLAYER_ON_FOOT_3D
        );
        let mode = locate_player_mode(command)?;
        self.process_locate_area_result(
            scripts,
            *ip,
            snapshot.position,
            snapshot.current_vehicle.is_some(),
            snapshot.stopped,
            stopped_required,
            mode,
            is_3d,
            &params[1..],
            false,
        )
    }

    pub(crate) fn locate_char_area_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_LOCATE_CHAR_ANY_MEANS_3D
                | COMMAND_LOCATE_CHAR_ON_FOOT_3D
                | COMMAND_LOCATE_CHAR_IN_CAR_3D
                | COMMAND_LOCATE_STOPPED_CHAR_ANY_MEANS_3D
                | COMMAND_LOCATE_STOPPED_CHAR_ON_FOOT_3D
                | COMMAND_LOCATE_STOPPED_CHAR_IN_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 8 } else { 6 })?;
        let snapshot = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingCharProbe)?
            .char_snapshot(params[0])
            .ok_or(ScriptVmError::MissingChar(params[0]))?;
        let stopped_required = matches!(
            command,
            COMMAND_LOCATE_STOPPED_CHAR_ANY_MEANS_2D
                | COMMAND_LOCATE_STOPPED_CHAR_ANY_MEANS_3D
                | COMMAND_LOCATE_STOPPED_CHAR_IN_CAR_2D
                | COMMAND_LOCATE_STOPPED_CHAR_IN_CAR_3D
                | COMMAND_LOCATE_STOPPED_CHAR_ON_FOOT_2D
                | COMMAND_LOCATE_STOPPED_CHAR_ON_FOOT_3D
        );
        let mode = locate_char_mode(command)?;
        self.process_locate_area_result(
            scripts,
            *ip,
            snapshot.position,
            snapshot.current_vehicle.is_some(),
            snapshot.stopped,
            stopped_required,
            mode,
            is_3d,
            &params[1..],
            false,
        )
    }

    pub(crate) fn locate_player_char_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        player_probe: &mut Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_LOCATE_PLAYER_ANY_MEANS_CHAR_3D
                | COMMAND_LOCATE_PLAYER_ON_FOOT_CHAR_3D
                | COMMAND_LOCATE_PLAYER_IN_CAR_CHAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 6 } else { 5 })?;
        let player = player_probe
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
        .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
        let target = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingCharProbe)?
            .char_snapshot(params[1])
            .ok_or(ScriptVmError::MissingChar(params[1]))?;
        let mode = locate_player_mode(command)?;
        self.process_locate_target_result(
            scripts,
            *ip,
            player.position,
            player.current_vehicle.is_some(),
            target.position,
            mode,
            is_3d,
            &params[2..],
            true,
        )
    }

    pub(crate) fn locate_char_char_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_LOCATE_CHAR_ANY_MEANS_CHAR_3D
                | COMMAND_LOCATE_CHAR_ON_FOOT_CHAR_3D
                | COMMAND_LOCATE_CHAR_IN_CAR_CHAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 6 } else { 5 })?;
        let actor = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingCharProbe)?
            .char_snapshot(params[0])
            .ok_or(ScriptVmError::MissingChar(params[0]))?;
        let target = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingCharProbe)?
            .char_snapshot(params[1])
            .ok_or(ScriptVmError::MissingChar(params[1]))?;
        let mode = locate_char_mode(command)?;
        self.process_locate_target_result(
            scripts,
            *ip,
            actor.position,
            actor.current_vehicle.is_some(),
            target.position,
            mode,
            is_3d,
            &params[2..],
            true,
        )
    }

    pub(crate) fn locate_player_car_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        player_probe: &mut Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_LOCATE_PLAYER_ANY_MEANS_CAR_3D
                | COMMAND_LOCATE_PLAYER_ON_FOOT_CAR_3D
                | COMMAND_LOCATE_PLAYER_IN_CAR_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 6 } else { 5 })?;
        let player = player_probe
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
        .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
        let target = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
            .vehicle_snapshot(params[1])
            .ok_or(ScriptVmError::MissingVehicle(params[1]))?;
        let actor_position = player
            .current_vehicle
            .and_then(|handle| {
                entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.vehicle_snapshot(handle))
                    .map(|vehicle| vehicle.position)
            })
            .unwrap_or(player.position);
        self.process_locate_target_result(
            scripts,
            *ip,
            actor_position,
            player.current_vehicle.is_some(),
            target.position,
            locate_player_mode(command)?,
            is_3d,
            &params[2..],
            false,
        )
    }

    pub(crate) fn locate_char_car_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_LOCATE_CHAR_ANY_MEANS_CAR_3D
                | COMMAND_LOCATE_CHAR_ON_FOOT_CAR_3D
                | COMMAND_LOCATE_CHAR_IN_CAR_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 6 } else { 5 })?;
        let actor = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingCharProbe)?
            .char_snapshot(params[0])
            .ok_or(ScriptVmError::MissingChar(params[0]))?;
        let target = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
            .vehicle_snapshot(params[1])
            .ok_or(ScriptVmError::MissingVehicle(params[1]))?;
        let actor_position = actor
            .current_vehicle
            .and_then(|handle| {
                entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.vehicle_snapshot(handle))
                    .map(|vehicle| vehicle.position)
            })
            .unwrap_or(actor.position);
        self.process_locate_target_result(
            scripts,
            *ip,
            actor_position,
            actor.current_vehicle.is_some(),
            target.position,
            locate_char_mode(command)?,
            is_3d,
            &params[2..],
            false,
        )
    }

    pub(crate) fn locate_car_area_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
        entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    ) -> Result<(), ScriptVmError> {
        let is_3d = matches!(
            command,
            COMMAND_LOCATE_CAR_3D | COMMAND_LOCATE_STOPPED_CAR_3D
        );
        let params = self.collect_parameters(scripts, ip, if is_3d { 8 } else { 6 })?;
        let vehicle = entity_runtime
            .as_deref_mut()
            .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
            .vehicle_snapshot(params[0])
            .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
        self.process_locate_area_result(
            scripts,
            *ip,
            vehicle.position,
            true,
            vehicle.stopped,
            matches!(
                command,
                COMMAND_LOCATE_STOPPED_CAR_2D | COMMAND_LOCATE_STOPPED_CAR_3D
            ),
            LocateMode::AnyMeans,
            is_3d,
            &params[1..],
            false,
        )
    }

    pub(crate) fn process_locate_area_result(
        &mut self,
        scripts: &mut CTheScripts,
        script_ip: u32,
        actor_position: CVector,
        actor_in_vehicle: bool,
        actor_stopped: bool,
        stopped_required: bool,
        mode: LocateMode,
        is_3d: bool,
        params: &[i32],
        buggy_target_highlight: bool,
    ) -> Result<(), ScriptVmError> {
        let center = if is_3d {
            CVector::new(
                Self::script_param_as_f32(params[0]),
                Self::script_param_as_f32(params[1]),
                Self::script_param_as_f32(params[2]),
            )
        } else {
            CVector::new(
                Self::script_param_as_f32(params[0]),
                Self::script_param_as_f32(params[1]),
                MAP_Z_LOW_LIMIT,
            )
        };
        let delta_offset = if is_3d { 3 } else { 2 };
        let delta = CVector::new(
            Self::script_param_as_f32(params[delta_offset]),
            Self::script_param_as_f32(params[delta_offset + 1]),
            if is_3d {
                Self::script_param_as_f32(params[delta_offset + 2])
            } else {
                0.0
            },
        );
        let debug = params[if is_3d { 6 } else { 4 }] != 0;
        self.process_locate_result(
            scripts,
            script_ip,
            actor_position,
            actor_in_vehicle,
            actor_stopped,
            stopped_required,
            mode,
            center,
            delta,
            is_3d,
            debug,
            buggy_target_highlight,
        );
        Ok(())
    }

    pub(crate) fn process_locate_target_result(
        &mut self,
        scripts: &mut CTheScripts,
        script_ip: u32,
        actor_position: CVector,
        actor_in_vehicle: bool,
        target_position: CVector,
        mode: LocateMode,
        is_3d: bool,
        params: &[i32],
        buggy_target_highlight: bool,
    ) -> Result<(), ScriptVmError> {
        let delta = CVector::new(
            Self::script_param_as_f32(params[0]),
            Self::script_param_as_f32(params[1]),
            if is_3d {
                Self::script_param_as_f32(params[2])
            } else {
                0.0
            },
        );
        let debug = params[if is_3d { 3 } else { 2 }] != 0;
        self.process_locate_result(
            scripts,
            script_ip,
            actor_position,
            actor_in_vehicle,
            true,
            false,
            mode,
            target_position,
            delta,
            is_3d,
            debug,
            buggy_target_highlight,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn process_locate_result(
        &mut self,
        scripts: &mut CTheScripts,
        script_ip: u32,
        actor_position: CVector,
        actor_in_vehicle: bool,
        actor_stopped: bool,
        stopped_required: bool,
        mode: LocateMode,
        center: CVector,
        delta: CVector,
        is_3d: bool,
        debug: bool,
        buggy_target_highlight: bool,
    ) {
        let in_area = if is_3d {
            actor_position.x >= center.x - delta.x
                && actor_position.x <= center.x + delta.x
                && actor_position.y >= center.y - delta.y
                && actor_position.y <= center.y + delta.y
                && actor_position.z >= center.z - delta.z
                && actor_position.z <= center.z + delta.z
        } else {
            actor_position.x >= center.x - delta.x
                && actor_position.x <= center.x + delta.x
                && actor_position.y >= center.y - delta.y
                && actor_position.y <= center.y + delta.y
        };
        let result = if stopped_required && !actor_stopped {
            false
        } else if in_area {
            match mode {
                LocateMode::AnyMeans => true,
                LocateMode::OnFoot => !actor_in_vehicle,
                LocateMode::InCar => actor_in_vehicle,
            }
        } else {
            false
        };
        self.update_compare_flag(result);

        if debug {
            scripts
                .pending_effects
                .push(ScriptCommandEffect::HighlightArea {
                    script_ip,
                    x1: center.x - delta.x,
                    y1: center.y - delta.y,
                    x2: center.x + delta.x,
                    y2: if buggy_target_highlight {
                        center.x + delta.x
                    } else {
                        center.y + delta.y
                    },
                    z: if is_3d { center.z } else { MAP_Z_LOW_LIMIT },
                });
        }
        if scripts.dbg_flag {
            if is_3d {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DebugCube {
                        x1: center.x - delta.x,
                        y1: center.y - delta.y,
                        z1: center.z - delta.z,
                        x2: center.x + delta.x,
                        y2: center.y + delta.y,
                        z2: center.z + delta.z,
                    });
            } else {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DebugSquare {
                        x1: center.x - delta.x,
                        y1: center.y - delta.y,
                        x2: center.x + delta.x,
                        y2: center.y + delta.y,
                    });
            }
        }
    }

    pub(crate) fn apply_binary_var_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        destination_type: i16,
        source_type: i16,
        operation: ScriptBinaryOp,
        is_float: bool,
        source_multiplier: f32,
    ) -> Result<(), ScriptVmError> {
        let destination = self.get_pointer_to_script_variable(scripts, ip, destination_type)?;
        let source = self.get_pointer_to_script_variable(scripts, ip, source_type)?;
        let lhs = scripts.get_script_var(destination, self)?;
        let rhs = scripts.get_script_var(source, self)?;
        let result = if is_float {
            let lhs = Self::script_param_as_f32(lhs);
            let rhs = Self::script_param_as_f32(rhs) * source_multiplier;
            Self::f32_to_script_param(match operation {
                ScriptBinaryOp::Add => lhs + rhs,
                ScriptBinaryOp::Sub => lhs - rhs,
                ScriptBinaryOp::Mul => lhs * rhs,
                ScriptBinaryOp::Div => lhs / rhs,
            })
        } else {
            match operation {
                ScriptBinaryOp::Add => lhs.wrapping_add(rhs),
                ScriptBinaryOp::Sub => lhs.wrapping_sub(rhs),
                ScriptBinaryOp::Mul => lhs.wrapping_mul(rhs),
                ScriptBinaryOp::Div => lhs / rhs,
            }
        };
        scripts.set_script_var(destination, self, result)
    }

    pub(crate) fn apply_timed_value_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        destination_type: i16,
        operation: ScriptBinaryOp,
        time_step: f32,
    ) -> Result<(), ScriptVmError> {
        let destination = self.get_pointer_to_script_variable(scripts, ip, destination_type)?;
        let value = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
        let current = Self::script_param_as_f32(scripts.get_script_var(destination, self)?);
        let scaled_value = time_step * value;
        let result = match operation {
            ScriptBinaryOp::Add => current + scaled_value,
            ScriptBinaryOp::Sub => current - scaled_value,
            _ => return Err(ScriptVmError::UnsupportedCommand(-1)),
        };
        scripts.set_script_var(destination, self, Self::f32_to_script_param(result))
    }

    pub(crate) fn copy_script_var(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        destination_type: i16,
        source_type: i16,
        float_to_int_cast: bool,
    ) -> Result<(), ScriptVmError> {
        let destination = self.get_pointer_to_script_variable(scripts, ip, destination_type)?;
        let source = self.get_pointer_to_script_variable(scripts, ip, source_type)?;
        let source_value = scripts.get_script_var(source, self)?;
        let value = if float_to_int_cast {
            Self::script_param_as_f32(source_value) as i32
        } else {
            source_value
        };
        scripts.set_script_var(destination, self, value)
    }

    pub(crate) fn copy_script_int_to_float(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        destination_type: i16,
        source_type: i16,
    ) -> Result<(), ScriptVmError> {
        let destination = self.get_pointer_to_script_variable(scripts, ip, destination_type)?;
        let source = self.get_pointer_to_script_variable(scripts, ip, source_type)?;
        let source_value = scripts.get_script_var(source, self)? as f32;
        scripts.set_script_var(destination, self, Self::f32_to_script_param(source_value))
    }

    pub(crate) fn abs_script_var(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        var_type: i16,
        is_float: bool,
    ) -> Result<(), ScriptVmError> {
        let pointer = self.get_pointer_to_script_variable(scripts, ip, var_type)?;
        let current = scripts.get_script_var(pointer, self)?;
        let result = if is_float {
            Self::f32_to_script_param(Self::script_param_as_f32(current).abs())
        } else {
            current.abs()
        };
        scripts.set_script_var(pointer, self, result)
    }

    pub(crate) fn process_int_compare_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
    ) -> Result<bool, ScriptVmError> {
        let result = match command {
            COMMAND_IS_INT_VAR_GREATER_THAN_NUMBER => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.collect_parameters(scripts, ip, 1)?[0];
                lhs > rhs
            }
            COMMAND_IS_INT_LVAR_GREATER_THAN_NUMBER => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.collect_parameters(scripts, ip, 1)?[0];
                lhs > rhs
            }
            COMMAND_IS_NUMBER_GREATER_THAN_INT_VAR => {
                let lhs = self.collect_parameters(scripts, ip, 1)?[0];
                let rhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                lhs > rhs
            }
            COMMAND_IS_NUMBER_GREATER_THAN_INT_LVAR => {
                let lhs = self.collect_parameters(scripts, ip, 1)?[0];
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs > rhs
            }
            COMMAND_IS_INT_VAR_GREATER_THAN_INT_VAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                lhs > rhs
            }
            COMMAND_IS_INT_LVAR_GREATER_THAN_INT_LVAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs > rhs
            }
            COMMAND_IS_INT_VAR_GREATER_THAN_INT_LVAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs > rhs
            }
            COMMAND_IS_INT_LVAR_GREATER_THAN_INT_VAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                lhs > rhs
            }
            COMMAND_IS_INT_VAR_GREATER_OR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.collect_parameters(scripts, ip, 1)?[0];
                lhs >= rhs
            }
            COMMAND_IS_INT_LVAR_GREATER_OR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.collect_parameters(scripts, ip, 1)?[0];
                lhs >= rhs
            }
            COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_INT_VAR => {
                let lhs = self.collect_parameters(scripts, ip, 1)?[0];
                let rhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                lhs >= rhs
            }
            COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_INT_LVAR => {
                let lhs = self.collect_parameters(scripts, ip, 1)?[0];
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs >= rhs
            }
            COMMAND_IS_INT_VAR_GREATER_OR_EQUAL_TO_INT_VAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                lhs >= rhs
            }
            COMMAND_IS_INT_LVAR_GREATER_OR_EQUAL_TO_INT_LVAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs >= rhs
            }
            COMMAND_IS_INT_VAR_GREATER_OR_EQUAL_TO_INT_LVAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs >= rhs
            }
            COMMAND_IS_INT_LVAR_GREATER_OR_EQUAL_TO_INT_VAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                lhs >= rhs
            }
            COMMAND_IS_INT_VAR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.collect_parameters(scripts, ip, 1)?[0];
                lhs == rhs
            }
            COMMAND_IS_INT_LVAR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.collect_parameters(scripts, ip, 1)?[0];
                lhs == rhs
            }
            COMMAND_IS_INT_VAR_EQUAL_TO_INT_VAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                lhs == rhs
            }
            COMMAND_IS_INT_LVAR_EQUAL_TO_INT_LVAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs == rhs
            }
            COMMAND_IS_INT_VAR_EQUAL_TO_INT_LVAR => {
                let lhs = self.read_script_var_i32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_i32(scripts, ip, VAR_LOCAL)?;
                lhs == rhs
            }
            _ => return Err(ScriptVmError::UnsupportedCommand(command)),
        };
        Ok(result)
    }

    pub(crate) fn process_float_compare_command(
        &mut self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
        command: i32,
    ) -> Result<bool, ScriptVmError> {
        let result = match command {
            COMMAND_IS_FLOAT_VAR_GREATER_THAN_NUMBER => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                lhs > rhs
            }
            COMMAND_IS_FLOAT_LVAR_GREATER_THAN_NUMBER => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                lhs > rhs
            }
            COMMAND_IS_NUMBER_GREATER_THAN_FLOAT_VAR => {
                let lhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                let rhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                lhs > rhs
            }
            COMMAND_IS_NUMBER_GREATER_THAN_FLOAT_LVAR => {
                let lhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs > rhs
            }
            COMMAND_IS_FLOAT_VAR_GREATER_THAN_FLOAT_VAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                lhs > rhs
            }
            COMMAND_IS_FLOAT_LVAR_GREATER_THAN_FLOAT_LVAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs > rhs
            }
            COMMAND_IS_FLOAT_VAR_GREATER_THAN_FLOAT_LVAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs > rhs
            }
            COMMAND_IS_FLOAT_LVAR_GREATER_THAN_FLOAT_VAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                lhs > rhs
            }
            COMMAND_IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                lhs >= rhs
            }
            COMMAND_IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                lhs >= rhs
            }
            COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_FLOAT_VAR => {
                let lhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                let rhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                lhs >= rhs
            }
            COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_FLOAT_LVAR => {
                let lhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs >= rhs
            }
            COMMAND_IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_FLOAT_VAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                lhs >= rhs
            }
            COMMAND_IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_FLOAT_LVAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs >= rhs
            }
            COMMAND_IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_FLOAT_LVAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs >= rhs
            }
            COMMAND_IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_FLOAT_VAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                lhs >= rhs
            }
            COMMAND_IS_FLOAT_VAR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                lhs == rhs
            }
            COMMAND_IS_FLOAT_LVAR_EQUAL_TO_NUMBER => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = Self::script_param_as_f32(self.collect_parameters(scripts, ip, 1)?[0]);
                lhs == rhs
            }
            COMMAND_IS_FLOAT_VAR_EQUAL_TO_FLOAT_VAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                lhs == rhs
            }
            COMMAND_IS_FLOAT_LVAR_EQUAL_TO_FLOAT_LVAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs == rhs
            }
            COMMAND_IS_FLOAT_VAR_EQUAL_TO_FLOAT_LVAR => {
                let lhs = self.read_script_var_f32(scripts, ip, VAR_GLOBAL)?;
                let rhs = self.read_script_var_f32(scripts, ip, VAR_LOCAL)?;
                lhs == rhs
            }
            _ => return Err(ScriptVmError::UnsupportedCommand(command)),
        };
        Ok(result)
    }

    pub(crate) fn read_script_var_i32(
        &self,
        scripts: &CTheScripts,
        ip: &mut u32,
        var_type: i16,
    ) -> Result<i32, ScriptVmError> {
        let pointer = self.get_pointer_to_script_variable(scripts, ip, var_type)?;
        scripts.get_script_var(pointer, self)
    }

    pub(crate) fn read_script_var_f32(
        &self,
        scripts: &CTheScripts,
        ip: &mut u32,
        var_type: i16,
    ) -> Result<f32, ScriptVmError> {
        Ok(Self::script_param_as_f32(
            self.read_script_var_i32(scripts, ip, var_type)?,
        ))
    }

    pub(crate) fn script_param_as_f32(value: i32) -> f32 {
        f32::from_bits(value as u32)
    }

    pub(crate) fn f32_to_script_param(value: f32) -> i32 {
        value.to_bits() as i32
    }

    pub(crate) fn apply_value_command(
        &mut self,
        scripts: &mut CTheScripts,
        pointer: ScriptVarPointer,
        value: i32,
        command: i32,
    ) -> Result<(), ScriptVmError> {
        let current = scripts.get_script_var(pointer, self)?;
        let result = match command {
            COMMAND_ADD_VAL_TO_INT_VAR | COMMAND_ADD_VAL_TO_INT_LVAR => current.wrapping_add(value),
            COMMAND_SUB_VAL_FROM_INT_VAR | COMMAND_SUB_VAL_FROM_INT_LVAR => {
                current.wrapping_sub(value)
            }
            COMMAND_MULT_INT_VAR_BY_VAL | COMMAND_MULT_INT_LVAR_BY_VAL => {
                current.wrapping_mul(value)
            }
            COMMAND_DIV_INT_VAR_BY_VAL | COMMAND_DIV_INT_LVAR_BY_VAL => current / value,
            COMMAND_ADD_VAL_TO_FLOAT_VAR | COMMAND_ADD_VAL_TO_FLOAT_LVAR => {
                (f32::from_bits(current as u32) + f32::from_bits(value as u32)).to_bits() as i32
            }
            COMMAND_SUB_VAL_FROM_FLOAT_VAR | COMMAND_SUB_VAL_FROM_FLOAT_LVAR => {
                (f32::from_bits(current as u32) - f32::from_bits(value as u32)).to_bits() as i32
            }
            COMMAND_MULT_FLOAT_VAR_BY_VAL | COMMAND_MULT_FLOAT_LVAR_BY_VAL => {
                (f32::from_bits(current as u32) * f32::from_bits(value as u32)).to_bits() as i32
            }
            COMMAND_DIV_FLOAT_VAR_BY_VAL | COMMAND_DIV_FLOAT_LVAR_BY_VAL => {
                (f32::from_bits(current as u32) / f32::from_bits(value as u32)).to_bits() as i32
            }
            _ => return Err(ScriptVmError::UnsupportedCommand(command)),
        };
        scripts.set_script_var(pointer, self, result)
    }

    pub(crate) fn collect_parameter_by_type(
        &self,
        scripts: &CTheScripts,
        ip: &mut u32,
        arg_type: u8,
    ) -> Result<i32, ScriptVmError> {
        match arg_type {
            ARGUMENT_INT32 => scripts.read_4_bytes_from_script(ip),
            ARGUMENT_GLOBALVAR => {
                let var_index = scripts.read_2_bytes_from_script(ip)? as usize;
                scripts.read_i32_at_script_space(var_index)
            }
            ARGUMENT_LOCALVAR => {
                let var_index = scripts.read_2_bytes_from_script(ip)? as usize;
                self.local_variables
                    .get(var_index)
                    .copied()
                    .ok_or(ScriptVmError::LocalVariableOutOfRange(var_index))
            }
            // re3 Read1ByteFromScript / Read2BytesFromScript return signed values.
            ARGUMENT_INT8 => Ok(i32::from(scripts.read_1_byte_from_script(ip)? as i8)),
            ARGUMENT_INT16 => Ok(i32::from(scripts.read_2_bytes_from_script(ip)? as i16)),
            ARGUMENT_FLOAT => Ok(scripts.read_float_from_script(ip)?.to_bits() as i32),
            arg => Err(ScriptVmError::InvalidArgumentType(arg)),
        }
    }

    pub(crate) fn collect_one_parameter(
        &self,
        scripts: &mut CTheScripts,
        ip: &mut u32,
    ) -> Result<i32, ScriptVmError> {
        self.collect_one_parameter_readonly(scripts, ip)
    }

    pub(crate) fn collect_one_parameter_readonly(
        &self,
        scripts: &CTheScripts,
        ip: &mut u32,
    ) -> Result<i32, ScriptVmError> {
        match scripts.read_1_byte_from_script(ip)? {
            ARGUMENT_INT32 => scripts.read_4_bytes_from_script(ip),
            ARGUMENT_GLOBALVAR => {
                let var_index = scripts.read_2_bytes_from_script(ip)? as usize;
                if var_index < 8 || var_index >= scripts.get_size_of_variable_space() {
                    return Err(ScriptVmError::GlobalVariableOutOfRange(var_index));
                }
                scripts.read_i32_at_script_space(var_index)
            }
            ARGUMENT_LOCALVAR => {
                let var_index = scripts.read_2_bytes_from_script(ip)? as usize;
                self.local_variables
                    .get(var_index)
                    .copied()
                    .ok_or(ScriptVmError::LocalVariableOutOfRange(var_index))
            }
            // re3 Read1ByteFromScript / Read2BytesFromScript return signed values.
            ARGUMENT_INT8 => Ok(i32::from(scripts.read_1_byte_from_script(ip)? as i8)),
            ARGUMENT_INT16 => Ok(i32::from(scripts.read_2_bytes_from_script(ip)? as i16)),
            ARGUMENT_FLOAT => Ok(scripts.read_float_from_script(ip)?.to_bits() as i32),
            arg => Err(ScriptVmError::InvalidArgumentType(arg)),
        }
    }
}
