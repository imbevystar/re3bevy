//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! SCM opcode dispatch (single verbatim dispatcher method).

#[allow(unused_imports)]
use crate::entities::EntityReference::EntityReference;
use super::Script::{
    CRunningScript, face_coord_heading_degrees,
    point_in_area_2d, point_in_area_3d, point_in_script_zone, remap_script_ped_model,
    script_jump_ip,
};
#[allow(unused_imports)]
use super::ScriptCommands::*;
#[allow(unused_imports)]
use super::ScriptTypes::{
    ANDOR_NONE, ANDS_1, ANDS_8, ARGUMENT_END, ARGUMENT_GLOBALVAR, CLEANUP_CAR, CLEANUP_CHAR,
    CLEANUP_OBJECT, CTheScripts, KEY_LENGTH_IN_SCRIPT, MAP_Z_LOW_LIMIT, MAX_NUM_SCRIPT_SPRITES,
    MAX_STACK_DEPTH, ORS_1, ORS_8, ScriptBinaryOp, ScriptCommandEffect, ScriptCommandOutcome,
    ScriptCreateCarRequest, ScriptCreateCharRequest, ScriptCreateObjectRequest,
    ScriptEntityMutation, ScriptEntityRuntimeAccess, ScriptObjectMotionRequest,
    ScriptPathNodeSnapshot, ScriptPlayerSnapshot, ScriptStatsEffect, ScriptVmError,
    ScriptWorldStateSnapshot, VAR_GLOBAL, VAR_LOCAL, run_script_entity_creation,
    run_script_object_motion,
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
use crate::control::Garages::{EGarageState, EGarageType};
#[allow(unused_imports)]
use crate::control::OnscreenTimer::COnscreenTimer;
#[allow(unused_imports)]
use crate::control::PathFind::CPathFind;
#[allow(unused_imports)]
use crate::control::Pickups::PICKUP_MINE_INACTIVE;
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
use crate::core::Fire::ScriptFireTarget;

fn allocate_collective_id(scripts: &mut CTheScripts) -> i32 {
    let index = scripts.next_free_collective_index.min(i32::MAX as usize) as i32;
    scripts.next_free_collective_index = if index == i32::MAX {
        0
    } else {
        index as usize + 1
    };
    index
}

fn add_peds_to_collective(scripts: &mut CTheScripts, collective_index: i32, members: Vec<i32>) {
    for handle in members {
        let Some(slot) = scripts
            .collectives
            .iter()
            .position(|entry| entry.col_index == -1)
        else {
            break;
        };
        scripts.collectives[slot].col_index = collective_index;
        scripts.collectives[slot].ped_index = handle;
    }
}

fn reserve_shared_fire_slot(
    scripts: &mut CTheScripts,
    entity_runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
) -> i32 {
    let slot = (0..scripts.script_fire_slots.len()).find(|slot| {
        !scripts.script_fire_slots[*slot]
            && entity_runtime
                .as_deref_mut()
                .and_then(|runtime| runtime.fire_slot_is_free(*slot))
                .unwrap_or(true)
    });
    let Some(slot) = slot else {
        return -1;
    };
    scripts
        .reserve_script_fire_slot_at(slot)
        .then_some(slot as i32)
        .unwrap_or(-1)
}
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
use crate::core::Streaming::MAXVEHICLESLOADED;
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
use crate::modelinfo::ModelIndices::{
    MI_BORGNINE, MI_CABBIE, MI_CARMINE, MI_DODO, MI_NAUTICALMINE, MI_RCBANDIT, MI_TAXI,
};
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
use crate::weapons::WeaponType::eWeaponType;
#[allow(unused_imports)]
use crate::weapons::WeaponType::script_weapon_type;
#[allow(unused_imports)]
use bevy::input::{ButtonInput, mouse::MouseButton};
#[allow(unused_imports)]
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
#[allow(unused_imports)]
use bevy::prelude::{KeyCode, error};
#[allow(unused_imports)]
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use re3bevy_assets::streaming::CdStream::CdStream;
#[allow(unused_imports)]
use re3bevy_core::core::GameState::{CGameState, ELevelName};
#[allow(unused_imports)]
use re3bevy_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral};

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
use re3bevy_core::core::Timer::CTimer;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

fn grab_phone_from_script_mirror(scripts: &mut CTheScripts, x: f32, y: f32) -> i32 {
    let start = scripts.script_phones_max.clamp(0, 50) as usize;
    let end = scripts.phone_max.clamp(0, 50) as usize;
    if start >= end {
        return -1;
    }

    let mut nearest = None;
    let mut nearest_distance = 100.0_f32;
    for phone_id in start..end {
        let phone = scripts.phone_positions[phone_id];
        let dx = phone.x - x;
        let dy = phone.y - y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance < nearest_distance {
            nearest_distance = distance;
            nearest = Some(phone_id);
        }
    }
    let Some(nearest) = nearest else {
        return -1;
    };

    scripts.phone_states[nearest] = 3;
    scripts.phone_positions.swap(start, nearest);
    scripts.phone_states.swap(start, nearest);
    scripts.phone_message_displayed.swap(start, nearest);
    scripts.script_phones_max += 1;
    scripts.script_phones_max - 1
}

impl CRunningScript {
    pub(crate) fn process_one_command_with_3d_ground_runtime(
        &mut self,
        scripts: &mut CTheScripts,
        time_ms: u32,
        time_step: f32,
        mut general: Option<&mut CGeneral>,
        mut clock: Option<&mut CClock>,
        mut camera_probe: Option<&mut dyn FnMut(CVector, f32) -> bool>,
        mut ground_probe: Option<&mut dyn FnMut(f32, f32) -> f32>,
        mut ground_3d_probe: Option<&mut dyn FnMut(f32, f32, f32) -> f32>,
        mut path_node_probe: Option<
            &mut dyn FnMut(CVector, u8, bool, bool) -> Option<ScriptPathNodeSnapshot>,
        >,
        mut entity_runtime: Option<&mut dyn ScriptEntityRuntimeAccess>,
        mut player_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        mut pad_probe: Option<&mut dyn FnMut(i32, i32) -> i16>,
        mut world_state_probe: Option<&mut dyn FnMut() -> ScriptWorldStateSnapshot>,
    ) -> Result<ScriptCommandOutcome, ScriptVmError> {
        scripts.commands_executed = scripts.commands_executed.saturating_add(1);
        let mut ip = self.ip;
        let raw = scripts.read_2_bytes_from_script(&mut ip)?;
        self.not_flag = raw & 0x8000 != 0;
        let command = i32::from(raw & 0x7FFF);
        let mut outcome = ScriptCommandOutcome {
            command,
            yielded: false,
            terminated: false,
            ip_before: self.ip,
            ip_after: self.ip,
            not_flag: self.not_flag,
            script_index: usize::MAX,
        };

        match command {
            COMMAND_NOP => {}
            COMMAND_WAIT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.wake_time = time_ms.saturating_add(params[0] as u32);
                self.skip_wake_time = false;
                outcome.yielded = true;
            }
            COMMAND_GOTO => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                ip = script_jump_ip(params[0]);
            }
            COMMAND_SHAKE_CAM => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ShakeCamera {
                        strength: params[0] as f32 / 1000.0,
                        position: None,
                    });
            }
            COMMAND_SHAKE_CAM_WITH_POINT => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ShakeCamera {
                        strength: params[0] as f32 / 1000.0,
                        position: Some(CVector::new(
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                            Self::script_param_as_f32(params[3]),
                        )),
                    });
            }
            COMMAND_SHAKE_PAD => {
                // Script2.cpp omits the visible CollectParameters call, but
                // ScriptDebug.cpp defines the command as three inputs and
                // StartShake consumes precisely those values.
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts.pending_effects.push(ScriptCommandEffect::ShakePad {
                    pad: params[0],
                    duration: i32::from(params[1] as i16).max(0) as u32,
                    frequency: u32::from(params[2] as u8),
                });
            }
            COMMAND_SET_VAR_INT | COMMAND_SET_VAR_FLOAT => {
                let pointer = self.get_pointer_to_script_variable(scripts, &mut ip, VAR_GLOBAL)?;
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.set_script_var(pointer, self, params[0])?;
            }
            COMMAND_SET_LVAR_INT | COMMAND_SET_LVAR_FLOAT => {
                let pointer = self.get_pointer_to_script_variable(scripts, &mut ip, VAR_LOCAL)?;
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.set_script_var(pointer, self, params[0])?;
            }
            COMMAND_ADD_VAL_TO_INT_VAR
            | COMMAND_ADD_VAL_TO_FLOAT_VAR
            | COMMAND_SUB_VAL_FROM_INT_VAR
            | COMMAND_SUB_VAL_FROM_FLOAT_VAR
            | COMMAND_MULT_INT_VAR_BY_VAL
            | COMMAND_MULT_FLOAT_VAR_BY_VAL
            | COMMAND_DIV_INT_VAR_BY_VAL
            | COMMAND_DIV_FLOAT_VAR_BY_VAL => {
                let pointer = self.get_pointer_to_script_variable(scripts, &mut ip, VAR_GLOBAL)?;
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.apply_value_command(scripts, pointer, params[0], command)?;
            }
            COMMAND_ADD_VAL_TO_INT_LVAR
            | COMMAND_ADD_VAL_TO_FLOAT_LVAR
            | COMMAND_SUB_VAL_FROM_INT_LVAR
            | COMMAND_SUB_VAL_FROM_FLOAT_LVAR
            | COMMAND_MULT_INT_LVAR_BY_VAL
            | COMMAND_MULT_FLOAT_LVAR_BY_VAL
            | COMMAND_DIV_INT_LVAR_BY_VAL
            | COMMAND_DIV_FLOAT_LVAR_BY_VAL => {
                let pointer = self.get_pointer_to_script_variable(scripts, &mut ip, VAR_LOCAL)?;
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.apply_value_command(scripts, pointer, params[0], command)?;
            }
            COMMAND_IS_INT_VAR_GREATER_THAN_NUMBER
            | COMMAND_IS_INT_LVAR_GREATER_THAN_NUMBER
            | COMMAND_IS_NUMBER_GREATER_THAN_INT_VAR
            | COMMAND_IS_NUMBER_GREATER_THAN_INT_LVAR
            | COMMAND_IS_INT_VAR_GREATER_THAN_INT_VAR
            | COMMAND_IS_INT_LVAR_GREATER_THAN_INT_LVAR
            | COMMAND_IS_INT_VAR_GREATER_THAN_INT_LVAR
            | COMMAND_IS_INT_LVAR_GREATER_THAN_INT_VAR
            | COMMAND_IS_INT_VAR_GREATER_OR_EQUAL_TO_NUMBER
            | COMMAND_IS_INT_LVAR_GREATER_OR_EQUAL_TO_NUMBER
            | COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_INT_VAR
            | COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_INT_LVAR
            | COMMAND_IS_INT_VAR_GREATER_OR_EQUAL_TO_INT_VAR
            | COMMAND_IS_INT_LVAR_GREATER_OR_EQUAL_TO_INT_LVAR
            | COMMAND_IS_INT_VAR_GREATER_OR_EQUAL_TO_INT_LVAR
            | COMMAND_IS_INT_LVAR_GREATER_OR_EQUAL_TO_INT_VAR
            | COMMAND_IS_INT_VAR_EQUAL_TO_NUMBER
            | COMMAND_IS_INT_LVAR_EQUAL_TO_NUMBER
            | COMMAND_IS_INT_VAR_EQUAL_TO_INT_VAR
            | COMMAND_IS_INT_LVAR_EQUAL_TO_INT_LVAR
            | COMMAND_IS_INT_VAR_EQUAL_TO_INT_LVAR => {
                let result = self.process_int_compare_command(scripts, &mut ip, command)?;
                self.update_compare_flag(result);
            }
            COMMAND_IS_FLOAT_VAR_GREATER_THAN_NUMBER
            | COMMAND_IS_FLOAT_LVAR_GREATER_THAN_NUMBER
            | COMMAND_IS_NUMBER_GREATER_THAN_FLOAT_VAR
            | COMMAND_IS_NUMBER_GREATER_THAN_FLOAT_LVAR
            | COMMAND_IS_FLOAT_VAR_GREATER_THAN_FLOAT_VAR
            | COMMAND_IS_FLOAT_LVAR_GREATER_THAN_FLOAT_LVAR
            | COMMAND_IS_FLOAT_VAR_GREATER_THAN_FLOAT_LVAR
            | COMMAND_IS_FLOAT_LVAR_GREATER_THAN_FLOAT_VAR
            | COMMAND_IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_NUMBER
            | COMMAND_IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_NUMBER
            | COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_FLOAT_VAR
            | COMMAND_IS_NUMBER_GREATER_OR_EQUAL_TO_FLOAT_LVAR
            | COMMAND_IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_FLOAT_VAR
            | COMMAND_IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_FLOAT_LVAR
            | COMMAND_IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_FLOAT_LVAR
            | COMMAND_IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_FLOAT_VAR
            | COMMAND_IS_FLOAT_VAR_EQUAL_TO_NUMBER
            | COMMAND_IS_FLOAT_LVAR_EQUAL_TO_NUMBER
            | COMMAND_IS_FLOAT_VAR_EQUAL_TO_FLOAT_VAR
            | COMMAND_IS_FLOAT_LVAR_EQUAL_TO_FLOAT_LVAR
            | COMMAND_IS_FLOAT_VAR_EQUAL_TO_FLOAT_LVAR => {
                let result = self.process_float_compare_command(scripts, &mut ip, command)?;
                self.update_compare_flag(result);
            }
            COMMAND_GOTO_IF_TRUE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if self.cond_result {
                    ip = script_jump_ip(params[0]);
                }
            }
            COMMAND_GOTO_IF_FALSE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if !self.cond_result {
                    ip = script_jump_ip(params[0]);
                }
            }
            COMMAND_TERMINATE_THIS_SCRIPT => {
                if self.mission_flag {
                    scripts.already_running_a_mission_script = false;
                }
                outcome.yielded = true;
                outcome.terminated = true;
            }
            COMMAND_START_NEW_SCRIPT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if params[0] < 0 {
                    return Err(ScriptVmError::IpOutOfRange);
                }
                let new_script = scripts.start_new_script(params[0] as u32)?;
                let mut arg_index = 0usize;
                loop {
                    let arg_type = scripts.read_1_byte_from_script(&mut ip)?;
                    if arg_type == ARGUMENT_END {
                        break;
                    }
                    let value = self.collect_parameter_by_type(scripts, &mut ip, arg_type)?;
                    if let Some(local) = scripts.scripts[new_script]
                        .local_variables
                        .get_mut(arg_index)
                    {
                        *local = value;
                    }
                    arg_index += 1;
                }
            }
            COMMAND_GOSUB => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let stack_index = self.stack_pointer as usize;
                if stack_index >= MAX_STACK_DEPTH {
                    return Err(ScriptVmError::StackOverflow);
                }
                self.stack[stack_index] = ip;
                self.stack_pointer += 1;
                ip = script_jump_ip(params[0]);
            }
            COMMAND_GOSUB_FILE => {
                // re3 Script3.cpp COMMAND_GOSUB_FILE:
                //   CollectParameters(2); push m_nIp; SetIP(ScriptParams[0]);
                // Note: unlike COMMAND_GOSUB, negative labels are NOT remapped here.
                // params[1] is multi-script filename token (ignored at runtime).
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let stack_index = self.stack_pointer as usize;
                if stack_index >= MAX_STACK_DEPTH {
                    return Err(ScriptVmError::StackOverflow);
                }
                self.stack[stack_index] = ip;
                self.stack_pointer += 1;
                if params[0] < 0 {
                    return Err(ScriptVmError::IpOutOfRange);
                }
                ip = params[0] as u32;
            }
            COMMAND_RETURN => {
                if self.stack_pointer == 0 {
                    return Err(ScriptVmError::StackUnderflow);
                }
                self.stack_pointer -= 1;
                ip = self.stack[self.stack_pointer as usize];
            }
            COMMAND_SET_PLAYER_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerHeading {
                        player_index: params[0],
                        heading_degrees: Self::script_param_as_f32(params[1]),
                    });
            }
            COMMAND_GET_PLAYER_CHAR => {
                // re3 Script3.cpp: ped pool index of CWorld::Players[n].m_pPed.
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let player = params[0].max(0) as usize;
                let handle = scripts
                    .player_char_handles
                    .get(player)
                    .copied()
                    .filter(|h| *h != 0)
                    .unwrap_or(1);
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_CAR_IN_AIR_PROPER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let unique_jump_dodo = self.script_name[..3] == *b"usj"
                    && self.script_name[3] == 0
                    && snapshot.current_vehicle_model == Some(MI_DODO as i32);
                self.update_compare_flag(
                    snapshot.collision_records.iter().all(Option::is_none) && !unique_jump_dodo,
                );
            }
            COMMAND_IS_CAR_IN_AIR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(snapshot.automobile_drive_wheels_on_ground == Some(0));
            }
            COMMAND_IS_CAR_UPSIDEDOWN | COMMAND_IS_CAR_UPRIGHT => {
                // re3 Script3.cpp reads the live vehicle matrix here; this is
                // distinct from the timed upside-down watchdog.
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(if command == COMMAND_IS_CAR_UPSIDEDOWN {
                    snapshot.up_z <= -0.97
                } else {
                    snapshot.up_z >= 0.0
                });
            }
            COMMAND_DECLARE_MISSION_FLAG => {
                // re3: OnAMissionFlag = (uint16)Read2BytesFromScript(&++m_nIp);
                // Skips the ARGUMENT_GLOBALVAR type byte, stores global offset.
                let _type = scripts.read_1_byte_from_script(&mut ip)?;
                let offset = scripts.read_2_bytes_from_script(&mut ip)? as u32;
                scripts.on_a_mission_flag = offset;
            }
            COMMAND_DECLARE_MISSION_FLAG_FOR_CONTACT => {
                // re3: CollectParameters(1); flag = Read2Bytes(&++m_nIp);
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let _type = scripts.read_1_byte_from_script(&mut ip)?;
                let offset = scripts.read_2_bytes_from_script(&mut ip)? as i32;
                if let Some(slot) = scripts
                    .on_a_mission_for_contact_flag
                    .get_mut(params[0].max(0) as usize)
                {
                    *slot = offset;
                }
            }
            COMMAND_DECLARE_BASE_BRIEF_ID_FOR_CONTACT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                if let Some(slot) = scripts
                    .base_brief_id_for_contact
                    .get_mut(params[0].max(0) as usize)
                {
                    *slot = params[1];
                }
            }
            COMMAND_SET_DEATHARREST_STATE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.deatharrest_enabled = params[0] == 1;
            }
            COMMAND_HAS_DEATHARREST_BEEN_EXECUTED => {
                self.update_compare_flag(self.deatharrest_executed);
            }
            COMMAND_IS_PLAYER_DEAD => {
                let _params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.update_compare_flag(scripts.player_wb_state == 1);
            }
            COMMAND_HAS_PLAYER_BEEN_ARRESTED => {
                let _params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.update_compare_flag(scripts.player_wb_state == 2);
            }
            COMMAND_IS_PLAYER_PLAYING => {
                let _params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.update_compare_flag(scripts.player_wb_state == 0);
            }
            COMMAND_CAN_PLAYER_START_MISSION => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                // re3: IsPedInControl() || m_nPedState == PED_DRIVING.
                self.update_compare_flag(snapshot.is_in_control || snapshot.is_driving);
            }
            COMMAND_SET_GANG_CAR_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetGangCarModel {
                        gang: params[0],
                        model: params[1],
                    });
            }
            COMMAND_SET_GANG_WEAPONS => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetGangWeapons {
                        gang: params[0],
                        weapon1: params[1],
                        weapon2: params[2],
                    });
            }
            COMMAND_SET_THREAT_FOR_PED_TYPE | COMMAND_CLEAR_THREAT_FOR_PED_TYPE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                if let Some(threats) = scripts.ped_type_threats.get_mut(params[0] as usize) {
                    if command == COMMAND_SET_THREAT_FOR_PED_TYPE {
                        *threats |= params[1] as u32;
                    } else {
                        *threats &= !(params[1] as u32);
                    }
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetThreatForPedType {
                        ped_type: params[0],
                        threat: params[1],
                        add: command == COMMAND_SET_THREAT_FOR_PED_TYPE,
                    });
            }
            COMMAND_IS_THREAT_FOR_PED_TYPE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                self.update_compare_flag(
                    scripts
                        .ped_type_threats
                        .get(params[0] as usize)
                        .is_some_and(|threats| *threats & params[1] as u32 != 0),
                );
            }
            COMMAND_SET_FIXED_CAMERA_POSITION => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetFixedCameraPosition {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        orientation: CVector::new(
                            Self::script_param_as_f32(params[3]),
                            Self::script_param_as_f32(params[4]),
                            Self::script_param_as_f32(params[5]),
                        ),
                    });
            }
            COMMAND_POINT_CAMERA_AT_POINT => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PointCameraAtPoint {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        switch_style: params[3],
                    });
            }
            COMMAND_POINT_CAMERA_AT_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PointCameraAtPlayer {
                        player_index: params[0],
                        mode: params[1],
                        switch_style: params[2],
                    });
            }
            COMMAND_POINT_CAMERA_AT_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                // re3 resolves the ped-pool handle and passes that entity to
                // TheCamera.TakeControl with the requested mode/switch style.
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PointCameraAtChar {
                        handle: params[0],
                        position: snapshot.position,
                        mode: params[1],
                        switch_style: params[2],
                    });
            }
            COMMAND_POINT_CAMERA_AT_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PointCameraAtCar {
                        handle: params[0],
                        position: snapshot.position,
                        mode: params[1],
                        switch_style: params[2],
                    });
            }
            COMMAND_RESTORE_CAMERA => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RestoreCamera);
            }
            COMMAND_RESTORE_CAMERA_JUMPCUT => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RestoreCameraJumpcut);
            }
            COMMAND_GET_FADING_STATUS => {
                // Filled by runtime via pending query pattern: compare flag updated
                // in apply path through bridge.camera_fading if needed. re3 reads
                // TheCamera.GetFading(); we store last known on scripts resource.
                self.update_compare_flag(scripts.camera_fading);
            }
            COMMAND_FORCE_WEATHER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ForceWeather(params[0] as i16));
            }
            COMMAND_FORCE_WEATHER_NOW => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ForceWeatherNow(params[0] as i16));
            }
            COMMAND_RELEASE_WEATHER => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ReleaseWeather);
            }
            COMMAND_FORCE_RAIN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetScriptsForceRain(params[0] != 0));
            }
            COMMAND_SET_TIME_SCALE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetTimeScale(
                        Self::script_param_as_f32(params[0]),
                    ));
            }
            COMMAND_STORE_WEATHER => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StoreWeather);
            }
            COMMAND_RESTORE_WEATHER => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RestoreWeather);
            }
            COMMAND_STORE_CLOCK => {
                let clock = clock.as_deref_mut().ok_or(ScriptVmError::MissingClock)?;
                clock.store_clock();
            }
            COMMAND_RESTORE_CLOCK => {
                let clock = clock.as_deref_mut().ok_or(ScriptVmError::MissingClock)?;
                clock.restore_clock();
            }
            COMMAND_ALTER_WANTED_LEVEL | COMMAND_ALTER_WANTED_LEVEL_NO_DROP => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetWantedLevel {
                        player_index: params[0],
                        level: params[1],
                        no_drop: command == COMMAND_ALTER_WANTED_LEVEL_NO_DROP,
                    });
            }
            COMMAND_SET_MAX_WANTED_LEVEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetMaximumWantedLevel(params[0]));
            }
            COMMAND_IS_WANTED_LEVEL_GREATER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.wanted_level > params[1]);
            }
            COMMAND_SET_POLICE_IGNORE_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPoliceIgnorePlayer {
                        player_index: params[0],
                        ignored: params[1] != 0,
                    });
            }
            COMMAND_SET_EVERYONE_IGNORE_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetEveryoneIgnorePlayer {
                        player_index: params[0],
                        ignored: params[1] != 0,
                    });
            }
            COMMAND_CLEAR_WANTED_LEVEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearWantedLevel {
                        player_index: params[0],
                    });
            }
            COMMAND_SET_WANTED_MULTIPLIER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetWantedMultiplier(
                        Self::script_param_as_f32(params[0]),
                    ));
            }
            COMMAND_CREATE_PICKUP => {
                // re3 Script3.cpp: CollectParameters(5); resolve used-object model;
                // GenerateNewOne; StoreParameters(1).
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let model = scripts.resolve_used_object_model(params[0]);
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    ) + 0.5; // Script.h PICKUP_PLACEMENT_OFFSET
                }
                let handle = scripts.alloc_script_pickup_handle();
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreatePickup {
                        handle,
                        model_id: model,
                        pickup_type: params[1],
                        quantity: 0,
                        position,
                    });
            }
            COMMAND_CREATE_PICKUP_WITH_AMMO => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    ) + 0.5;
                }
                let handle = scripts.alloc_script_pickup_handle();
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreatePickup {
                        handle,
                        model_id: scripts.resolve_used_object_model(params[0]),
                        pickup_type: params[1],
                        quantity: params[2],
                        position,
                    });
            }
            COMMAND_CREATE_FLOATING_PACKAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    ) + 0.5;
                }
                let handle = scripts.alloc_script_pickup_handle();
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateFloatingPackage { handle, position });
            }
            COMMAND_MAKE_OBJECT_TARGETTABLE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::MakeObjectTargettable { handle: params[0] });
            }
            COMMAND_CREATE_OBJECT | COMMAND_CREATE_OBJECT_NO_OFFSET => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                let model_id = scripts.resolve_used_object_model(params[0]);
                let add_model_offset = command == COMMAND_CREATE_OBJECT;
                let request = ScriptEntityMutation::CreateObject(ScriptCreateObjectRequest {
                    model_id,
                    position,
                    add_model_offset,
                });
                let handle = if let Some(handle) = run_script_entity_creation(
                    &mut entity_runtime,
                    request,
                    general.as_deref_mut(),
                )? {
                    handle
                } else {
                    let handle = scripts.alloc_script_object_handle();
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::CreateObject {
                            handle,
                            model_id,
                            position,
                            add_model_offset,
                        });
                    handle
                };
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .add_entity_to_list(handle, CLEANUP_OBJECT);
                }
            }
            COMMAND_DELETE_OBJECT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DeleteObject { handle: params[0] });
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .remove_entity_from_list(params[0], CLEANUP_OBJECT);
                }
            }
            COMMAND_DONT_REMOVE_OBJECT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .mission_cleanup
                    .remove_entity_from_list(params[0], CLEANUP_OBJECT);
            }
            COMMAND_DONT_REMOVE_CHAR | COMMAND_DONT_REMOVE_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if command == COMMAND_DONT_REMOVE_CHAR {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingCharProbe)?
                        .char_snapshot(params[0])
                        .ok_or(ScriptVmError::MissingChar(params[0]))?;
                    scripts
                        .mission_cleanup
                        .remove_entity_from_list(params[0], CLEANUP_CHAR);
                } else {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(params[0])
                        .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                    scripts
                        .mission_cleanup
                        .remove_entity_from_list(params[0], CLEANUP_CAR);
                }
            }
            COMMAND_SET_GARAGE | COMMAND_SET_GARAGE_WITH_CAR_MODEL => {
                let count = if command == COMMAND_SET_GARAGE { 7 } else { 8 };
                let params = self.collect_parameters(scripts, &mut ip, count)?;
                let min = CVector::new(
                    Self::script_param_as_f32(params[0]).min(Self::script_param_as_f32(params[3])),
                    Self::script_param_as_f32(params[1]).min(Self::script_param_as_f32(params[4])),
                    Self::script_param_as_f32(params[2]).min(Self::script_param_as_f32(params[5])),
                );
                let max = CVector::new(
                    Self::script_param_as_f32(params[0]).max(Self::script_param_as_f32(params[3])),
                    Self::script_param_as_f32(params[1]).max(Self::script_param_as_f32(params[4])),
                    Self::script_param_as_f32(params[2]).max(Self::script_param_as_f32(params[5])),
                );
                let handle = scripts.garages_created;
                scripts.garages_created += 1;
                if let Ok(index) = usize::try_from(handle) {
                    if let Some(state) = scripts.garage_states.get_mut(index) {
                        *state = match EGarageType::from_re3(params[6] as u8) {
                            EGarageType::Bombshop1
                            | EGarageType::Bombshop2
                            | EGarageType::Bombshop3
                            | EGarageType::Respray
                            | EGarageType::Crusher => EGarageState::Opened as u8,
                            _ => EGarageState::FullyClosed as u8,
                        };
                    }
                }
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddGarage {
                        handle,
                        min,
                        max,
                        garage_type: params[6],
                        target_model: if count == 8 { params[7] } else { 0 },
                    });
            }
            COMMAND_SET_TARGET_CAR_FOR_MISSION_GARAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle_handle = if params[1] >= 0 {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(params[1])
                        .ok_or(ScriptVmError::MissingVehicle(params[1]))?;
                    Some(params[1])
                } else {
                    None
                };
                if vehicle_handle.is_some() {
                    if let Ok(index) = usize::try_from(params[0]) {
                        if let Some(state) = scripts.garage_states.get_mut(index) {
                            if *state == EGarageState::ClosedContainsCar as u8 {
                                *state = EGarageState::FullyClosed as u8;
                            }
                        }
                    }
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetTargetCarForMissionGarage {
                        handle: params[0],
                        vehicle_handle,
                    });
            }
            COMMAND_IS_CAR_IN_MISSION_GARAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let dropped_off = usize::try_from(params[0])
                    .ok()
                    .and_then(|index| scripts.garage_states.get(index))
                    .is_some_and(|state| *state == EGarageState::ClosedContainsCar as u8);
                self.update_compare_flag(dropped_off);
            }
            COMMAND_NO_SPECIAL_CAMERA_FOR_THIS_GARAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetGarageCameraFollowsPlayer { handle: params[0] });
            }
            COMMAND_SET_ROTATING_GARAGE_DOOR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetRotatingGarageDoor { handle: params[0] });
            }
            COMMAND_ACTIVATE_CRANE
            | COMMAND_ACTIVATE_MILITARY_CRANE
            | COMMAND_ACTIVATE_CRUSHER_CRANE => {
                let params = self.collect_parameters(scripts, &mut ip, 10)?;
                let x1 = Self::script_param_as_f32(params[2]);
                let y1 = Self::script_param_as_f32(params[3]);
                let x2 = Self::script_param_as_f32(params[4]);
                let y2 = Self::script_param_as_f32(params[5]);
                let is_military = command == COMMAND_ACTIVATE_MILITARY_CRANE;
                let is_crusher = command == COMMAND_ACTIVATE_CRUSHER_CRANE;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ActivateCrane {
                        pickup_min: CVector::new(x1.min(x2), y1.min(y2), 0.0),
                        pickup_max: CVector::new(x1.max(x2), y1.max(y2), 0.0),
                        dropoff: CVector::new(
                            Self::script_param_as_f32(params[6]),
                            Self::script_param_as_f32(params[7]),
                            Self::script_param_as_f32(params[8]),
                        ),
                        heading_radians: Self::script_param_as_f32(params[9]).to_radians(),
                        crane_position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            0.0,
                        ),
                        is_crusher,
                        is_military,
                    });
            }
            COMMAND_DEACTIVATE_CRANE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DeactivateCrane {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            0.0,
                        ),
                    });
            }
            COMMAND_CHANGE_GARAGE_TYPE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                if let Ok(index) = usize::try_from(params[0]) {
                    if let Some(state) = scripts.garage_states.get_mut(index) {
                        *state = EGarageState::FullyClosed as u8;
                    }
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ChangeGarageType {
                        handle: params[0],
                        garage_type: params[1],
                        target_model: 0,
                    });
            }
            COMMAND_CHANGE_GARAGE_TYPE_WITH_CAR_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                if let Ok(index) = usize::try_from(params[0]) {
                    if let Some(state) = scripts.garage_states.get_mut(index) {
                        *state = EGarageState::FullyClosed as u8;
                    }
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ChangeGarageType {
                        handle: params[0],
                        garage_type: params[1],
                        target_model: params[2],
                    });
            }
            COMMAND_OPEN_GARAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if let Ok(index) = usize::try_from(params[0]) {
                    if let Some(state) = scripts.garage_states.get_mut(index) {
                        if matches!(
                            *state,
                            value if value == EGarageState::FullyClosed as u8
                                || value == EGarageState::Closing as u8
                                || value == EGarageState::ClosedContainsCar as u8
                        ) {
                            *state = EGarageState::Opening as u8;
                        }
                    }
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::OpenGarage { handle: params[0] });
            }
            COMMAND_CLOSE_GARAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if let Ok(index) = usize::try_from(params[0]) {
                    if let Some(state) = scripts.garage_states.get_mut(index) {
                        if matches!(
                            *state,
                            value if value == EGarageState::Opened as u8
                                || value == EGarageState::Opening as u8
                        ) {
                            *state = EGarageState::Closing as u8;
                        }
                    }
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CloseGarage { handle: params[0] });
            }
            COMMAND_ACTIVATE_GARAGE | COMMAND_DEACTIVATE_GARAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetGarageActive {
                        handle: params[0],
                        active: command == COMMAND_ACTIVATE_GARAGE,
                    });
            }
            COMMAND_IS_GARAGE_OPEN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let is_open = usize::try_from(params[0])
                    .ok()
                    .and_then(|index| scripts.garage_states.get(index))
                    .is_some_and(|state| {
                        *state == EGarageState::Opened as u8
                            || *state == EGarageState::OpenedContainsCar as u8
                    });
                self.update_compare_flag(is_open);
            }
            COMMAND_IS_GARAGE_CLOSED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let is_closed = usize::try_from(params[0])
                    .ok()
                    .and_then(|index| scripts.garage_states.get(index))
                    .is_some_and(|state| *state == EGarageState::FullyClosed as u8);
                self.update_compare_flag(is_closed);
            }
            COMMAND_HAS_RESPRAY_HAPPENED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let happened = usize::try_from(params[0])
                    .ok()
                    .and_then(|index| scripts.garage_respray_happened.get_mut(index))
                    .is_some_and(|happened| std::mem::take(happened));
                self.update_compare_flag(happened);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearGarageResprayHappened { handle: params[0] });
            }
            COMMAND_HAS_IMPORT_GARAGE_SLOT_BEEN_FILLED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let filled = usize::try_from(params[0])
                    .ok()
                    .and_then(|garage| scripts.garage_collection_masks.get(garage))
                    .and_then(|mask| {
                        let bit = u32::try_from(params[1].saturating_sub(1)).ok()?;
                        (bit < i32::BITS).then_some(*mask & (1_i32 << bit) != 0)
                    })
                    .unwrap_or(false);
                self.update_compare_flag(filled);
            }
            COMMAND_DOES_GARAGE_CONTAIN_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let runtime = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?;
                runtime
                    .vehicle_snapshot(params[1])
                    .ok_or(ScriptVmError::MissingVehicle(params[1]))?;
                let live_result = runtime.garage_contains_vehicle(params[0], params[1]);
                self.update_compare_flag(live_result.unwrap_or_else(|| {
                    scripts.garage_vehicle_containment.iter().any(|entry| {
                        entry.garage_handle == params[0] && entry.vehicle_handle == params[1]
                    })
                }));
            }
            COMMAND_SET_FREE_BOMBS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetFreeBombs {
                        enabled: params[0] != 0,
                    });
            }
            COMMAND_SET_ZONE_PED_INFO => {
                let label = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 10)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetZonePedInfo {
                        label,
                        day: params[0] != 0,
                        ped_density: params[1],
                        gang_density: [
                            params[2], params[3], params[4], params[5], params[6], params[7],
                            params[8], 0, 0,
                        ],
                        cop_density: params[9],
                    });
            }
            COMMAND_SET_PED_DENSITY => {
                let label = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetZonePedDensity {
                        label,
                        day_density: params[0],
                        night_density: params[1],
                    });
            }
            COMMAND_SET_ZONE_CAR_INFO => {
                let label = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 16)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetZoneCarInfo {
                        label,
                        day: params[0] != 0,
                        car_density: params[1],
                        gang_counts: [
                            params[2], params[3], params[4], params[5], params[6], params[7],
                            params[8], 0, 0,
                        ],
                        cop_count: params[9],
                        car_counts: [
                            params[10], params[11], params[12], params[13], params[14], params[15],
                        ],
                    });
            }
            COMMAND_SET_ZONE_GROUP => {
                let label = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetZoneGroup {
                        label,
                        day: params[0] != 0,
                        ped_group: params[1],
                    });
            }
            COMMAND_SWITCH_ROADS_ON
            | COMMAND_SWITCH_ROADS_OFF
            | COMMAND_SWITCH_PED_ROADS_ON
            | COMMAND_SWITCH_PED_ROADS_OFF => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let p1 = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let p2 = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwitchRoadsInArea {
                        min: CVector::new(p1.x.min(p2.x), p1.y.min(p2.y), p1.z.min(p2.z)),
                        max: CVector::new(p1.x.max(p2.x), p1.y.max(p2.y), p1.z.max(p2.z)),
                        pedestrian: matches!(
                            command,
                            COMMAND_SWITCH_PED_ROADS_ON | COMMAND_SWITCH_PED_ROADS_OFF
                        ),
                        disable: matches!(
                            command,
                            COMMAND_SWITCH_ROADS_OFF | COMMAND_SWITCH_PED_ROADS_OFF
                        ),
                    });
            }
            COMMAND_SWITCH_PED_ROADS_ON_ANGLED
            | COMMAND_SWITCH_PED_ROADS_OFF_ANGLED
            | COMMAND_SWITCH_ROADS_ON_ANGLED
            | COMMAND_SWITCH_ROADS_OFF_ANGLED => {
                let params = self.collect_parameters(scripts, &mut ip, 7)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwitchRoadsInAngledArea {
                        p1: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        p2: CVector::new(
                            Self::script_param_as_f32(params[3]),
                            Self::script_param_as_f32(params[4]),
                            Self::script_param_as_f32(params[5]),
                        ),
                        length: Self::script_param_as_f32(params[6]),
                        pedestrian: matches!(
                            command,
                            COMMAND_SWITCH_PED_ROADS_ON_ANGLED
                                | COMMAND_SWITCH_PED_ROADS_OFF_ANGLED
                        ),
                        enable: matches!(
                            command,
                            COMMAND_SWITCH_PED_ROADS_ON_ANGLED | COMMAND_SWITCH_ROADS_ON_ANGLED
                        ),
                    });
            }
            COMMAND_LOAD_SPECIAL_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let name = scripts.read_key_from_script(&mut ip)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RequestSpecialModel {
                        model_id: params[0],
                        name,
                    });
            }
            COMMAND_CHAR_LOOK_AT_CHAR_ALWAYS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharLookAtChar {
                        source: params[0],
                        target: params[1],
                    });
            }
            COMMAND_CHAR_LOOK_AT_PLAYER_ALWAYS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharLookAtPlayer {
                        source: params[0],
                        player_index: params[1],
                    });
            }
            COMMAND_PLAYER_LOOK_AT_CHAR_ALWAYS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerLookAtChar {
                        player_index: params[0],
                        target: params[1],
                    });
            }
            COMMAND_STOP_CHAR_LOOKING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StopCharLooking(params[0]));
            }
            COMMAND_STOP_PLAYER_LOOKING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StopPlayerLooking(params[0]));
            }
            COMMAND_SET_ANIM_GROUP_FOR_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharAnimGroup {
                        handle: params[0],
                        anim_group: params[1],
                    });
            }
            COMMAND_SET_ANIM_GROUP_FOR_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerAnimGroup {
                        player_index: params[0],
                        anim_group: params[1],
                    });
            }
            COMMAND_MARK_MODEL_AS_NO_LONGER_NEEDED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::MarkModelAsNoLongerNeeded {
                        model_id: params[0],
                    });
            }
            COMMAND_UNLOAD_SPECIAL_CHARACTER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::UnloadSpecialCharacter {
                        char_id: params[0] - 1,
                    });
            }
            COMMAND_LOAD_SPECIAL_CHARACTER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let name = scripts.read_key_from_script(&mut ip)?;
                if let Ok(slot) = usize::try_from(params[0] - 1) {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::RequestSpecialCharacter { slot, name });
                }
            }
            COMMAND_HAS_SPECIAL_CHARACTER_LOADED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let loaded = usize::try_from(params[0] - 1)
                    .ok()
                    .and_then(|slot| scripts.special_char_loaded.get(slot).copied())
                    .unwrap_or(false);
                self.update_compare_flag(loaded);
            }
            COMMAND_LOAD_COLLISION => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::LoadCollision { level: params[0] });
            }
            COMMAND_MAKE_PLAYER_SAFE_FOR_CUTSCENE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::MakePlayerSafeForCutscene {
                        player_index: params[0],
                    });
            }
            COMMAND_SWITCH_WORLD_PROCESSING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetWorldProcessingEnabled(
                        params[0] != 0,
                    ));
            }
            COMMAND_REMOVE_ALL_PLAYER_WEAPONS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.pending_effects.push(
                    ScriptCommandEffect::RemoveAllPlayerWeapons { player_index: params[0] },
                );
            }
            COMMAND_LOAD_END_OF_GAME_TUNE => scripts.pending_effects.push(ScriptCommandEffect::LoadEndOfGameTune),
            COMMAND_PLAY_END_OF_GAME_TUNE => scripts.pending_effects.push(ScriptCommandEffect::PlayEndOfGameTune),
            COMMAND_STOP_END_OF_GAME_TUNE => scripts.pending_effects.push(ScriptCommandEffect::StopEndOfGameTune),
            COMMAND_SWITCH_RUBBISH => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetRubbishVisible(params[0] != 0));
            }
            COMMAND_SWITCH_STREAMING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetStreamingEnabled(params[0] != 0));
            }
            COMMAND_MARK_ROADS_BETWEEN_LEVELS | COMMAND_MARK_PED_ROADS_BETWEEN_LEVELS => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let p1 = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let p2 = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::MarkRoadsBetweenLevels {
                        min: CVector::new(p1.x.min(p2.x), p1.y.min(p2.y), p1.z.min(p2.z)),
                        max: CVector::new(p1.x.max(p2.x), p1.y.max(p2.y), p1.z.max(p2.z)),
                        pedestrian: command == COMMAND_MARK_PED_ROADS_BETWEEN_LEVELS,
                    });
            }
            COMMAND_REMOVE_BLIP => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveBlip { handle: params[0] });
            }
            COMMAND_CHANGE_BLIP_DISPLAY => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ChangeBlipDisplay {
                        handle: params[0],
                        display: params[1],
                    });
            }
            COMMAND_CHANGE_BLIP_COLOUR | COMMAND_DIM_BLIP | COMMAND_CHANGE_BLIP_SCALE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let effect = match command {
                    COMMAND_CHANGE_BLIP_COLOUR => ScriptCommandEffect::ChangeBlipColour {
                        handle: params[0],
                        colour: params[1],
                    },
                    COMMAND_DIM_BLIP => ScriptCommandEffect::ChangeBlipBrightness {
                        handle: params[0],
                        bright: params[1],
                    },
                    _ => ScriptCommandEffect::ChangeBlipScale {
                        handle: params[0],
                        scale: params[1],
                    },
                };
                scripts.pending_effects.push(effect);
            }
            COMMAND_ADD_SPRITE_BLIP_FOR_CONTACT_POINT | COMMAND_ADD_SPRITE_BLIP_FOR_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                let handle = scripts.next_script_blip_handle;
                scripts.next_script_blip_handle = scripts.next_script_blip_handle.saturating_add(1);
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateCoordinateBlip {
                        handle,
                        position,
                        sprite: params[3],
                        contact_point: command == COMMAND_ADD_SPRITE_BLIP_FOR_CONTACT_POINT,
                        colour: if command == COMMAND_ADD_SPRITE_BLIP_FOR_CONTACT_POINT {
                            2
                        } else {
                            5
                        },
                        display: 3,
                        scale: 1,
                    });
            }
            COMMAND_ADD_BLIP_FOR_CAR
            | COMMAND_ADD_BLIP_FOR_CHAR
            | COMMAND_ADD_BLIP_FOR_OBJECT
            | COMMAND_ADD_SPRITE_BLIP_FOR_CAR
            | COMMAND_ADD_SPRITE_BLIP_FOR_CHAR
            | COMMAND_ADD_SPRITE_BLIP_FOR_OBJECT
            | COMMAND_ADD_BLIP_FOR_CAR_OLD
            | COMMAND_ADD_BLIP_FOR_CHAR_OLD
            | COMMAND_ADD_BLIP_FOR_OBJECT_OLD => {
                let old = matches!(
                    command,
                    COMMAND_ADD_BLIP_FOR_CAR_OLD
                        | COMMAND_ADD_BLIP_FOR_CHAR_OLD
                        | COMMAND_ADD_BLIP_FOR_OBJECT_OLD
                );
                let sprite = matches!(
                    command,
                    COMMAND_ADD_SPRITE_BLIP_FOR_CAR
                        | COMMAND_ADD_SPRITE_BLIP_FOR_CHAR
                        | COMMAND_ADD_SPRITE_BLIP_FOR_OBJECT
                );
                let count = if old {
                    3
                } else if sprite {
                    2
                } else {
                    1
                };
                let params = self.collect_parameters(scripts, &mut ip, count)?;
                let entity_handle = params[0];
                let entity_type = match command {
                    COMMAND_ADD_BLIP_FOR_CAR
                    | COMMAND_ADD_SPRITE_BLIP_FOR_CAR
                    | COMMAND_ADD_BLIP_FOR_CAR_OLD => 1,
                    COMMAND_ADD_BLIP_FOR_CHAR
                    | COMMAND_ADD_SPRITE_BLIP_FOR_CHAR
                    | COMMAND_ADD_BLIP_FOR_CHAR_OLD => 2,
                    _ => 3,
                };
                let colour = if old {
                    params[1]
                } else {
                    match entity_type {
                        1 => 0,
                        2 => 1,
                        _ => 6,
                    }
                };
                let display = if old { params[2] } else { 3 };
                let handle = scripts.next_script_blip_handle;
                scripts.next_script_blip_handle = scripts.next_script_blip_handle.saturating_add(1);
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateEntityBlip {
                        handle,
                        entity_handle,
                        entity_type,
                        colour,
                        display,
                        sprite: if sprite { params[1] } else { 0 },
                        scale: if old || sprite { 1 } else { 3 },
                    });
            }
            COMMAND_ADD_BLIP_FOR_PICKUP => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let handle = scripts.next_script_blip_handle;
                scripts.next_script_blip_handle = scripts.next_script_blip_handle.saturating_add(1);
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreatePickupBlip {
                        handle,
                        pickup_handle: params[0],
                    });
            }
            COMMAND_ADD_SPRITE_BLIP_FOR_PICKUP => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let handle = scripts.next_script_blip_handle;
                scripts.next_script_blip_handle = scripts.next_script_blip_handle.saturating_add(1);
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateSpritePickupBlip {
                        handle,
                        pickup_handle: params[0],
                        sprite: params[1],
                    });
            }
            COMMAND_ADD_BLIP_FOR_COORD
            | COMMAND_ADD_BLIP_FOR_CONTACT_POINT
            | COMMAND_ADD_BLIP_FOR_COORD_OLD => {
                let old = command == COMMAND_ADD_BLIP_FOR_COORD_OLD;
                let count = if old { 5 } else { 3 };
                let params = self.collect_parameters(scripts, &mut ip, count)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                let contact_point = command == COMMAND_ADD_BLIP_FOR_CONTACT_POINT;
                let handle = scripts.next_script_blip_handle;
                scripts.next_script_blip_handle = scripts.next_script_blip_handle.saturating_add(1);
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateCoordinateBlip {
                        handle,
                        position,
                        sprite: 0,
                        contact_point,
                        colour: if old {
                            params[3]
                        } else if contact_point {
                            2
                        } else {
                            5
                        },
                        display: if old { params[4] } else { 3 },
                        scale: if old { 1 } else { 3 },
                    });
            }
            COMMAND_CREATE_COLLECTABLE1 => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    ) + 0.5;
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateCollectable { position });
            }
            COMMAND_GET_COLLECTABLE1S_COLLECTED => {
                scripts.script_params[0] = scripts.collected_packages;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_SET_COLLECTABLE1_TOTAL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCollectableTotal(params[0]));
            }
            COMMAND_ADD_SCORE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts.pending_effects.push(ScriptCommandEffect::AddScore {
                    player_index: params[0],
                    amount: params[1],
                });
            }
            COMMAND_IS_SCORE_GREATER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.money > params[1]);
            }
            COMMAND_STORE_SCORE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                scripts.script_params[0] = snapshot.money;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_GAME_TIMER => {
                scripts.script_params[0] = time_ms as i32;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_STORE_WANTED_LEVEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                scripts.script_params[0] = snapshot.wanted_level;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_PLAYER_STOPPED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.stopped);
            }
            COMMAND_IS_CHAR_STOPPED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.stopped);
            }
            COMMAND_MESSAGE_WAIT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                self.wake_time = time_ms.saturating_add(params[0] as u32);
                if params[1] != 0 {
                    self.skip_wake_time = true;
                }
                outcome.yielded = true;
            }
            COMMAND_GET_OBJECT_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let pos = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.object_snapshot(params[0]))
                    .map(|snapshot| snapshot.position)
                    .unwrap_or(CVector::ZERO);
                scripts.script_params[0] = Self::f32_to_script_param(pos.x);
                scripts.script_params[1] = Self::f32_to_script_param(pos.y);
                scripts.script_params[2] = Self::f32_to_script_param(pos.z);
                self.store_parameters(scripts, &mut ip, 3)?;
            }
            COMMAND_SET_OBJECT_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetObjectCoordinates {
                        handle: params[0],
                        position,
                    });
            }
            COMMAND_PLACE_OBJECT_RELATIVE_TO_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PlaceObjectRelativeToCar {
                        object_handle: params[0],
                        vehicle_handle: params[1],
                        offset: CVector::new(
                            Self::script_param_as_f32(params[2]),
                            Self::script_param_as_f32(params[3]),
                            Self::script_param_as_f32(params[4]),
                        ),
                        time_step,
                    });
            }
            COMMAND_SET_OBJECT_VELOCITY | COMMAND_ADD_TO_OBJECT_VELOCITY => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetObjectVelocity {
                        handle: params[0],
                        velocity: CVector::new(
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                            Self::script_param_as_f32(params[3]),
                        ),
                        add: command == COMMAND_ADD_TO_OBJECT_VELOCITY,
                    });
            }
            COMMAND_SET_OBJECT_COLLISION => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetObjectCollision {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_OBJECT_DYNAMIC => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetObjectDynamic {
                        handle: params[0],
                        dynamic: params[1] != 0,
                    });
            }
            COMMAND_SET_OBJECT_ROTATION => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetObjectRotation {
                        handle: params[0],
                        rotation: CVector::new(
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                            Self::script_param_as_f32(params[3]),
                        ),
                    });
            }
            COMMAND_ROTATE_OBJECT => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let request = ScriptObjectMotionRequest::Rotate {
                    handle: params[0],
                    heading_target_degrees: Self::script_param_as_f32(params[1]),
                    rotate_by_degrees: Self::script_param_as_f32(params[2])
                        * (time_step / re3bevy_core::core::Timer::CTimer::DEFAULT_TIME_STEP),
                    check_collision: params[3] != 0,
                };
                let reached_or_blocked =
                    run_script_object_motion(&mut entity_runtime, request, general.as_deref_mut())?;
                self.update_compare_flag(reached_or_blocked);
            }
            COMMAND_SLIDE_OBJECT => {
                let params = self.collect_parameters(scripts, &mut ip, 8)?;
                let time_step_fix =
                    time_step / re3bevy_core::core::Timer::CTimer::DEFAULT_TIME_STEP;
                let request = ScriptObjectMotionRequest::Slide {
                    handle: params[0],
                    target: CVector::new(
                        Self::script_param_as_f32(params[1]),
                        Self::script_param_as_f32(params[2]),
                        Self::script_param_as_f32(params[3]),
                    ),
                    slide_by: CVector::new(
                        Self::script_param_as_f32(params[4]) * time_step_fix,
                        Self::script_param_as_f32(params[5]) * time_step_fix,
                        Self::script_param_as_f32(params[6]) * time_step_fix,
                    ),
                    check_collision: params[7] != 0,
                };
                let reached_or_blocked =
                    run_script_object_motion(&mut entity_runtime, request, general.as_deref_mut())?;
                self.update_compare_flag(reached_or_blocked);
            }
            COMMAND_SET_OBJECT_DRAW_LAST => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetObjectDrawLast {
                        handle: params[0],
                        draw_last: params[1] != 0,
                    });
            }
            COMMAND_DOES_OBJECT_EXIST => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let exists = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.object_snapshot(params[0]))
                    .is_some();
                self.update_compare_flag(exists);
            }
            COMMAND_HAS_OBJECT_BEEN_DAMAGED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let damaged = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.object_snapshot(params[0]))
                    .is_some_and(|snapshot| !snapshot.alive);
                self.update_compare_flag(damaged);
            }
            COMMAND_HAS_PICKUP_BEEN_COLLECTED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let collected = scripts
                    .collected_pickup_handles
                    .iter()
                    .position(|handle| *handle == params[0]);
                if let Some(index) = collected {
                    scripts.collected_pickup_handles.swap_remove(index);
                }
                self.update_compare_flag(collected.is_some());
            }
            COMMAND_REMOVE_PICKUP => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemovePickup { handle: params[0] });
            }
            COMMAND_LINE => {
                let _ = self.collect_parameters(scripts, &mut ip, 6)?;
            }
            COMMAND_CREATE_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                let player_index = params[0].max(0) as usize;
                // Synthetic ped-pool handle for GET_PLAYER_CHAR until CPools is wired.
                if let Some(slot) = scripts.player_char_handles.get_mut(player_index) {
                    *slot = (player_index as i32) + 1;
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreatePlayer {
                        index: params[0],
                        position,
                        needs_ground: position.z <= MAP_Z_LOW_LIMIT,
                    });
                scripts.script_params[0] = params[0];
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_SET_PLAYER_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerCoordinates {
                        index: params[0],
                        position,
                        needs_ground: position.z <= MAP_Z_LOW_LIMIT,
                    });
            }
            COMMAND_GET_PLAYER_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                scripts.script_params[0] = Self::f32_to_script_param(snapshot.position.x);
                scripts.script_params[1] = Self::f32_to_script_param(snapshot.position.y);
                scripts.script_params[2] = Self::f32_to_script_param(snapshot.position.z);
                self.store_parameters(scripts, &mut ip, 3)?;
            }
            COMMAND_IS_PLAYER_IN_AREA_2D => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                let x1 = Self::script_param_as_f32(params[1]);
                let y1 = Self::script_param_as_f32(params[2]);
                let x2 = Self::script_param_as_f32(params[3]);
                let y2 = Self::script_param_as_f32(params[4]);
                self.update_compare_flag(point_in_area_2d(snapshot.position, x1, y1, x2, y2));
                if params[5] != 0 {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::HighlightArea {
                            script_ip: ip,
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
            COMMAND_IS_PLAYER_IN_AREA_3D => {
                let params = self.collect_parameters(scripts, &mut ip, 8)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                let x1 = Self::script_param_as_f32(params[1]);
                let y1 = Self::script_param_as_f32(params[2]);
                let z1 = Self::script_param_as_f32(params[3]);
                let x2 = Self::script_param_as_f32(params[4]);
                let y2 = Self::script_param_as_f32(params[5]);
                let z2 = Self::script_param_as_f32(params[6]);
                self.update_compare_flag(point_in_area_3d(
                    snapshot.position,
                    x1,
                    y1,
                    z1,
                    x2,
                    y2,
                    z2,
                ));
                if params[7] != 0 {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::HighlightArea {
                            script_ip: ip,
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
            COMMAND_CREATE_CHAR => {
                // re3 Script.cpp COMMAND_CREATE_CHAR: remap special cop/emergency
                // model IDs, construct the ped, ground-snap + 1.0f, CWorld::Add,
                // store CPools ped handle, then mission-cleanup register.
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let ped_type = params[0];
                let model_id = remap_script_ped_model(ped_type, params[1]);
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                position.z += 1.0;
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                let handle = run_script_entity_creation(
                    &mut entity_runtime,
                    ScriptEntityMutation::CreateChar(ScriptCreateCharRequest {
                        ped_type,
                        model_id,
                        position,
                        vehicle_handle: None,
                        passenger_seat: None,
                    }),
                    Some(general),
                )?
                .ok_or(ScriptVmError::MissingScriptEntityAllocator)?;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .add_entity_to_list(handle, CLEANUP_CHAR);
                }
            }
            COMMAND_UNDRESS_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let model_name = scripts.read_key_from_script(&mut ip)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::UndressChar {
                        handle: params[0],
                        model_name,
                    });
            }
            COMMAND_DRESS_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DressChar { handle: params[0] });
            }
            COMMAND_DELETE_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DeleteChar { handle: params[0] });
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .remove_entity_from_list(params[0], CLEANUP_CHAR);
                }
            }
            COMMAND_REMOVE_CHAR_ELEGANTLY => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveCharElegantly { handle: params[0] });
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .remove_entity_from_list(params[0], CLEANUP_CHAR);
                }
            }
            COMMAND_CHAR_WANDER_DIR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let mut path = params[1];
                if !(0..=7).contains(&path) {
                    let general = general
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingRandomGenerator)?;
                    path = general.get_random_number_in_range_i32(0, 7);
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CharWanderDir {
                        handle: params[0],
                        path,
                    });
            }
            COMMAND_CHAR_FOLLOW_PATH => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CharFollowPath {
                        handle: params[0],
                        position,
                        needs_ground: position.z <= MAP_Z_LOW_LIMIT,
                    });
            }
            COMMAND_CHAR_SET_IDLE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CharSetIdle { handle: params[0] });
            }
            COMMAND_GET_CHAR_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let runtime = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?;
                let char_snapshot = runtime
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                // Script.cpp returns m_pMyVehicle->GetPosition() when the
                // character is seated, not the ped's seat/root transform.
                let snapshot = char_snapshot
                    .current_vehicle
                    .and_then(|handle| runtime.vehicle_snapshot(handle))
                    .unwrap_or(char_snapshot);
                scripts.script_params[0] = Self::f32_to_script_param(snapshot.position.x);
                scripts.script_params[1] = Self::f32_to_script_param(snapshot.position.y);
                scripts.script_params[2] = Self::f32_to_script_param(snapshot.position.z);
                self.store_parameters(scripts, &mut ip, 3)?;
            }
            COMMAND_SET_CHAR_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharCoordinates {
                        handle: params[0],
                        position,
                        needs_ground: position.z <= MAP_Z_LOW_LIMIT,
                    });
            }
            COMMAND_IS_CHAR_STILL_ALIVE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.alive);
            }
            COMMAND_IS_CHAR_DEAD => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0]);
                self.update_compare_flag(snapshot.is_none_or(|snapshot| !snapshot.alive));
            }
            COMMAND_IS_CHAR_IN_AREA_2D => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.process_entity_area_2d_effects(scripts, ip, snapshot.position, &params[1..=5]);
            }
            COMMAND_IS_CHAR_IN_AREA_3D => {
                let params = self.collect_parameters(scripts, &mut ip, 8)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.process_entity_area_3d_effects(scripts, ip, snapshot.position, &params[1..=7]);
            }
            COMMAND_CREATE_RANDOM_CHAR => {
                // re3 Script4.cpp COMMAND_CREATE_RANDOM_CHAR chooses a loaded civilian;
                // when none is available it falls back to MI_MALE01.
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                position.z += 1.0;
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                let handle = run_script_entity_creation(
                    &mut entity_runtime,
                    ScriptEntityMutation::CreateChar(ScriptCreateCharRequest {
                        ped_type: 4,
                        model_id: crate::modelinfo::ModelIndices::MI_MALE01 as i32,
                        position,
                        vehicle_handle: None,
                        passenger_seat: None,
                    }),
                    Some(general),
                )?
                .ok_or(ScriptVmError::MissingScriptEntityAllocator)?;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .add_entity_to_list(handle, CLEANUP_CHAR);
                }
            }
            COMMAND_CREATE_CHAR_INSIDE_CAR => {
                // re3 Script.cpp COMMAND_CREATE_CHAR_INSIDE_CAR: create mission
                // ped at vehicle position, make it driver, status physics/cruise.
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let vehicle_snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let ped_type = params[1];
                let model_id = remap_script_ped_model(ped_type, params[2]);
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                let handle = run_script_entity_creation(
                    &mut entity_runtime,
                    ScriptEntityMutation::CreateChar(ScriptCreateCharRequest {
                        ped_type,
                        model_id,
                        position: vehicle_snapshot.position,
                        vehicle_handle: Some(params[0]),
                        passenger_seat: None,
                    }),
                    Some(general),
                )?
                .ok_or(ScriptVmError::MissingScriptEntityAllocator)?;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .add_entity_to_list(handle, CLEANUP_CHAR);
                }
            }
            COMMAND_WARP_PLAYER_FROM_CAR_TO_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::WarpPlayerFromCarToCoord {
                        player_index: params[0],
                        position,
                        needs_ground: false,
                    });
            }
            COMMAND_CREATE_CAR => {
                // re3 Script.cpp COMMAND_CREATE_CAR: construct mission vehicle,
                // ground-snap + model base offset (allocator owns exact vehicle class),
                // store CPools vehicle handle, then mission-cleanup register.
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                bevy::log::debug!(
                    "VM CREATE_CAR thread_ip={} mission={} model={}",
                    self.ip,
                    self.is_mission_script,
                    params[0]
                );
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                let handle = run_script_entity_creation(
                    &mut entity_runtime,
                    ScriptEntityMutation::CreateCar(ScriptCreateCarRequest {
                        model_id: params[0],
                        position,
                        time_ms,
                        clear_space_for_mission_entity: true,
                    }),
                    Some(general),
                )?
                .ok_or(ScriptVmError::MissingScriptEntityAllocator)?;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .add_entity_to_list(handle, CLEANUP_CAR);
                }
            }
            COMMAND_GET_CAR_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[0] = Self::f32_to_script_param(snapshot.position.x);
                scripts.script_params[1] = Self::f32_to_script_param(snapshot.position.y);
                scripts.script_params[2] = Self::f32_to_script_param(snapshot.position.z);
                self.store_parameters(scripts, &mut ip, 3)?;
            }
            COMMAND_SET_CAR_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarCoordinates {
                        handle: params[0],
                        position,
                        needs_ground: position.z <= MAP_Z_LOW_LIMIT,
                    });
            }
            COMMAND_IS_CAR_STILL_ALIVE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let alive = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.vehicle_snapshot(params[0]))
                    .is_some_and(|snapshot| snapshot.alive);
                self.update_compare_flag(alive);
            }
            COMMAND_IS_CAR_DEAD => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let dead = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.vehicle_snapshot(params[0]))
                    .is_none_or(|snapshot| !snapshot.alive);
                self.update_compare_flag(dead);
            }
            COMMAND_IS_CAR_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let matches_model = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.vehicle_snapshot(params[0]))
                    .is_some_and(|snapshot| snapshot.current_vehicle_model == Some(params[1]));
                self.update_compare_flag(matches_model);
            }
            COMMAND_IS_CHAR_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let matches_model = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.char_snapshot(params[0]))
                    .is_some_and(|snapshot| snapshot.model_index == params[1]);
                self.update_compare_flag(matches_model);
            }
            COMMAND_IS_CAR_DEAD_IN_AREA_2D | COMMAND_IS_CAR_DEAD_IN_AREA_3D => {
                let is_3d = command == COMMAND_IS_CAR_DEAD_IN_AREA_3D;
                let params =
                    self.collect_parameters(scripts, &mut ip, if is_3d { 8 } else { 6 })?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let x1 = Self::script_param_as_f32(params[1]);
                let y1 = Self::script_param_as_f32(params[2]);
                let (z1, x2, y2, z2, highlight) = if is_3d {
                    (
                        Self::script_param_as_f32(params[3]),
                        Self::script_param_as_f32(params[4]),
                        Self::script_param_as_f32(params[5]),
                        Self::script_param_as_f32(params[6]),
                        params[7] != 0,
                    )
                } else {
                    (
                        MAP_Z_LOW_LIMIT,
                        Self::script_param_as_f32(params[3]),
                        Self::script_param_as_f32(params[4]),
                        MAP_Z_LOW_LIMIT,
                        params[5] != 0,
                    )
                };
                let in_area = if is_3d {
                    point_in_area_3d(snapshot.position, x1, y1, z1, x2, y2, z2)
                } else {
                    point_in_area_2d(snapshot.position, x1, y1, x2, y2)
                };
                self.update_compare_flag(
                    snapshot.entity_status == eEntityStatus::STATUS_WRECKED as i32 && in_area,
                );
                if highlight {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::HighlightArea {
                            script_ip: ip,
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
            COMMAND_IS_CAR_STUCK_ON_ROOF => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(
                    scripts
                        .upside_down_cars
                        .has_car_been_upside_down_for_a_while(params[0]),
                );
            }
            COMMAND_ADD_UPSIDEDOWN_CAR_CHECK | COMMAND_REMOVE_UPSIDEDOWN_CAR_CHECK => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                if command == COMMAND_ADD_UPSIDEDOWN_CAR_CHECK {
                    scripts.upside_down_cars.add_car_to_check(params[0]);
                } else {
                    scripts.upside_down_cars.remove_car_from_check(params[0]);
                }
            }
            COMMAND_IS_CAR_STOPPED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(snapshot.stopped);
            }
            COMMAND_SET_CAR_CRUISE_SPEED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarCruiseSpeed {
                        handle: params[0],
                        speed: Self::script_param_as_f32(params[1]),
                    });
            }
            COMMAND_BOAT_GOTO_COORDS => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::BoatGotoCoordinates {
                        handle: params[0],
                        needs_water_level: position.z <= MAP_Z_LOW_LIMIT,
                        position,
                        anti_reverse_timer_ms: time_ms,
                    });
            }
            COMMAND_BOAT_STOP => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::BoatStop { handle: params[0] });
            }
            COMMAND_SET_BOAT_CRUISE_SPEED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetBoatCruiseSpeed {
                        handle: params[0],
                        speed: Self::script_param_as_f32(params[1]),
                    });
            }
            COMMAND_SET_CAR_DRIVING_STYLE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarDrivingStyle {
                        handle: params[0],
                        style: params[1],
                    });
            }
            COMMAND_SET_CAR_MISSION => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarMission {
                        handle: params[0],
                        mission: params[1],
                        anti_reverse_timer_ms: time_ms,
                    });
            }
            COMMAND_SET_CAR_RAM_CAR
            | COMMAND_SET_CAR_BLOCK_CAR
            | COMMAND_IS_COLL_IN_CARS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let runtime = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?;
                if command == COMMAND_IS_COLL_IN_CARS {
                    runtime
                        .vehicle_snapshot(params[0])
                        .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                    runtime
                        .vehicle_snapshot(params[1])
                        .ok_or(ScriptVmError::MissingVehicle(params[1]))?;
                }
                let ram = command == COMMAND_SET_CAR_RAM_CAR;
                if runtime
                    .set_car_target_now(params[0], params[1], ram)
                    .is_none()
                {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetCarTarget {
                            handle: params[0],
                            target: params[1],
                            ram,
                        });
                }
            }
            COMMAND_GET_CAR_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[0] = vehicle
                    .current_vehicle_model
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_CAR_SPEED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                // re3 `CVehicle::GetSpeed().Magnitude() * GAME_SPEED_TO_METERS_PER_SECOND`.
                scripts.script_params[0] = Self::f32_to_script_param(vehicle.speed * 50.0);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_CAR_COLOURS => {
                // re3 Script6.cpp: resolve the CVehicle pool handle, copy its
                // two live palette indices into ScriptParams, then store both
                // output variables encoded directly after the input handle.
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[0] = vehicle
                    .vehicle_colour1
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[1] = vehicle
                    .vehicle_colour2
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.store_parameters(scripts, &mut ip, 2)?;
            }
            COMMAND_GET_CAR_FORWARD_X | COMMAND_GET_CAR_FORWARD_Y => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                // CEntity heading is derived from the matrix right basis; GTA's
                // forward basis is (-sin(heading), cos(heading)). It is unit length
                // after the same Magnitude2D normalization used by re3.
                let heading = vehicle.heading_degrees.to_radians();
                let forward = if command == COMMAND_GET_CAR_FORWARD_X {
                    -heading.sin()
                } else {
                    heading.cos()
                };
                scripts.script_params[0] = Self::f32_to_script_param(forward);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_BOAT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(vehicle.vehicle_is_boat);
            }
            COMMAND_IS_CAR_IN_WATER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                // re3 does not assert this pool lookup; a null vehicle returns false.
                let in_water = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .is_some_and(|vehicle| vehicle.vehicle_in_water);
                self.update_compare_flag(in_water);
            }
            COMMAND_IS_FIRST_CAR_COLOUR | COMMAND_IS_SECOND_CAR_COLOUR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let colour = if command == COMMAND_IS_FIRST_CAR_COLOUR {
                    vehicle.vehicle_colour1
                } else {
                    vehicle.vehicle_colour2
                };
                self.update_compare_flag(colour == Some(params[1]));
            }
            COMMAND_IS_PLAYER_IN_ZONE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let label = scripts.read_key_from_script(&mut ip)?;
                let player =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(point_in_script_zone(
                    player.position,
                    &label,
                    &scripts.zone_bounds,
                ));
            }
            COMMAND_IS_CHAR_IN_ZONE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let label = scripts.read_key_from_script(&mut ip)?;
                let character = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(point_in_script_zone(
                    character.position,
                    &label,
                    &scripts.zone_bounds,
                ));
            }
            COMMAND_IS_TAXI => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(matches!(
                    vehicle.current_vehicle_model,
                    Some(model)
                        if model == MI_TAXI as i32
                            || model == MI_CABBIE as i32
                            || model == MI_BORGNINE as i32
                ));
            }
            COMMAND_IS_PLAYER_IN_TAXI => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let player =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(matches!(
                    player.current_vehicle_model,
                    Some(model)
                        if model == MI_TAXI as i32
                            || model == MI_CABBIE as i32
                            || model == MI_BORGNINE as i32
                ));
            }
            COMMAND_IS_CAR_IN_AREA_2D => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.process_entity_area_2d_effects(scripts, ip, snapshot.position, &params[1..=5]);
            }
            COMMAND_IS_CAR_IN_AREA_3D => {
                let params = self.collect_parameters(scripts, &mut ip, 8)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.process_entity_area_3d_effects(scripts, ip, snapshot.position, &params[1..=7]);
            }
            COMMAND_DELETE_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DeleteCar { handle: params[0] });
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .remove_entity_from_list(params[0], CLEANUP_CAR);
                }
            }
            COMMAND_CAR_GOTO_COORDINATES => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CarGotoCoordinates {
                        handle: params[0],
                        position,
                        needs_ground: position.z <= MAP_Z_LOW_LIMIT,
                        accurate: false,
                        anti_reverse_timer_ms: time_ms,
                    });
            }
            COMMAND_GET_CLOSEST_CHAR_NODE
            | COMMAND_GET_CLOSEST_CAR_NODE
            | COMMAND_GET_CLOSEST_CAR_NODE_WITH_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                let path_type = if command == COMMAND_GET_CLOSEST_CHAR_NODE {
                    crate::control::PathFind::PATH_PED
                } else {
                    crate::control::PathFind::PATH_CAR
                };
                let with_heading = command == COMMAND_GET_CLOSEST_CAR_NODE_WITH_HEADING;
                let node = path_node_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingPathNodeProbe)?(
                    position,
                    path_type,
                    with_heading,
                    with_heading,
                )
                .ok_or(ScriptVmError::PathNodeNotFound)?;
                scripts.script_params[0] = Self::f32_to_script_param(node.position.x);
                scripts.script_params[1] = Self::f32_to_script_param(node.position.y);
                scripts.script_params[2] = Self::f32_to_script_param(node.position.z);
                if with_heading {
                    scripts.script_params[3] = Self::f32_to_script_param(node.heading_degrees);
                    self.store_parameters(scripts, &mut ip, 4)?;
                } else {
                    self.store_parameters(scripts, &mut ip, 3)?;
                }
            }
            COMMAND_CAR_GOTO_COORDINATES_ACCURATE => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CarGotoCoordinates {
                        handle: params[0],
                        position,
                        needs_ground: position.z <= MAP_Z_LOW_LIMIT,
                        accurate: true,
                        anti_reverse_timer_ms: time_ms,
                    });
            }
            COMMAND_APPLY_BRAKES_TO_PLAYERS_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ApplyBrakesToPlayersCar {
                        player_index: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_CAR_WANDER_RANDOMLY => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CarWanderRandomly { handle: params[0] });
            }
            COMMAND_CAR_SET_IDLE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CarSetIdle { handle: params[0] });
            }
            COMMAND_SET_CHAR_OBJ_NO_OBJ => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.push_char_objective(scripts, params[0], 0, None, None, None, 0.0, None);
            }
            COMMAND_SET_CHAR_OBJ_WAIT_ON_FOOT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.push_char_objective(scripts, params[0], 1, None, None, None, 0.0, None);
            }
            COMMAND_SET_CHAR_OBJ_FLEE_ON_FOOT_TILL_SAFE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.push_char_objective(scripts, params[0], 2, None, None, None, 0.0, None);
            }
            COMMAND_SET_CHAR_OBJ_GUARD_SPOT => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut pos = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if pos.z <= MAP_Z_LOW_LIMIT {
                    pos.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        pos.x, pos.y
                    );
                }
                self.push_char_objective(scripts, params[0], 3, None, None, Some(pos), 0.0, None);
            }
            COMMAND_SET_CHAR_OBJ_GUARD_AREA
            | COMMAND_SET_CHAR_OBJ_GOTO_AREA_ON_FOOT
            | COMMAND_SET_CHAR_OBJ_RUN_TO_AREA
            | COMMAND_SET_CHAR_OBJ_GOTO_AREA_ANY_MEANS => {
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let x1 = Self::script_param_as_f32(params[1]);
                let y1 = Self::script_param_as_f32(params[2]);
                let x2 = Self::script_param_as_f32(params[3]);
                let y2 = Self::script_param_as_f32(params[4]);
                let inf_x = x1.min(x2);
                let sup_x = x1.max(x2);
                let inf_y = y1.min(y2);
                let sup_y = y1.max(y2);
                let mut pos = CVector::new((inf_x + sup_x) * 0.5, (inf_y + sup_y) * 0.5, 0.0);
                pos.z = ground_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingGroundProbe)?(pos.x, pos.y);
                let radius = (pos.x - inf_x).max(pos.y - inf_y);
                let objective = match command {
                    COMMAND_SET_CHAR_OBJ_RUN_TO_AREA => 22,
                    COMMAND_SET_CHAR_OBJ_GOTO_AREA_ANY_MEANS => 20,
                    COMMAND_SET_CHAR_OBJ_GOTO_AREA_ON_FOOT => 21,
                    _ => 3,
                };
                self.push_char_objective(
                    scripts,
                    params[0],
                    objective,
                    None,
                    None,
                    Some(pos),
                    radius,
                    None,
                );
            }
            COMMAND_SET_CHAR_OBJ_WAIT_IN_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.push_char_objective(scripts, params[0], 5, None, None, None, 0.0, None);
            }
            COMMAND_SET_CHAR_OBJ_KILL_CHAR_ON_FOOT
            | COMMAND_SET_CHAR_OBJ_KILL_CHAR_ANY_MEANS
            | COMMAND_SET_CHAR_OBJ_FLEE_CHAR_ON_FOOT_TILL_SAFE
            | COMMAND_SET_CHAR_OBJ_FLEE_CHAR_ON_FOOT_ALWAYS
            | COMMAND_SET_CHAR_OBJ_GOTO_CHAR_ON_FOOT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let objective = match command {
                    COMMAND_SET_CHAR_OBJ_KILL_CHAR_ON_FOOT => 7,
                    COMMAND_SET_CHAR_OBJ_KILL_CHAR_ANY_MEANS => 8,
                    COMMAND_SET_CHAR_OBJ_FLEE_CHAR_ON_FOOT_TILL_SAFE => 9,
                    COMMAND_SET_CHAR_OBJ_FLEE_CHAR_ON_FOOT_ALWAYS => 10,
                    _ => 11,
                };
                self.push_char_objective(
                    scripts,
                    params[0],
                    objective,
                    Some(params[1]),
                    None,
                    None,
                    0.0,
                    None,
                );
            }
            COMMAND_SET_CHAR_OBJ_KILL_PLAYER_ON_FOOT
            | COMMAND_SET_CHAR_OBJ_KILL_PLAYER_ANY_MEANS
            | COMMAND_SET_CHAR_OBJ_FLEE_PLAYER_ON_FOOT_TILL_SAFE
            | COMMAND_SET_CHAR_OBJ_FLEE_PLAYER_ON_FOOT_ALWAYS
            | COMMAND_SET_CHAR_OBJ_GOTO_PLAYER_ON_FOOT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let player_handle = scripts
                    .player_char_handles
                    .get(params[1].max(0) as usize)
                    .copied()
                    .unwrap_or(params[1].saturating_add(1));
                let objective = match command {
                    COMMAND_SET_CHAR_OBJ_KILL_PLAYER_ON_FOOT => 7,
                    COMMAND_SET_CHAR_OBJ_KILL_PLAYER_ANY_MEANS => 8,
                    COMMAND_SET_CHAR_OBJ_FLEE_PLAYER_ON_FOOT_TILL_SAFE => 9,
                    COMMAND_SET_CHAR_OBJ_FLEE_PLAYER_ON_FOOT_ALWAYS => 10,
                    _ => 11,
                };
                self.push_char_objective(
                    scripts,
                    params[0],
                    objective,
                    Some(player_handle),
                    None,
                    None,
                    0.0,
                    None,
                );
            }
            COMMAND_SET_CHAR_OBJ_LEAVE_CAR
            | COMMAND_SET_CHAR_OBJ_ENTER_CAR_AS_PASSENGER
            | COMMAND_SET_CHAR_OBJ_ENTER_CAR_AS_DRIVER
            | COMMAND_SET_CHAR_OBJ_DESTROY_CAR
            | COMMAND_SET_CHAR_OBJ_FLEE_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let objective = match command {
                    COMMAND_SET_CHAR_OBJ_LEAVE_CAR => 13,
                    COMMAND_SET_CHAR_OBJ_ENTER_CAR_AS_PASSENGER => 14,
                    COMMAND_SET_CHAR_OBJ_ENTER_CAR_AS_DRIVER => 15,
                    COMMAND_SET_CHAR_OBJ_DESTROY_CAR => 19,
                    _ => 34,
                };
                self.push_char_objective(
                    scripts,
                    params[0],
                    objective,
                    None,
                    Some(params[1]),
                    None,
                    0.0,
                    None,
                );
            }
            COMMAND_SET_CHAR_OBJ_GOTO_COORD_ON_FOOT | COMMAND_SET_CHAR_OBJ_RUN_TO_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut pos = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    0.0,
                );
                pos.z = ground_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingGroundProbe)?(pos.x, pos.y);
                let objective = if command == COMMAND_SET_CHAR_OBJ_RUN_TO_COORD {
                    22
                } else {
                    21
                };
                self.push_char_objective(
                    scripts,
                    params[0],
                    objective,
                    None,
                    None,
                    Some(pos),
                    0.0,
                    None,
                );
            }
            COMMAND_SET_CHAR_AS_LEADER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                self.push_char_objective(
                    scripts,
                    params[0],
                    26,
                    Some(params[1]),
                    None,
                    None,
                    0.0,
                    None,
                );
            }
            COMMAND_SET_PLAYER_AS_LEADER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let player_handle = scripts
                    .player_char_handles
                    .get(params[1].max(0) as usize)
                    .copied()
                    .unwrap_or(params[1].saturating_add(1));
                self.push_char_objective(
                    scripts,
                    params[0],
                    26,
                    Some(player_handle),
                    None,
                    None,
                    0.0,
                    None,
                );
            }
            COMMAND_LEAVE_GROUP => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharObjective {
                        handle: params[0],
                        objective: 0,
                        target_char: None,
                        target_car: None,
                        position: None,
                        radius: 0.0,
                        route: None,
                    });
            }
            COMMAND_SET_CHAR_OBJ_FOLLOW_ROUTE => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                self.push_char_objective(
                    scripts,
                    params[0],
                    27,
                    None,
                    None,
                    None,
                    0.0,
                    Some((params[1], params[2])),
                );
            }
            COMMAND_ADD_ROUTE_POINT => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddRoutePoint {
                        route: params[0],
                        position: CVector::new(
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                            Self::script_param_as_f32(params[3]),
                        ),
                    });
            }
            COMMAND_SET_CHAR_OBJ_FOLLOW_CHAR_IN_FORMATION => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                self.push_char_objective(
                    scripts,
                    params[0],
                    12,
                    Some(params[1]),
                    None,
                    None,
                    params[2] as f32,
                    None,
                );
            }
            COMMAND_SET_CHAR_OBJ_CATCH_TRAIN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.push_char_objective(scripts, params[0], 30, None, None, None, 0.0, None);
            }
            COMMAND_SET_CHAR_OBJ_HAIL_TAXI => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.push_char_objective(scripts, params[0], 29, None, None, None, 0.0, None);
            }
            COMMAND_SET_CHAR_OBJ_STEAL_ANY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.push_char_objective(scripts, params[0], 32, None, None, None, 0.0, None);
            }
            COMMAND_ADD_PEDS_IN_AREA_TO_COLL => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let x = Self::script_param_as_f32(params[0]);
                let y = Self::script_param_as_f32(params[1]);
                let z = ground_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingGroundProbe)?(x, y);
                let radius = Self::script_param_as_f32(params[2]);
                let members = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCollectiveProbe)?
                    .collective_peds_in_area(CVector::new(x, y, z), radius)
                    .ok_or(ScriptVmError::MissingCollectiveProbe)?;
                let collective_index = allocate_collective_id(scripts);
                add_peds_to_collective(scripts, collective_index, members);
                scripts.script_params[0] = collective_index;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_ADD_PEDS_IN_VEHICLE_TO_COLL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let members = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCollectiveProbe)?
                    .collective_peds_in_vehicle(params[0])
                    .ok_or(ScriptVmError::MissingCollectiveProbe)?;
                let collective_index = allocate_collective_id(scripts);
                add_peds_to_collective(scripts, collective_index, members);
                scripts.script_params[0] = collective_index;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_CLEAR_COLL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                for member in &mut scripts.collectives {
                    if member.col_index == params[0] {
                        member.col_index = -1;
                        member.ped_index = 0;
                    }
                }
            }
            COMMAND_GET_NUMBER_OF_PEDS_IN_COLL => {
                let _params = self.collect_parameters(scripts, &mut ip, 1)?;
                let runtime = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCollectiveProbe)?;
                let mut total = 0;
                for index in 0..scripts.collectives.len() {
                    let Some(member) = scripts.collectives.get(index) else {
                        continue;
                    };
                    let handle = member.ped_index;
                    if runtime.char_snapshot(handle).is_some() {
                        total += 1;
                    } else if let Some(member) = scripts.collectives.get_mut(index) {
                        member.col_index = -1;
                        member.ped_index = 0;
                    }
                }
                scripts.script_params[0] = total;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_COLL_IN_AREA_2D
            | COMMAND_IS_COLL_IN_AREA_ON_FOOT_2D
            | COMMAND_IS_COLL_IN_AREA_IN_CAR_2D
            | COMMAND_IS_COLL_STOPPED_IN_AREA_2D
            | COMMAND_IS_COLL_STOPPED_IN_AREA_ON_FOOT_2D
            | COMMAND_IS_COLL_STOPPED_IN_AREA_IN_CAR_2D => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let mut min_x = Self::script_param_as_f32(params[1]);
                let mut min_y = Self::script_param_as_f32(params[2]);
                let mut max_x = Self::script_param_as_f32(params[3]);
                let mut max_y = Self::script_param_as_f32(params[4]);
                if min_x > max_x {
                    std::mem::swap(&mut min_x, &mut max_x);
                }
                if min_y > max_y {
                    std::mem::swap(&mut min_y, &mut max_y);
                }
                let stopped_only = matches!(
                    command,
                    COMMAND_IS_COLL_STOPPED_IN_AREA_2D
                        | COMMAND_IS_COLL_STOPPED_IN_AREA_ON_FOOT_2D
                        | COMMAND_IS_COLL_STOPPED_IN_AREA_IN_CAR_2D
                );
                let on_foot_only = matches!(
                    command,
                    COMMAND_IS_COLL_IN_AREA_ON_FOOT_2D
                        | COMMAND_IS_COLL_STOPPED_IN_AREA_ON_FOOT_2D
                );
                let in_car_only = matches!(
                    command,
                    COMMAND_IS_COLL_IN_AREA_IN_CAR_2D
                        | COMMAND_IS_COLL_STOPPED_IN_AREA_IN_CAR_2D
                );
                let runtime = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCollectiveProbe)?;
                let mut result = true;
                for index in 0..scripts.collectives.len() {
                    let Some(member) = scripts.collectives.get(index) else {
                        continue;
                    };
                    if member.col_index != params[0] {
                        continue;
                    }
                    let handle = member.ped_index;
                    let Some(snapshot) = runtime.char_snapshot(handle) else {
                        if let Some(member) = scripts.collectives.get_mut(index) {
                            member.col_index = -1;
                            member.ped_index = 0;
                        }
                        continue;
                    };
                    let in_car = snapshot.current_vehicle.is_some();
                    let vehicle_snapshot = match snapshot.current_vehicle {
                        Some(vehicle) => Some(
                            runtime
                                .vehicle_snapshot(vehicle)
                                .ok_or(ScriptVmError::MissingVehicle(vehicle))?,
                        ),
                        None => None,
                    };
                    let stopped = vehicle_snapshot
                        .as_ref()
                        .map_or(snapshot.stopped, |vehicle| vehicle.stopped);
                    if stopped_only && !stopped {
                        result = false;
                        break;
                    }
                    let position = vehicle_snapshot
                        .map_or(snapshot.position, |vehicle| vehicle.position);
                    let in_area = min_x <= position.x
                        && max_x >= position.x
                        && min_y <= position.y
                        && max_y >= position.y;
                    result = in_area
                        && (!on_foot_only || !in_car)
                        && (!in_car_only || in_car);
                    if !result {
                        break;
                    }
                }
                // Native's debug parameter highlights the requested bounds;
                // that presentation hook is still tracked in the audit.
                self.update_compare_flag(result);
            }
            COMMAND_LOCATE_COLL_ANY_MEANS_2D
            | COMMAND_LOCATE_COLL_ON_FOOT_2D
            | COMMAND_LOCATE_COLL_IN_CAR_2D
            | COMMAND_LOCATE_STOPPED_COLL_ANY_MEANS_2D
            | COMMAND_LOCATE_STOPPED_COLL_ON_FOOT_2D
            | COMMAND_LOCATE_STOPPED_COLL_IN_CAR_2D
            | COMMAND_LOCATE_COLL_ANY_MEANS_CHAR_2D
            | COMMAND_LOCATE_COLL_ON_FOOT_CHAR_2D
            | COMMAND_LOCATE_COLL_IN_CAR_CHAR_2D
            | COMMAND_LOCATE_COLL_ANY_MEANS_CAR_2D
            | COMMAND_LOCATE_COLL_ON_FOOT_CAR_2D
            | COMMAND_LOCATE_COLL_IN_CAR_CAR_2D
            | COMMAND_LOCATE_COLL_ANY_MEANS_PLAYER_2D
            | COMMAND_LOCATE_COLL_ON_FOOT_PLAYER_2D
            | COMMAND_LOCATE_COLL_IN_CAR_PLAYER_2D => {
                let area_target = matches!(
                    command,
                    COMMAND_LOCATE_COLL_ANY_MEANS_2D
                        | COMMAND_LOCATE_COLL_ON_FOOT_2D
                        | COMMAND_LOCATE_COLL_IN_CAR_2D
                        | COMMAND_LOCATE_STOPPED_COLL_ANY_MEANS_2D
                        | COMMAND_LOCATE_STOPPED_COLL_ON_FOOT_2D
                        | COMMAND_LOCATE_STOPPED_COLL_IN_CAR_2D
                );
                let params = self.collect_parameters(
                    scripts,
                    &mut ip,
                    if area_target { 6 } else { 5 },
                )?;
                let (center, dx, dy) = if area_target {
                    (
                        CVector::new(
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                            0.0,
                        ),
                        Self::script_param_as_f32(params[3]),
                        Self::script_param_as_f32(params[4]),
                    )
                } else {
                    let target_index = params[1];
                    let is_char_target = matches!(
                        command,
                        COMMAND_LOCATE_COLL_ANY_MEANS_CHAR_2D
                            | COMMAND_LOCATE_COLL_ON_FOOT_CHAR_2D
                            | COMMAND_LOCATE_COLL_IN_CAR_CHAR_2D
                    );
                    let is_car_target = matches!(
                        command,
                        COMMAND_LOCATE_COLL_ANY_MEANS_CAR_2D
                            | COMMAND_LOCATE_COLL_ON_FOOT_CAR_2D
                            | COMMAND_LOCATE_COLL_IN_CAR_CAR_2D
                    );
                    let target_position = if is_char_target {
                        let snapshot = entity_runtime
                            .as_deref_mut()
                            .ok_or(ScriptVmError::MissingCharProbe)?
                            .char_snapshot(target_index)
                            .ok_or(ScriptVmError::MissingChar(target_index))?;
                        if let Some(vehicle) = snapshot.current_vehicle {
                            entity_runtime
                                .as_deref_mut()
                                .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                                .vehicle_snapshot(vehicle)
                                .ok_or(ScriptVmError::MissingVehicle(vehicle))?
                                .position
                        } else {
                            snapshot.position
                        }
                    } else if is_car_target {
                        entity_runtime
                            .as_deref_mut()
                            .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                            .vehicle_snapshot(target_index)
                            .ok_or(ScriptVmError::MissingVehicle(target_index))?
                            .position
                    } else {
                        player_probe
                            .as_deref_mut()
                            .ok_or(ScriptVmError::MissingPlayerProbe)?(target_index)
                            .ok_or(ScriptVmError::MissingPlayer(target_index))?
                            .position
                    };
                    (
                        target_position,
                        Self::script_param_as_f32(params[2]),
                        Self::script_param_as_f32(params[3]),
                    )
                };
                let min_x = center.x - dx;
                let max_x = center.x + dx;
                let min_y = center.y - dy;
                let max_y = center.y + dy;
                let on_foot_only = matches!(
                    command,
                    COMMAND_LOCATE_COLL_ON_FOOT_2D
                        | COMMAND_LOCATE_STOPPED_COLL_ON_FOOT_2D
                        | COMMAND_LOCATE_COLL_ON_FOOT_CHAR_2D
                        | COMMAND_LOCATE_COLL_ON_FOOT_CAR_2D
                        | COMMAND_LOCATE_COLL_ON_FOOT_PLAYER_2D
                );
                let in_car_only = matches!(
                    command,
                    COMMAND_LOCATE_COLL_IN_CAR_2D
                        | COMMAND_LOCATE_STOPPED_COLL_IN_CAR_2D
                        | COMMAND_LOCATE_COLL_IN_CAR_CHAR_2D
                        | COMMAND_LOCATE_COLL_IN_CAR_CAR_2D
                        | COMMAND_LOCATE_COLL_IN_CAR_PLAYER_2D
                );
                let stopped_only = matches!(
                    command,
                    COMMAND_LOCATE_STOPPED_COLL_ANY_MEANS_2D
                        | COMMAND_LOCATE_STOPPED_COLL_ON_FOOT_2D
                        | COMMAND_LOCATE_STOPPED_COLL_IN_CAR_2D
                );
                let runtime = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCollectiveProbe)?;
                let mut result = true;
                for index in 0..scripts.collectives.len() {
                    let Some(member) = scripts.collectives.get(index) else {
                        continue;
                    };
                    if member.col_index != params[0] {
                        continue;
                    }
                    let handle = member.ped_index;
                    let Some(snapshot) = runtime.char_snapshot(handle) else {
                        if let Some(member) = scripts.collectives.get_mut(index) {
                            member.col_index = -1;
                            member.ped_index = 0;
                        }
                        continue;
                    };
                    let vehicle_snapshot = match snapshot.current_vehicle {
                        Some(vehicle) => Some(
                            runtime
                                .vehicle_snapshot(vehicle)
                                .ok_or(ScriptVmError::MissingVehicle(vehicle))?,
                        ),
                        None => None,
                    };
                    let in_car = vehicle_snapshot.is_some();
                    if stopped_only
                        && !vehicle_snapshot
                            .as_ref()
                            .map_or(snapshot.stopped, |vehicle| vehicle.stopped)
                    {
                        result = false;
                        break;
                    }
                    let position = vehicle_snapshot
                        .map_or(snapshot.position, |vehicle| vehicle.position);
                    result = min_x <= position.x
                        && max_x >= position.x
                        && min_y <= position.y
                        && max_y >= position.y
                        && (!on_foot_only || !in_car)
                        && (!in_car_only || in_car);
                    if !result {
                        break;
                    }
                }
                // `debug` requests native important-area drawing; this
                // presentation hook is tracked as a remaining integration.
                self.update_compare_flag(result);
            }
            COMMAND_SET_COLL_OBJ_NO_OBJ
            | COMMAND_SET_COLL_OBJ_WAIT_ON_FOOT
            | COMMAND_SET_COLL_OBJ_FLEE_ON_FOOT_TILL_SAFE
            | COMMAND_SET_COLL_OBJ_WAIT_IN_CAR
            | COMMAND_SET_COLL_OBJ_LEAVE_CAR
            | COMMAND_SET_COLL_OBJ_CATCH_TRAIN
            | COMMAND_SET_COLL_OBJ_STEAL_ANY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let objective = match command {
                    COMMAND_SET_COLL_OBJ_NO_OBJ => 0,
                    COMMAND_SET_COLL_OBJ_WAIT_ON_FOOT => 1,
                    COMMAND_SET_COLL_OBJ_FLEE_ON_FOOT_TILL_SAFE => 2,
                    COMMAND_SET_COLL_OBJ_WAIT_IN_CAR => 5,
                    COMMAND_SET_COLL_OBJ_LEAVE_CAR => 13,
                    COMMAND_SET_COLL_OBJ_CATCH_TRAIN => 30,
                    _ => 32,
                };
                self.push_collective_objective(
                    scripts,
                    &mut entity_runtime,
                    params[0],
                    objective,
                    None,
                    None,
                    None,
                    0.0,
                    None,
                )?;
            }
            COMMAND_SET_COLL_OBJ_GUARD_SPOT => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(position.x, position.y);
                }
                self.push_collective_objective(
                    scripts,
                    &mut entity_runtime,
                    params[0],
                    3,
                    None,
                    None,
                    Some(position),
                    0.0,
                    None,
                )?;
            }
            COMMAND_SET_COLL_OBJ_GUARD_AREA
            | COMMAND_SET_COLL_OBJ_GOTO_AREA_ON_FOOT
            | COMMAND_SET_COLL_OBJ_RUN_TO_AREA
            | COMMAND_SET_COLL_OBJ_GOTO_AREA_ANY_MEANS => {
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let x1 = Self::script_param_as_f32(params[1]);
                let y1 = Self::script_param_as_f32(params[2]);
                let x2 = Self::script_param_as_f32(params[3]);
                let y2 = Self::script_param_as_f32(params[4]);
                let min_x = x1.min(x2);
                let max_x = x1.max(x2);
                let min_y = y1.min(y2);
                let max_y = y1.max(y2);
                let mut position = CVector::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5, 0.0);
                position.z = ground_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingGroundProbe)?(position.x, position.y);
                let radius = (position.x - min_x).max(position.y - min_y);
                let objective = match command {
                    COMMAND_SET_COLL_OBJ_RUN_TO_AREA => 22,
                    COMMAND_SET_COLL_OBJ_GOTO_AREA_ANY_MEANS => 20,
                    COMMAND_SET_COLL_OBJ_GOTO_AREA_ON_FOOT => 21,
                    _ => 3,
                };
                self.push_collective_objective(
                    scripts,
                    &mut entity_runtime,
                    params[0],
                    objective,
                    None,
                    None,
                    Some(position),
                    radius,
                    None,
                )?;
            }
            COMMAND_SET_COLL_OBJ_KILL_CHAR_ON_FOOT
            | COMMAND_SET_COLL_OBJ_KILL_CHAR_ANY_MEANS
            | COMMAND_SET_COLL_OBJ_FLEE_CHAR_ON_FOOT_TILL_SAFE
            | COMMAND_SET_COLL_OBJ_FLEE_CHAR_ON_FOOT_ALWAYS
            | COMMAND_SET_COLL_OBJ_GOTO_CHAR_ON_FOOT
            | COMMAND_SET_COLL_OBJ_KILL_PLAYER_ON_FOOT
            | COMMAND_SET_COLL_OBJ_KILL_PLAYER_ANY_MEANS
            | COMMAND_SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_TILL_SAFE
            | COMMAND_SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_ALWAYS
            | COMMAND_SET_COLL_OBJ_GOTO_PLAYER_ON_FOOT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let (target, objective) = match command {
                    COMMAND_SET_COLL_OBJ_KILL_CHAR_ON_FOOT
                    | COMMAND_SET_COLL_OBJ_KILL_PLAYER_ON_FOOT => (params[1], 7),
                    COMMAND_SET_COLL_OBJ_KILL_CHAR_ANY_MEANS
                    | COMMAND_SET_COLL_OBJ_KILL_PLAYER_ANY_MEANS => (params[1], 8),
                    COMMAND_SET_COLL_OBJ_FLEE_CHAR_ON_FOOT_TILL_SAFE
                    | COMMAND_SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_TILL_SAFE => (params[1], 9),
                    COMMAND_SET_COLL_OBJ_FLEE_CHAR_ON_FOOT_ALWAYS
                    | COMMAND_SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_ALWAYS => (params[1], 10),
                    _ => (params[1], 11),
                };
                let target = if matches!(
                    command,
                    COMMAND_SET_COLL_OBJ_KILL_PLAYER_ON_FOOT
                        | COMMAND_SET_COLL_OBJ_KILL_PLAYER_ANY_MEANS
                        | COMMAND_SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_TILL_SAFE
                        | COMMAND_SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_ALWAYS
                        | COMMAND_SET_COLL_OBJ_GOTO_PLAYER_ON_FOOT
                ) {
                    scripts
                        .player_char_handles
                        .get(target.max(0) as usize)
                        .copied()
                        .unwrap_or(target.saturating_add(1))
                } else {
                    target
                };
                self.push_collective_objective(
                    scripts,
                    &mut entity_runtime,
                    params[0],
                    objective,
                    Some(target),
                    None,
                    None,
                    0.0,
                    None,
                )?;
            }
            COMMAND_SET_COLL_OBJ_ENTER_CAR_AS_PASSENGER
            | COMMAND_SET_COLL_OBJ_ENTER_CAR_AS_DRIVER
            | COMMAND_SET_COLL_OBJ_DESTROY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let objective = match command {
                    COMMAND_SET_COLL_OBJ_ENTER_CAR_AS_PASSENGER => 14,
                    COMMAND_SET_COLL_OBJ_ENTER_CAR_AS_DRIVER => 15,
                    _ => 19,
                };
                self.push_collective_objective(
                    scripts,
                    &mut entity_runtime,
                    params[0],
                    objective,
                    None,
                    Some(params[1]),
                    None,
                    0.0,
                    None,
                )?;
            }
            COMMAND_SET_COLL_OBJ_FOLLOW_ROUTE => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                self.push_collective_objective(
                    scripts,
                    &mut entity_runtime,
                    params[0],
                    27,
                    None,
                    None,
                    None,
                    0.0,
                    Some((params[1], params[2])),
                )?;
            }
            COMMAND_SET_COLL_OBJ_GOTO_COORD_ON_FOOT | COMMAND_SET_COLL_OBJ_RUN_TO_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let x = Self::script_param_as_f32(params[1]);
                let y = Self::script_param_as_f32(params[2]);
                let z = ground_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingGroundProbe)?(x, y);
                self.push_collective_objective(
                    scripts,
                    &mut entity_runtime,
                    params[0],
                    if command == COMMAND_SET_COLL_OBJ_RUN_TO_COORD { 22 } else { 21 },
                    None,
                    None,
                    Some(CVector::new(x, y, z)),
                    0.0,
                    None,
                )?;
            }
            COMMAND_CLEAR_CHAR_THREAT_SEARCH => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearCharThreatSearch { handle: params[0] });
            }
            COMMAND_SET_CHAR_THREAT_SEARCH => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharThreatSearch {
                        handle: params[0],
                        threats: params[1] as u32,
                    });
            }
            COMMAND_IS_CHAR_OBJECTIVE_PASSED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.script_objective_completed);
            }
            COMMAND_IS_CHAR_IN_PLAYERS_GROUP => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let ped = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                let player =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[1])
                    .ok_or(ScriptVmError::MissingPlayer(params[1]))?;
                let player_slot = (player.ped_handle >> 8).max(0) as usize;
                self.update_compare_flag(ped.leader_slot == Some(player_slot));
            }
            COMMAND_SET_SWAT_REQUIRED | COMMAND_SET_FBI_REQUIRED | COMMAND_SET_ARMY_REQUIRED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let kind = match command {
                    COMMAND_SET_SWAT_REQUIRED => 0,
                    COMMAND_SET_FBI_REQUIRED => 1,
                    _ => 2,
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetSpecialPoliceRequired {
                        kind,
                        required: params[0] != 0,
                    });
            }
            COMMAND_SET_CAR_STATUS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarStatus {
                        handle: params[0],
                        status: params[1],
                    });
            }
            COMMAND_START_PACMAN_RACE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.pacman_pills_eaten = 0;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartPacManRace(params[0]));
            }
            COMMAND_START_PACMAN_RECORD => scripts
                .pending_effects
                .push(ScriptCommandEffect::StartPacManRecord),
            COMMAND_GET_NUMBER_OF_POWER_PILLS_EATEN => {
                scripts.script_params[0] = scripts.pacman_pills_eaten;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_CLEAR_PACMAN => scripts
                .pending_effects
                .push(ScriptCommandEffect::ClearPacMan),
            COMMAND_START_PACMAN_SCRAMBLE => {
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartPacManScramble {
                        position,
                        scramble_mult: Self::script_param_as_f32(params[3]),
                        count: params[4] as i16,
                    });
            }
            COMMAND_GET_NUMBER_OF_POWER_PILLS_CARRIED => {
                scripts.script_params[0] = scripts.pacman_pills_carried;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_CLEAR_NUMBER_OF_POWER_PILLS_CARRIED => {
                scripts.pacman_pills_carried = 0;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ResetPacManPillsCarried);
            }
            COMMAND_IS_CAR_ON_SCREEN | COMMAND_IS_CHAR_ON_SCREEN | COMMAND_IS_OBJECT_ON_SCREEN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = match command {
                    COMMAND_IS_CAR_ON_SCREEN => entity_runtime
                        .as_deref_mut()
                        .and_then(|runtime| runtime.vehicle_snapshot(params[0])),
                    COMMAND_IS_CHAR_ON_SCREEN => entity_runtime
                        .as_deref_mut()
                        .and_then(|runtime| runtime.char_snapshot(params[0])),
                    _ => entity_runtime
                        .as_deref_mut()
                        .and_then(|runtime| runtime.object_snapshot(params[0])),
                };
                let visible = snapshot.is_some_and(|snapshot| {
                    camera_probe
                        .as_deref_mut()
                        .map(|probe| probe(snapshot.bound_center, snapshot.bound_radius))
                        .unwrap_or(false)
                });
                self.update_compare_flag(visible);
            }
            COMMAND_SET_CHAR_PERSONALITY => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharPersonality {
                        handle: params[0],
                        personality: params[1],
                    });
            }
            COMMAND_MARK_CHAR_AS_NO_LONGER_NEEDED
            | COMMAND_MARK_CAR_AS_NO_LONGER_NEEDED
            | COMMAND_MARK_OBJECT_AS_NO_LONGER_NEEDED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let handle = params[0];
                let (entity_type, effect) = match command {
                    COMMAND_MARK_CHAR_AS_NO_LONGER_NEEDED => (
                        CLEANUP_CHAR,
                        ScriptCommandEffect::MarkCharAsNoLongerNeeded { handle },
                    ),
                    COMMAND_MARK_CAR_AS_NO_LONGER_NEEDED => (
                        CLEANUP_CAR,
                        ScriptCommandEffect::MarkCarAsNoLongerNeeded { handle },
                    ),
                    _ => (
                        CLEANUP_OBJECT,
                        ScriptCommandEffect::MarkObjectAsNoLongerNeeded { handle },
                    ),
                };
                if self.mission_flag {
                    scripts
                        .mission_cleanup
                        .remove_entity_from_list(handle, entity_type);
                }
                scripts.pending_effects.push(effect);
            }
            COMMAND_SET_PED_DENSITY_MULTIPLIER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPedDensityMultiplier(
                        Self::script_param_as_f32(params[0]),
                    ));
            }
            COMMAND_FORCE_RANDOM_PED_TYPE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ForceRandomPedType(params[0]));
            }
            COMMAND_SET_CAR_DENSITY_MULTIPLIER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarDensityMultiplier(
                        Self::script_param_as_f32(params[0]),
                    ));
            }
            COMMAND_SET_CHAR_CANT_BE_DRAGGED_OUT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharCantBeDraggedOut {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_ADD_EXPLOSION => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddExplosion {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        explosion_type: params[3],
                    });
            }
            COMMAND_EXPLODE_CHAR_HEAD => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ExplodeCharHead {
                        handle: params[0],
                        nasty_game: scripts.nasty_game,
                    });
            }
            COMMAND_CLEAR_AREA => {
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearArea {
                        position,
                        radius: Self::script_param_as_f32(params[3]),
                        remove_projectiles_and_tidy_shadows: params[4] != 0,
                    });
            }
            COMMAND_CLEAR_AREA_OF_CHARS => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let first = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let second = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearAreaOfChars {
                        min: CVector::new(
                            first.x.min(second.x),
                            first.y.min(second.y),
                            first.z.min(second.z),
                        ),
                        max: CVector::new(
                            first.x.max(second.x),
                            first.y.max(second.y),
                            first.z.max(second.z),
                        ),
                    });
            }
            COMMAND_CLEAR_AREA_OF_CARS => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let first = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let second = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearAreaOfCars {
                        min: CVector::new(
                            first.x.min(second.x),
                            first.y.min(second.y),
                            first.z.min(second.z),
                        ),
                        max: CVector::new(
                            first.x.max(second.x),
                            first.y.max(second.y),
                            first.z.max(second.z),
                        ),
                    });
            }
            COMMAND_CLEAR_NUMBER_OF_POWER_PILLS_EATEN => {
                scripts.pacman_pills_eaten = 0;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ResetPacManPillsEaten);
            }
            COMMAND_ADD_POWER_PILL => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddPowerPill(position));
            }
            COMMAND_IS_PROJECTILE_IN_AREA | COMMAND_DESTROY_PROJECTILES_IN_AREA => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let first = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let second = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                let min = CVector::new(
                    first.x.min(second.x),
                    first.y.min(second.y),
                    first.z.min(second.z),
                );
                let max = CVector::new(
                    first.x.max(second.x),
                    first.y.max(second.y),
                    first.z.max(second.z),
                );
                let is_projectile_weapon = |weapon_type: u8| {
                    matches!(
                        weapon_type,
                        value if value == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER as u8
                            || value == eWeaponType::WEAPONTYPE_MOLOTOV as u8
                            || value == eWeaponType::WEAPONTYPE_GRENADE as u8
                    )
                };
                let in_area = |position: CVector| {
                    position.x >= min.x
                        && position.x <= max.x
                        && position.y >= min.y
                        && position.y <= max.y
                        && position.z >= min.z
                        && position.z <= max.z
                };
                let found = scripts
                    .projectile_snapshots
                    .iter()
                    .any(|(weapon_type, position)| {
                        is_projectile_weapon(*weapon_type) && in_area(*position)
                    });
                self.update_compare_flag(found);
                if command == COMMAND_DESTROY_PROJECTILES_IN_AREA {
                    scripts
                        .projectile_snapshots
                        .retain(|(weapon_type, position)| {
                            !is_projectile_weapon(*weapon_type) || !in_area(*position)
                        });
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::DestroyProjectilesInArea { min, max });
                }
                if scripts.dbg_flag {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::DebugCube {
                            x1: min.x,
                            y1: min.y,
                            z1: min.z,
                            x2: max.x,
                            y2: max.y,
                            z2: max.z,
                        });
                }
            }
            COMMAND_DROP_MINE | COMMAND_DROP_NAUTICAL_MINE => {
                // re3 Script3.cpp: both commands deliberately use
                // PICKUP_MINE_INACTIVE; the nautical model does not select the
                // nautical pickup type here.
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    ) + 0.5;
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateUntrackedPickup {
                        model_id: if command == COMMAND_DROP_MINE {
                            MI_CARMINE as i32
                        } else {
                            MI_NAUTICALMINE as i32
                        },
                        pickup_type: i32::from(PICKUP_MINE_INACTIVE),
                        quantity: 0,
                        position,
                    });
            }
            COMMAND_IS_EXPLOSION_IN_AREA => {
                let params = self.collect_parameters(scripts, &mut ip, 7)?;
                let first = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                let second = CVector::new(
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                    Self::script_param_as_f32(params[6]),
                );
                let min = CVector::new(
                    first.x.min(second.x),
                    first.y.min(second.y),
                    first.z.min(second.z),
                );
                let max = CVector::new(
                    first.x.max(second.x),
                    first.y.max(second.y),
                    first.z.max(second.z),
                );
                self.update_compare_flag(scripts.explosion_snapshots.iter().any(
                    |(explosion_type, position)| {
                        *explosion_type == params[0]
                            && position.x >= min.x
                            && position.x <= max.x
                            && position.y >= min.y
                            && position.y <= max.y
                            && position.z >= min.z
                            && position.z <= max.z
                    },
                ));
            }
            COMMAND_IS_EXPLOSION_IN_ZONE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let label = scripts.read_key_from_script(&mut ip)?;
                self.update_compare_flag(scripts.explosion_snapshots.iter().any(
                    |(explosion_type, position)| {
                        *explosion_type == params[0]
                            && point_in_script_zone(*position, &label, &scripts.zone_bounds)
                    },
                ));
            }
            COMMAND_CREATE_RANDOM_CAR_FOR_CAR_PARK => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                if scripts.num_random_cars < 30 {
                    let general = general
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingRandomGenerator)?;
                    let mut model = -1;
                    let mut index = general
                        .get_random_number_in_range_i32(0, MAXVEHICLESLOADED as i32)
                        as usize;
                    for _ in 0..MAXVEHICLESLOADED {
                        if model != -1 {
                            break;
                        }
                        model = scripts.loaded_vehicle_models[index];
                        // re3 continues before advancing the index when the
                        // randomly selected streaming slot is empty.
                        if model == -1 {
                            continue;
                        }
                        if !matches!(
                            model,
                            91 | 92
                                | 94
                                | 95
                                | 96
                                | 100
                                | 101
                                | 102
                                | 103
                                | 105
                                | 108
                                | 109
                                | 110
                                | 111
                                | 112
                                | 119
                                | 128
                                | 129
                                | 130
                                | 151
                                | 152
                                | 153
                        ) {
                            model = -1;
                        }
                        index += 1;
                        if index >= MAXVEHICLESLOADED {
                            index = 0;
                        }
                    }
                    if model != -1 {
                        scripts.num_random_cars += 1;
                        scripts.pending_effects.push(
                            ScriptCommandEffect::CreateRandomCarForCarPark {
                                model_id: model,
                                position: CVector::new(
                                    Self::script_param_as_f32(params[0]),
                                    Self::script_param_as_f32(params[1]),
                                    Self::script_param_as_f32(params[2]),
                                ),
                                heading_degrees: Self::script_param_as_f32(params[3]),
                            },
                        );
                    }
                }
            }
            COMMAND_REMOVE_PARTICLE_EFFECTS_IN_AREA => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveParticleEffectsInArea {
                        min: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        max: CVector::new(
                            Self::script_param_as_f32(params[3]),
                            Self::script_param_as_f32(params[4]),
                            Self::script_param_as_f32(params[5]),
                        ),
                    });
            }
            COMMAND_SET_CHAR_WAIT_STATE => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharWaitState {
                        handle: params[0],
                        wait_state: params[1],
                        time_ms: (params[2] >= 0).then_some(params[2]),
                        now_ms: time_ms,
                    });
            }
            COMMAND_SET_CAMERA_BEHIND_PLAYER => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCameraBehindPlayer);
            }
            COMMAND_SET_CAMERA_IN_FRONT_OF_PLAYER => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCameraInFrontOfPlayer);
            }
            COMMAND_ENABLE_PLAYER_CONTROL_CAMERA => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::EnablePlayerControlCamera);
            }
            COMMAND_SET_MOTION_BLUR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetMotionBlur(params[0]));
            }
            COMMAND_WARP_PLAYER_INTO_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::WarpPlayerIntoCar {
                        player_index: params[0],
                        vehicle_handle: params[1],
                    });
            }
            COMMAND_WARP_CHAR_INTO_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::WarpCharIntoCar {
                        char_handle: params[0],
                        vehicle_handle: params[1],
                    });
            }
            COMMAND_CREATE_CHAR_AS_PASSENGER => {
                // re3 Script2.cpp COMMAND_CREATE_CHAR_AS_PASSENGER: create mission
                // ped in vehicle passenger slot (or first free when slot < 0).
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let vehicle_snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let ped_type = params[1];
                let model_id = remap_script_ped_model(ped_type, params[2]);
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                let handle = run_script_entity_creation(
                    &mut entity_runtime,
                    ScriptEntityMutation::CreateChar(ScriptCreateCharRequest {
                        ped_type,
                        model_id,
                        position: vehicle_snapshot.position,
                        vehicle_handle: Some(params[0]),
                        passenger_seat: Some(params[3]),
                    }),
                    Some(general),
                )?
                .ok_or(ScriptVmError::MissingScriptEntityAllocator)?;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                if self.is_mission_script {
                    scripts
                        .mission_cleanup
                        .add_entity_to_list(handle, CLEANUP_CHAR);
                }
            }
            COMMAND_ADD_INT_VAR_TO_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Add,
                false,
                1.0,
            )?,
            COMMAND_ADD_INT_LVAR_TO_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Add,
                false,
                1.0,
            )?,
            COMMAND_ADD_INT_VAR_TO_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Add,
                false,
                1.0,
            )?,
            COMMAND_ADD_INT_LVAR_TO_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Add,
                false,
                1.0,
            )?,
            COMMAND_ADD_FLOAT_VAR_TO_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Add,
                true,
                1.0,
            )?,
            COMMAND_ADD_FLOAT_LVAR_TO_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Add,
                true,
                1.0,
            )?,
            COMMAND_ADD_FLOAT_VAR_TO_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Add,
                true,
                1.0,
            )?,
            COMMAND_ADD_FLOAT_LVAR_TO_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Add,
                true,
                1.0,
            )?,
            COMMAND_SUB_INT_VAR_FROM_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Sub,
                false,
                1.0,
            )?,
            COMMAND_SUB_INT_LVAR_FROM_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Sub,
                false,
                1.0,
            )?,
            COMMAND_SUB_INT_VAR_FROM_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Sub,
                false,
                1.0,
            )?,
            COMMAND_SUB_INT_LVAR_FROM_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Sub,
                false,
                1.0,
            )?,
            COMMAND_SUB_FLOAT_VAR_FROM_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Sub,
                true,
                1.0,
            )?,
            COMMAND_SUB_FLOAT_LVAR_FROM_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Sub,
                true,
                1.0,
            )?,
            COMMAND_SUB_FLOAT_VAR_FROM_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Sub,
                true,
                1.0,
            )?,
            COMMAND_SUB_FLOAT_LVAR_FROM_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Sub,
                true,
                1.0,
            )?,
            COMMAND_MULT_INT_VAR_BY_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Mul,
                false,
                1.0,
            )?,
            COMMAND_MULT_INT_VAR_BY_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Mul,
                false,
                1.0,
            )?,
            COMMAND_MULT_INT_LVAR_BY_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Mul,
                false,
                1.0,
            )?,
            COMMAND_MULT_INT_LVAR_BY_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Mul,
                false,
                1.0,
            )?,
            COMMAND_MULT_FLOAT_VAR_BY_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Mul,
                true,
                1.0,
            )?,
            COMMAND_MULT_FLOAT_VAR_BY_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Mul,
                true,
                1.0,
            )?,
            COMMAND_MULT_FLOAT_LVAR_BY_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Mul,
                true,
                1.0,
            )?,
            COMMAND_MULT_FLOAT_LVAR_BY_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Mul,
                true,
                1.0,
            )?,
            COMMAND_DIV_INT_VAR_BY_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Div,
                false,
                1.0,
            )?,
            COMMAND_DIV_INT_VAR_BY_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Div,
                false,
                1.0,
            )?,
            COMMAND_DIV_INT_LVAR_BY_INT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Div,
                false,
                1.0,
            )?,
            COMMAND_DIV_INT_LVAR_BY_INT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Div,
                false,
                1.0,
            )?,
            COMMAND_DIV_FLOAT_VAR_BY_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Div,
                true,
                1.0,
            )?,
            COMMAND_DIV_FLOAT_VAR_BY_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Div,
                true,
                1.0,
            )?,
            COMMAND_DIV_FLOAT_LVAR_BY_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Div,
                true,
                1.0,
            )?,
            COMMAND_DIV_FLOAT_LVAR_BY_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Div,
                true,
                1.0,
            )?,
            COMMAND_ADD_TIMED_VAL_TO_FLOAT_VAR => self.apply_timed_value_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                ScriptBinaryOp::Add,
                time_step,
            )?,
            COMMAND_ADD_TIMED_VAL_TO_FLOAT_LVAR => self.apply_timed_value_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                ScriptBinaryOp::Add,
                time_step,
            )?,
            COMMAND_ADD_TIMED_FLOAT_VAR_TO_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Add,
                true,
                time_step,
            )?,
            COMMAND_ADD_TIMED_FLOAT_LVAR_TO_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Add,
                true,
                time_step,
            )?,
            COMMAND_ADD_TIMED_FLOAT_VAR_TO_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Add,
                true,
                time_step,
            )?,
            COMMAND_ADD_TIMED_FLOAT_LVAR_TO_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Add,
                true,
                time_step,
            )?,
            COMMAND_SUB_TIMED_VAL_FROM_FLOAT_VAR => self.apply_timed_value_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                ScriptBinaryOp::Sub,
                time_step,
            )?,
            COMMAND_SUB_TIMED_VAL_FROM_FLOAT_LVAR => self.apply_timed_value_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                ScriptBinaryOp::Sub,
                time_step,
            )?,
            COMMAND_SUB_TIMED_FLOAT_VAR_FROM_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Sub,
                true,
                time_step,
            )?,
            COMMAND_SUB_TIMED_FLOAT_LVAR_FROM_FLOAT_VAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_GLOBAL,
                VAR_LOCAL,
                ScriptBinaryOp::Sub,
                true,
                time_step,
            )?,
            COMMAND_SUB_TIMED_FLOAT_VAR_FROM_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_GLOBAL,
                ScriptBinaryOp::Sub,
                true,
                time_step,
            )?,
            COMMAND_SUB_TIMED_FLOAT_LVAR_FROM_FLOAT_LVAR => self.apply_binary_var_command(
                scripts,
                &mut ip,
                VAR_LOCAL,
                VAR_LOCAL,
                ScriptBinaryOp::Sub,
                true,
                time_step,
            )?,
            COMMAND_SET_VAR_INT_TO_VAR_INT => {
                self.copy_script_var(scripts, &mut ip, VAR_GLOBAL, VAR_GLOBAL, false)?
            }
            COMMAND_SET_VAR_INT_TO_LVAR_INT => {
                self.copy_script_var(scripts, &mut ip, VAR_GLOBAL, VAR_LOCAL, false)?
            }
            COMMAND_SET_LVAR_INT_TO_VAR_INT => {
                self.copy_script_var(scripts, &mut ip, VAR_LOCAL, VAR_GLOBAL, false)?
            }
            COMMAND_SET_LVAR_INT_TO_LVAR_INT => {
                self.copy_script_var(scripts, &mut ip, VAR_LOCAL, VAR_LOCAL, false)?
            }
            COMMAND_SET_VAR_FLOAT_TO_VAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_GLOBAL, VAR_GLOBAL, false)?
            }
            COMMAND_SET_VAR_FLOAT_TO_LVAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_GLOBAL, VAR_LOCAL, false)?
            }
            COMMAND_SET_LVAR_FLOAT_TO_VAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_LOCAL, VAR_GLOBAL, false)?
            }
            COMMAND_SET_LVAR_FLOAT_TO_LVAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_LOCAL, VAR_LOCAL, false)?
            }
            COMMAND_CSET_VAR_INT_TO_VAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_GLOBAL, VAR_GLOBAL, true)?
            }
            COMMAND_CSET_VAR_INT_TO_LVAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_GLOBAL, VAR_LOCAL, true)?
            }
            COMMAND_CSET_LVAR_INT_TO_VAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_LOCAL, VAR_GLOBAL, true)?
            }
            COMMAND_CSET_LVAR_INT_TO_LVAR_FLOAT => {
                self.copy_script_var(scripts, &mut ip, VAR_LOCAL, VAR_LOCAL, true)?
            }
            COMMAND_CSET_VAR_FLOAT_TO_VAR_INT => {
                self.copy_script_int_to_float(scripts, &mut ip, VAR_GLOBAL, VAR_GLOBAL)?
            }
            COMMAND_CSET_VAR_FLOAT_TO_LVAR_INT => {
                self.copy_script_int_to_float(scripts, &mut ip, VAR_GLOBAL, VAR_LOCAL)?
            }
            COMMAND_CSET_LVAR_FLOAT_TO_VAR_INT => {
                self.copy_script_int_to_float(scripts, &mut ip, VAR_LOCAL, VAR_GLOBAL)?
            }
            COMMAND_CSET_LVAR_FLOAT_TO_LVAR_INT => {
                self.copy_script_int_to_float(scripts, &mut ip, VAR_LOCAL, VAR_LOCAL)?
            }
            COMMAND_ABS_VAR_INT => self.abs_script_var(scripts, &mut ip, VAR_GLOBAL, false)?,
            COMMAND_ABS_LVAR_INT => self.abs_script_var(scripts, &mut ip, VAR_LOCAL, false)?,
            COMMAND_ABS_VAR_FLOAT => self.abs_script_var(scripts, &mut ip, VAR_GLOBAL, true)?,
            COMMAND_ABS_LVAR_FLOAT => self.abs_script_var(scripts, &mut ip, VAR_LOCAL, true)?,
            COMMAND_GENERATE_RANDOM_FLOAT => {
                let pointer = self.get_pointer_to_script_variable(scripts, &mut ip, VAR_GLOBAL)?;
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                general.get_random_number();
                general.get_random_number();
                general.get_random_number();
                let value = (f32::from(general.get_random_number()) / 65536.0).to_bits() as i32;
                scripts.set_script_var(pointer, self, value)?;
            }
            COMMAND_GENERATE_RANDOM_INT => {
                let pointer = self.get_pointer_to_script_variable(scripts, &mut ip, VAR_GLOBAL)?;
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                scripts.set_script_var(pointer, self, i32::from(general.get_random_number()))?;
            }
            COMMAND_GET_TIME_OF_DAY => {
                let clock = clock.as_deref_mut().ok_or(ScriptVmError::MissingClock)?;
                scripts.script_params[0] = i32::from(clock.hours());
                scripts.script_params[1] = i32::from(clock.minutes());
                self.store_parameters(scripts, &mut ip, 2)?;
            }
            COMMAND_SET_TIME_OF_DAY => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let clock = clock.as_deref_mut().ok_or(ScriptVmError::MissingClock)?;
                clock.set_game_clock(params[0] as u8, params[1] as u8, time_ms);
            }
            COMMAND_GET_MINUTES_TO_TIME_OF_DAY => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let clock = clock.as_deref_mut().ok_or(ScriptVmError::MissingClock)?;
                scripts.script_params[0] =
                    clock.get_game_clock_minutes_until(params[0] as u8, params[1] as u8);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_PRINT_BIG | COMMAND_PRINT_BIG_Q => {
                let key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts.pending_effects.push(ScriptCommandEffect::PrintBig {
                    key,
                    time_ms: params[0],
                    style: params[1] - 1,
                    queue: command == COMMAND_PRINT_BIG_Q,
                });
            }
            COMMAND_PRINT => {
                let key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts.pending_effects.push(ScriptCommandEffect::Print {
                    key,
                    time_ms: params[0],
                    flag: params[1],
                });
            }
            COMMAND_PRINT_NOW => {
                let key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts.pending_effects.push(ScriptCommandEffect::PrintNow {
                    key,
                    time_ms: params[0],
                    flag: params[1],
                });
            }
            COMMAND_PRINT_SOON => {
                let key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PrintSoon {
                        key,
                        time_ms: params[0],
                        flag: params[1],
                    });
            }
            COMMAND_PRINT_STRING_IN_STRING_NOW => {
                let source_key = scripts.read_key_from_script(&mut ip)?;
                let inserted_key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PrintStringNow {
                        source_key,
                        inserted_key,
                        time_ms: params[0],
                        flag: params[1],
                    });
            }
            COMMAND_PRINT_WITH_NUMBER_BIG => {
                self.print_big_with_numbers_command(scripts, &mut ip, 1, false)?;
            }
            COMMAND_PRINT_WITH_NUMBER_BIG_Q => {
                self.print_big_with_numbers_command(scripts, &mut ip, 1, true)?;
            }
            COMMAND_PRINT_WITH_NUMBER => {
                self.print_with_numbers_command(scripts, &mut ip, 1, 0)?;
            }
            COMMAND_PRINT_WITH_NUMBER_NOW => {
                self.print_with_numbers_command(scripts, &mut ip, 1, 1)?;
            }
            COMMAND_PRINT_WITH_2_NUMBERS => {
                self.print_with_numbers_command(scripts, &mut ip, 2, 0)?;
            }
            COMMAND_PRINT_WITH_2_NUMBERS_NOW => {
                self.print_with_numbers_command(scripts, &mut ip, 2, 1)?;
            }
            COMMAND_PRINT_WITH_2_NUMBERS_SOON => {
                self.print_with_numbers_command(scripts, &mut ip, 2, 2)?;
            }
            COMMAND_PRINT_WITH_3_NUMBERS => {
                self.print_with_numbers_command(scripts, &mut ip, 3, 0)?;
            }
            COMMAND_PRINT_WITH_3_NUMBERS_NOW => {
                self.print_with_numbers_command(scripts, &mut ip, 3, 1)?;
            }
            COMMAND_PRINT_WITH_3_NUMBERS_SOON => {
                self.print_with_numbers_command(scripts, &mut ip, 3, 2)?;
            }
            COMMAND_PRINT_WITH_4_NUMBERS => {
                self.print_with_numbers_command(scripts, &mut ip, 4, 0)?;
            }
            COMMAND_PRINT_WITH_4_NUMBERS_NOW => {
                self.print_with_numbers_command(scripts, &mut ip, 4, 1)?;
            }
            COMMAND_PRINT_WITH_4_NUMBERS_SOON => {
                self.print_with_numbers_command(scripts, &mut ip, 4, 2)?;
            }
            COMMAND_PRINT_WITH_5_NUMBERS => {
                self.print_with_numbers_command(scripts, &mut ip, 5, 0)?;
            }
            COMMAND_PRINT_WITH_5_NUMBERS_NOW => {
                self.print_with_numbers_command(scripts, &mut ip, 5, 1)?;
            }
            COMMAND_PRINT_WITH_5_NUMBERS_SOON => {
                self.print_with_numbers_command(scripts, &mut ip, 5, 2)?;
            }
            COMMAND_PRINT_WITH_6_NUMBERS => {
                self.print_with_numbers_command(scripts, &mut ip, 6, 0)?;
            }
            COMMAND_PRINT_WITH_6_NUMBERS_NOW => {
                self.print_with_numbers_command(scripts, &mut ip, 6, 1)?;
            }
            COMMAND_PRINT_WITH_6_NUMBERS_SOON => {
                self.print_with_numbers_command(scripts, &mut ip, 6, 2)?;
            }
            COMMAND_PRINT_WITH_2_NUMBERS_BIG => {
                self.print_big_with_numbers_command(scripts, &mut ip, 2, false)?;
            }
            COMMAND_PRINT_WITH_3_NUMBERS_BIG => {
                self.print_big_with_numbers_command(scripts, &mut ip, 3, false)?;
            }
            COMMAND_PRINT_WITH_4_NUMBERS_BIG => {
                self.print_big_with_numbers_command(scripts, &mut ip, 4, false)?;
            }
            COMMAND_PRINT_WITH_5_NUMBERS_BIG => {
                self.print_big_with_numbers_command(scripts, &mut ip, 5, false)?;
            }
            COMMAND_PRINT_WITH_6_NUMBERS_BIG => {
                self.print_big_with_numbers_command(scripts, &mut ip, 6, false)?;
            }
            COMMAND_CLEAR_PRINTS => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearPrints);
            }
            COMMAND_PRINT_HELP => {
                let key = scripts.read_key_from_script(&mut ip)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PrintHelp { key });
            }
            COMMAND_CLEAR_HELP => {
                scripts.pending_effects.push(ScriptCommandEffect::ClearHelp);
            }
            COMMAND_CLEAR_THIS_PRINT => {
                let key = scripts.read_key_from_script(&mut ip)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearThisPrint { key });
            }
            COMMAND_CLEAR_THIS_BIG_PRINT => {
                let key = scripts.read_key_from_script(&mut ip)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearThisBigPrint { key });
            }
            COMMAND_DISPLAY_ONSCREEN_TIMER | COMMAND_DISPLAY_ONSCREEN_TIMER_WITH_STRING => {
                if scripts.read_1_byte_from_script(&mut ip)? != ARGUMENT_GLOBALVAR {
                    return Err(ScriptVmError::InvalidVariableType);
                }
                let variable_offset = u32::from(scripts.read_2_bytes_from_script(&mut ip)?);
                let key = if command == COMMAND_DISPLAY_ONSCREEN_TIMER_WITH_STRING {
                    Some(scripts.read_key_from_script(&mut ip)?)
                } else {
                    None
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddOnscreenClock {
                        variable_offset,
                        key,
                    });
            }
            COMMAND_CLEAR_ONSCREEN_TIMER => {
                if scripts.read_1_byte_from_script(&mut ip)? != ARGUMENT_GLOBALVAR {
                    return Err(ScriptVmError::InvalidVariableType);
                }
                let variable_offset = u32::from(scripts.read_2_bytes_from_script(&mut ip)?);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearOnscreenClock { variable_offset });
            }
            COMMAND_DISPLAY_ONSCREEN_COUNTER | COMMAND_DISPLAY_ONSCREEN_COUNTER_WITH_STRING => {
                if scripts.read_1_byte_from_script(&mut ip)? != ARGUMENT_GLOBALVAR {
                    return Err(ScriptVmError::InvalidVariableType);
                }
                let variable_offset = u32::from(scripts.read_2_bytes_from_script(&mut ip)?);
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let key = if command == COMMAND_DISPLAY_ONSCREEN_COUNTER_WITH_STRING {
                    Some(scripts.read_key_from_script(&mut ip)?)
                } else {
                    None
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddOnscreenCounter {
                        variable_offset,
                        counter_type: params[0],
                        key,
                    });
            }
            COMMAND_CLEAR_ONSCREEN_COUNTER => {
                if scripts.read_1_byte_from_script(&mut ip)? != ARGUMENT_GLOBALVAR {
                    return Err(ScriptVmError::InvalidVariableType);
                }
                let variable_offset = u32::from(scripts.read_2_bytes_from_script(&mut ip)?);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearOnscreenCounter { variable_offset });
            }
            COMMAND_FREEZE_ONSCREEN_TIMER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::FreezeOnscreenTimer(params[0] != 0));
            }
            COMMAND_START_SCRIPT_FIRE => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                let slot = reserve_shared_fire_slot(scripts, &mut entity_runtime);
                scripts.script_params[0] = slot;
                self.store_parameters(scripts, &mut ip, 1)?;
                if slot >= 0 {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StartScriptFire {
                            slot,
                            position,
                            target: None,
                        });
                }
            }
            COMMAND_START_CAR_FIRE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let slot = reserve_shared_fire_slot(scripts, &mut entity_runtime);
                scripts.script_params[0] = slot;
                self.store_parameters(scripts, &mut ip, 1)?;
                if slot >= 0 {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StartScriptFire {
                            slot,
                            position: vehicle.position,
                            target: Some(ScriptFireTarget::Vehicle(params[0])),
                        });
                }
            }
            COMMAND_START_CHAR_FIRE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let ped = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                let slot = reserve_shared_fire_slot(scripts, &mut entity_runtime);
                scripts.script_params[0] = slot;
                self.store_parameters(scripts, &mut ip, 1)?;
                if slot >= 0 {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StartScriptFire {
                            slot,
                            position: ped.position,
                            target: Some(ScriptFireTarget::Char(params[0])),
                        });
                }
            }
            COMMAND_IS_SCRIPT_FIRE_EXTINGUISHED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let extinguished = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.script_fire_is_extinguished(params[0]))
                    .unwrap_or_else(|| {
                        usize::try_from(params[0])
                            .ok()
                            .and_then(|slot| scripts.script_fire_slots.get(slot))
                            .is_none_or(|active| !*active)
                    });
                self.update_compare_flag(extinguished);
            }
            COMMAND_REMOVE_SCRIPT_FIRE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.release_script_fire_slot(params[0]);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveScriptFire { slot: params[0] });
            }
            COMMAND_REMOVE_ALL_SCRIPT_FIRES => {
                scripts.script_fire_slots.fill(false);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveAllScriptFires);
            }
            COMMAND_SET_SCRIPT_FIRE_AUDIO => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetScriptFireAudio {
                        slot: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SAVE_PLAYER_FROM_FIRES => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ExtinguishPlayerFires {
                        player_index: params[0],
                    });
            }
            COMMAND_SET_FREE_RESPRAYS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetFreeResprays {
                        enabled: params[0] != 0,
                    });
            }
            COMMAND_ADD_ONE_OFF_SOUND => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddOneOffSound {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        sound_id: params[3],
                    });
            }
            COMMAND_ADD_CONTINUOUS_SOUND => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let handle = scripts.alloc_audio_script_object_handle();
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddContinuousSound {
                        handle,
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        sound_id: params[3],
                    });
            }
            COMMAND_REMOVE_SOUND => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveSound { handle: params[0] });
            }
            COMMAND_IS_POINT_ON_SCREEN => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut pos = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if pos.z <= -100.0 {
                    let ground_probe = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?;
                    pos.z = ground_probe(pos.x, pos.y);
                }
                let radius = Self::script_param_as_f32(params[3]);
                let camera_probe = camera_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCameraProbe)?;
                self.update_compare_flag(camera_probe(pos, radius));
            }
            COMMAND_SCRIPT_NAME => {
                // re3 Script4.cpp: raw 8-byte key from script space, lowercased.
                let key = scripts.read_key_from_script(&mut ip)?;
                let mut name = [0u8; KEY_LENGTH_IN_SCRIPT];
                for (dst, src) in name.iter_mut().zip(key.iter()) {
                    *dst = src.to_ascii_lowercase();
                }
                self.script_name = name;
            }
            COMMAND_SET_TOTAL_NUMBER_OF_MISSIONS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetTotalNumberOfMissions(params[0]));
            }
            COMMAND_SET_PROGRESS_TOTAL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetProgressTotal(params[0]));
            }
            COMMAND_PLAYER_MADE_PROGRESS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.pending_effects.push(ScriptCommandEffect::Stats(
                    ScriptStatsEffect::AddProgress(params[0]),
                ));
            }
            COMMAND_INDUSTRIAL_PASSED => {
                scripts.pending_effects.push(ScriptCommandEffect::Stats(
                    ScriptStatsEffect::IndustrialPassed,
                ));
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PlayRadioAnnouncement {
                        announcement:
                            crate::audio::audio_enums::STREAMED_SOUND_ANNOUNCE_COMMERCIAL_OPEN,
                    });
            }
            COMMAND_COMMERCIAL_PASSED => {
                scripts.pending_effects.push(ScriptCommandEffect::Stats(
                    ScriptStatsEffect::CommercialPassed,
                ));
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PlayRadioAnnouncement {
                        announcement:
                            crate::audio::audio_enums::STREAMED_SOUND_ANNOUNCE_SUBURBAN_OPEN,
                    });
            }
            COMMAND_SUBURBAN_PASSED => scripts.pending_effects.push(ScriptCommandEffect::Stats(
                ScriptStatsEffect::SuburbanPassed,
            )),
            COMMAND_REGISTER_JUMP_DISTANCE | COMMAND_REGISTER_JUMP_HEIGHT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let value = Self::script_param_as_f32(params[0]);
                let effect = if command == COMMAND_REGISTER_JUMP_DISTANCE {
                    ScriptStatsEffect::RegisterJumpDistance(value)
                } else {
                    ScriptStatsEffect::RegisterJumpHeight(value)
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::Stats(effect));
            }
            COMMAND_REGISTER_JUMP_FLIPS
            | COMMAND_REGISTER_JUMP_SPINS
            | COMMAND_REGISTER_JUMP_STUNT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let effect = match command {
                    COMMAND_REGISTER_JUMP_FLIPS => ScriptStatsEffect::RegisterJumpFlips(params[0]),
                    COMMAND_REGISTER_JUMP_SPINS => ScriptStatsEffect::RegisterJumpSpins(params[0]),
                    _ => ScriptStatsEffect::RegisterJumpStunt(params[0]),
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::Stats(effect));
            }
            COMMAND_REGISTER_UNIQUE_JUMP_FOUND => scripts.pending_effects.push(
                ScriptCommandEffect::Stats(ScriptStatsEffect::RegisterUniqueJump),
            ),
            COMMAND_SET_UNIQUE_JUMPS_TOTAL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.pending_effects.push(ScriptCommandEffect::Stats(
                    ScriptStatsEffect::SetUniqueJumpTotal(params[0]),
                ));
            }
            COMMAND_REGISTER_PASSENGER_DROPPED_OFF_TAXI => scripts.pending_effects.push(
                ScriptCommandEffect::Stats(ScriptStatsEffect::RegisterTaxiPassenger),
            ),
            COMMAND_REGISTER_MONEY_MADE_TAXI => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.pending_effects.push(ScriptCommandEffect::Stats(
                    ScriptStatsEffect::AddTaxiMoney(params[0]),
                ));
            }
            COMMAND_REGISTER_MISSION_GIVEN => scripts.pending_effects.push(
                ScriptCommandEffect::Stats(ScriptStatsEffect::RegisterMissionGiven),
            ),
            COMMAND_REGISTER_MISSION_PASSED => {
                let name = scripts.read_key_from_script(&mut ip)?;
                scripts.pending_effects.push(ScriptCommandEffect::Stats(
                    ScriptStatsEffect::RegisterMissionPassed(name),
                ));
            }
            COMMAND_PLAY_MISSION_PASSED_TUNE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PlayMissionPassedTune { tune_id: params[0] });
            }
            COMMAND_LOAD_MISSION_AUDIO => {
                let key = scripts.read_key_from_script(&mut ip)?;
                scripts.mission_audio_loading_status = 0;
                scripts.mission_audio_finished = false;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::LoadMissionAudio { name: key });
            }
            COMMAND_HAS_MISSION_AUDIO_LOADED => {
                self.update_compare_flag(scripts.mission_audio_loading_status == 1);
            }
            COMMAND_PLAY_MISSION_AUDIO => {
                scripts.mission_audio_finished = false;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::PlayMissionAudio);
            }
            COMMAND_HAS_MISSION_AUDIO_FINISHED => {
                self.update_compare_flag(scripts.mission_audio_finished);
            }
            COMMAND_SET_MISSION_AUDIO_POSITION => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetMissionAudioPosition {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                    });
            }
            COMMAND_CLEAR_MISSION_AUDIO => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearMissionAudio);
            }
            COMMAND_SET_RADIO_CHANNEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetRadioChannel {
                        channel: params[0],
                        position: params[1],
                    });
            }
            COMMAND_SET_CHAR_SAY => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharSay {
                        handle: params[0],
                        sound_id: params[1],
                    });
            }
            COMMAND_LOAD_CUTSCENE => {
                let key = scripts.read_key_from_script(&mut ip)?;
                scripts.next_cutscene_object_handle = 0;
                scripts.pending_cutscene_model_snapshots.clear();
                scripts.cutscene_time_ms = 0;
                scripts.cutscene_finished = false;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::LoadCutscene { name: key });
            }
            COMMAND_START_CUTSCENE => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartCutscene);
            }
            COMMAND_CLEAR_CUTSCENE => {
                scripts.next_cutscene_object_handle = 0;
                scripts.pending_cutscene_model_snapshots.clear();
                scripts.cutscene_time_ms = 0;
                scripts.cutscene_finished = false;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearCutscene);
            }
            COMMAND_CREATE_CUTSCENE_OBJECT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let handle = scripts.next_cutscene_object_handle;
                scripts.next_cutscene_object_handle += 1;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateCutsceneObject {
                        handle,
                        model_id: params[0],
                    });
            }
            COMMAND_SET_CUTSCENE_ANIM => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let anim = scripts.read_key_from_script(&mut ip)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCutsceneAnim {
                        object_handle: params[0],
                        anim_name: anim,
                    });
            }
            COMMAND_GET_CUTSCENE_TIME => {
                scripts.script_params[0] = scripts.cutscene_time_ms;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_SET_CUTSCENE_OFFSET => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCutsceneOffset {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                    });
            }
            COMMAND_HAS_CUTSCENE_FINISHED => {
                // re3: CCutsceneMgr::HasCutsceneFinished() ==
                // TheCamera.GetPositionAlongSpline() == 1.0f
                self.update_compare_flag(scripts.cutscene_finished);
            }
            COMMAND_CREATE_CUTSCENE_HEAD => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let handle = scripts.next_cutscene_object_handle;
                scripts.next_cutscene_object_handle += 1;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateCutsceneHead {
                        handle,
                        source_object_handle: params[0],
                        model_id: params[1],
                    });
            }
            COMMAND_SET_CUTSCENE_HEAD_ANIM => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let anim = scripts.read_key_from_script(&mut ip)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCutsceneHeadAnim {
                        object_handle: params[0],
                        anim_name: anim,
                    });
            }
            COMMAND_REQUEST_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let model_id = scripts.resolve_used_object_model(params[0]);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RequestModel { model_id });
            }
            COMMAND_HAS_MODEL_LOADED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let model_id = scripts.resolve_used_object_model(params[0]);
                let loaded = usize::try_from(model_id)
                    .ok()
                    .and_then(|model_id| scripts.model_loaded.get(model_id))
                    .copied()
                    .unwrap_or(false);
                self.update_compare_flag(loaded);
            }
            COMMAND_LOAD_ALL_MODELS_NOW => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::LoadAllModelsNow);
            }
            COMMAND_GRAB_PHONE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let x = Self::script_param_as_f32(params[0]);
                let y = Self::script_param_as_f32(params[1]);
                let phone_id = grab_phone_from_script_mirror(scripts, x, y);
                scripts.script_params[0] = phone_id;
                self.store_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::GrabPhone { x, y });
            }
            COMMAND_SET_REPEATED_PHONE_MESSAGE | COMMAND_SET_PHONE_MESSAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let key = scripts.read_key_from_script(&mut ip)?;
                if let Ok(slot) = usize::try_from(params[0])
                    && let Some(state) = scripts.phone_states.get_mut(slot)
                {
                    *state = if command == COMMAND_SET_REPEATED_PHONE_MESSAGE {
                        5
                    } else {
                        4
                    };
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPhoneMessages {
                        phone_id: params[0],
                        keys: [
                            key,
                            [0; KEY_LENGTH_IN_SCRIPT],
                            [0; KEY_LENGTH_IN_SCRIPT],
                            [0; KEY_LENGTH_IN_SCRIPT],
                            [0; KEY_LENGTH_IN_SCRIPT],
                            [0; KEY_LENGTH_IN_SCRIPT],
                        ],
                        count: 1,
                        repeated: command == COMMAND_SET_REPEATED_PHONE_MESSAGE,
                    });
            }
            COMMAND_SET_2_REPEATED_PHONE_MESSAGES
            | COMMAND_SET_2_PHONE_MESSAGES
            | COMMAND_SET_3_REPEATED_PHONE_MESSAGES
            | COMMAND_SET_3_PHONE_MESSAGES
            | COMMAND_SET_4_REPEATED_PHONE_MESSAGES
            | COMMAND_SET_4_PHONE_MESSAGES
            | COMMAND_SET_5_REPEATED_PHONE_MESSAGES
            | COMMAND_SET_5_PHONE_MESSAGES
            | COMMAND_SET_6_REPEATED_PHONE_MESSAGES
            | COMMAND_SET_6_PHONE_MESSAGES => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let (count, repeated) = match command {
                    COMMAND_SET_2_REPEATED_PHONE_MESSAGES => (2, true),
                    COMMAND_SET_2_PHONE_MESSAGES => (2, false),
                    COMMAND_SET_3_REPEATED_PHONE_MESSAGES => (3, true),
                    COMMAND_SET_3_PHONE_MESSAGES => (3, false),
                    COMMAND_SET_4_REPEATED_PHONE_MESSAGES => (4, true),
                    COMMAND_SET_4_PHONE_MESSAGES => (4, false),
                    COMMAND_SET_5_REPEATED_PHONE_MESSAGES => (5, true),
                    COMMAND_SET_5_PHONE_MESSAGES => (5, false),
                    COMMAND_SET_6_REPEATED_PHONE_MESSAGES => (6, true),
                    _ => (6, false),
                };
                let mut keys = [[0; KEY_LENGTH_IN_SCRIPT]; 6];
                for slot in keys.iter_mut().take(count) {
                    *slot = scripts.read_key_from_script(&mut ip)?;
                }
                if let Ok(slot) = usize::try_from(params[0])
                    && let Some(state) = scripts.phone_states.get_mut(slot)
                {
                    *state = if repeated { 5 } else { 4 };
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPhoneMessages {
                        phone_id: params[0],
                        keys,
                        count: count as u8,
                        repeated,
                    });
            }
            COMMAND_HAS_PHONE_DISPLAYED_MESSAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let displayed = usize::try_from(params[0])
                    .ok()
                    .and_then(|slot| scripts.phone_states.get(slot))
                    .is_some_and(|state| {
                        !scripts.phone_displaying_message && matches!(*state, 6 | 7 | 8)
                    });
                self.update_compare_flag(displayed);
            }
            COMMAND_IS_PHONE_DISPLAYING_MESSAGE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.update_compare_flag(scripts.phone_displaying_message_slot == params[0]);
            }
            COMMAND_TURN_PHONE_OFF => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if let Ok(slot) = usize::try_from(params[0])
                    && let Some(state) = scripts.phone_states.get_mut(slot)
                {
                    *state = 3;
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::TurnPhoneOff {
                        phone_id: params[0],
                    });
            }
            COMMAND_TURN_PHONE_ON => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if let Ok(slot) = usize::try_from(params[0])
                    && let Some(state) = scripts.phone_states.get_mut(slot)
                {
                    *state = 9;
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::TurnPhoneOn {
                        phone_id: params[0],
                    });
            }
            COMMAND_IS_PLAYER_LIFTING_A_PHONE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let lifting_phone = player_probe
                    .as_deref_mut()
                    .and_then(|probe| probe(params[0]))
                    .is_some_and(|player| player.is_making_call);
                self.update_compare_flag(lifting_phone);
            }
            COMMAND_DO_FADE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                // The C++ VM executes `TheCamera.Fade` before the next SCM
                // opcode is evaluated. Our render-side effect is applied
                // after the VM pass, so keep the query mirror in sync here;
                // otherwise an immediate `GET_FADING_STATUS` observes the
                // previous frame and skips the authored white-flash wait.
                // `CCamera::Fade` sets m_bFading even for a zero timeout;
                // `ProcessFade` clears it on the camera pass that follows.
                scripts.camera_fading = true;
                scripts.pending_effects.push(ScriptCommandEffect::DoFade {
                    time_ms: params[0],
                    direction: params[1],
                });
            }
            COMMAND_SET_FADING_COLOUR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetFadingColour {
                        r: params[0],
                        g: params[1],
                        b: params[2],
                    });
            }
            COMMAND_ADD_AMMO_TO_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddAmmoToPlayer {
                        player_index: params[0],
                        weapon_type: params[1],
                        ammo: params[2],
                    });
            }
            COMMAND_ADD_AMMO_TO_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddAmmoToChar {
                        handle: params[0],
                        weapon_type: params[1],
                        ammo: params[2],
                    });
            }
            COMMAND_ADD_AMMO_TO_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddAmmoToCar {
                        handle: params[0],
                        ammo: params[2],
                    });
            }
            COMMAND_SET_PLAYER_AMMO => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerAmmo {
                        player_index: params[0],
                        weapon_type: params[1],
                        ammo: params[2],
                    });
            }
            COMMAND_SET_CHAR_AMMO => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharAmmo {
                        handle: params[0],
                        weapon_type: params[1],
                        ammo: params[2],
                    });
            }
            COMMAND_SET_CAR_AMMO => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarAmmo {
                        handle: params[0],
                        ammo: params[2],
                    });
            }
            COMMAND_SET_CURRENT_PLAYER_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCurrentPlayerWeapon {
                        player_index: params[0],
                        weapon_type: params[1],
                    });
            }
            COMMAND_SET_CURRENT_CHAR_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCurrentCharWeapon {
                        handle: params[0],
                        weapon_type: params[1],
                    });
            }
            COMMAND_GET_AMMO_IN_PLAYER_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                let slot = params[1].max(0) as usize;
                scripts.script_params[0] = snapshot.weapon_ammo.get(slot).copied().unwrap_or(0);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_AMMO_IN_CHAR_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                let slot = params[1].max(0) as usize;
                scripts.script_params[0] = snapshot.weapon_ammo.get(slot).copied().unwrap_or(0);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_CURRENT_PLAYER_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.current_weapon == params[1]);
            }
            COMMAND_IS_CURRENT_CHAR_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.current_weapon == params[1]);
            }
            COMMAND_GET_CURRENT_PLAYER_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                scripts.script_params[0] = snapshot.current_weapon;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_CURRENT_CHAR_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts.script_params[0] = snapshot.current_weapon;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GIVE_WEAPON_TO_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::GiveWeaponToPlayer {
                        player_index: params[0],
                        weapon_type: params[1],
                        ammo: params[2],
                    });
            }
            COMMAND_GIVE_WEAPON_TO_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::GiveWeaponToChar {
                        handle: params[0],
                        weapon_type: params[1],
                        ammo: params[2],
                    });
            }
            COMMAND_GIVE_WEAPON_TO_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddAmmoToCar {
                        handle: params[0],
                        ammo: params[2],
                    });
            }
            COMMAND_SET_CHAR_ONLY_DAMAGED_BY_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharOnlyDamagedByPlayer {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CHAR_BLEEDING => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharBleeding {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CHAR_RUNNING => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharRunning {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CHAR_STAY_IN_SAME_PLACE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharStayInSamePlace {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CHAR_HEED_THREATS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharRespondsToThreats {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_PLAYER_HEED_THREATS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerRespondsToThreats {
                        player_index: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_PLAYER_VISIBLE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerVisible {
                        player_index: params[0],
                        visible: params[1] != 0,
                    });
            }
            COMMAND_SET_CHAR_VISIBLE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharVisible {
                        handle: params[0],
                        visible: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_ONLY_DAMAGED_BY_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarOnlyDamagedByPlayer {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_CAN_BE_DAMAGED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarCanBeDamaged {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_CHANGE_CAR_LOCK | COMMAND_LOCK_CAR_DOORS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarLock {
                        handle: params[0],
                        lock: params[1],
                    });
            }
            COMMAND_CHANGE_CAR_COLOUR => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarColour {
                        handle: params[0],
                        colour1: params[1],
                        colour2: params[2],
                    });
            }
            // re3 Script2/6.cpp heading commands.
            COMMAND_GET_PLAYER_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let player_index = params[0].max(0) as usize;
                let handle = scripts
                    .player_char_handles
                    .get(player_index)
                    .copied()
                    .filter(|handle| *handle != 0)
                    .unwrap_or(params[0].saturating_add(1));
                let player = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(handle)
                    .ok_or(ScriptVmError::MissingChar(handle))?;
                let heading = if let Some(vehicle_handle) = player.current_vehicle {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(vehicle_handle)
                        .ok_or(ScriptVmError::MissingVehicle(vehicle_handle))?
                        .heading_degrees
                } else {
                    player.heading_degrees
                };
                scripts.script_params[0] = Self::f32_to_script_param(
                    re3bevy_core::core::General::CGeneral::limit_angle(heading),
                );
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_SET_CHAR_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharHeading {
                        handle: params[0],
                        heading_degrees: Self::script_param_as_f32(params[1]),
                    });
            }
            COMMAND_TURN_CHAR_TO_FACE_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                if snapshot.current_vehicle.is_none() {
                    let heading = face_coord_heading_degrees(
                        snapshot.position,
                        Self::script_param_as_f32(params[1]),
                        Self::script_param_as_f32(params[2]),
                    );
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetCharHeading {
                            handle: params[0],
                            heading_degrees: heading,
                        });
                }
            }
            COMMAND_TURN_PLAYER_TO_FACE_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                if snapshot.current_vehicle.is_none() {
                    let heading = face_coord_heading_degrees(
                        snapshot.position,
                        Self::script_param_as_f32(params[1]),
                        Self::script_param_as_f32(params[2]),
                    );
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetPlayerHeading {
                            player_index: params[0],
                            heading_degrees: heading,
                        });
                }
            }
            COMMAND_TURN_CHAR_TO_FACE_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let source = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                let target = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[1])
                    .ok_or(ScriptVmError::MissingChar(params[1]))?;
                let source_position = if let Some(vehicle_handle) = source.current_vehicle {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(vehicle_handle)
                        .ok_or(ScriptVmError::MissingVehicle(vehicle_handle))?
                        .position
                } else {
                    source.position
                };
                let target_position = if let Some(vehicle_handle) = target.current_vehicle {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(vehicle_handle)
                        .ok_or(ScriptVmError::MissingVehicle(vehicle_handle))?
                        .position
                } else {
                    target.position
                };
                if source.current_vehicle.is_none() {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetCharHeading {
                            handle: params[0],
                            heading_degrees: face_coord_heading_degrees(
                                source_position,
                                target_position.x,
                                target_position.y,
                            ),
                        });
                }
            }
            COMMAND_TURN_CHAR_TO_FACE_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let source = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                let target =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[1])
                    .ok_or(ScriptVmError::MissingPlayer(params[1]))?;
                let source_position = if let Some(vehicle_handle) = source.current_vehicle {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(vehicle_handle)
                        .ok_or(ScriptVmError::MissingVehicle(vehicle_handle))?
                        .position
                } else {
                    source.position
                };
                let target_position = if let Some(vehicle_handle) = target.current_vehicle {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(vehicle_handle)
                        .ok_or(ScriptVmError::MissingVehicle(vehicle_handle))?
                        .position
                } else {
                    target.position
                };
                if source.current_vehicle.is_none() {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetCharHeading {
                            handle: params[0],
                            heading_degrees: face_coord_heading_degrees(
                                source_position,
                                target_position.x,
                                target_position.y,
                            ),
                        });
                }
            }
            COMMAND_TURN_PLAYER_TO_FACE_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let source =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                let target = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[1])
                    .ok_or(ScriptVmError::MissingChar(params[1]))?;
                let source_position = if let Some(vehicle_handle) = source.current_vehicle {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(vehicle_handle)
                        .ok_or(ScriptVmError::MissingVehicle(vehicle_handle))?
                        .position
                } else {
                    source.position
                };
                let target_position = if let Some(vehicle_handle) = target.current_vehicle {
                    entity_runtime
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                        .vehicle_snapshot(vehicle_handle)
                        .ok_or(ScriptVmError::MissingVehicle(vehicle_handle))?
                        .position
                } else {
                    target.position
                };
                if source.current_vehicle.is_none() {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetPlayerHeading {
                            player_index: params[0],
                            heading_degrees: face_coord_heading_degrees(
                                source_position,
                                target_position.x,
                                target_position.y,
                            ),
                        });
                }
            }
            COMMAND_SET_CAR_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarHeading {
                        handle: params[0],
                        heading_degrees: Self::script_param_as_f32(params[1]),
                    });
            }
            COMMAND_TURN_CAR_TO_FACE_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::TurnCarToFaceCoord {
                        handle: params[0],
                        x: Self::script_param_as_f32(params[1]),
                        y: Self::script_param_as_f32(params[2]),
                    });
            }
            COMMAND_GET_CHAR_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts.script_params[0] = Self::f32_to_script_param(
                    re3bevy_core::core::General::CGeneral::limit_angle(snapshot.heading_degrees),
                );
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_CAR_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[0] = Self::f32_to_script_param(
                    re3bevy_core::core::General::CGeneral::limit_angle(snapshot.heading_degrees),
                );
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_OBJECT_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let heading = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.object_snapshot(params[0]))
                    .map(|snapshot| {
                        re3bevy_core::core::General::CGeneral::limit_angle(snapshot.heading_degrees)
                    })
                    .unwrap_or(0.0);
                scripts.script_params[0] = Self::f32_to_script_param(heading);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_SET_OBJECT_HEADING => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetObjectHeading {
                        handle: params[0],
                        heading_degrees: Self::script_param_as_f32(params[1]),
                    });
            }
            COMMAND_SET_CAR_VISIBLE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarVisible {
                        handle: params[0],
                        visible: params[1] != 0,
                    });
            }
            COMMAND_REMOVE_ROUTE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveRoute { route: params[0] });
            }
            COMMAND_SET_CAR_STRONG => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarStrong {
                        handle: params[0],
                        strong: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_BIG_WHEELS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarBigWheels {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_HEAVY => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarHeavy {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CAN_RESPRAY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarCanRespray {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_COMEDY_CONTROLS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarComedyControls {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_ANCHOR_BOAT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AnchorBoat {
                        handle: params[0],
                        anchored: params[1] != 0,
                    });
            }
            COMMAND_SWITCH_CAR_SIREN => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwitchCarSiren {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_IS_CAR_CRUSHED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.update_compare_flag(scripts.crushed_car_id == params[0]);
            }
            // re3 Script2.cpp COMMAND_CREATE_CAR_GENERATOR — 12 in, store generator index.
            COMMAND_CREATE_CAR_GENERATOR => {
                let params = self.collect_parameters(scripts, &mut ip, 12)?;
                // re3 returns NumOfCarGenerators before increment; provisional index
                // matches apply order (create effects append sequentially).
                let provisional = scripts.car_generators_created;
                scripts.car_generators_created = scripts.car_generators_created.saturating_add(1);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateCarGenerator {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        angle_degrees: Self::script_param_as_f32(params[3]),
                        model_index: params[4],
                        color1: params[5] as i16,
                        color2: params[6] as i16,
                        force_spawn: params[7].clamp(0, 255) as u8,
                        alarm: params[8].clamp(0, 255) as u8,
                        doorlock: params[9].clamp(0, 255) as u8,
                        min_delay: params[10].max(0) as u16,
                        max_delay: params[11].max(0) as u16,
                    });
                scripts.script_params[0] = provisional;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            // re3 Script2.cpp COMMAND_SWITCH_CAR_GENERATOR
            COMMAND_SWITCH_CAR_GENERATOR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwitchCarGenerator {
                        index: params[0],
                        count: params[1],
                    });
            }
            // re3 Script4.cpp COMMAND_SWAP_NEAREST_BUILDING_MODEL — IPL model swap.
            COMMAND_SWAP_NEAREST_BUILDING_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let old_model = scripts.resolve_used_object_model(params[4]);
                let new_model = scripts.resolve_used_object_model(params[5]);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwapNearestBuildingModel {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        radius: Self::script_param_as_f32(params[3]),
                        old_model,
                        new_model,
                    });
            }
            // re3 Script4.cpp COMMAND_SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE
            COMMAND_SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let model_id = scripts.resolve_used_object_model(params[4]);
                scripts.pending_effects.push(
                    ScriptCommandEffect::SetVisibilityOfClosestObjectOfType {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        radius: Self::script_param_as_f32(params[3]),
                        model_id,
                        visible: params[5] != 0,
                    },
                );
            }
            COMMAND_SET_CAR_PROOFS => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarProofs {
                        handle: params[0],
                        bullet: params[1] != 0,
                        fire: params[2] != 0,
                        explosion: params[3] != 0,
                        collision: params[4] != 0,
                        melee: params[5] != 0,
                    });
            }
            COMMAND_SET_ALL_CARS_CAN_BE_DAMAGED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetAllCarsCanBeDamaged(params[0] != 0));
            }
            COMMAND_SET_UPSIDEDOWN_CAR_NOT_DAMAGED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetUpsideDownCarNotDamaged {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_PLAYER_HEALTH => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerHealth {
                        player_index: params[0],
                        health: params[1] as f32,
                    });
            }
            COMMAND_SET_CHAR_HEALTH => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharHealth {
                        handle: params[0],
                        health: params[1] as f32,
                    });
            }
            COMMAND_SET_CHAR_ACCURACY => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharAccuracy {
                        handle: params[0],
                        accuracy: params[1],
                    });
            }
            COMMAND_SET_CAR_HEALTH => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarHealth {
                        handle: params[0],
                        health: params[1] as f32,
                    });
            }
            COMMAND_GET_PLAYER_HEALTH => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                scripts.script_params[0] = snapshot.health as i32;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_CHAR_HEALTH => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts.script_params[0] = snapshot.health as i32;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_CAR_HEALTH => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[0] = snapshot.health as i32;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_PLAYER_HEALTH_GREATER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.health > params[1] as f32);
            }
            COMMAND_IS_CHAR_HEALTH_GREATER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.health > params[1] as f32);
            }
            COMMAND_IS_CAR_HEALTH_GREATER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(snapshot.health > params[1] as f32);
            }
            COMMAND_ADD_ARMOUR_TO_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddArmourToPlayer {
                        player_index: params[0],
                        armour: params[1] as f32,
                    });
            }
            COMMAND_ADD_ARMOUR_TO_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddArmourToChar {
                        handle: params[0],
                        armour: params[1] as f32,
                    });
            }
            COMMAND_SET_PLAYER_CONTROL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerControl {
                        player_index: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_PLAYER_NEVER_GETS_TIRED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerNeverGetsTired {
                        player_index: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_PLAYER_FAST_RELOAD => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetPlayerFastReload {
                        player_index: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_GET_OUT_OF_JAIL_FREE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetGetOutOfJailFree {
                        player_index: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_FREE_HEALTH_CARE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetFreeHealthCare {
                        player_index: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SWITCH_WIDESCREEN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwitchWidescreen {
                        enabled: params[0] != 0,
                    });
            }
            COMMAND_LOAD_SCENE => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::LoadScene {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                    });
            }
            COMMAND_SET_NEAR_CLIP => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetNearClip(Self::script_param_as_f32(
                        params[0],
                    )));
            }
            COMMAND_DRAW_LIGHT => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DrawLight {
                        position: CVector::new(
                            Self::script_param_as_f32(params[0]),
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                        ),
                        color: [params[3] as u8, params[4] as u8, params[5] as u8],
                    });
            }
            COMMAND_DRAW_CORONA => {
                let params = self.collect_parameters(scripts, &mut ip, 9)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DrawCorona {
                        id: (self as *const Self as usize).wrapping_add(ip as usize) as u32,
                        position,
                        size: Self::script_param_as_f32(params[3]),
                        texture_type: params[4],
                        flare_type: params[5],
                        colour: [params[6] as u8, params[7] as u8, params[8] as u8],
                    });
            }
            COMMAND_START_CHASE_SCENE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartChaseScene(
                        Self::script_param_as_f32(params[0]),
                    ));
            }
            COMMAND_STOP_CHASE_SCENE => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StopChaseScene);
            }
            COMMAND_REMOVE_CAR_FROM_CHASE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveCarFromChase(params[0]));
            }
            COMMAND_DISPLAY_TEXT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let position = [
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                ];
                let text_key = scripts.read_key_from_script(&mut ip)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].position = position;
                scripts.intro_text_lines[index].text_key = text_key;
                scripts.number_of_intro_text_lines_this_frame = scripts
                    .number_of_intro_text_lines_this_frame
                    .saturating_add(1);
            }
            COMMAND_SET_TEXT_SCALE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].scale = [
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                ];
            }
            COMMAND_SET_TEXT_COLOUR => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].color = [
                    params[0] as u8,
                    params[1] as u8,
                    params[2] as u8,
                    params[3] as u8,
                ];
            }
            COMMAND_SET_TEXT_JUSTIFY => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].justify = params[0] != 0;
            }
            COMMAND_SET_TEXT_CENTRE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].centered = params[0] != 0;
            }
            COMMAND_SET_TEXT_WRAPX => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].wrap_x = Self::script_param_as_f32(params[0]);
            }
            COMMAND_SET_TEXT_CENTRE_SIZE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].center_size = Self::script_param_as_f32(params[0]);
            }
            COMMAND_SET_TEXT_BACKGROUND => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].background = params[0] != 0;
            }
            COMMAND_SET_TEXT_BACKGROUND_COLOUR => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].background_color = [
                    params[0] as u8,
                    params[1] as u8,
                    params[2] as u8,
                    params[3] as u8,
                ];
            }
            COMMAND_SET_TEXT_BACKGROUND_ONLY_TEXT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].background_only = params[0] != 0;
            }
            COMMAND_SET_TEXT_PROPORTIONAL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].proportional = params[0] != 0;
            }
            COMMAND_SET_TEXT_FONT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                scripts.intro_text_lines[index].font = params[0];
            }
            COMMAND_SET_TEXT_DRAW_BEFORE_FADE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                if let Some(line) = scripts.intro_text_lines.get_mut(index) {
                    line.before_fade = params[0] != 0;
                }
            }
            COMMAND_SET_TEXT_RIGHT_JUSTIFY => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_text_lines_this_frame as usize;
                if let Some(line) = scripts.intro_text_lines.get_mut(index) {
                    line.right_justify = params[0] != 0;
                }
            }
            COMMAND_SET_SPRITES_DRAW_BEFORE_FADE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let index = scripts.number_of_intro_rectangles_this_frame as usize;
                if let Some(rect) = scripts.intro_rectangles.get_mut(index) {
                    rect.before_fade = params[0] != 0;
                }
            }
            COMMAND_DRAW_RECT => {
                let params = self.collect_parameters(scripts, &mut ip, 8)?;
                let index = scripts.number_of_intro_rectangles_this_frame as usize;
                if let Some(rect) = scripts.intro_rectangles.get_mut(index) {
                    let x = Self::script_param_as_f32(params[0]);
                    let y = Self::script_param_as_f32(params[1]);
                    let width = Self::script_param_as_f32(params[2]);
                    let height = Self::script_param_as_f32(params[3]);
                    rect.is_used = true;
                    rect.texture_id = -1;
                    rect.rect = [x, y, x + width, y + height];
                    rect.color = [
                        params[4] as u8,
                        params[5] as u8,
                        params[6] as u8,
                        params[7] as u8,
                    ];
                    scripts.number_of_intro_rectangles_this_frame = scripts
                        .number_of_intro_rectangles_this_frame
                        .saturating_add(1);
                }
            }
            COMMAND_DRAW_SPRITE => {
                let params = self.collect_parameters(scripts, &mut ip, 9)?;
                let index = scripts.number_of_intro_rectangles_this_frame as usize;
                if let Some(rect) = scripts.intro_rectangles.get_mut(index) {
                    let x = Self::script_param_as_f32(params[1]);
                    let y = Self::script_param_as_f32(params[2]);
                    let width = Self::script_param_as_f32(params[3]);
                    let height = Self::script_param_as_f32(params[4]);
                    rect.is_used = true;
                    rect.texture_id = params[0] - 1;
                    rect.rect = [x, y, x + width, y + height];
                    rect.color = [
                        params[5] as u8,
                        params[6] as u8,
                        params[7] as u8,
                        params[8] as u8,
                    ];
                    scripts.number_of_intro_rectangles_this_frame = scripts
                        .number_of_intro_rectangles_this_frame
                        .saturating_add(1);
                }
            }
            COMMAND_LOAD_SPRITE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let mut name = scripts.read_key_from_script(&mut ip)?;
                name.make_ascii_lowercase();
                let slot = params[0].saturating_sub(1);
                if let Ok(slot) = usize::try_from(slot)
                    && let Some(target) = scripts.script_sprite_names.get_mut(slot)
                {
                    *target = name;
                }
            }
            COMMAND_LOAD_TEXTURE_DICTIONARY => {
                scripts.script_texture_dictionary = Some(scripts.read_key_from_script(&mut ip)?);
            }
            COMMAND_REMOVE_TEXTURE_DICTIONARY => {
                scripts.script_texture_dictionary = None;
                scripts.script_sprite_names = [[0; KEY_LENGTH_IN_SCRIPT]; MAX_NUM_SCRIPT_SPRITES];
            }
            COMMAND_USE_TEXT_COMMANDS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.use_text_commands = if params[0] != 0 { 2 } else { 1 };
            }
            COMMAND_CLEAR_SMALL_PRINTS => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ClearSmallPrints);
            }
            COMMAND_ACTIVATE_SAVE_MENU => {
                // re3 Script4.cpp COMMAND_ACTIVATE_SAVE_MENU:
                //   FrontEndMenuManager.m_bSaveMenuActive = true;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ActivateSaveMenu);
            }
            COMMAND_HAS_SAVE_GAME_FINISHED => {
                // re3 Script4.cpp COMMAND_HAS_SAVE_GAME_FINISHED:
                //   UpdateCompareFlag(!FrontEndMenuManager.m_bMenuActive);
                let snapshot = world_state_probe
                    .as_deref_mut()
                    .map(|probe| probe())
                    .ok_or(ScriptVmError::MissingWorldStateProbe)?;
                self.update_compare_flag(!snapshot.menu_active);
            }
            COMMAND_HAS_MILITARY_CRANE_COLLECTED_ALL_CARS => {
                // re3 Script6.cpp COMMAND_HAS_MILITARY_CRANE_COLLECTED_ALL_CARS:
                //   UpdateCompareFlag(
                //     CCranes::HaveAllCarsBeenCollectedByMilitaryCrane());
                let snapshot = world_state_probe
                    .as_deref_mut()
                    .map(|probe| probe())
                    .ok_or(ScriptVmError::MissingWorldStateProbe)?;
                self.update_compare_flag(snapshot.military_crane_all_cars_collected);
            }
            COMMAND_ADD_PAGER_MESSAGE => {
                let key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddPagerMessage {
                        key,
                        speed: params[0] as u16,
                        priority: params[1] as u16,
                        unused: params[2] as u16,
                        numbers: None,
                    });
            }
            COMMAND_ADD_PAGER_MESSAGE_WITH_NUMBER => {
                let key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddPagerMessage {
                        key,
                        speed: params[1] as u16,
                        priority: params[2] as u16,
                        unused: params[3] as u16,
                        numbers: Some([params[0], -1, -1, -1, -1, -1]),
                    });
            }
            COMMAND_START_BOAT_FOAM_ANIMATION => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartBoatFoamAnimation);
            }
            COMMAND_UPDATE_BOAT_FOAM_ANIMATION => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::UpdateBoatFoamAnimation { handle: params[0] });
            }
            COMMAND_FLASH_HUD_OBJECT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::FlashHudObject(params[0]));
            }
            COMMAND_LOAD_SPLASH_SCREEN => {
                let bytes = scripts.read_key_from_script(&mut ip)?.to_ascii_lowercase();
                let mut name = [0_u8; 8];
                name.copy_from_slice(&bytes);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::LoadSplashScreen(name));
            }
            COMMAND_ADD_PARTICLE_EFFECT => {
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddMovingParticleEffect {
                        object_type: params[0],
                        position,
                        target: CVector::ZERO,
                        size: 0.0,
                        life_time: 0,
                        color: [0; 4],
                        remove: params[4] != 0,
                    });
            }
            COMMAND_ADD_MOVING_PARTICLE_EFFECT => {
                let params = self.collect_parameters(scripts, &mut ip, 12)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                let color = if params[0] == 14 {
                    [params[8] as u8, params[9] as u8, params[10] as u8, u8::MAX]
                } else {
                    [0; 4]
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::AddMovingParticleEffect {
                        object_type: params[0],
                        position,
                        target: CVector::new(
                            Self::script_param_as_f32(params[4]),
                            Self::script_param_as_f32(params[5]),
                            Self::script_param_as_f32(params[6]),
                        ),
                        size: Self::script_param_as_f32(params[7]).max(0.0),
                        // re3 passes ScriptParams[11] as CParticleObject::AddObject
                        // `lifeTime`; the remove flag is the fixed final `1`.
                        life_time: params[11] as u32,
                        color,
                        remove: true,
                    });
            }
            COMMAND_SET_MUSIC_DOES_FADE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetMusicDoesFade(params[0] != 0));
            }
            COMMAND_SET_INTRO_IS_PLAYING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetIntroIsPlaying {
                        playing: params[0] != 0,
                    });
            }
            COMMAND_DEBUG_ON => {
                scripts.dbg_flag = true;
            }
            COMMAND_DEBUG_OFF => {
                scripts.dbg_flag = false;
            }
            COMMAND_RETURN_TRUE => {
                self.update_compare_flag(true);
            }
            COMMAND_RETURN_FALSE => {
                self.update_compare_flag(false);
            }
            COMMAND_ANDOR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let state = params[0] as u16;
                self.and_or_state = state;
                if state == ANDOR_NONE {
                    self.cond_result = false;
                } else if (ANDS_1..=ANDS_8).contains(&state) {
                    self.cond_result = true;
                    self.and_or_state = self.and_or_state.saturating_add(1);
                } else if (ORS_1..=ORS_8).contains(&state) {
                    self.cond_result = false;
                    self.and_or_state = self.and_or_state.saturating_add(1);
                } else {
                    return Err(ScriptVmError::InvalidAndOrState(state));
                }
            }
            COMMAND_LOAD_AND_LAUNCH_MISSION => {
                // re3 Script6.cpp: public command is a no-op on this PC path.
            }
            COMMAND_LOAD_AND_LAUNCH_MISSION_INTERNAL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let mission_script = scripts.load_and_launch_mission_internal(params[0])?;
                scripts.scripts[mission_script].is_mission_script = true;
                scripts.scripts[mission_script].mission_flag = true;
            }
            COMMAND_LAUNCH_MISSION => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                if params[0] < 0 {
                    return Err(ScriptVmError::IpOutOfRange);
                }
                let mission_script = scripts.start_new_script(params[0] as u32)?;
                scripts.scripts[mission_script].is_mission_script = true;
            }
            COMMAND_MISSION_HAS_FINISHED => {
                if self.is_mission_script {
                    scripts.queue_mission_cleanup_effects();
                }
            }
            COMMAND_STORE_CAR_CHAR_IS_IN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                let vehicle_handle = snapshot
                    .current_vehicle
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.store_vehicle_index = vehicle_handle;
                scripts.script_params[0] = scripts.store_vehicle_index;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StoreCarCharIsIn {
                        char_handle: params[0],
                        vehicle_handle,
                    });
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_STORE_CAR_PLAYER_IS_IN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                if let Some(vehicle_handle) = snapshot.current_vehicle {
                    scripts.store_vehicle_index = vehicle_handle;
                    scripts.script_params[0] = scripts.store_vehicle_index;
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::StoreCarPlayerIsIn {
                            player_index: params[0],
                            vehicle_handle,
                        });
                    self.store_parameters(scripts, &mut ip, 1)?;
                }
                else {
                    // Script.cpp:2498 leaves the output value unchanged on foot.
                    // Consume its address nevertheless: returning before it is
                    // consumed corrupts the instruction stream (native MAYHEM).
                    let mut marker_ip = ip;
                    let variable_kind = match scripts.read_1_byte_from_script(&mut marker_ip)? {
                        crate::control::ScriptTypes::ARGUMENT_GLOBALVAR => VAR_GLOBAL,
                        crate::control::ScriptTypes::ARGUMENT_LOCALVAR => VAR_LOCAL,
                        argument => return Err(ScriptVmError::InvalidArgumentType(argument)),
                    };
                    self.get_pointer_to_script_variable(scripts, &mut ip, variable_kind)?;
                }
            }
            COMMAND_STORE_CAR_CHAR_IS_IN_NO_SAVE => {
                // re3 Script4.cpp:1672-1680. Unlike STORE_CAR_CHAR_IS_IN,
                // this only returns the current pool handle; it does not
                // transfer vehicle ownership into mission cleanup state.
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts.script_params[0] = snapshot.current_vehicle.unwrap_or(-1);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_STORE_CAR_PLAYER_IS_IN_NO_SAVE => {
                // re3 Script4.cpp:1682-1690. GetIndex(nil) yields the invalid
                // handle, but the output variable is still always consumed.
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                scripts.script_params[0] = snapshot.current_vehicle.unwrap_or(-1);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_CHAR_IN_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle == Some(params[1]));
            }
            COMMAND_IS_PLAYER_IN_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle == Some(params[1]));
            }
            COMMAND_IS_CHAR_IN_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle_model == Some(params[1]));
            }
            COMMAND_IS_PLAYER_IN_MODEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle_model == Some(params[1]));
            }
            COMMAND_IS_CHAR_IN_ANY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle.is_some());
            }
            COMMAND_IS_PLAYER_IN_ANY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle.is_some());
            }
            COMMAND_IS_PLAYER_SITTING_IN_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(
                    snapshot.is_driving && snapshot.current_vehicle == Some(params[1]),
                );
            }
            COMMAND_IS_PLAYER_SITTING_IN_ANY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.is_driving);
            }
            COMMAND_IS_CHAR_SITTING_IN_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle == Some(params[1]));
            }
            COMMAND_IS_CHAR_SITTING_IN_ANY_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle.is_some());
            }
            COMMAND_IS_PLAYER_ON_FOOT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle.is_none());
            }
            COMMAND_IS_CHAR_ON_FOOT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let snapshot = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(snapshot.current_vehicle.is_none());
            }
            COMMAND_IS_BUTTON_PRESSED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let pad_probe = pad_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingPadProbe)?;
                self.update_compare_flag(pad_probe(params[0], params[1]) != 0);
            }
            COMMAND_GET_PAD_STATE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let button = scripts.script_params[1];
                let pad_probe = pad_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingPadProbe)?;
                scripts.script_params[0] = i32::from(pad_probe(params[0], button));
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GIVE_REMOTE_CONTROLLED_CAR_TO_PLAYER
            | COMMAND_GIVE_REMOTE_CONTROLLED_MODEL_TO_PLAYER => {
                let count = if command == COMMAND_GIVE_REMOTE_CONTROLLED_CAR_TO_PLAYER {
                    5
                } else {
                    6
                };
                let params = self.collect_parameters(scripts, &mut ip, count)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::GiveRemoteControlledCar {
                        player_index: params[0],
                        position,
                        rotation_radians: Self::script_param_as_f32(params[4]).to_radians(),
                        model_id: if count == 6 {
                            params[5]
                        } else {
                            MI_RCBANDIT as i32
                        },
                    });
            }
            COMMAND_ADD_HOSPITAL_RESTART
            | COMMAND_ADD_POLICE_RESTART
            | COMMAND_OVERRIDE_NEXT_RESTART
            | COMMAND_RESTART_CRITICAL_MISSION => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let heading = Self::script_param_as_f32(params[3]);
                let needs_ground = position.z <= MAP_Z_LOW_LIMIT;
                if matches!(
                    command,
                    COMMAND_OVERRIDE_NEXT_RESTART | COMMAND_RESTART_CRITICAL_MISSION
                ) {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::OverrideNextRestart {
                            position,
                            heading,
                            needs_ground,
                        });
                    if command == COMMAND_RESTART_CRITICAL_MISSION {
                        scripts
                            .pending_effects
                            .push(ScriptCommandEffect::PlayerFailedCriticalMission);
                    }
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::AddRestartPoint {
                            police: command == COMMAND_ADD_POLICE_RESTART,
                            position,
                            heading,
                            needs_ground,
                        });
                }
            }
            COMMAND_SWITCH_TAXI_TIMER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwitchTaxiTimer {
                        enabled: params[0] != 0,
                        time_ms,
                    });
            }
            COMMAND_CANCEL_OVERRIDE_RESTART => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CancelOverrideRestart);
            }
            COMMAND_OVERRIDE_HOSPITAL_LEVEL | COMMAND_OVERRIDE_POLICE_STATION_LEVEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(if command == COMMAND_OVERRIDE_HOSPITAL_LEVEL {
                        ScriptCommandEffect::OverrideHospitalLevel(params[0])
                    } else {
                        ScriptCommandEffect::OverridePoliceLevel(params[0])
                    });
            }
            COMMAND_START_KILL_FRENZY | COMMAND_START_KILL_FRENZY_HEADSHOT => {
                let text_key = scripts.read_key_from_script(&mut ip)?;
                let params = self.collect_parameters(scripts, &mut ip, 8)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartKillFrenzy {
                        weapon_type: params[0],
                        time_ms: params[1],
                        kills: params[2] as u16,
                        models: [params[3], params[4], params[5], params[6]],
                        text_key,
                        standard_sound: params[7] != 0,
                        need_head_shot: command == COMMAND_START_KILL_FRENZY_HEADSHOT,
                    });
            }
            COMMAND_READ_KILL_FRENZY_STATUS => {
                scripts.script_params[0] = i32::from(scripts.kill_frenzy_status);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_REGISTER_KILL_FRENZY_PASSED => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterKillFrenzyPassed);
            }
            COMMAND_RESET_NUM_OF_MODELS_KILLED_BY_PLAYER => {
                scripts.models_killed_by_player.fill(0);
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ResetModelsKilledByPlayer);
            }
            COMMAND_GET_NUM_OF_MODELS_KILLED_BY_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.script_params[0] = usize::try_from(params[0])
                    .ok()
                    .and_then(|index| scripts.models_killed_by_player.get(index).copied())
                    .map(i32::from)
                    .unwrap_or(0);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_SQRT | COMMAND_SIN | COMMAND_COS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let value = Self::script_param_as_f32(params[0]);
                let value = match command {
                    COMMAND_SIN => value.to_radians().sin(),
                    COMMAND_COS => value.to_radians().cos(),
                    _ => value.sqrt(),
                };
                scripts.script_params[0] = Self::f32_to_script_param(value);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_CONTROLLER_MODE => {
                scripts.script_params[0] = 0;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_COLLISION_IN_MEMORY => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                self.update_compare_flag(scripts.collision_in_memory == params[0]);
            }
            COMMAND_SET_TAXI_LIGHTS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                if vehicle.vehicle_is_boat {
                    return Err(ScriptVmError::MissingVehicle(params[0]));
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetTaxiLights {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_ALL_TAXI_LIGHTS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let enabled = params[0] != 0;
                let request = ScriptEntityMutation::SetAllTaxiLights(enabled);
                if entity_runtime
                    .as_deref_mut()
                    .is_some_and(|runtime| runtime.supports_mutation(&request))
                {
                    let _ = entity_runtime
                        .as_deref_mut()
                        .and_then(|runtime| runtime.mutate(request, None));
                } else {
                    scripts
                        .pending_effects
                        .push(ScriptCommandEffect::SetAllTaxiLights(enabled));
                }
            }
            COMMAND_FLASH_CAR | COMMAND_FLASH_CHAR | COMMAND_FLASH_OBJECT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                match command {
                    COMMAND_FLASH_CAR => {
                        entity_runtime
                            .as_deref_mut()
                            .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                            .vehicle_snapshot(params[0])
                            .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                        scripts.pending_effects.push(ScriptCommandEffect::FlashCar {
                            handle: params[0],
                            enabled: params[1] != 0,
                        });
                    }
                    COMMAND_FLASH_CHAR => {
                        entity_runtime
                            .as_deref_mut()
                            .ok_or(ScriptVmError::MissingCharProbe)?
                            .char_snapshot(params[0])
                            .ok_or(ScriptVmError::MissingChar(params[0]))?;
                        scripts
                            .pending_effects
                            .push(ScriptCommandEffect::FlashChar {
                                handle: params[0],
                                enabled: params[1] != 0,
                            });
                    }
                    _ => {
                        scripts
                            .pending_effects
                            .push(ScriptCommandEffect::FlashObject {
                                handle: params[0],
                                enabled: params[1] != 0,
                            });
                    }
                }
            }
            COMMAND_ARM_CAR_WITH_BOMB => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                if vehicle.vehicle_is_boat {
                    return Err(ScriptVmError::MissingVehicle(params[0]));
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ArmCarWithBomb {
                        handle: params[0],
                        bomb_type: params[1],
                    });
            }
            COMMAND_EXPLODE_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::ExplodeCar { handle: params[0] });
            }
            COMMAND_IS_CAR_DOOR_CLOSED => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let closed = u32::try_from(params[1])
                    .ok()
                    .filter(|door| *door < u8::BITS)
                    .is_some_and(|door| vehicle.vehicle_closed_doors & (1_u8 << door) != 0);
                self.update_compare_flag(closed);
            }
            COMMAND_IS_CAR_PASSENGER_SEAT_FREE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let free = usize::try_from(params[1]).ok().is_some_and(|seat| {
                    seat < usize::from(vehicle.vehicle_num_max_passengers)
                        && vehicle.vehicle_passenger_handles[seat].is_none()
                });
                self.update_compare_flag(free);
            }
            COMMAND_GET_CHAR_IN_CAR_PASSENGER_SEAT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[0] = usize::try_from(params[1])
                    .ok()
                    .and_then(|seat| vehicle.vehicle_passenger_handles.get(seat))
                    .copied()
                    .flatten()
                    .unwrap_or(-1);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_CREATE_SINGLE_PARTICLE => {
                let params = self.collect_parameters(scripts, &mut ip, 8)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::CreateSingleParticle {
                        particle_type: params[0],
                        position: CVector::new(
                            Self::script_param_as_f32(params[1]),
                            Self::script_param_as_f32(params[2]),
                            Self::script_param_as_f32(params[3]),
                        ),
                        direction: CVector::new(
                            Self::script_param_as_f32(params[4]),
                            Self::script_param_as_f32(params[5]),
                            Self::script_param_as_f32(params[6]),
                        ),
                        size: Self::script_param_as_f32(params[7]),
                    });
            }
            COMMAND_IS_CAR_ARMED_WITH_ANY_BOMB => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(vehicle.vehicle_bomb_type.is_some_and(|bomb| bomb != 0));
            }
            COMMAND_IS_CAR_ARMED_WITH_BOMB => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(vehicle.vehicle_bomb_type == Some(params[1]));
            }
            COMMAND_IS_CAR_VISIBLY_DAMAGED => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                self.update_compare_flag(vehicle.vehicle_damaged);
            }
            COMMAND_IS_CHAR_MALE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let ped = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(ped.ped_type != Some(5) && ped.ped_type != Some(20));
            }
            COMMAND_HAS_CHAR_SPOTTED_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let ped = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                player_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingPlayerProbe)?(params[1])
                .ok_or(ScriptVmError::MissingPlayer(params[1]))?;
                self.update_compare_flag(ped.can_see_player);
            }
            COMMAND_HAS_CHAR_SPOTTED_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let visible = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharVisibilityProbe)?
                    .char_can_see_char(params[0], params[1])
                    .ok_or(ScriptVmError::MissingCharVisibilityProbe)?;
                self.update_compare_flag(visible);
            }
            COMMAND_CAN_CHAR_SEE_DEAD_CHAR => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let can_see_dead_char = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharVisibilityProbe)?
                    .char_can_see_dead_char(params[0], params[1])
                    .ok_or(ScriptVmError::MissingCharVisibilityProbe)?;
                self.update_compare_flag(can_see_dead_char);
            }
            COMMAND_SET_THREAT_REACTION_RANGE_MULTIPLIER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingThreatReactionRangeProbe)?
                    .set_threat_reaction_range_multiplier(Self::script_param_as_f32(params[0]))
                    .ok_or(ScriptVmError::MissingThreatReactionRangeProbe)?;
            }
            COMMAND_GET_CLOSEST_OBJECT_OF_TYPE => {
                let params = self.collect_parameters(scripts, &mut ip, 5)?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(position.x, position.y);
                }
                let model_index = scripts.resolve_used_object_model(params[4]);
                let handle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingClosestObjectProbe)?
                    .closest_object_of_type(
                        position,
                        Self::script_param_as_f32(params[3]),
                        model_index,
                        general.as_deref_mut(),
                    )
                    .ok_or(ScriptVmError::MissingClosestObjectProbe)?;
                scripts.script_params[0] = handle;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_POINT_OBSCURED_BY_A_MISSION_ENTITY => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                let center = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let extent = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                let a = CVector::new(
                    center.x - extent.x,
                    center.y - extent.y,
                    center.z - extent.z,
                );
                let b = CVector::new(
                    center.x + extent.x,
                    center.y + extent.y,
                    center.z + extent.z,
                );
                let min = CVector::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
                let max = CVector::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
                let live_result = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.any_mission_entity_intersects_cube(min, max));
                self.update_compare_flag(live_result.unwrap_or_else(|| {
                    scripts.mission_entity_bounds.iter().any(|entity| {
                        let radius = entity.radius;
                        entity.position.x + radius >= min.x
                            && entity.position.x - radius <= max.x
                            && entity.position.y + radius >= min.y
                            && entity.position.y - radius <= max.y
                            && entity.position.z + radius >= min.z
                            && entity.position.z - radius <= max.z
                    })
                }));
            }
            COMMAND_IS_AREA_OCCUPIED => {
                let params = self.collect_parameters(scripts, &mut ip, 11)?;
                let a = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                let b = CVector::new(
                    Self::script_param_as_f32(params[3]),
                    Self::script_param_as_f32(params[4]),
                    Self::script_param_as_f32(params[5]),
                );
                let min = CVector::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
                let max = CVector::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
                let entity_types = [
                    params[6] != 0,
                    params[7] != 0,
                    params[8] != 0,
                    params[9] != 0,
                    params[10] != 0,
                ];
                let live_result = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.any_entity_intersects_cube(min, max, entity_types));
                self.update_compare_flag(live_result.unwrap_or_else(|| {
                    scripts.area_entity_bounds.iter().any(|entity| {
                        let enabled = (1..=5).contains(&entity.entity_type)
                            && params[5 + entity.entity_type as usize] != 0;
                        let radius = entity.radius;
                        enabled
                            && entity.position.x + radius >= min.x
                            && entity.position.x - radius <= max.x
                            && entity.position.y + radius >= min.y
                            && entity.position.y - radius <= max.y
                            && entity.position.z + radius >= min.z
                            && entity.position.z - radius <= max.z
                    })
                }));
            }
            COMMAND_START_DRUG_RUN => {
                scripts.drug_run_completed = false;
                scripts.drug_plane_shot_down = false;
                scripts.drug_plane_position = None;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartDrugRun);
            }
            COMMAND_HAS_DRUG_RUN_BEEN_COMPLETED => {
                self.update_compare_flag(scripts.drug_run_completed);
            }
            COMMAND_HAS_DRUG_PLANE_BEEN_SHOT_DOWN => {
                self.update_compare_flag(scripts.drug_plane_shot_down);
            }
            COMMAND_FIND_DRUG_PLANE_COORDINATES => {
                let position = scripts.drug_plane_position.unwrap_or(CVector::ZERO);
                scripts.script_params[0] = position.x.to_bits() as i32;
                scripts.script_params[1] = position.y.to_bits() as i32;
                scripts.script_params[2] = position.z.to_bits() as i32;
                self.store_parameters(scripts, &mut ip, 3)?;
            }
            COMMAND_START_DRUG_DROP_OFF => {
                scripts.drop_off_plane_shot_down = false;
                scripts.drop_off_plane_position = None;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartDrugDropOff);
            }
            COMMAND_HAS_DROP_OFF_PLANE_BEEN_SHOT_DOWN => {
                self.update_compare_flag(scripts.drop_off_plane_shot_down);
            }
            COMMAND_FIND_DROP_OFF_PLANE_COORDINATES => {
                let position = scripts.drop_off_plane_position.unwrap_or(CVector::ZERO);
                scripts.script_params[0] = position.x.to_bits() as i32;
                scripts.script_params[1] = position.y.to_bits() as i32;
                scripts.script_params[2] = position.z.to_bits() as i32;
                self.store_parameters(scripts, &mut ip, 3)?;
            }
            COMMAND_START_CATALINA_HELI => {
                scripts.catalina_heli_shot_down = false;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::StartCatalinaHeli);
            }
            COMMAND_SWITCH_HELICOPTER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SwitchHelicopter(params[0] != 0));
            }
            COMMAND_CATALINA_HELI_TAKE_OFF => scripts
                .pending_effects
                .push(ScriptCommandEffect::CatalinaHeliTakeOff),
            COMMAND_REMOVE_CATALINA_HELI => {
                scripts.catalina_heli_handle = -1;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RemoveCatalinaHeli);
            }
            COMMAND_HAS_CATALINA_HELI_BEEN_SHOT_DOWN => {
                self.update_compare_flag(scripts.catalina_heli_shot_down);
            }
            COMMAND_GRAB_CATALINA_HELI => {
                scripts.script_params[0] = scripts.catalina_heli_handle;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_CATALINA_HELI_FLY_AWAY => scripts
                .pending_effects
                .push(ScriptCommandEffect::CatalinaHeliFlyAway),
            COMMAND_ARE_ANY_CAR_CHEATS_ACTIVATED => {
                self.update_compare_flag(scripts.car_cheats_activated);
            }
            COMMAND_IS_NASTY_GAME => self.update_compare_flag(scripts.nasty_game),
            COMMAND_IS_FRENCH_GAME => self.update_compare_flag(scripts.french_game),
            COMMAND_IS_GERMAN_GAME => self.update_compare_flag(scripts.german_game),
            COMMAND_START_CREDITS => scripts
                .pending_effects
                .push(ScriptCommandEffect::StartCredits),
            COMMAND_STOP_CREDITS => scripts
                .pending_effects
                .push(ScriptCommandEffect::StopCredits),
            COMMAND_ARE_CREDITS_FINISHED => self.update_compare_flag(scripts.credits_done),
            COMMAND_ARE_MEASUREMENTS_IN_METRES => self.update_compare_flag(true),
            COMMAND_CONVERT_METRES_TO_FEET => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.script_params[0] =
                    Self::f32_to_script_param(Self::script_param_as_f32(params[0]) / 0.3048);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_CONVERT_METRES_TO_FEET_INT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.script_params[0] = (params[0] as f32 * 3.28084) as i32;
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_REGISTER_FASTEST_TIME => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterFastestTime {
                        index: params[0],
                        time: params[1],
                    });
            }
            COMMAND_REGISTER_HIGHEST_SCORE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterHighestScore {
                        index: params[0],
                        score: params[1],
                    });
            }
            COMMAND_REGISTER_EL_BURRO_TIME => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterElBurroTime(params[0]));
            }
            COMMAND_REGISTER_4X4_ONE_TIME
            | COMMAND_REGISTER_4X4_TWO_TIME
            | COMMAND_REGISTER_4X4_THREE_TIME
            | COMMAND_REGISTER_4X4_MAYHEM_TIME => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let race = match command {
                    COMMAND_REGISTER_4X4_ONE_TIME => 0,
                    COMMAND_REGISTER_4X4_TWO_TIME => 1,
                    COMMAND_REGISTER_4X4_THREE_TIME => 2,
                    _ => 3,
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::Register4x4Time {
                        race,
                        time: params[0],
                    });
            }
            COMMAND_REGISTER_LIFE_SAVED => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterLifeSaved);
            }
            COMMAND_REGISTER_CRIMINAL_CAUGHT => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterCriminalCaught);
            }
            COMMAND_REGISTER_AMBULANCE_LEVEL => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterAmbulanceLevel(params[0]));
            }
            COMMAND_REGISTER_FIRE_EXTINGUISHED => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterFireExtinguished);
            }
            COMMAND_REGISTER_LONGEST_DODO_FLIGHT => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterLongestDodoFlight(params[0]));
            }
            COMMAND_REGISTER_DEFUSE_BOMB_TIME => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::RegisterDefuseBombTime(params[0]));
            }
            COMMAND_SET_TOTAL_NUMBER_OF_KILL_FRENZIES => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetTotalKillFrenzies(params[0]));
            }
            COMMAND_GENERATE_RANDOM_FLOAT_IN_RANGE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                scripts.script_params[0] =
                    Self::f32_to_script_param(general.get_random_number_in_range_f32(
                        Self::script_param_as_f32(params[0]),
                        Self::script_param_as_f32(params[1]),
                    ));
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GENERATE_RANDOM_INT_IN_RANGE => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let general = general
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingRandomGenerator)?;
                scripts.script_params[0] =
                    general.get_random_number_in_range_i32(params[0], params[1]);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_IS_PLAYER_IN_AREA_ON_FOOT_2D
            | COMMAND_IS_PLAYER_IN_AREA_IN_CAR_2D
            | COMMAND_IS_PLAYER_STOPPED_IN_AREA_2D
            | COMMAND_IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_2D
            | COMMAND_IS_PLAYER_STOPPED_IN_AREA_IN_CAR_2D
            | COMMAND_IS_PLAYER_IN_AREA_ON_FOOT_3D
            | COMMAND_IS_PLAYER_IN_AREA_IN_CAR_3D
            | COMMAND_IS_PLAYER_STOPPED_IN_AREA_3D
            | COMMAND_IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_3D
            | COMMAND_IS_PLAYER_STOPPED_IN_AREA_IN_CAR_3D => {
                self.player_in_area_check_command(
                    scripts,
                    &mut ip,
                    command,
                    &mut player_probe,
                    &mut entity_runtime,
                )?;
            }
            COMMAND_IS_PLAYER_IN_ANGLED_AREA_2D
            | COMMAND_IS_PLAYER_IN_ANGLED_AREA_ON_FOOT_2D
            | COMMAND_IS_PLAYER_IN_ANGLED_AREA_IN_CAR_2D
            | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_2D
            | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_2D
            | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_2D
            | COMMAND_IS_PLAYER_IN_ANGLED_AREA_3D
            | COMMAND_IS_PLAYER_IN_ANGLED_AREA_ON_FOOT_3D
            | COMMAND_IS_PLAYER_IN_ANGLED_AREA_IN_CAR_3D
            | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_3D
            | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_3D
            | COMMAND_IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_3D => {
                self.player_in_angled_area_check_command(
                    scripts,
                    &mut ip,
                    command,
                    &mut player_probe,
                    &mut entity_runtime,
                )?;
            }
            COMMAND_IS_CHAR_IN_AREA_ON_FOOT_2D
            | COMMAND_IS_CHAR_IN_AREA_IN_CAR_2D
            | COMMAND_IS_CHAR_STOPPED_IN_AREA_2D
            | COMMAND_IS_CHAR_STOPPED_IN_AREA_ON_FOOT_2D
            | COMMAND_IS_CHAR_STOPPED_IN_AREA_IN_CAR_2D
            | COMMAND_IS_CHAR_IN_AREA_ON_FOOT_3D
            | COMMAND_IS_CHAR_IN_AREA_IN_CAR_3D
            | COMMAND_IS_CHAR_STOPPED_IN_AREA_3D
            | COMMAND_IS_CHAR_STOPPED_IN_AREA_ON_FOOT_3D
            | COMMAND_IS_CHAR_STOPPED_IN_AREA_IN_CAR_3D => {
                self.char_in_area_check_command(scripts, &mut ip, command, &mut entity_runtime)?;
            }
            COMMAND_IS_CAR_STOPPED_IN_AREA_2D | COMMAND_IS_CAR_STOPPED_IN_AREA_3D => {
                self.car_in_area_check_command(scripts, &mut ip, command, &mut entity_runtime)?;
            }
            COMMAND_IS_PLAYER_SHOOTING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let player =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(player.is_shooting);
            }
            COMMAND_IS_CHAR_SHOOTING => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let ped = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                self.update_compare_flag(ped.is_shooting);
            }
            COMMAND_HAS_CHAR_BEEN_DAMAGED_BY_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                // re3 deliberately accepts a stale/deleted ped handle here and
                // returns false instead of asserting.
                let ped = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0]);
                self.update_compare_flag(
                    ped.is_some_and(|ped| ped.last_weapon_damage == params[1]),
                );
            }
            COMMAND_HAS_CAR_BEEN_DAMAGED_BY_WEAPON => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                // Same null-pool behaviour as the ped form in Script3.cpp.
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0]);
                self.update_compare_flag(
                    vehicle.is_some_and(|vehicle| vehicle.last_weapon_damage == params[1]),
                );
            }
            COMMAND_IS_PLAYER_PRESSING_HORN => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let player =
                    player_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                    .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                // Script.cpp requires PED_DRIVING in addition to GetHorn();
                // passengers using the same pad do not satisfy the command.
                self.update_compare_flag(player.is_driving && player.horn_pressed);
            }
            COMMAND_IS_PLAYER_IN_REMOTE_MODE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                player_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingPlayerProbe)?(params[0])
                .ok_or(ScriptVmError::MissingPlayer(params[0]))?;
                self.update_compare_flag(scripts.player_in_remote_mode);
            }
            COMMAND_IS_PLAYER_TOUCHING_OBJECT
            | COMMAND_IS_CHAR_TOUCHING_OBJECT
            | COMMAND_IS_PLAYER_TOUCHING_OBJECT_ON_FOOT
            | COMMAND_IS_CHAR_TOUCHING_OBJECT_ON_FOOT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                let object_exists = entity_runtime
                    .as_deref_mut()
                    .and_then(|runtime| runtime.object_snapshot(params[1]))
                    .is_some();
                if !object_exists {
                    self.update_compare_flag(false);
                } else {
                    let object_reference = EntityReference::Object(params[1]);
                    let player_command = matches!(
                        command,
                        COMMAND_IS_PLAYER_TOUCHING_OBJECT
                            | COMMAND_IS_PLAYER_TOUCHING_OBJECT_ON_FOOT
                    );
                    let on_foot_only = matches!(
                        command,
                        COMMAND_IS_PLAYER_TOUCHING_OBJECT_ON_FOOT
                            | COMMAND_IS_CHAR_TOUCHING_OBJECT_ON_FOOT
                    );
                    let ped = if player_command {
                        let ped_handle = player_probe
                            .as_deref_mut()
                            .and_then(|probe| probe(params[0]))
                            .map(|player| player.ped_handle);
                        ped_handle.and_then(|handle| {
                            entity_runtime
                                .as_deref_mut()
                                .and_then(|runtime| runtime.char_snapshot(handle))
                        })
                    } else {
                        entity_runtime
                            .as_deref_mut()
                            .and_then(|runtime| runtime.char_snapshot(params[0]))
                    };
                    if let Some(ped) = ped {
                        if on_foot_only && ped.current_vehicle.is_some() {
                            self.update_compare_flag(false);
                        } else if !on_foot_only && let Some(vehicle_handle) = ped.current_vehicle {
                            let touching = entity_runtime
                                .as_deref_mut()
                                .and_then(|runtime| runtime.vehicle_snapshot(vehicle_handle))
                                .is_some_and(|physical| {
                                    physical
                                        .collision_records
                                        .iter()
                                        .flatten()
                                        .any(|&reference| reference == object_reference)
                                });
                            self.update_compare_flag(touching);
                        } else {
                            self.update_compare_flag(
                                ped.collision_records
                                    .iter()
                                    .flatten()
                                    .any(|&reference| reference == object_reference),
                            );
                        }
                    } else {
                        self.update_compare_flag(false);
                    }
                }
            }
            COMMAND_GET_NUMBER_OF_PASSENGERS | COMMAND_GET_MAXIMUM_NUMBER_OF_PASSENGERS => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                let vehicle = entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts.script_params[0] = if command == COMMAND_GET_NUMBER_OF_PASSENGERS {
                    i32::from(vehicle.vehicle_num_passengers)
                } else {
                    i32::from(vehicle.vehicle_num_max_passengers)
                };
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_GET_GROUND_Z_FOR_3D_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                let x = Self::script_param_as_f32(params[0]);
                let y = Self::script_param_as_f32(params[1]);
                let z = Self::script_param_as_f32(params[2]);
                let ground = ground_3d_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingGroundProbe)?(x, y, z);
                scripts.script_params[0] = Self::f32_to_script_param(ground);
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_ADD_SPHERE => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                let mut center = CVector::new(
                    Self::script_param_as_f32(params[0]),
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                );
                if center.z <= MAP_Z_LOW_LIMIT {
                    center.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        center.x, center.y
                    );
                }
                let marker_id = ip as i32;
                scripts.script_params[0] = scripts.add_script_sphere(
                    marker_id,
                    center,
                    Self::script_param_as_f32(params[3]),
                );
                self.store_parameters(scripts, &mut ip, 1)?;
            }
            COMMAND_REMOVE_SPHERE => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts.remove_script_sphere(params[0]);
            }
            COMMAND_SET_CHAR_PROOFS => {
                let params = self.collect_parameters(scripts, &mut ip, 6)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharProofs {
                        handle: params[0],
                        bullet: params[1] != 0,
                        fire: params[2] != 0,
                        explosion: params[3] != 0,
                        collision: params[4] != 0,
                        melee: params[5] != 0,
                    });
            }
            COMMAND_SET_CHAR_USE_PEDNODE_SEEK => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharUsePedNodeSeek {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_WATERTIGHT => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarWaterTight {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_AVOID_LEVEL_TRANSITIONS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarAvoidLevelTransitions {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_STAYS_IN_CURRENT_LEVEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarStaysInCurrentLevel {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CAR_IGNORE_LEVEL_TRANSITIONS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarIgnoreLevelTransitions {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CHAR_STAYS_IN_CURRENT_LEVEL => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharStaysInCurrentLevel {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CHAR_IGNORE_LEVEL_TRANSITIONS => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharIgnoreLevelTransitions {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_MAKE_CRAIGS_CAR_A_BIT_STRONGER => {
                let params = self.collect_parameters(scripts, &mut ip, 2)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCarMoreResistantToDamage {
                        handle: params[0],
                        enabled: params[1] != 0,
                    });
            }
            COMMAND_SET_CHARS_CHATTING => {
                let params = self.collect_parameters(scripts, &mut ip, 3)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[1])
                    .ok_or(ScriptVmError::MissingChar(params[1]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetCharsChatting {
                        first: params[0],
                        second: params[1],
                        duration_ms: params[2] as u32,
                    });
            }
            COMMAND_SET_GENERATE_CARS_AROUND_CAMERA => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetGenerateCarsAroundCamera(
                        params[0] != 0,
                    ));
            }
            COMMAND_SET_JAMES_CAR_ON_PATH_TO_PLAYER => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                let player = player_probe
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingPlayerProbe)?(0)
                .ok_or(ScriptVmError::MissingPlayer(0))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetJamesCarOnPathToPlayer {
                        handle: params[0],
                        player_position: player.position,
                    });
            }
            COMMAND_BLOW_UP_RC_BUGGY => {
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::BlowUpRemoteCar);
            }
            COMMAND_DRAW_SHADOW => {
                let params = self.collect_parameters(scripts, &mut ip, 10)?;
                let position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                let angle = Self::script_param_as_f32(params[4]);
                let length = Self::script_param_as_f32(params[5]);
                let (x, y) = if angle == 0.0 {
                    (0.0, length)
                } else {
                    (angle.sin() * length, angle.cos() * length)
                };
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::DrawShadow {
                        texture: params[0] as u8,
                        position,
                        front: [-x, y],
                        side: [y, x],
                        intensity: params[6] as i16,
                        color: [params[7] as u8, params[8] as u8, params[9] as u8],
                    });
            }
            COMMAND_WARP_CHAR_FROM_CAR_TO_COORD => {
                let params = self.collect_parameters(scripts, &mut ip, 4)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingCharProbe)?
                    .char_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingChar(params[0]))?;
                let mut position = CVector::new(
                    Self::script_param_as_f32(params[1]),
                    Self::script_param_as_f32(params[2]),
                    Self::script_param_as_f32(params[3]),
                );
                if position.z <= MAP_Z_LOW_LIMIT {
                    position.z = ground_probe
                        .as_deref_mut()
                        .ok_or(ScriptVmError::MissingGroundProbe)?(
                        position.x, position.y
                    );
                }
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::WarpCharFromCarToCoord {
                        handle: params[0],
                        position,
                        needs_ground: false,
                    });
            }
            // re3 Script6.cpp COMMAND_SET_ALL_OCCUPANTS_OF_CAR_LEAVE_CAR.
            COMMAND_SET_ALL_OCCUPANTS_OF_CAR_LEAVE_CAR => {
                let params = self.collect_parameters(scripts, &mut ip, 1)?;
                entity_runtime
                    .as_deref_mut()
                    .ok_or(ScriptVmError::MissingVehicleEntityProbe)?
                    .vehicle_snapshot(params[0])
                    .ok_or(ScriptVmError::MissingVehicle(params[0]))?;
                scripts
                    .pending_effects
                    .push(ScriptCommandEffect::SetAllOccupantsOfCarLeaveCar { handle: params[0] });
            }
            COMMAND_LOCATE_PLAYER_ANY_MEANS_2D
            | COMMAND_LOCATE_PLAYER_ON_FOOT_2D
            | COMMAND_LOCATE_PLAYER_IN_CAR_2D
            | COMMAND_LOCATE_STOPPED_PLAYER_ANY_MEANS_2D
            | COMMAND_LOCATE_STOPPED_PLAYER_ON_FOOT_2D
            | COMMAND_LOCATE_STOPPED_PLAYER_IN_CAR_2D
            | COMMAND_LOCATE_PLAYER_ANY_MEANS_3D
            | COMMAND_LOCATE_PLAYER_ON_FOOT_3D
            | COMMAND_LOCATE_PLAYER_IN_CAR_3D
            | COMMAND_LOCATE_STOPPED_PLAYER_ANY_MEANS_3D
            | COMMAND_LOCATE_STOPPED_PLAYER_ON_FOOT_3D
            | COMMAND_LOCATE_STOPPED_PLAYER_IN_CAR_3D => {
                self.locate_player_area_command(scripts, &mut ip, command, &mut player_probe)?;
            }
            COMMAND_LOCATE_CHAR_ANY_MEANS_2D
            | COMMAND_LOCATE_CHAR_ON_FOOT_2D
            | COMMAND_LOCATE_CHAR_IN_CAR_2D
            | COMMAND_LOCATE_STOPPED_CHAR_ANY_MEANS_2D
            | COMMAND_LOCATE_STOPPED_CHAR_ON_FOOT_2D
            | COMMAND_LOCATE_STOPPED_CHAR_IN_CAR_2D
            | COMMAND_LOCATE_CHAR_ANY_MEANS_3D
            | COMMAND_LOCATE_CHAR_ON_FOOT_3D
            | COMMAND_LOCATE_CHAR_IN_CAR_3D
            | COMMAND_LOCATE_STOPPED_CHAR_ANY_MEANS_3D
            | COMMAND_LOCATE_STOPPED_CHAR_ON_FOOT_3D
            | COMMAND_LOCATE_STOPPED_CHAR_IN_CAR_3D => {
                self.locate_char_area_command(scripts, &mut ip, command, &mut entity_runtime)?;
            }
            COMMAND_LOCATE_PLAYER_ANY_MEANS_CHAR_2D
            | COMMAND_LOCATE_PLAYER_ON_FOOT_CHAR_2D
            | COMMAND_LOCATE_PLAYER_IN_CAR_CHAR_2D
            | COMMAND_LOCATE_PLAYER_ANY_MEANS_CHAR_3D
            | COMMAND_LOCATE_PLAYER_ON_FOOT_CHAR_3D
            | COMMAND_LOCATE_PLAYER_IN_CAR_CHAR_3D => {
                self.locate_player_char_command(
                    scripts,
                    &mut ip,
                    command,
                    &mut player_probe,
                    &mut entity_runtime,
                )?;
            }
            COMMAND_LOCATE_CHAR_ANY_MEANS_CHAR_2D
            | COMMAND_LOCATE_CHAR_ON_FOOT_CHAR_2D
            | COMMAND_LOCATE_CHAR_IN_CAR_CHAR_2D
            | COMMAND_LOCATE_CHAR_ANY_MEANS_CHAR_3D
            | COMMAND_LOCATE_CHAR_ON_FOOT_CHAR_3D
            | COMMAND_LOCATE_CHAR_IN_CAR_CHAR_3D => {
                self.locate_char_char_command(scripts, &mut ip, command, &mut entity_runtime)?;
            }
            COMMAND_LOCATE_PLAYER_ANY_MEANS_CAR_2D
            | COMMAND_LOCATE_PLAYER_ON_FOOT_CAR_2D
            | COMMAND_LOCATE_PLAYER_IN_CAR_CAR_2D
            | COMMAND_LOCATE_PLAYER_ANY_MEANS_CAR_3D
            | COMMAND_LOCATE_PLAYER_ON_FOOT_CAR_3D
            | COMMAND_LOCATE_PLAYER_IN_CAR_CAR_3D => {
                self.locate_player_car_command(
                    scripts,
                    &mut ip,
                    command,
                    &mut player_probe,
                    &mut entity_runtime,
                )?;
            }
            COMMAND_LOCATE_CHAR_ANY_MEANS_CAR_2D
            | COMMAND_LOCATE_CHAR_ON_FOOT_CAR_2D
            | COMMAND_LOCATE_CHAR_IN_CAR_CAR_2D
            | COMMAND_LOCATE_CHAR_ANY_MEANS_CAR_3D
            | COMMAND_LOCATE_CHAR_ON_FOOT_CAR_3D
            | COMMAND_LOCATE_CHAR_IN_CAR_CAR_3D => {
                self.locate_char_car_command(scripts, &mut ip, command, &mut entity_runtime)?;
            }
            // re3 Script6.cpp: FailCurrentMission is a two-frame latch. The
            // script scheduler redirects active mission threads on the next
            // update after housekeeping decrements it to one.
            COMMAND_FAIL_CURRENT_MISSION => scripts.fail_current_mission = 2,
            COMMAND_LOCATE_CAR_2D
            | COMMAND_LOCATE_CAR_3D
            | COMMAND_LOCATE_STOPPED_CAR_2D
            | COMMAND_LOCATE_STOPPED_CAR_3D => {
                self.locate_car_area_command(scripts, &mut ip, command, &mut entity_runtime)?;
            }
            // re3's default case executes script_assert(0). Never advance past an
            // unported opcode by guessed arity or fabricate missing side effects.
            _ => return Err(ScriptVmError::UnsupportedCommand(command)),
        }

        self.ip = ip;
        outcome.ip_after = ip;
        Ok(outcome)
    }
}
