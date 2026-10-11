//! re3 reference: references/re-GTA/src/save/MemoryCard.cpp
//! re3 reference: references/re-GTA/src/save/MemoryCard.h
//!
//! `CMemoryCard` state shared by boot/load/frontend flow.

use bevy::prelude::{Resource, error};
use re3bevy_core::core::GameState::{ELevelName, GtaAppState};

use super::{Date::CDate, GenericGameStorage::SaveStorageError, PCSave::eSaveStatus};
use super::{
    GenericGameStorage::SaveBlockKind,
    SaveGameBlocks::{
        apply_known_subsystem_blocks, build_post_load_restore_plan_from_generic_load, find_block,
        load_zones_block_into_world,
    },
};

use crate::core::Game::PcSaveLoadParams;

pub const CARD_ONE: usize = 0;
pub const CARD_TWO: usize = 1;
pub const MAX_CARDS: usize = 2;
pub const MAX_SLOTS: usize = 8;
pub const GUFF_FILE_SIZE: usize = 147_096;
pub const SAVE_FILE_SIZE: usize = 201_729;

#[derive(Default)]
struct RestoredVehicleCounts {
    random: i32,
    mission: i32,
    parked: i32,
    permanent: i32,
    law_enforcer: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MCSTATE {
    MCSTATE_OK = 0,
    MCSTATE_NEED_500KB,
    MCSTATE_NEED_200KB,
    MCSTATE_NOCARD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SLOTINFO {
    SLOT_PRESENT = 0,
    SLOT_NOTPRESENT,
    SLOT_CORRUPTED,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCardError {
    ERR_NONE = 0,
    ERR_NOFORMAT = 1,
    ERR_DIRNOENTRY = 2,
    ERR_OPENNOENTRY = 3,
    ERR_DELETENOENTRY = 4,
    ERR_DELETEDENIED = 5,
    ERR_DELETEFAILED = 6,
    ERR_WRITEFULLDEVICE = 7,
    ERR_WRITENOENTRY = 8,
    ERR_WRITEDENIED = 9,
    ERR_FLUSHNOENTRY = 10,
    ERR_WRITEFAILED = 11,
    ERR_FORMATFAILED = 12,
    ERR_FILETABLENOENTRY = 13,
    ERR_DIRFULLDEVICE = 14,
    ERR_DIRBADENTRY = 15,
    ERR_FILEFULLDEVICE = 16,
    ERR_FILENOPATHENTRY = 17,
    ERR_FILEDENIED = 18,
    ERR_FILEUPLIMIT = 19,
    ERR_READNOENTRY = 20,
    ERR_READDENIED = 21,
    ERR_LOADFAILED = 22,
    ERR_SAVEFAILED = 23,
    ERR_DATACORRUPTED = 24,
    ERR_NOROOTDIR = 25,
    NO_ERR_SUCCESS = 26,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCardFileEntry {
    pub entry_name: String,
    pub file_size_byte: u32,
    pub create: CDate,
    pub modify: CDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CMemoryCardInfo {
    pub port: i32,
    pub slot: i32,
    pub card_type: i32,
    pub free: i32,
    pub format: i32,
    pub dir: String,
    pub table: Vec<MemoryCardFileEntry>,
}

impl Default for CMemoryCardInfo {
    fn default() -> Self {
        Self {
            port: 0,
            slot: 0,
            card_type: 0,
            free: 0,
            format: 0,
            dir: String::new(),
            table: Vec::with_capacity(15),
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CMemoryCard {
    pub m_bWantToLoad: bool,
    pub JustLoadedDontFadeInYet: bool,
    pub StillToFadeOut: bool,
    pub b_FoundRecentSavedGameWantToLoad: bool,
    pub TimeStartedCountingForFade: u32,
    pub TimeToStayFadedBeforeFadeOut: u32,
    pub LastBlockSize: u32,
    pub ValidSaveName: String,
    pub MostRecentFile: String,
    pub SaveFileNameJustSaved: String,
    pub pErrorMsg: String,
    pub nError: MemoryCardError,
    pub LoadFileName: String,
    pub CompileDateAndTime: CDate,
    pub m_LanguageToLoad: i32,
    pub m_LevelToLoad: ELevelName,
    pub CurrentCard: usize,
    pub Cards: [CMemoryCardInfo; MAX_CARDS],
    pub Slots: [SLOTINFO; MAX_SLOTS],
    pub SlotFileName: [String; MAX_SLOTS],
    pub SlotSaveDate: [String; MAX_SLOTS],
}

impl Default for CMemoryCard {
    fn default() -> Self {
        Self::new()
    }
}

impl CMemoryCard {
    pub fn new() -> Self {
        Self {
            m_bWantToLoad: false,
            JustLoadedDontFadeInYet: false,
            StillToFadeOut: false,
            b_FoundRecentSavedGameWantToLoad: false,
            TimeStartedCountingForFade: 0,
            TimeToStayFadedBeforeFadeOut: 1750,
            LastBlockSize: 0,
            ValidSaveName: String::new(),
            MostRecentFile: String::new(),
            SaveFileNameJustSaved: String::new(),
            pErrorMsg: String::new(),
            nError: MemoryCardError::ERR_NONE,
            LoadFileName: String::new(),
            CompileDateAndTime: CDate::default(),
            m_LanguageToLoad: 0,
            m_LevelToLoad: ELevelName::Generic,
            CurrentCard: CARD_ONE,
            Cards: std::array::from_fn(|_| CMemoryCardInfo::default()),
            Slots: [SLOTINFO::SLOT_NOTPRESENT; MAX_SLOTS],
            SlotFileName: std::array::from_fn(|_| String::new()),
            SlotSaveDate: std::array::from_fn(|_| String::new()),
        }
    }

    pub fn Init(&mut self) -> i32 {
        *self = Self::new();
        MemoryCardError::NO_ERR_SUCCESS as i32
    }

    pub fn GetError(&self) -> MemoryCardError {
        self.nError
    }

    pub fn GetErrorMessage(&self) -> &str {
        &self.pErrorMsg
    }

    pub fn GetLanguageToLoad(&self) -> i32 {
        self.m_LanguageToLoad
    }

    pub fn GetLevelToLoad(&self) -> ELevelName {
        self.m_LevelToLoad
    }

    pub fn GetInfoOnSpecificSlot(&self, slot_id: usize) -> SLOTINFO {
        self.Slots
            .get(slot_id)
            .copied()
            .unwrap_or(SLOTINFO::SLOT_NOTPRESENT)
    }

    pub fn GetDateAndTimeOfSavedGame(&self, slot_id: usize) -> Option<&str> {
        self.SlotSaveDate.get(slot_id).map(String::as_str)
    }

    pub fn GetNameOfSavedGame(&self, slot_id: usize) -> Option<&str> {
        self.SlotFileName.get(slot_id).map(String::as_str)
    }

    pub fn LoadSlotToBuffer(&mut self, slot_id: usize) {
        self.LoadFileName = format!("BASLUS-00385GTA3_{}", slot_id + 1);
        self.m_bWantToLoad = true;
    }

    pub fn DeleteSlot(&mut self, slot_id: usize) {
        if let Some(slot) = self.Slots.get_mut(slot_id) {
            *slot = SLOTINFO::SLOT_NOTPRESENT;
        }
        if let Some(name) = self.SlotFileName.get_mut(slot_id) {
            name.clear();
        }
        if let Some(date) = self.SlotSaveDate.get_mut(slot_id) {
            date.clear();
        }
    }

    pub fn CheckCardStateAtGameStartUp(&self, _card_id: usize) -> MCSTATE {
        MCSTATE::MCSTATE_OK
    }

    pub fn ClearError(&mut self) {
        self.nError = MemoryCardError::NO_ERR_SUCCESS;
        self.pErrorMsg.clear();
    }

    pub fn SetError(&mut self, error: MemoryCardError) {
        self.nError = error;
        self.pErrorMsg = memory_card_error_message(error).to_owned();
    }

    pub fn ApplyPcSaveStatus(&mut self, status: eSaveStatus) {
        match status {
            eSaveStatus::SAVESTATUS_SUCCESSFUL => self.ClearError(),
            eSaveStatus::SAVESTATUS_ERR_DATA_INVALID => {
                self.SetError(MemoryCardError::ERR_DATACORRUPTED)
            }
            eSaveStatus::SAVESTATUS_ERR_LOAD_OPEN
            | eSaveStatus::SAVESTATUS_ERR_LOAD_READ
            | eSaveStatus::SAVESTATUS_ERR_LOAD_CLOSE => {
                self.SetError(MemoryCardError::ERR_LOADFAILED)
            }
            eSaveStatus::SAVESTATUS_ERR_SAVE_CREATE
            | eSaveStatus::SAVESTATUS_ERR_SAVE_WRITE
            | eSaveStatus::SAVESTATUS_ERR_SAVE_CLOSE => {
                self.SetError(MemoryCardError::ERR_SAVEFAILED)
            }
            eSaveStatus::SAVESTATUS_DELETEFAILED8
            | eSaveStatus::SAVESTATUS_DELETEFAILED9
            | eSaveStatus::SAVESTATUS_DELETEFAILED10 => {
                self.SetError(MemoryCardError::ERR_DELETEFAILED)
            }
        }
    }

    pub fn ApplySaveStorageResult<T>(
        &mut self,
        result: &Result<T, SaveStorageError>,
        fallback_status: eSaveStatus,
    ) {
        match result {
            Ok(_) => self.ClearError(),
            Err(SaveStorageError::ChecksumMismatch { .. })
            | Err(SaveStorageError::MissingChecksum)
            | Err(SaveStorageError::InvalidBlockOrder { .. })
            | Err(SaveStorageError::MissingBlock { .. })
            | Err(SaveStorageError::BlockTooLarge { .. })
            | Err(SaveStorageError::WrongBlockCount { .. })
            | Err(SaveStorageError::SaveImageTooLarge { .. })
            | Err(SaveStorageError::Buffer(_)) => self.SetError(MemoryCardError::ERR_DATACORRUPTED),
            Err(SaveStorageError::Io(_)) => self.ApplyPcSaveStatus(fallback_status),
        }
    }
}

pub fn memory_card_error_message(error: MemoryCardError) -> &'static str {
    match error {
        MemoryCardError::ERR_NONE | MemoryCardError::NO_ERR_SUCCESS => "",
        MemoryCardError::ERR_DATACORRUPTED => "FESZ_CR",
        MemoryCardError::ERR_LOADFAILED => "FES_LOE",
        MemoryCardError::ERR_SAVEFAILED => "FESZ_SE",
        MemoryCardError::ERR_DELETEFAILED => "FES_DEE",
        MemoryCardError::ERR_NOFORMAT => "SLONNF",
        MemoryCardError::ERR_DIRNOENTRY
        | MemoryCardError::ERR_OPENNOENTRY
        | MemoryCardError::ERR_DELETENOENTRY
        | MemoryCardError::ERR_WRITENOENTRY
        | MemoryCardError::ERR_READNOENTRY => "SLONNO",
        MemoryCardError::ERR_WRITEFULLDEVICE
        | MemoryCardError::ERR_DIRFULLDEVICE
        | MemoryCardError::ERR_FILEFULLDEVICE => "SLNSP",
        _ => "SLONDR",
    }
}

// === FAZ10b: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn process_pending_pc_save_load(mut params: PcSaveLoadParams) {
    if !params.memory_card.m_bWantToLoad {
        return;
    }

    let Some(mut before_rw) = params.before_rw else {
        params
            .memory_card
            .ApplyPcSaveStatus(eSaveStatus::SAVESTATUS_ERR_LOAD_OPEN);
        return;
    };

    let slot = params
        .menu
        .as_ref()
        .and_then(|menu| usize::try_from(menu.curr_save_slot).ok())
        .filter(|slot| *slot < crate::save::GenericGameStorage::SLOT_COUNT)
        .or_else(|| pc_save_slot_from_memory_card_name(&params.memory_card.LoadFileName));
    let Some(slot) = slot else {
        params
            .memory_card
            .ApplyPcSaveStatus(eSaveStatus::SAVESTATUS_ERR_DATA_INVALID);
        params.memory_card.m_bWantToLoad = false;
        return;
    };

    let load = match params.pc_save.LoadSlotGeneric(&mut params.storage, slot) {
        Ok(load) => {
            params.memory_card.ClearError();
            load
        }
        Err(error) => {
            error!("C_PcSave load: GenericLoad failed: {error}");
            params
                .memory_card
                .ApplySaveStorageResult::<()>(&Err(error), eSaveStatus::SAVESTATUS_ERR_LOAD_OPEN);
            params.memory_card.m_bWantToLoad = false;
            return;
        }
    };

    params.game_state.curr_level = load.simple_vars.curr_level;

    if let Some(scripts) = params.scripts.as_deref_mut() {
        if let Err(error) = scripts.load_all_scripts_with_world(&load.scripts, &mut params.file_loader.world) {
            error!("C_PcSave load: script block apply failed: {error}");
            params
                .memory_card
                .SetError(crate::save::MemoryCard::MemoryCardError::ERR_DATACORRUPTED);
            params.memory_card.m_bWantToLoad = false;
            return;
        }
    } else {
        error!("C_PcSave load: CTheScripts resource is unavailable");
        params
            .memory_card
            .ApplyPcSaveStatus(eSaveStatus::SAVESTATUS_ERR_LOAD_READ);
        params.memory_card.m_bWantToLoad = false;
        return;
    }

    if let Some(clock) = params.clock.as_deref_mut() {
        clock.restore_save_state(
            load.simple_vars.game_clock_hours,
            load.simple_vars.game_clock_minutes,
            load.simple_vars.milliseconds_per_game_minute.max(1) as u32,
            load.simple_vars.last_clock_tick as u32,
        );
    }
    if let Some(timer) = params.timer.as_deref_mut() {
        timer.set_time_in_milliseconds(load.simple_vars.timer_ms);
        timer.set_time_scale(load.simple_vars.time_scale);
        timer.set_time_step(load.simple_vars.time_step);
        timer.set_time_step_non_clipped(load.simple_vars.time_step_non_clipped);
        timer.set_frame_counter(load.simple_vars.frame_counter);
    }
    if let Some(camera) = params.camera.as_deref_mut() {
        camera.position = load.simple_vars.camera_position;
        camera.car_zoom_indicator = load.simple_vars.car_zoom_indicator as i32;
        camera.ped_zoom_indicator = load.simple_vars.ped_zoom_indicator as i32;
    }
    if let Some(pads) = params.pads.as_deref_mut() {
        pads.get_pad_mut(0).mode = load.simple_vars.pad_mode;
    }
    if let Some(weather) = params.weather.as_deref_mut() {
        weather.old_weather_type = load.simple_vars.old_weather_type;
        weather.new_weather_type = load.simple_vars.new_weather_type;
        weather.forced_weather_type = load.simple_vars.forced_weather_type;
        weather.interpolation_value = load.simple_vars.weather_interpolation;
        weather.weather_type_in_list = i32::from(load.simple_vars.weather_type_in_list);
    }

    let apply_result = {
        let file_loader = &mut *params.file_loader;
        apply_known_subsystem_blocks(
            &load.subsystem_blocks,
            &mut file_loader.path_find,
            params
                .restart
                .as_deref_mut()
                .expect("CRestart is initialised before LoadingGame"),
            params.gangs.as_deref_mut(),
            params.stats.as_deref_mut(),
            Some(&mut file_loader.radar),
            Some(&mut *params.streaming),
            params.car_generators.as_deref_mut(),
            params.phone_info.as_deref_mut(),
            params.ped_type.as_deref_mut(),
            params.audio_script_objects.as_deref_mut(),
            params.player_info.as_deref_mut(),
            params.pickups.as_deref_mut(),
            params.garages.as_deref_mut(),
            None,
            params.particles.as_deref_mut(),
            params.cranes.as_deref_mut(),
            params.pools.as_deref_mut(),
        )
    };

    if let Err(error) = apply_result {
        error!("C_PcSave load: subsystem block apply failed: {error}");
        params
            .memory_card
            .SetError(crate::save::MemoryCard::MemoryCardError::ERR_DATACORRUPTED);
        params.memory_card.m_bWantToLoad = false;
        return;
    }

    if let Some(block) = find_block(&load.subsystem_blocks, SaveBlockKind::Zones) {
        if let Err(error) = load_zones_block_into_world(block, &mut params.file_loader.world.zones)
        {
            error!("C_PcSave load: world zones apply failed: {error}");
            params
                .memory_card
                .SetError(crate::save::MemoryCard::MemoryCardError::ERR_DATACORRUPTED);
            params.memory_card.m_bWantToLoad = false;
            return;
        }
    }

    let Some(mut pools) = params.pools else {
        params
            .memory_card
            .ApplyPcSaveStatus(eSaveStatus::SAVESTATUS_ERR_LOAD_READ);
        params.memory_card.m_bWantToLoad = false;
        return;
    };

    let restore_plan = build_post_load_restore_plan_from_generic_load(
        &pools,
        params.pickups.as_deref().map(|pickups| &*pickups),
        &load,
    );
    let save_runtime = restore_plan.into_runtime_state();
    save_runtime.apply_to_pools(&mut pools);
    if let Some(replay) = save_runtime.replay_runtime.clone() {
        *params.replay_runtime = replay;
    }

    // CPools::LoadPedPool requests the saved special model before creating
    // the player. Preserve installation DIR/DFF/TXD resolution through streaming.
    for (_, record) in pools.compatible_ped_records() {
        let end = record.model_name.iter().position(|byte| *byte == 0)
            .unwrap_or(record.model_name.len());
        let name = match std::str::from_utf8(&record.model_name[..end]) {
            Ok(name) if !name.is_empty() => name,
            _ => {
                error!(target: "re3bevy::save", "CPools::LoadPedPool: invalid special model name");
                params.memory_card.SetError(MemoryCardError::ERR_DATACORRUPTED);
                params.memory_card.m_bWantToLoad = false;
                return;
            }
        };
        let model_index = record.model_index as usize;
        let known_name = params.file_loader.model_info.get_model_info(model_index)
            .map(|model| model.base().name.as_str());
        if known_name.is_none()
            || (!known_name.is_some_and(|known| known.eq_ignore_ascii_case(name))
                && params.streaming.extra_objects_dir.find_item(name).is_none())
        {
            error!(target: "re3bevy::save", "CPools::LoadPedPool: original special model '{name}' is unavailable");
            params.memory_card.ApplyPcSaveStatus(eSaveStatus::SAVESTATUS_ERR_LOAD_READ);
            params.memory_card.m_bWantToLoad = false;
            return;
        }
        params.streaming.request_special_model(
            model_index, name, crate::core::Streaming::STREAMFLAGS_DONT_REMOVE,
            &mut *params.file_loader,
        );
    }

    for (_, record) in pools.compatible_vehicle_records() {
        params.streaming.request_model_with_adapter(
            record.model_index as usize, crate::core::Streaming::STREAMFLAGS_DEPENDENCY,
            &*params.file_loader,
        );
    }
    for (_, record) in pools.compatible_object_records() {
        params.streaming.request_model_with_adapter(
            record.model_index as usize, crate::core::Streaming::STREAMFLAGS_DEPENDENCY,
            &*params.file_loader,
        );
    }
    if let Err(error) = params.file_loader.load_all_requested_models(
        &mut params.streaming, &mut before_rw.cd_stream, false,
    ) {
        error!(target: "re3bevy::save", "CPools::Load*Pool original model streaming failed: {error}");
        params.memory_card.ApplyPcSaveStatus(eSaveStatus::SAVESTATUS_ERR_LOAD_READ);
        params.memory_card.m_bWantToLoad = false;
        return;
    }

    if let Err(error) = save_runtime.execute_post_load_commands(
        &mut params.file_loader,
        &mut params.streaming,
        &mut before_rw.cd_stream,
    ) {
        error!("C_PcSave load: RestoreForStartLoad command failed: {error}");
        params
            .memory_card
            .ApplyPcSaveStatus(eSaveStatus::SAVESTATUS_ERR_LOAD_READ);
        params.memory_card.m_bWantToLoad = false;
        return;
    }

    let mut restored_car_counts = RestoredVehicleCounts::default();
    for (_, record) in pools.compatible_vehicle_records() {
        match record.base.vehicle_created_by {
            crate::vehicles::Vehicle::RANDOM_VEHICLE => restored_car_counts.random += 1,
            crate::vehicles::Vehicle::MISSION_VEHICLE => restored_car_counts.mission += 1,
            crate::vehicles::Vehicle::PARKED_VEHICLE => restored_car_counts.parked += 1,
            crate::vehicles::Vehicle::PERMANENT_VEHICLE => restored_car_counts.permanent += 1,
            _ => {},
        }
        if record.base.is_law_enforcer { restored_car_counts.law_enforcer += 1; }
    }
    if let Err(reason) = super::SavePoolRuntime::restore_live_vehicle_and_object_pools(
        &mut pools, &mut params.script_entities, &params.file_loader, &params.handling,
        &mut params.general, load.simple_vars.timer_ms, params.taxi_lights.all_enabled,
    ) {
        error!(target: "re3bevy::save", "CPools::LoadVehiclePool/LoadObjectPool failed: {reason}");
        params.memory_card.SetError(MemoryCardError::ERR_DATACORRUPTED);
        params.memory_card.m_bWantToLoad = false;
        return;
    }
    if let Some(scripts) = params.scripts.as_deref_mut()
        && let Err(reason) = scripts.resolve_loaded_object_invisibility(
            &pools,
            &mut params.script_entities,
        )
    {
        error!(target: "re3bevy::save", "CTheScripts::LoadAllScripts object invisibility restore failed: {reason}");
        params.memory_card.SetError(MemoryCardError::ERR_DATACORRUPTED);
        params.memory_card.m_bWantToLoad = false;
        return;
    }
    if let Some(car_ctrl) = params.car_ctrl.as_deref_mut() {
        car_ctrl.num_random_cars = restored_car_counts.random;
        car_ctrl.num_mission_cars = restored_car_counts.mission;
        car_ctrl.num_parked_cars = restored_car_counts.parked;
        car_ctrl.num_permanent_cars = restored_car_counts.permanent;
        car_ctrl.num_law_enforcer_cars = restored_car_counts.law_enforcer;
    }
    if let Err(reason) = super::SaveRuntime::restore_live_player_pool(
        &mut pools, &mut params.world_players, &mut params.general,
        params.ped_type.as_deref(),
    ) {
        error!(target: "re3bevy::save", "CPools::LoadPedPool failed: {reason}");
        params.memory_card.SetError(MemoryCardError::ERR_DATACORRUPTED);
        params.memory_card.m_bWantToLoad = false;
        return;
    }
    if let Some(player) = params.world_players.player(0) {
        params.script_bridge.player_spawned = true;
        params.script_bridge.player_index = 0;
        params.script_bridge.player_position = player.ped.physical.entity.get_position();
        params.script_bridge.player_visible = true;
        params.script_bridge.player_control_enabled = true;
        params.script_bridge.playing_intro = false;
        params.script_bridge.cutscene_requested = false;
        params.script_bridge.cutscene_started = false;
    }
    params.world_players.flush_pending();
    if let Some(camera) = params.camera.as_deref_mut() {
        camera.restore_with_jump_cut();
    }

    params.memory_card.m_LevelToLoad = load.simple_vars.curr_level;
    params.memory_card.m_bWantToLoad = false;
    params.memory_card.JustLoadedDontFadeInYet = true;
    params.game_state.playing_intro = false;
    if let Some(mut menu) = params.menu {
        menu.menu_active = false;
        menu.game_not_loaded = false;
        menu.menu_state_changed = true;
    }
    params.next_state.set(GtaAppState::InGame);
}

pub(crate) fn pc_save_slot_from_memory_card_name(name: &str) -> Option<usize> {
    let suffix = name.rsplit('_').next()?;
    let one_based = suffix.parse::<usize>().ok()?;
    one_based.checked_sub(1)
}

// FAZ10a: moved to control/Replay.rs verbatim (3 items): fn service_replay_playback_runtime, fn clear_replay_runtime_entities, fn apply_replay_entity_mutations
