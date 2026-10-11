//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! `CTheScripts::Init` runtime state without opcode execution.

use std::io;

use re3bevy_assets::install::FileMgr::CFileMgr;
use re3bevy_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral};
use re3bevy_formats::formats::Script::{
    MAX_NUM_MISSION_SCRIPTS, SIZE_MAIN_SCRIPT, SIZE_MISSION_SCRIPT, SIZE_SCRIPT_SPACE, ScmFile,
    USED_OBJECT_NAME_LENGTH,
};

use crate::{
    control::Garages::{EGarageState, NUM_GARAGES},
    core::Fire::NUM_FIRES,
    core::Streaming::MAXVEHICLESLOADED,
};

pub use super::ScriptCommands::*;

// S1: script VM types live in control/ScriptTypes.rs; re-exported here
pub use super::ScriptTypes::{
    ANDOR_NONE, ANDS_1, ANDS_8, ARGUMENT_END, ARGUMENT_FLOAT, ARGUMENT_GLOBALVAR, ARGUMENT_INT8,
    ARGUMENT_INT16, ARGUMENT_INT32, ARGUMENT_LOCALVAR, CLEANUP_CAR, CLEANUP_CHAR, CLEANUP_OBJECT,
    CLEANUP_UNUSED, CMissionCleanup, CStuckCarCheck, CTheScripts, CUpsideDownCarCheck,
    CleanupEntity, IntroScriptRectangle, IntroTextLine, KEY_LENGTH_IN_SCRIPT, MAP_Z_LOW_LIMIT,
    MAX_CLEANUP, MAX_NUM_BUILDING_SWAPS, MAX_NUM_COLLECTIVES, MAX_NUM_CONTACTS,
    MAX_NUM_INTRO_RECTANGLES, MAX_NUM_INTRO_TEXT_LINES, MAX_NUM_INVISIBILITY_SETTINGS,
    MAX_NUM_SCRIPT_SPHERES, MAX_NUM_SCRIPT_SPRITES, MAX_NUM_SCRIPTS, MAX_NUM_STORED_LINES,
    MAX_NUM_USED_OBJECTS, MAX_STACK_DEPTH, MAX_STUCK_CAR_CHECKS, MAX_UPSIDEDOWN_CAR_CHECKS,
    NUM_LOCAL_VARS, NUM_TIMERS, ORS_1, ORS_8, ScriptAreaEntityBounds, ScriptCommandEffect,
    ScriptCommandOutcome, ScriptCreateCarRequest, ScriptCreateCharRequest,
    ScriptCreateObjectRequest, ScriptEntityMutation, ScriptEntityMutationResult,
    ScriptEntityRuntimeAccess, ScriptEntitySnapshot, ScriptGarageVehicleContainment, ScriptHandle,
    ScriptMissionEntityBounds, ScriptObjectMotionRequest, ScriptPathNodeSnapshot,
    ScriptPlayerSnapshot, ScriptSphere, ScriptStatsEffect, ScriptVarPointer, ScriptVehicleProbe,
    ScriptVmError, ScriptWorldStateSnapshot, ScriptZoneBounds, StuckCarData, TBuildingSwap,
    TCollectiveData, TUsedObject, UPSIDEDOWN_MOVE_SPEED_THRESHOLD, UPSIDEDOWN_TIMER_THRESHOLD,
    UPSIDEDOWN_TURN_SPEED_THRESHOLD, UPSIDEDOWN_UP_THRESHOLD, UpsideDownCarData, VAR_GLOBAL,
    VAR_LOCAL,
};
pub(crate) use super::ScriptTypes::{
    LocateMode, ScriptEntityReadProbeRuntime, reborrow_script_entity_runtime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CRunningScript {
    pub next: Option<usize>,
    pub prev: Option<usize>,
    pub script_name: [u8; 8],
    pub ip: u32,
    pub stack: [u32; MAX_STACK_DEPTH],
    pub stack_pointer: u16,
    pub local_variables: [i32; NUM_LOCAL_VARS + NUM_TIMERS],
    pub cond_result: bool,
    pub is_mission_script: bool,
    pub skip_wake_time: bool,
    pub wake_time: u32,
    pub and_or_state: u16,
    pub not_flag: bool,
    pub deatharrest_enabled: bool,
    pub deatharrest_executed: bool,
    pub mission_flag: bool,
    /// ECS frame latch for MESSAGE_WAIT's skip-button early return.
    pub skip_message_wait_this_frame: bool,
}

impl CTheScripts {
    /// re3 `CTheScripts::Init` MAIN load:
    ///   CFileMgr::SetDir("data"); OpenFile("main.scm"); Read(..., SIZE_MAIN_SCRIPT);
    /// Full multi-script MAIN.SCM on disk is larger than 128 KiB; only the main
    /// segment is read into script space (missions load later from the same file).
    pub fn init(file_mgr: &mut CFileMgr) -> io::Result<Self> {
        let main_scm_bytes = file_mgr.read_file_to_end("data\\main.scm")?;
        let scm = ScmFile::try_parse_main_scm(&main_scm_bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(Self::from_scm(scm, main_scm_bytes))
    }

    /// re3 `CTheScripts::UpdateObjectIndices`: resolve MAIN.SCM's negative
    /// used-object references after IDE model info has been loaded.
    pub fn update_object_indices(&mut self, model_info: &crate::modelinfo::ModelInfo::CModelInfo) {
        for object in self
            .used_objects
            .iter_mut()
            .take(self.number_of_used_objects as usize)
        {
            let length = object
                .name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(object.name.len());
            let name = String::from_utf8_lossy(&object.name[..length]);
            object.index = model_info
                .get_model_info_by_name(name.as_ref())
                .and_then(|(id, _)| i32::try_from(id).ok())
                .unwrap_or(0);
        }
    }

    pub(crate) fn from_scm(scm: ScmFile, main_scm_bytes: Vec<u8>) -> Self {
        let mut used_objects = vec![
            TUsedObject {
                name: [0; USED_OBJECT_NAME_LENGTH],
                index: 0,
            };
            MAX_NUM_USED_OBJECTS
        ];
        for (index, object) in scm
            .used_objects
            .iter()
            .take(MAX_NUM_USED_OBJECTS)
            .enumerate()
        {
            used_objects[index] = TUsedObject {
                name: object.name,
                index: object.index,
            };
        }

        let mut multi_script_array = [0; MAX_NUM_MISSION_SCRIPTS];
        for (index, offset) in scm
            .multiscript
            .offsets
            .iter()
            .take(MAX_NUM_MISSION_SCRIPTS)
            .enumerate()
        {
            multi_script_array[index] = *offset;
        }

        let number_of_used_objects = scm.used_objects.len().min(MAX_NUM_USED_OBJECTS) as u16;
        let number_of_mission_scripts = scm
            .multiscript
            .number_of_mission_scripts
            .max(0)
            .min(MAX_NUM_MISSION_SCRIPTS as i16) as u16;
        let largest_mission_script_size = scm.multiscript.largest_mission_script_size.max(0) as u32;
        let main_script_size = scm.multiscript.main_script_size.max(0) as u32;

        let variable_space_size = scm.variable_space_size.max(0) as usize;

        Self {
            script_space: scm.script_space,
            main_scm_bytes,
            loaded_main_bytes: scm.loaded_main_bytes,
            variable_space_size,
            idle_script_count: MAX_NUM_SCRIPTS,
            active_script_count: 0,
            store_vehicle_index: -1,
            store_vehicle_was_random: true,
            on_a_mission_flag: 0,
            base_brief_id_for_contact: [0; MAX_NUM_CONTACTS],
            on_a_mission_for_contact_flag: [0; MAX_NUM_CONTACTS],
            collectives: vec![
                TCollectiveData {
                    col_index: -1,
                    ped_index: 0,
                };
                MAX_NUM_COLLECTIVES
            ],
            next_free_collective_index: 0,
            last_random_ped_id: -1,
            used_objects,
            number_of_used_objects,
            already_running_a_mission_script: false,
            using_a_multi_script_file: true,
            multi_script_array,
            number_of_mission_scripts,
            last_loaded_mission_index: None,
            largest_mission_script_size,
            main_script_size,
            fail_current_mission: 0,
            countdown_to_make_player_unsafe: 0,
            dbg_flag: false,
            delay_making_player_unsafe_this_time: 1,
            num_script_debug_lines: 0,
            script_spheres: vec![ScriptSphere::default(); MAX_NUM_SCRIPT_SPHERES],
            intro_text_lines: vec![IntroTextLine::default(); MAX_NUM_INTRO_TEXT_LINES],
            number_of_intro_text_lines_this_frame: 0,
            use_text_commands: 0,
            intro_rectangles: vec![IntroScriptRectangle::default(); MAX_NUM_INTRO_RECTANGLES],
            number_of_intro_rectangles_this_frame: 0,
            script_texture_dictionary: None,
            script_sprite_names: [[0; KEY_LENGTH_IN_SCRIPT]; MAX_NUM_SCRIPT_SPRITES],
            building_swaps: vec![TBuildingSwap::default(); MAX_NUM_BUILDING_SWAPS],
            invisibility_settings: vec![None; MAX_NUM_INVISIBILITY_SETTINGS],
            scripts: {
                let mut scripts = Vec::with_capacity(MAX_NUM_SCRIPTS);
                for _ in 0..MAX_NUM_SCRIPTS {
                    scripts.push(CRunningScript::new());
                }
                scripts
            },
            // re3 CTheScripts::Init pushes every slot to the idle list head in
            // slot order, leaving slot 127 at the head; StartNewScript hands
            // out the head. With pop() taking the Vec end, ascending order
            // reproduces re3's 127-first LIFO exactly.
            idle_scripts: (0..MAX_NUM_SCRIPTS).collect(),
            active_scripts: Vec::new(),
            mission_cleanup: CMissionCleanup::new(),
            upside_down_cars: CUpsideDownCarCheck::new(),
            stuck_cars: CStuckCarCheck::new(),
            script_params: [0; 32],
            pending_effects: Vec::new(),
            commands_executed: 0,
            scripts_updated: 0,
            last_vm_error: None,
            vm_fault_recovery_pending: false,
            object_names_read: true,
            object_indices_updated: false,
            multiscript_offsets_read: true,
            next_script_pickup_handle: 1,
            collected_pickup_handles: Vec::new(),
            next_script_object_handle: 1,
            next_audio_script_object_handle: 1,
            next_script_blip_handle: 1,
            next_cutscene_object_handle: 0,
            pending_cutscene_model_snapshots: Vec::new(),
            cutscene_time_ms: 0,
            cutscene_finished: false,
            player_char_handles: [0; 2],
            camera_fading: false,
            mission_audio_loading_status: 1,
            mission_audio_finished: true,
            phone_message_displayed: [false; 50],
            phone_positions: [CVector::ZERO; 50],
            phone_states: [0; 50],
            phone_max: 0,
            script_phones_max: 0,
            phone_displaying_message: false,
            phone_displaying_message_slot: -1,
            special_char_loaded: [false; 4],
            model_loaded: [false; crate::core::Streaming::MODELINFOSIZE],
            script_fire_slots: [false; NUM_FIRES],
            car_generators_created: 0,
            garages_created: 1,
            garage_states: {
                let mut states = [EGarageState::FullyClosed as u8; NUM_GARAGES];
                // CGarages::Init creates the crusher opened in slot zero.
                states[0] = EGarageState::Opened as u8;
                states
            },
            garage_respray_happened: [false; NUM_GARAGES],
            garage_collection_masks: [0; NUM_GARAGES],
            crushed_car_id: -1,
            garage_vehicle_containment: Vec::new(),
            player_wb_state: 0,
            collected_packages: 0,
            kill_frenzy_status: crate::control::Darkel::KILLFRENZY_NONE,
            models_killed_by_player: [0; crate::modelinfo::ModelIndices::NUM_DEFAULT_MODELS],
            ped_type_threats: [0; crate::peds::PedType::NUM_PEDTYPES],
            nasty_game: true,
            french_game: false,
            german_game: false,
            credits_done: true,
            car_cheats_activated: false,
            player_in_remote_mode: false,
            collision_in_memory: 0,
            loaded_vehicle_models: [-1; MAXVEHICLESLOADED],
            num_random_cars: 0,
            drug_run_completed: false,
            drug_plane_shot_down: false,
            drug_plane_position: None,
            drop_off_plane_shot_down: false,
            drop_off_plane_position: None,
            catalina_heli_shot_down: false,
            catalina_heli_handle: -1,
            pacman_pills_eaten: 0,
            pacman_pills_carried: 0,
            projectile_snapshots: Vec::new(),
            explosion_snapshots: Vec::new(),
            zone_bounds: Vec::new(),
            mission_entity_bounds: Vec::new(),
            area_entity_bounds: Vec::new(),
            static_area_entity_bounds_count: 0,
            static_area_world_entity_count: 0,
        }
    }

    pub(crate) fn reserve_script_fire_slot_at(&mut self, slot: usize) -> bool {
        let Some(occupied) = self.script_fire_slots.get_mut(slot) else {
            return false;
        };
        if *occupied {
            return false;
        }
        *occupied = true;
        true
    }

    pub(crate) fn release_script_fire_slot(&mut self, slot: i32) {
        if let Ok(slot) = usize::try_from(slot) {
            if let Some(occupied) = self.script_fire_slots.get_mut(slot) {
                *occupied = false;
            }
        }
    }

    /// re3 `CTheScripts::GetActualScriptSphereIndex`.
    pub fn actual_script_sphere_index(&self, handle: i32) -> Option<usize> {
        if handle == -1 {
            return None;
        }
        let handle = handle as u32;
        let slot = (handle & 0xFFFF) as usize;
        let generation = (handle >> 16) as i32;
        self.script_spheres
            .get(slot)
            .filter(|sphere| sphere.index == generation)
            .map(|_| slot)
    }

    /// re3 `CTheScripts::AddScriptSphere`, including its 16-bit generation handle.
    pub fn add_script_sphere(&mut self, id: i32, center: CVector, radius: f32) -> i32 {
        let Some(slot) = self.script_spheres.iter().position(|sphere| !sphere.in_use) else {
            return -1;
        };
        let sphere = &mut self.script_spheres[slot];
        sphere.index = if sphere.index >= i32::from(u16::MAX) - 1 {
            1
        } else {
            sphere.index + 1
        };
        sphere.in_use = true;
        sphere.id = id;
        sphere.center = [center.x, center.y, center.z];
        sphere.radius = radius;
        (slot as i32) | (sphere.index << 16)
    }

    /// re3 `CTheScripts::RemoveScriptSphere`.
    pub fn remove_script_sphere(&mut self, handle: i32) {
        let Some(slot) = self.actual_script_sphere_index(handle) else {
            return;
        };
        let sphere = &mut self.script_spheres[slot];
        sphere.in_use = false;
        sphere.id = 0;
    }

    pub fn load_and_launch_mission_internal(
        &mut self,
        mission_index: i32,
    ) -> Result<usize, ScriptVmError> {
        if mission_index < 0 || mission_index as usize >= self.number_of_mission_scripts as usize {
            return Err(ScriptVmError::InvalidMissionScriptIndex(mission_index));
        }
        let offset = self.multi_script_array[mission_index as usize];
        if offset < 0 || offset as usize >= self.main_scm_bytes.len() {
            return Err(ScriptVmError::MissionScriptOffsetOutOfRange(offset));
        }

        let source_start = offset as usize;
        let source_end = source_start
            .saturating_add(SIZE_MISSION_SCRIPT)
            .min(self.main_scm_bytes.len());
        if source_end <= source_start {
            return Err(ScriptVmError::MissionScriptRangeInvalid {
                offset,
                end: source_end,
            });
        }
        let mission_start = SIZE_MAIN_SCRIPT;
        // re3 Script6.cpp seeks to the selected mission offset and requests a
        // full SIZE_MISSION_SCRIPT read into the shared mission slot. It does
        // not clear the slot first or stop at the following mission offset.
        // A short EOF read therefore leaves the previous tail intact.
        let mission_len = source_end - source_start;
        self.script_space[mission_start..mission_start + mission_len]
            .copy_from_slice(&self.main_scm_bytes[source_start..source_end]);

        let script_index = self.start_new_script(SIZE_MAIN_SCRIPT as u32)?;
        self.scripts[script_index].is_mission_script = true;
        self.scripts[script_index].mission_flag = true;
        self.already_running_a_mission_script = true;
        self.last_loaded_mission_index = Some(mission_index);
        bevy::log::debug!(
            "VM load_and_launch_mission_internal idx={} script_index={}",
            mission_index,
            script_index
        );
        Ok(script_index)
    }

    /// re3: negative model ids index `UsedObjectArray[-model].index`.
    pub fn resolve_used_object_model(&self, model: i32) -> i32 {
        if model >= 0 {
            return model;
        }
        let index = (-model) as usize;
        self.used_objects
            .get(index)
            .map(|object| object.index)
            .unwrap_or(model)
    }

    pub fn alloc_script_pickup_handle(&mut self) -> i32 {
        let handle = self.next_script_pickup_handle;
        self.next_script_pickup_handle = self.next_script_pickup_handle.saturating_add(1);
        handle
    }

    pub fn alloc_script_object_handle(&mut self) -> i32 {
        let slot = self.next_script_object_handle.max(1);
        self.next_script_object_handle = if slot >= 449 { 1 } else { slot + 1 };
        (slot << 8) | 1
    }

    pub fn alloc_audio_script_object_handle(&mut self) -> i32 {
        let slot = self.next_audio_script_object_handle.max(1);
        self.next_audio_script_object_handle = if slot >= 255 { 1 } else { slot + 1 };
        // CPool handles are (slot << 8) | id. Use id=1 for script-created objects.
        (slot << 8) | 1
    }

    pub fn start_new_script(&mut self, ip: u32) -> Result<usize, ScriptVmError> {
        let script_index = self
            .idle_scripts
            .pop()
            .ok_or(ScriptVmError::NoIdleScripts)?;
        self.scripts[script_index].init();
        self.scripts[script_index].set_ip(ip);
        self.active_scripts.insert(0, script_index);
        self.refresh_links();
        self.idle_script_count = self.idle_scripts.len();
        self.active_script_count = self.active_scripts.len();
        Ok(script_index)
    }

    /// re3 `CTheScripts::StartTestScript` — boots MAIN.SCM at IP 0.
    pub fn start_test_script(&mut self) -> Result<usize, ScriptVmError> {
        self.start_new_script(0)
    }

    pub fn remove_script_from_active(&mut self, script_index: usize) {
        self.active_scripts.retain(|index| *index != script_index);
        if !self.idle_scripts.contains(&script_index) {
            // re3 AddScriptToList(&pIdleScripts) returns terminated slots to
            // the idle list head, so the next StartNewScript reuses them
            // first (LIFO). With pop() taking the Vec end, push is the head.
            self.idle_scripts.push(script_index);
        }
        self.refresh_links();
        self.idle_script_count = self.idle_scripts.len();
        self.active_script_count = self.active_scripts.len();
    }

    /// Materialise re3 `CMissionCleanup::Process` in the ECS command stream.
    /// The opcode and debug switcher share this drain so abandoned mission
    /// entities, audio and transient game state take the same cleanup path.
    pub fn queue_mission_cleanup_effects(&mut self) {
        for entity in self.mission_cleanup.take_for_process() {
            let effect = match entity.entity_type {
                CLEANUP_CAR => ScriptCommandEffect::MarkCarAsNoLongerNeeded {
                    handle: entity.id,
                },
                CLEANUP_CHAR => ScriptCommandEffect::MarkCharAsNoLongerNeeded {
                    handle: entity.id,
                },
                CLEANUP_OBJECT => ScriptCommandEffect::MarkObjectAsNoLongerNeeded {
                    handle: entity.id,
                },
                _ => continue,
            };
            self.pending_effects.push(effect);
        }
        self.pending_effects.push(ScriptCommandEffect::ClearMissionAudio);
        self.pending_effects
            .push(ScriptCommandEffect::MissionCleanupProcessed);
    }

    pub fn process_frame_housekeeping(&mut self, time_step_ms: u32, replay_playing_back: bool) {
        if replay_playing_back {
            return;
        }
        self.commands_executed = 0;
        self.scripts_updated = 0;
        // Do not clear pending_effects here: consumers (Messages / runtime
        // adapters) drain them. Clearing would drop CreatePlayer / cutscene
        // effects produced by StartTestScript before InGame systems run.
        let _ = time_step_ms;
        if self.fail_current_mission != 0 {
            self.fail_current_mission -= 1;
        }
        if self.countdown_to_make_player_unsafe != 0 {
            self.countdown_to_make_player_unsafe -= 1;
        }
        if self.use_text_commands != 0 {
            for line in &mut self.intro_text_lines {
                line.reset();
            }
            self.number_of_intro_text_lines_this_frame = 0;
            for rect in &mut self.intro_rectangles {
                rect.is_used = false;
                rect.before_fade = false;
            }
            self.number_of_intro_rectangles_this_frame = 0;
            if self.use_text_commands == 1 {
                self.use_text_commands = 0;
            }
        }
    }

    pub fn process_frame_vehicle_checks<U, S>(
        &mut self,
        time_step_ms: u32,
        timer_ms: u32,
        upside_down_probe: U,
        stuck_probe: S,
    ) where
        U: FnMut(i32) -> ScriptVehicleProbe,
        S: FnMut(i32) -> ScriptVehicleProbe,
    {
        self.upside_down_cars
            .update_timers(time_step_ms, upside_down_probe);
        self.stuck_cars.process(timer_ms, stuck_probe);
    }

    pub fn read_1_byte_from_script(&self, ip: &mut u32) -> Result<u8, ScriptVmError> {
        let index = *ip as usize;
        let byte = *self
            .script_space
            .get(index)
            .ok_or(ScriptVmError::IpOutOfRange)?;
        *ip += 1;
        Ok(byte)
    }

    pub fn read_2_bytes_from_script(&self, ip: &mut u32) -> Result<u16, ScriptVmError> {
        let index = *ip as usize;
        let bytes = self
            .script_space
            .get(index..index + 2)
            .ok_or(ScriptVmError::IpOutOfRange)?;
        *ip += 2;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_4_bytes_from_script(&self, ip: &mut u32) -> Result<i32, ScriptVmError> {
        let index = *ip as usize;
        let bytes = self
            .script_space
            .get(index..index + 4)
            .ok_or(ScriptVmError::IpOutOfRange)?;
        *ip += 4;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// re3 `CTheScripts::ReadFloatFromScript`: signed 16-bit fixed-point / 16.
    pub fn read_float_from_script(&self, ip: &mut u32) -> Result<f32, ScriptVmError> {
        let raw = self.read_2_bytes_from_script(ip)? as i16;
        Ok(f32::from(raw) / 16.0)
    }

    pub fn read_key_from_script(
        &self,
        ip: &mut u32,
    ) -> Result<[u8; KEY_LENGTH_IN_SCRIPT], ScriptVmError> {
        let index = *ip as usize;
        let end = index + KEY_LENGTH_IN_SCRIPT;
        let bytes = self
            .script_space
            .get(index..end)
            .ok_or(ScriptVmError::IpOutOfRange)?;
        *ip = ip.wrapping_add(KEY_LENGTH_IN_SCRIPT as u32);
        let mut key = [0u8; KEY_LENGTH_IN_SCRIPT];
        key.copy_from_slice(bytes);
        Ok(key)
    }

    pub fn get_size_of_variable_space(&self) -> usize {
        self.variable_space_size
    }

    /// re3 `CTheScripts::IsPlayerOnAMission`.
    pub fn is_player_on_a_mission(&self) -> bool {
        self.on_a_mission_flag != 0
            && self
                .read_i32_at_script_space(self.on_a_mission_flag as usize)
                .is_ok_and(|value| value == 1)
    }

    pub(crate) fn read_i32_at_script_space(&self, index: usize) -> Result<i32, ScriptVmError> {
        let bytes = self
            .script_space
            .get(index..index + 4)
            .ok_or(ScriptVmError::GlobalVariableOutOfRange(index))?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(crate) fn write_i32_at_script_space(
        &mut self,
        index: usize,
        value: i32,
    ) -> Result<(), ScriptVmError> {
        let bytes = value.to_le_bytes();
        let target = self
            .script_space
            .get_mut(index..index + 4)
            .ok_or(ScriptVmError::GlobalVariableOutOfRange(index))?;
        target.copy_from_slice(&bytes);
        Ok(())
    }

    pub fn get_script_var(
        &self,
        pointer: ScriptVarPointer,
        script: &CRunningScript,
    ) -> Result<i32, ScriptVmError> {
        match pointer {
            ScriptVarPointer::Global(index) => self.read_i32_at_script_space(index),
            ScriptVarPointer::Local(index) => script
                .local_variables
                .get(index)
                .copied()
                .ok_or(ScriptVmError::LocalVariableOutOfRange(index)),
        }
    }

    pub fn set_script_var(
        &mut self,
        pointer: ScriptVarPointer,
        script: &mut CRunningScript,
        value: i32,
    ) -> Result<(), ScriptVmError> {
        match pointer {
            ScriptVarPointer::Global(index) => self.write_i32_at_script_space(index, value),
            ScriptVarPointer::Local(index) => {
                let local = script
                    .local_variables
                    .get_mut(index)
                    .ok_or(ScriptVmError::LocalVariableOutOfRange(index))?;
                *local = value;
                Ok(())
            }
        }
    }

    pub fn process_one_command_for_script(
        &mut self,
        script_index: usize,
        time_ms: u32,
    ) -> Result<ScriptCommandOutcome, ScriptVmError> {
        self.process_one_command_for_script_with_context(
            script_index,
            time_ms,
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn process_one_command_for_script_with_context(
        &mut self,
        script_index: usize,
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
        let mut entity_runtime = ScriptEntityReadProbeRuntime {
            char_probe,
            vehicle_probe: vehicle_entity_probe,
            object_probe,
        };
        self.process_one_command_for_script_with_entity_runtime_context(
            script_index,
            time_ms,
            time_step,
            general,
            clock,
            camera_probe,
            ground_probe,
            None,
            path_node_probe,
            Some(&mut entity_runtime),
            player_probe,
            pad_probe,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_one_command_for_script_with_entity_runtime_context(
        &mut self,
        script_index: usize,
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
        entity_runtime: Option<&mut dyn ScriptEntityRuntimeAccess>,
        player_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        pad_probe: Option<&mut dyn FnMut(i32, i32) -> i16>,
        world_state_probe: Option<&mut dyn FnMut() -> ScriptWorldStateSnapshot>,
    ) -> Result<ScriptCommandOutcome, ScriptVmError> {
        let mut script = self
            .scripts
            .get(script_index)
            .cloned()
            .ok_or(ScriptVmError::InvalidScriptIndex(script_index))?;
        let outcome = script.process_one_command_with_3d_ground_runtime(
            self,
            time_ms,
            time_step,
            general,
            clock,
            camera_probe,
            ground_probe,
            ground_3d_probe,
            path_node_probe,
            entity_runtime,
            player_probe,
            pad_probe,
            world_state_probe,
        )?;
        let mut outcome = outcome;
        outcome.script_index = script_index;
        if outcome.terminated {
            self.remove_script_from_active(script_index);
        } else {
            self.scripts[script_index] = script;
        }
        Ok(outcome)
    }

    pub fn process_active_scripts_with_context(
        &mut self,
        time_ms: u32,
        time_step_ms: f32,
        time_step: f32,
        general: &mut CGeneral,
        clock: &mut CClock,
        camera_probe: Option<&mut dyn FnMut(CVector, f32) -> bool>,
        ground_probe: Option<&mut dyn FnMut(f32, f32) -> f32>,
        player_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>>,
        char_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        vehicle_entity_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        object_probe: Option<&mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
        pad_probe: Option<&mut dyn FnMut(i32, i32) -> i16>,
        replay_playing_back: bool,
    ) -> Result<Vec<ScriptCommandOutcome>, ScriptVmError> {
        let mut entity_runtime = ScriptEntityReadProbeRuntime {
            char_probe,
            vehicle_probe: vehicle_entity_probe,
            object_probe,
        };
        self.process_active_scripts_with_entity_runtime_context_and_effect_flush(
            time_ms,
            time_step_ms,
            time_step,
            general,
            clock,
            camera_probe,
            ground_probe,
            None,
            None,
            Some(&mut entity_runtime),
            player_probe,
            pad_probe,
            None,
            replay_playing_back,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_active_scripts_with_entity_runtime_context_and_effect_flush(
        &mut self,
        time_ms: u32,
        time_step_ms: f32,
        time_step: f32,
        general: &mut CGeneral,
        clock: &mut CClock,
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
        replay_playing_back: bool,
        mut effect_flush: Option<&mut dyn FnMut(&mut CTheScripts)>,
    ) -> Result<Vec<ScriptCommandOutcome>, ScriptVmError> {
        self.process_active_scripts_with_command_executor(
            time_ms,
            time_step_ms,
            replay_playing_back,
            |scripts, script_index| {
                let outcome = scripts.process_one_command_for_script_with_entity_runtime_context(
                    script_index,
                    time_ms,
                    time_step,
                    Some(&mut *general),
                    Some(&mut *clock),
                    camera_probe
                        .as_deref_mut()
                        .map(|probe| probe as &mut dyn FnMut(CVector, f32) -> bool),
                    ground_probe
                        .as_deref_mut()
                        .map(|probe| probe as &mut dyn FnMut(f32, f32) -> f32),
                    ground_3d_probe
                        .as_deref_mut()
                        .map(|probe| probe as &mut dyn FnMut(f32, f32, f32) -> f32),
                    path_node_probe.as_deref_mut().map(|probe| {
                        probe
                            as &mut dyn FnMut(
                                CVector,
                                u8,
                                bool,
                                bool,
                            )
                                -> Option<ScriptPathNodeSnapshot>
                    }),
                    reborrow_script_entity_runtime(&mut entity_runtime),
                    player_probe
                        .as_deref_mut()
                        .map(|probe| probe as &mut dyn FnMut(i32) -> Option<ScriptPlayerSnapshot>),
                    pad_probe
                        .as_deref_mut()
                        .map(|probe| probe as &mut dyn FnMut(i32, i32) -> i16),
                    world_state_probe
                        .as_deref_mut()
                        .map(|probe| probe as &mut dyn FnMut() -> ScriptWorldStateSnapshot),
                )?;
                if let Some(flush) = effect_flush.as_deref_mut() {
                    flush(scripts);
                }
                Ok(outcome)
            },
        )
    }

    /// Run the active-script scheduler while delegating each individual opcode
    /// to a short-lived executor. Production uses this boundary to release
    /// read-only world probes between opcodes and synchronously apply streaming
    /// effects before the following SCM command, matching re3's direct calls.
    pub fn process_active_scripts_with_command_executor<F>(
        &mut self,
        time_ms: u32,
        time_step_ms: f32,
        replay_playing_back: bool,
        execute_command: F,
    ) -> Result<Vec<ScriptCommandOutcome>, ScriptVmError>
    where
        F: FnMut(&mut CTheScripts, usize) -> Result<ScriptCommandOutcome, ScriptVmError>,
    {
        self.process_active_scripts_with_selected_executor(time_ms, time_step_ms,
            replay_playing_back, None, execute_command)
    }

    /// The debug transition may run only the outgoing native cleanup root.
    /// Other live threads retain their timers, wake state and instruction pointer.
    pub(crate) fn process_active_scripts_with_selected_executor<F>(
        &mut self,
        time_ms: u32,
        time_step_ms: f32,
        replay_playing_back: bool,
        only_script: Option<usize>,
        mut execute_command: F,
    ) -> Result<Vec<ScriptCommandOutcome>, ScriptVmError>
    where
        F: FnMut(&mut CTheScripts, usize) -> Result<ScriptCommandOutcome, ScriptVmError>,
    {
        if replay_playing_back {
            return Ok(Vec::new());
        }
        self.process_frame_housekeeping(time_step_ms as u32, false);
        let active_scripts = self.active_scripts.clone();
        let mut outcomes = Vec::new();
        'active_scripts: for script_index in active_scripts {
            if only_script.is_some_and(|selected| selected != script_index)
                || !self.active_scripts.contains(&script_index) {
                continue;
            }
            self.scripts_updated = self.scripts_updated.saturating_add(1);
            {
                let script = self
                    .scripts
                    .get_mut(script_index)
                    .ok_or(ScriptVmError::InvalidScriptIndex(script_index))?;
                script.update_timers(time_step_ms);
            }
            self.do_deatharrest_check(script_index)?;
            {
                let script = self
                    .scripts
                    .get_mut(script_index)
                    .ok_or(ScriptVmError::InvalidScriptIndex(script_index))?;
                if script.mission_flag
                    && self.fail_current_mission == 1
                    && script.stack_pointer == 1
                {
                    script.stack_pointer -= 1;
                    script.set_ip(script.stack[script.stack_pointer as usize]);
                }
            }
            if self.scripts[script_index].skip_message_wait_this_frame {
                self.scripts[script_index].skip_message_wait_this_frame = false;
                continue;
            }
            if self.scripts[script_index].wake_time > time_ms {
                continue;
            }
            // re3 processes continuously until the command yields or terminates.
            loop {
                let command_result = execute_command(self, script_index);
                match command_result {
                    Ok(outcome) => {
                        let intro_streaming_barrier = outcome.command
                            == COMMAND_SET_INTRO_IS_PLAYING
                            && matches!(
                                self.pending_effects.last(),
                                Some(ScriptCommandEffect::SetIntroIsPlaying { playing: false })
                            );
                        let streaming_barrier = matches!(
                            outcome.command,
                            COMMAND_LOAD_SCENE
                                | COMMAND_LOAD_ALL_MODELS_NOW
                                | COMMAND_LOAD_COLLISION
                                | COMMAND_START_CHASE_SCENE
                        ) || intro_streaming_barrier;
                        outcomes.push(outcome);
                        // Both C++ blocking SCM streaming opcodes wrap their
                        // CStreaming call in CTimer::Stop/Update and do not
                        // return to the VM until residency is complete. Bevy
                        // drains them cooperatively, so stop the whole VM slice
                        // at the same command boundary.
                        if streaming_barrier {
                            break 'active_scripts;
                        }
                        if outcome.yielded
                            || outcome.terminated
                            || !self.active_scripts.contains(&script_index)
                        {
                            break;
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        self.dbg_flag = false;
        Ok(outcomes)
    }

    /// re3 `CRunningScript::DoDeatharrestCheck`.
    ///
    /// Mission SCMs keep their fail/cleanup handler in the bottom GOSUB stack
    /// entry. On death, arrest, or a watched upside-down vehicle, the original
    /// VM jumps there before executing the mission's next opcode. That handler
    /// owns mission-failed UI, STORE/RESTORE_CLOCK/WEATHER symmetry, cleanup,
    /// and any mission-specific restart/cutscene flow.
    fn do_deatharrest_check(&mut self, script_index: usize) -> Result<(), ScriptVmError> {
        let script = self
            .scripts
            .get(script_index)
            .ok_or(ScriptVmError::InvalidScriptIndex(script_index))?;
        let should_check = script.is_mission_script && script.deatharrest_enabled;
        if !should_check || !self.is_player_on_a_mission() {
            return Ok(());
        }

        let restarting_after_death = self.player_wb_state == 1;
        let restarting_after_arrest = self.player_wb_state == 2;
        if !restarting_after_death
            && !restarting_after_arrest
            && !self.upside_down_cars.are_any_cars_upside_down()
        {
            return Ok(());
        }

        {
            let script = self
                .scripts
                .get_mut(script_index)
                .ok_or(ScriptVmError::InvalidScriptIndex(script_index))?;
            if script.stack_pointer == 0 {
                return Err(ScriptVmError::StackUnderflow);
            }
            while script.stack_pointer > 1 {
                script.stack_pointer -= 1;
            }
            script.stack_pointer -= 1;
            script.set_ip(script.stack[script.stack_pointer as usize]);
            script.deatharrest_executed = true;
            script.wake_time = 0;
        }

        self.write_i32_at_script_space(self.on_a_mission_flag as usize, 0)?;
        self.pending_effects
            .push(ScriptCommandEffect::ClearSmallPrints);
        Ok(())
    }

    fn refresh_links(&mut self) {
        for script in &mut self.scripts {
            script.next = None;
            script.prev = None;
        }
        let active = self.active_scripts.clone();
        for (pos, script_index) in active.iter().copied().enumerate() {
            self.scripts[script_index].prev = pos.checked_sub(1).map(|p| active[p]);
            self.scripts[script_index].next = active.get(pos + 1).copied();
        }
        let idle = self.idle_scripts.clone();
        for (pos, script_index) in idle.iter().copied().enumerate() {
            self.scripts[script_index].prev = pos.checked_sub(1).map(|p| idle[p]);
            self.scripts[script_index].next = idle.get(pos + 1).copied();
        }
    }
}

impl Default for ScriptSphere {
    fn default() -> Self {
        Self {
            in_use: false,
            index: 1,
            id: 0,
            center: [0.0, 0.0, 0.0],
            radius: 0.0,
        }
    }
}

impl Default for IntroScriptRectangle {
    fn default() -> Self {
        Self {
            is_used: false,
            before_fade: false,
            texture_id: -1,
            rect: [0.0; 4],
            color: [255, 255, 255, 255],
        }
    }
}

impl IntroTextLine {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl Default for IntroTextLine {
    fn default() -> Self {
        Self {
            scale: [0.48, 1.12],
            color: [225, 225, 225, 255],
            justify: false,
            centered: false,
            background: false,
            background_only: false,
            wrap_x: 182.0,
            center_size: 640.0,
            background_color: [128, 128, 128, 128],
            proportional: true,
            before_fade: false,
            right_justify: false,
            font: 1,
            position: [0.0; 2],
            text_key: [0; 8],
        }
    }
}

impl Default for TBuildingSwap {
    fn default() -> Self {
        Self {
            building_ref: None,
            new_model: -1,
            old_model: -1,
        }
    }
}

impl Default for CleanupEntity {
    fn default() -> Self {
        Self {
            entity_type: CLEANUP_UNUSED,
            id: 0,
        }
    }
}

impl CMissionCleanup {
    pub fn new() -> Self {
        let mut cleanup = Self {
            entities: vec![CleanupEntity::default(); MAX_CLEANUP],
            count: 0,
        };
        cleanup.init();
        cleanup
    }

    pub fn init(&mut self) {
        self.count = 0;
        for entity in &mut self.entities {
            entity.entity_type = CLEANUP_UNUSED;
            entity.id = 0;
        }
    }

    pub fn find_free(&self) -> Option<usize> {
        self.entities
            .iter()
            .position(|entity| entity.entity_type == CLEANUP_UNUSED)
    }

    pub fn add_entity_to_list(&mut self, id: i32, entity_type: u8) -> bool {
        let Some(index) = self.find_free() else {
            return false;
        };
        self.entities[index].id = id;
        self.entities[index].entity_type = entity_type;
        self.count = self.count.saturating_add(1);
        true
    }

    pub fn remove_entity_from_list(&mut self, id: i32, entity_type: u8) {
        for entity in &mut self.entities {
            if entity.entity_type == entity_type && entity.id == id {
                entity.id = 0;
                entity.entity_type = CLEANUP_UNUSED;
                self.count = self.count.saturating_sub(1);
            }
        }
    }

    /// re3 `CMissionCleanup::Process` consumes every occupied slot in array
    /// order.  The ECS runtime applies the corresponding `CleanUpThis*`
    /// effects after the VM slice, so preserve the handles before resetting
    /// the fixed cleanup list.
    pub fn take_for_process(&mut self) -> Vec<CleanupEntity> {
        let mut pending = Vec::with_capacity(self.count as usize);
        for entity in &mut self.entities {
            if entity.entity_type == CLEANUP_UNUSED {
                continue;
            }
            pending.push(entity.clone());
            entity.id = 0;
            entity.entity_type = CLEANUP_UNUSED;
        }
        self.count = 0;
        pending
    }
}

impl CUpsideDownCarCheck {
    pub fn new() -> Self {
        let mut check = Self {
            cars: [UpsideDownCarData {
                vehicle_index: -1,
                upside_down_timer: 0,
            }; MAX_UPSIDEDOWN_CAR_CHECKS],
        };
        check.init();
        check
    }

    pub fn init(&mut self) {
        for car in &mut self.cars {
            car.vehicle_index = -1;
            car.upside_down_timer = 0;
        }
    }

    pub fn is_probe_upside_down(probe: ScriptVehicleProbe) -> bool {
        probe.exists
            && probe.up_z <= UPSIDEDOWN_UP_THRESHOLD
            && probe.move_speed_magnitude < UPSIDEDOWN_MOVE_SPEED_THRESHOLD
            && probe.turn_speed_magnitude < UPSIDEDOWN_TURN_SPEED_THRESHOLD
    }

    pub fn update_timers<F>(&mut self, time_step_ms: u32, mut probe_for_vehicle: F)
    where
        F: FnMut(i32) -> ScriptVehicleProbe,
    {
        for car in &mut self.cars {
            if car.vehicle_index < 0 {
                continue;
            }
            let probe = probe_for_vehicle(car.vehicle_index);
            if !probe.exists {
                car.vehicle_index = -1;
                car.upside_down_timer = 0;
            } else if Self::is_probe_upside_down(probe) {
                car.upside_down_timer = car.upside_down_timer.saturating_add(time_step_ms);
            } else {
                car.upside_down_timer = 0;
            }
        }
    }

    pub fn are_any_cars_upside_down(&self) -> bool {
        self.cars
            .iter()
            .any(|car| car.vehicle_index >= 0 && car.upside_down_timer > UPSIDEDOWN_TIMER_THRESHOLD)
    }

    pub fn add_car_to_check(&mut self, id: i32) -> bool {
        let Some(car) = self.cars.iter_mut().find(|car| car.vehicle_index < 0) else {
            return false;
        };
        car.vehicle_index = id;
        car.upside_down_timer = 0;
        true
    }

    pub fn remove_car_from_check(&mut self, id: i32) {
        for car in &mut self.cars {
            if car.vehicle_index == id {
                car.vehicle_index = -1;
                car.upside_down_timer = 0;
            }
        }
    }

    pub fn has_car_been_upside_down_for_a_while(&self, id: i32) -> bool {
        self.cars.iter().any(|car| {
            car.vehicle_index == id && car.upside_down_timer > UPSIDEDOWN_TIMER_THRESHOLD
        })
    }
}

impl StuckCarData {
    pub fn reset(&mut self) {
        self.vehicle_index = -1;
        self.pos = CVector::new(-5000.0, -5000.0, -5000.0);
        self.last_check = -1;
        self.radius = 0.0;
        self.stuck_time = 0;
        self.stuck = false;
    }
}

impl Default for StuckCarData {
    fn default() -> Self {
        let mut data = Self {
            vehicle_index: -1,
            pos: CVector::ZERO,
            last_check: -1,
            radius: 0.0,
            stuck_time: 0,
            stuck: false,
        };
        data.reset();
        data
    }
}

impl CStuckCarCheck {
    pub fn new() -> Self {
        let mut check = Self {
            cars: [StuckCarData::default(); MAX_STUCK_CAR_CHECKS],
        };
        check.init();
        check
    }

    pub fn init(&mut self) {
        for car in &mut self.cars {
            car.reset();
        }
    }

    pub fn process<F>(&mut self, timer_ms: u32, mut probe_for_vehicle: F)
    where
        F: FnMut(i32) -> ScriptVehicleProbe,
    {
        for car in &mut self.cars {
            if car.vehicle_index < 0 {
                continue;
            }
            if timer_ms <= car.stuck_time + car.last_check as u32 {
                continue;
            }
            let probe = probe_for_vehicle(car.vehicle_index);
            if !probe.exists {
                car.reset();
                continue;
            }
            let distance = sub_vec3(probe.position, car.pos).magnitude();
            car.stuck = distance < car.radius;
            car.pos = probe.position;
            car.last_check = timer_ms as i32;
        }
    }

    pub fn add_car_to_check(
        &mut self,
        id: i32,
        radius: f32,
        time: u32,
        timer_ms: u32,
        position: CVector,
    ) -> bool {
        let Some(car) = self.cars.iter_mut().find(|car| car.vehicle_index < 0) else {
            return false;
        };
        car.vehicle_index = id;
        car.pos = position;
        car.last_check = timer_ms as i32;
        car.radius = radius;
        car.stuck_time = time;
        car.stuck = false;
        true
    }

    pub fn remove_car_from_check(&mut self, id: i32) {
        for car in &mut self.cars {
            if car.vehicle_index == id {
                car.reset();
            }
        }
    }

    pub fn has_car_been_stuck_for_a_while(&self, id: i32) -> bool {
        self.cars
            .iter()
            .any(|car| car.vehicle_index == id && car.stuck)
    }
}

pub const SCRIPT_SPACE_LEN: usize = SIZE_SCRIPT_SPACE;

pub(crate) fn face_coord_heading_degrees(position: CVector, target_x: f32, target_y: f32) -> f32 {
    let mut heading =
        get_atan_of_xy(position.x - target_x, position.y - target_y) + std::f32::consts::FRAC_PI_2;
    if heading > std::f32::consts::TAU {
        heading -= std::f32::consts::TAU;
    }
    heading.to_degrees()
}

/// `CGeneral::GetATanOfXY`; preserve its branch order instead of replacing it
/// with a differently oriented `atan2(y, x)` call.
pub(crate) fn get_atan_of_xy(x: f32, y: f32) -> f32 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let x_abs = x.abs();
    let y_abs = y.abs();
    if x_abs < y_abs {
        if y > 0.0 {
            if x > 0.0 {
                std::f32::consts::FRAC_PI_2 - (x / y).atan2(1.0)
            } else {
                std::f32::consts::FRAC_PI_2 + (-x / y).atan2(1.0)
            }
        } else if x > 0.0 {
            1.5 * std::f32::consts::PI + (x / -y).atan2(1.0)
        } else {
            1.5 * std::f32::consts::PI - (-x / -y).atan2(1.0)
        }
    } else if y > 0.0 {
        if x > 0.0 {
            (y / x).atan2(1.0)
        } else {
            std::f32::consts::PI - (y / -x).atan2(1.0)
        }
    } else if x > 0.0 {
        std::f32::consts::TAU - (-y / x).atan2(1.0)
    } else {
        std::f32::consts::PI + (-y / -x).atan2(1.0)
    }
}

pub(crate) fn sub_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

pub(crate) fn remap_script_ped_model(ped_type: i32, model_id: i32) -> i32 {
    // Script.cpp rewrites these model IDs to constructor enums because C++
    // instantiates CCopPed/CEmergencyPed directly. The ECS constructor keeps
    // the ped type and model as separate data, so preserving the authored
    // model ID is the equivalent operation here. Feeding COP_FBI=1 etc. back
    // through the generic model field made mission cops render as the wrong
    // ped (or no ped at all).
    let _ = ped_type;
    model_id
}

pub(crate) fn point_in_area_2d(point: CVector, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
}

pub(crate) fn point_in_area_3d(
    point: CVector,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
) -> bool {
    let min_z = z1.min(z2);
    let max_z = z1.max(z2);
    point_in_area_2d(point, x1, y1, x2, y2) && point.z >= min_z && point.z <= max_z
}

pub(crate) fn locate_player_mode(command: i32) -> Result<LocateMode, ScriptVmError> {
    match command {
        COMMAND_LOCATE_PLAYER_ANY_MEANS_2D
        | COMMAND_LOCATE_PLAYER_ANY_MEANS_3D
        | COMMAND_LOCATE_STOPPED_PLAYER_ANY_MEANS_2D
        | COMMAND_LOCATE_STOPPED_PLAYER_ANY_MEANS_3D
        | COMMAND_LOCATE_PLAYER_ANY_MEANS_CHAR_2D
        | COMMAND_LOCATE_PLAYER_ANY_MEANS_CHAR_3D
        | COMMAND_LOCATE_PLAYER_ANY_MEANS_CAR_2D
        | COMMAND_LOCATE_PLAYER_ANY_MEANS_CAR_3D => Ok(LocateMode::AnyMeans),
        COMMAND_LOCATE_PLAYER_ON_FOOT_2D
        | COMMAND_LOCATE_PLAYER_ON_FOOT_3D
        | COMMAND_LOCATE_STOPPED_PLAYER_ON_FOOT_2D
        | COMMAND_LOCATE_STOPPED_PLAYER_ON_FOOT_3D
        | COMMAND_LOCATE_PLAYER_ON_FOOT_CHAR_2D
        | COMMAND_LOCATE_PLAYER_ON_FOOT_CHAR_3D
        | COMMAND_LOCATE_PLAYER_ON_FOOT_CAR_2D
        | COMMAND_LOCATE_PLAYER_ON_FOOT_CAR_3D => Ok(LocateMode::OnFoot),
        COMMAND_LOCATE_PLAYER_IN_CAR_2D
        | COMMAND_LOCATE_PLAYER_IN_CAR_3D
        | COMMAND_LOCATE_STOPPED_PLAYER_IN_CAR_2D
        | COMMAND_LOCATE_STOPPED_PLAYER_IN_CAR_3D
        | COMMAND_LOCATE_PLAYER_IN_CAR_CHAR_2D
        | COMMAND_LOCATE_PLAYER_IN_CAR_CHAR_3D
        | COMMAND_LOCATE_PLAYER_IN_CAR_CAR_2D
        | COMMAND_LOCATE_PLAYER_IN_CAR_CAR_3D => Ok(LocateMode::InCar),
        _ => Err(ScriptVmError::UnsupportedCommand(command)),
    }
}

pub(crate) fn locate_char_mode(command: i32) -> Result<LocateMode, ScriptVmError> {
    match command {
        COMMAND_LOCATE_CHAR_ANY_MEANS_2D
        | COMMAND_LOCATE_CHAR_ANY_MEANS_3D
        | COMMAND_LOCATE_STOPPED_CHAR_ANY_MEANS_2D
        | COMMAND_LOCATE_STOPPED_CHAR_ANY_MEANS_3D
        | COMMAND_LOCATE_CHAR_ANY_MEANS_CHAR_2D
        | COMMAND_LOCATE_CHAR_ANY_MEANS_CHAR_3D
        | COMMAND_LOCATE_CHAR_ANY_MEANS_CAR_2D
        | COMMAND_LOCATE_CHAR_ANY_MEANS_CAR_3D => Ok(LocateMode::AnyMeans),
        COMMAND_LOCATE_CHAR_ON_FOOT_2D
        | COMMAND_LOCATE_CHAR_ON_FOOT_3D
        | COMMAND_LOCATE_STOPPED_CHAR_ON_FOOT_2D
        | COMMAND_LOCATE_STOPPED_CHAR_ON_FOOT_3D
        | COMMAND_LOCATE_CHAR_ON_FOOT_CHAR_2D
        | COMMAND_LOCATE_CHAR_ON_FOOT_CHAR_3D
        | COMMAND_LOCATE_CHAR_ON_FOOT_CAR_2D
        | COMMAND_LOCATE_CHAR_ON_FOOT_CAR_3D => Ok(LocateMode::OnFoot),
        COMMAND_LOCATE_CHAR_IN_CAR_2D
        | COMMAND_LOCATE_CHAR_IN_CAR_3D
        | COMMAND_LOCATE_STOPPED_CHAR_IN_CAR_2D
        | COMMAND_LOCATE_STOPPED_CHAR_IN_CAR_3D
        | COMMAND_LOCATE_CHAR_IN_CAR_CHAR_2D
        | COMMAND_LOCATE_CHAR_IN_CAR_CHAR_3D
        | COMMAND_LOCATE_CHAR_IN_CAR_CAR_2D
        | COMMAND_LOCATE_CHAR_IN_CAR_CAR_3D => Ok(LocateMode::InCar),
        _ => Err(ScriptVmError::UnsupportedCommand(command)),
    }
}

pub(crate) fn point_in_script_zone(
    position: CVector,
    label: &[u8; KEY_LENGTH_IN_SCRIPT],
    zones: &[ScriptZoneBounds],
) -> bool {
    let end = label
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(label.len());
    zones.iter().any(|zone| {
        let zone_end = zone
            .label
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(zone.label.len());
        zone.label[..zone_end].eq_ignore_ascii_case(&label[..end])
            && position.x >= zone.min.x
            && position.x <= zone.max.x
            && position.y >= zone.min.y
            && position.y <= zone.max.y
            && position.z >= zone.min.z
            && position.z <= zone.max.z
    })
}

pub(crate) fn script_jump_ip(value: i32) -> u32 {
    if value >= 0 {
        value as u32
    } else {
        (SIZE_MAIN_SCRIPT as i32 - value) as u32
    }
}

// === FAZ8a: moved from core/Game.rs (verbatim, parity-preserving) ===
// Script-entity snapshots, script-side entity lifecycle helpers and pickup
// materialization. Schedule stays in core/Game.rs.

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

// S2: snapshot/audio helpers live in control/ScriptSnapshots.rs; re-exported here
pub(crate) use super::ScriptSnapshots::{
    materialize_pickup_object, mission_audio_sample_from_key, queue_ped_script_say,
    remove_materialized_pickup, remove_script_vehicle_and_occupants, script_char_entity_snapshot,
    script_char_say_sound, script_key_to_string, script_object_entity_snapshot,
    script_object_snapshot, script_ped_snapshot_from_entity, script_sound_frontend_sound,
    script_vehicle_base_can_be_deleted, script_vehicle_entity_snapshot,
    script_vehicle_snapshot_from_entity, set_all_occupants_of_car_leave_car,
};

// FAZ7b: moved to animation/CutsceneMgr.rs verbatim (7 items): struct LoadedCutsceneRuntimeData, fn load_cutscene_head_animation, fn load_cutscene_runtime_data, fn restore_after_cutscene_clear, fn apply_cutscene_finish, fn update_cutscene_mgr, fn sync_cutscene_pad_controls

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
use bevy::ecs::system::SystemParam;
#[allow(unused_imports)]
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
#[allow(unused_imports)]
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use re3bevy_assets::streaming::CdStream::CdStream;
#[allow(unused_imports)]
use re3bevy_core::core::GameState::{CGameState, ELevelName};

// Active-script evaluation and same-pass overlays share canonical runtime definitions.
pub(crate) use super::ScriptRuntime::SamePassOverlay;
#[cfg(test)]
pub(crate) use super::ScriptRuntime::process_active_scripts;

// Script domain effects are defined and scheduled by their owning modules.
pub use super::ScriptDomainEffects::apply_script_player_clothing_effects;
pub(crate) use super::ScriptDomainEffects::update_game_logic;
#[cfg(test)]
pub(crate) use super::ScriptDomainEffects::apply_script_pacman_effects;

// Dynamic entity operations use the canonical live ECS access layer.
pub(crate) use super::ScriptDynamicEntities::{
    apply_script_dynamic_entity_effects, apply_script_object_motion,
    clear_exciting_stuff_from_area_runtime, set_script_snapshot_bounds,
};

// Public script application entry points and streaming integration.
pub use super::ScriptWorldEffects::{
    apply_script_effects_to_world, apply_script_effects_to_world_with_streaming,
};
pub(crate) use super::ScriptWorldEffects::{
    flush_immediate_script_streaming_effect,
    refresh_script_streaming_snapshots,
};

// Script-side ECS stores used by the active-script bridge.

// ScriptEntities owns handle identity, stores and actor lifecycle.
pub use super::ScriptEntities::{
    ScriptEntityIndex, ScriptEntityWorld, ScriptGameplayBridge, ScriptObjectEntity, ScriptObjectIdentity, ScriptObjectRef, ScriptObjectMut,
    ScriptObjectStore, ScriptPedEntity, ScriptPedStore, ScriptPickupIndex, ScriptRadarBlipIndex,
    ScriptTaxiLightState, ScriptVehicleEntity, ScriptVehicleStore,
};
pub(crate) use super::ScriptEntities::{
    remove_script_object, script_key_string,
    script_vehicle_is_stopped, spawn_script_object,
};
#[cfg(test)]
pub(crate) use super::ScriptEntities::{
    script_entity_test_world, spawn_script_test_ped, spawn_script_test_vehicle,
    with_script_entity_test_world,
};

#[cfg(test)]
mod streaming_barrier_tests {
    use super::*;
    use re3bevy_formats::formats::Script::ScmFile;

    #[test]
    fn mission_deatharrest_redirects_to_cleanup_handler_before_next_opcode() {
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        scripts.script_space.resize(64, 0);
        scripts.on_a_mission_flag = 4;
        scripts
            .write_i32_at_script_space(scripts.on_a_mission_flag as usize, 1)
            .expect("mission flag");
        scripts.player_wb_state = 1;

        let script_index = scripts.start_new_script(9000).expect("mission script");
        {
            let script = &mut scripts.scripts[script_index];
            script.is_mission_script = true;
            script.mission_flag = true;
            script.stack[0] = 1234;
            script.stack[1] = 5678;
            script.stack_pointer = 2;
            script.wake_time = u32::MAX;
        }

        scripts
            .process_active_scripts_with_command_executor(
                100,
                16.0,
                false,
                |scripts, script_index| {
                    let script = &scripts.scripts[script_index];
                    assert_eq!(script.ip, 1234);
                    assert_eq!(script.stack_pointer, 0);
                    assert_eq!(script.wake_time, 0);
                    assert!(script.deatharrest_executed);
                    assert_eq!(
                        scripts
                            .read_i32_at_script_space(scripts.on_a_mission_flag as usize)
                            .expect("mission flag"),
                        0
                    );
                    Ok(ScriptCommandOutcome {
                        command: COMMAND_WAIT,
                        yielded: true,
                        terminated: false,
                        ip_before: 1234,
                        ip_after: 1236,
                        not_flag: false,
                        script_index,
                    })
                },
            )
            .expect("scheduler");

        assert!(matches!(
            scripts.pending_effects.as_slice(),
            [ScriptCommandEffect::ClearSmallPrints]
        ));
    }

    #[test]
    fn load_scene_stops_the_entire_vm_slice_at_the_opcode_boundary() {
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        scripts.start_new_script(0).expect("first script");
        scripts.start_new_script(0).expect("second script");
        let mut calls = 0usize;

        let outcomes = scripts
            .process_active_scripts_with_command_executor(
                0,
                0.0,
                false,
                |_scripts, script_index| {
                    calls += 1;
                    Ok(ScriptCommandOutcome {
                        command: COMMAND_LOAD_SCENE,
                        yielded: false,
                        terminated: false,
                        ip_before: 0,
                        ip_after: 2,
                        not_flag: false,
                        script_index,
                    })
                },
            )
            .expect("scheduler");

        assert_eq!(
            calls, 1,
            "no other SCM command may run before LoadScene completes"
        );
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].command, COMMAND_LOAD_SCENE);
    }

    #[test]
    fn load_all_models_now_stops_the_entire_vm_slice_at_the_opcode_boundary() {
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        scripts.start_new_script(0).expect("first script");
        scripts.start_new_script(0).expect("second script");
        let mut calls = 0usize;

        let outcomes = scripts
            .process_active_scripts_with_command_executor(
                0,
                0.0,
                false,
                |_scripts, script_index| {
                    calls += 1;
                    Ok(ScriptCommandOutcome {
                        command: COMMAND_LOAD_ALL_MODELS_NOW,
                        yielded: false,
                        terminated: false,
                        ip_before: 0,
                        ip_after: 2,
                        not_flag: false,
                        script_index,
                    })
                },
            )
            .expect("scheduler");

        assert_eq!(
            calls, 1,
            "no other SCM command may run before LoadAllModelsNow completes"
        );
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].command, COMMAND_LOAD_ALL_MODELS_NOW);
    }

    #[test]
    fn collision_and_chase_loads_stop_the_vm_slice_at_the_blocking_cpp_boundary() {
        for command in [COMMAND_LOAD_COLLISION, COMMAND_START_CHASE_SCENE] {
            let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
            scripts.start_new_script(0).expect("first script");
            scripts.start_new_script(0).expect("second script");
            let mut calls = 0usize;

            let outcomes = scripts
                .process_active_scripts_with_command_executor(
                    0,
                    0.0,
                    false,
                    |_scripts, script_index| {
                        calls += 1;
                        Ok(ScriptCommandOutcome {
                            command,
                            yielded: false,
                            terminated: false,
                            ip_before: 0,
                            ip_after: 2,
                            not_flag: false,
                            script_index,
                        })
                    },
                )
                .expect("scheduler");

            assert_eq!(
                calls, 1,
                "blocking command {command} must stop the whole VM slice"
            );
            assert_eq!(outcomes.len(), 1);
            assert_eq!(outcomes[0].command, command);
        }
    }
}
