//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! SCM value/type definitions shared by the script VM, opcode modules and script effects.

use crate::entities::EntityReference::EntityReference;
use bevy::prelude::{Component, Resource};
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};
use re3bevy_formats::formats::Script::MAX_NUM_MISSION_SCRIPTS;

use crate::weapons::WeaponType::WEAPONTYPE_TOTAL_INVENTORY_WEAPONS;
use crate::{
    control::Garages::NUM_GARAGES,
    core::Fire::{NUM_FIRES, ScriptFireTarget},
    core::Streaming::MAXVEHICLESLOADED,
};

pub use super::ScriptCommands::*;

use super::Script::CRunningScript;
pub const MAX_NUM_SCRIPTS: usize = 128;
pub const MAX_NUM_CONTACTS: usize = 16;
pub const MAX_NUM_INTRO_TEXT_LINES: usize = 2;
pub const MAX_NUM_INTRO_RECTANGLES: usize = 16;
pub const MAX_NUM_SCRIPT_SPRITES: usize = 16;
pub const MAX_NUM_SCRIPT_SPHERES: usize = 16;
pub const MAX_NUM_COLLECTIVES: usize = 32;
pub const MAX_NUM_USED_OBJECTS: usize = 200;
pub const MAX_NUM_BUILDING_SWAPS: usize = 25;
pub const MAX_NUM_INVISIBILITY_SETTINGS: usize = 20;
pub const MAX_NUM_STORED_LINES: usize = 1024;
pub const MAX_STACK_DEPTH: usize = 6;
pub const NUM_LOCAL_VARS: usize = 16;
pub const NUM_TIMERS: usize = 2;
pub const MAX_CLEANUP: usize = 50;
pub const MAX_UPSIDEDOWN_CAR_CHECKS: usize = 6;
pub const MAX_STUCK_CAR_CHECKS: usize = 6;
pub const KEY_LENGTH_IN_SCRIPT: usize = 8;
pub const UPSIDEDOWN_UP_THRESHOLD: f32 = -0.97;
pub const UPSIDEDOWN_MOVE_SPEED_THRESHOLD: f32 = 0.01;
pub const UPSIDEDOWN_TURN_SPEED_THRESHOLD: f32 = 0.02;
pub const UPSIDEDOWN_TIMER_THRESHOLD: u32 = 1000;
pub const MAP_Z_LOW_LIMIT: f32 = -100.0;

pub const CLEANUP_UNUSED: u8 = 0;
pub const CLEANUP_CAR: u8 = 1;
pub const CLEANUP_CHAR: u8 = 2;
pub const CLEANUP_OBJECT: u8 = 3;

pub const ARGUMENT_END: u8 = 0;
pub const ARGUMENT_INT32: u8 = 1;
pub const ARGUMENT_GLOBALVAR: u8 = 2;
pub const ARGUMENT_LOCALVAR: u8 = 3;
pub const ARGUMENT_INT8: u8 = 4;
pub const ARGUMENT_INT16: u8 = 5;
pub const ARGUMENT_FLOAT: u8 = 6;

pub const VAR_LOCAL: i16 = 1;
pub const VAR_GLOBAL: i16 = 2;

pub const ANDOR_NONE: u16 = 0;
pub const ANDS_1: u16 = 1;
pub const ANDS_8: u16 = 8;
pub const ORS_1: u16 = 21;
pub const ORS_8: u16 = 28;

#[derive(Debug, Clone, Resource)]
pub struct CTheScripts {
    pub script_space: Vec<u8>,
    pub main_scm_bytes: Vec<u8>,
    pub loaded_main_bytes: usize,
    /// re3 `GetSizeOfVariableSpace()` — raw i32 at script-space offset 3.
    pub variable_space_size: usize,
    pub idle_script_count: usize,
    pub active_script_count: usize,
    pub store_vehicle_index: i32,
    pub store_vehicle_was_random: bool,
    pub on_a_mission_flag: u32,
    pub base_brief_id_for_contact: [i32; MAX_NUM_CONTACTS],
    pub on_a_mission_for_contact_flag: [i32; MAX_NUM_CONTACTS],
    pub collectives: Vec<TCollectiveData>,
    pub next_free_collective_index: usize,
    pub last_random_ped_id: i32,
    pub used_objects: Vec<TUsedObject>,
    pub number_of_used_objects: u16,
    pub already_running_a_mission_script: bool,
    pub using_a_multi_script_file: bool,
    pub multi_script_array: [i32; MAX_NUM_MISSION_SCRIPTS],
    pub number_of_mission_scripts: u16,
    /// Source MAIN.SCM mission index, retained after the mission terminates so
    /// debug navigation never has to compare mutable VM script-space bytes.
    pub last_loaded_mission_index: Option<i32>,
    pub largest_mission_script_size: u32,
    pub main_script_size: u32,
    pub fail_current_mission: u8,
    pub countdown_to_make_player_unsafe: u8,
    pub dbg_flag: bool,
    pub delay_making_player_unsafe_this_time: u8,
    pub num_script_debug_lines: u16,
    pub script_spheres: Vec<ScriptSphere>,
    pub intro_text_lines: Vec<IntroTextLine>,
    pub number_of_intro_text_lines_this_frame: u16,
    pub use_text_commands: u8,
    pub intro_rectangles: Vec<IntroScriptRectangle>,
    pub number_of_intro_rectangles_this_frame: u16,
    /// re3 `CTheScripts::ScriptSprites` texture-dictionary state.
    /// The VM owns the authored slot/name mapping; Bevy owns decoded Images.
    pub script_texture_dictionary: Option<[u8; KEY_LENGTH_IN_SCRIPT]>,
    pub script_sprite_names: [[u8; KEY_LENGTH_IN_SCRIPT]; MAX_NUM_SCRIPT_SPRITES],
    pub building_swaps: Vec<TBuildingSwap>,
    pub invisibility_settings: Vec<Option<ScriptInvisibilityTarget>>,
    pub scripts: Vec<CRunningScript>,
    pub idle_scripts: Vec<usize>,
    pub active_scripts: Vec<usize>,
    pub mission_cleanup: CMissionCleanup,
    pub upside_down_cars: CUpsideDownCarCheck,
    pub stuck_cars: CStuckCarCheck,
    pub script_params: [i32; 32],
    pub pending_effects: Vec<ScriptCommandEffect>,
    pub commands_executed: u16,
    pub scripts_updated: u16,
    /// Last VM fault seen by the runtime. Successful SCM side effects before
    /// the failing opcode are already live, so the world bridge uses this as
    /// a one-shot signal to restore transient cutscene/streaming state.
    pub last_vm_error: Option<ScriptVmError>,
    pub vm_fault_recovery_pending: bool,
    pub object_names_read: bool,
    pub object_indices_updated: bool,
    pub multiscript_offsets_read: bool,
    /// Script-visible pickup handles awaiting CPickups generation-handle mapping.
    pub next_script_pickup_handle: i32,
    /// Provisional handles whose mapped CPickups entries appeared in
    /// `aPickUpsCollected`; consumed by HAS_PICKUP_BEEN_COLLECTED.
    pub collected_pickup_handles: Vec<i32>,
    /// Synthetic CPools object handle for CREATE_OBJECT until full CWorld object pool is wired.
    pub next_script_object_handle: i32,
    /// Synthetic CPools audio-script-object handle for ADD_CONTINUOUS_SOUND.
    pub next_audio_script_object_handle: i32,
    /// Provisional SCM radar handle mapped to CRadar's generation handle on apply.
    pub next_script_blip_handle: i32,
    /// Cutscene object slot handle returned to SCM until CPools object handles are wired.
    pub next_cutscene_object_handle: i32,
    /// Model clones captured at the exact SCM CREATE_CUTSCENE_* command boundary.
    /// re3's CEntity::SetModelIndex calls CreateInstance immediately; special
    /// cutscene model slots (185..189) may be replaced again before our deferred
    /// ECS/world-effect pass runs, so the live model slot is no longer a valid
    /// identity for an already-created cutscene object.
    pub(crate) pending_cutscene_model_snapshots: Vec<(
        i32,
        crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot,
    )>,
    /// Last known CCutsceneMgr::GetCutsceneTimeInMilleseconds value for SCM reads.
    pub cutscene_time_ms: i32,
    /// re3 `CCutsceneMgr::HasCutsceneFinished` — TheCamera position along spline == 1.
    pub cutscene_finished: bool,
    /// re3 ped-pool style handles for player chars (GET_PLAYER_CHAR).
    pub player_char_handles: [i32; 2],
    /// Last known TheCamera.GetFading() for GET_FADING_STATUS.
    pub camera_fading: bool,
    /// Last known DMAudio.GetMissionAudioLoadingStatus() for SCM reads.
    pub mission_audio_loading_status: u8,
    /// Last known DMAudio.IsMissionAudioSampleFinished() for SCM reads.
    pub mission_audio_finished: bool,
    /// Last known phone message displayed state for `HAS_PHONE_DISPLAYED_MESSAGE`.
    pub phone_message_displayed: [bool; 50],
    /// Script-pass mirror of CPhoneInfo. Phone opcodes are synchronous in re3,
    /// while the Bevy world mutation is deferred until the script-effect pass.
    pub phone_positions: [CVector; 50],
    pub phone_states: [u8; 50],
    pub phone_max: i32,
    pub script_phones_max: i32,
    pub phone_displaying_message: bool,
    pub phone_displaying_message_slot: i32,
    /// CStreaming special-character slots MI_SPECIAL01..04.
    pub special_char_loaded: [bool; 4],
    /// Frame-start `CStreaming::HasModelLoaded` mirror. SCM may request any
    /// streamed DFF id, including map objects above `NUM_DEFAULT_MODELS`
    /// (e.g. Luigi's Club uses 243/256/257).
    pub model_loaded: [bool; crate::core::Streaming::MODELINFOSIZE],
    /// SCM-visible reservations for re3's fixed `gFireManager.m_aFires` pool.
    pub script_fire_slots: [bool; NUM_FIRES],
    /// Provisional CREATE_CAR_GENERATOR index counter (mirrors NumOfCarGenerators).
    pub car_generators_created: i32,
    /// CGarages starts with the crusher in slot zero.
    pub garages_created: i32,
    /// Frame-start `CGarage::m_eGarageState` mirror. Commands also update it
    /// synchronously so a following predicate in the same SCM slice observes
    /// the same state that re3's direct C++ calls expose.
    pub garage_states: [u8; NUM_GARAGES],
    pub garage_respray_happened: [bool; NUM_GARAGES],
    /// Per-garage `CarTypesCollected` mask selected through that garage's type.
    pub garage_collection_masks: [i32; NUM_GARAGES],
    pub crushed_car_id: i32,
    /// Exact live COL-sphere containment pairs for
    /// `CGarages::IsThisCarWithinGarageArea`.
    pub garage_vehicle_containment: Vec<ScriptGarageVehicleContainment>,
    /// Frame-start mirror of CWorld::Players[PlayerInFocus].m_WBState.
    pub player_wb_state: i8,
    pub collected_packages: i32,
    /// Frame-start mirrors of CDarkel globals used by synchronous SCM read opcodes.
    pub kill_frenzy_status: u16,
    pub models_killed_by_player: [u16; crate::modelinfo::ModelIndices::NUM_DEFAULT_MODELS],
    /// Frame-start mirror of `CPedType::ms_apPedType[].m_threats` for
    /// synchronous SCM comparison commands.
    pub ped_type_threats: [u32; crate::peds::PedType::NUM_PEDTYPES],
    /// Frame mirrors of CGame regional flags for synchronous script predicates.
    pub nasty_game: bool,
    pub french_game: bool,
    pub german_game: bool,
    pub credits_done: bool,
    /// Mirror of `CVehicle::bAllDodosCheat || CVehicle::bCheat3`.
    pub car_cheats_activated: bool,
    /// Frame-start mirror of `CPlayerInfo::IsPlayerInRemoteMode`.
    pub player_in_remote_mode: bool,
    pub collision_in_memory: i32,
    /// Frame-start mirrors used by `CREATE_RANDOM_CAR_FOR_CAR_PARK`.
    pub loaded_vehicle_models: [i32; MAXVEHICLESLOADED],
    pub num_random_cars: i32,
    pub drug_run_completed: bool,
    pub drug_plane_shot_down: bool,
    pub drug_plane_position: Option<CVector>,
    pub drop_off_plane_shot_down: bool,
    pub drop_off_plane_position: Option<CVector>,
    pub catalina_heli_shot_down: bool,
    pub catalina_heli_handle: i32,
    /// Frame-start mirrors of `CPacManPickups` and the current player
    /// vehicle. Mutating opcodes update these immediately so subsequent
    /// commands in the same SCM slice observe re3's direct-call semantics.
    pub pacman_pills_eaten: i32,
    pub pacman_pills_carried: i32,
    /// Frame-start weapon-pool mirrors for synchronous area predicates.
    pub projectile_snapshots: Vec<(u8, CVector)>,
    pub explosion_snapshots: Vec<(i32, CVector)>,
    /// Runtime mirror of CTheZones navigation bounds for synchronous predicates.
    pub zone_bounds: Vec<ScriptZoneBounds>,
    /// Mission-created ped/vehicle/object origin spheres used by
    /// `CWorld::FindMissionEntitiesIntersectingCube`.
    pub mission_entity_bounds: Vec<ScriptMissionEntityBounds>,
    /// Live entity origin spheres grouped by re3 `eEntityType`, used by
    /// `CWorld::FindObjectsIntersectingCube`.
    pub area_entity_bounds: Vec<ScriptAreaEntityBounds>,
    /// Prefix of `area_entity_bounds` backed by immutable IPL world entities.
    /// re3 queries sector lists directly; the Bevy bridge caches this static
    /// mirror instead of rebuilding thousands of bounds every VM frame.
    pub static_area_entity_bounds_count: usize,
    /// IPL entity count used to validate the cached static prefix.
    pub static_area_world_entity_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TCollectiveData {
    pub col_index: i32,
    pub ped_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TUsedObject {
    pub name: [u8; 24],
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptSphere {
    pub in_use: bool,
    pub index: i32,
    pub id: i32,
    pub center: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntroTextLine {
    pub scale: [f32; 2],
    pub color: [u8; 4],
    pub justify: bool,
    pub centered: bool,
    pub background: bool,
    pub background_only: bool,
    pub wrap_x: f32,
    pub center_size: f32,
    pub background_color: [u8; 4],
    pub proportional: bool,
    pub before_fade: bool,
    pub right_justify: bool,
    pub font: i32,
    pub position: [f32; 2],
    pub text_key: [u8; 8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntroScriptRectangle {
    pub is_used: bool,
    pub before_fade: bool,
    pub texture_id: i32,
    /// Script-space CRect: left, top, right, bottom.
    pub rect: [f32; 4],
    pub color: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TBuildingSwap {
    pub building_ref: Option<i32>,
    pub new_model: i32,
    pub old_model: i32,
}

/// Native InvisibilitySettingArray may reference a static world entity or a
/// live pooled CObject. Pool-slot references read from a save are resolved only
/// after the compatible object pool has recreated its Bevy entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptInvisibilityTarget {
    WorldEntity(i32),
    ScriptObject(i32),
    ObjectPoolSlot(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupEntity {
    pub entity_type: u8,
    pub id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CMissionCleanup {
    pub entities: Vec<CleanupEntity>,
    pub count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsideDownCarData {
    pub vehicle_index: i32,
    pub upside_down_timer: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CUpsideDownCarCheck {
    pub cars: [UpsideDownCarData; MAX_UPSIDEDOWN_CAR_CHECKS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StuckCarData {
    pub vehicle_index: i32,
    pub pos: CVector,
    pub last_check: i32,
    pub radius: f32,
    pub stuck_time: u32,
    pub stuck: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CStuckCarCheck {
    pub cars: [StuckCarData; MAX_STUCK_CAR_CHECKS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptVehicleProbe {
    pub exists: bool,
    pub position: CVector,
    pub up_z: f32,
    pub move_speed_magnitude: f32,
    pub turn_speed_magnitude: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptVarPointer {
    Global(usize),
    Local(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptZoneBounds {
    pub label: [u8; KEY_LENGTH_IN_SCRIPT],
    pub min: CVector,
    pub max: CVector,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptMissionEntityBounds {
    pub position: CVector,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptAreaEntityBounds {
    /// re3 `ENTITY_TYPE_BUILDING..ENTITY_TYPE_DUMMY` (1..=5).
    pub entity_type: u8,
    pub position: CVector,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptGarageVehicleContainment {
    pub garage_handle: i32,
    pub vehicle_handle: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptVmError {
    IpOutOfRange,
    InvalidArgumentType(u8),
    InvalidVariableType,
    GlobalVariableOutOfRange(usize),
    LocalVariableOutOfRange(usize),
    StackOverflow,
    StackUnderflow,
    NoIdleScripts,
    MissingRandomGenerator,
    MissingClock,
    MissingCameraProbe,
    MissingGroundProbe,
    MissingPathNodeProbe,
    PathNodeNotFound,
    MissingPlayerProbe,
    MissingPlayer(i32),
    MissingCharProbe,
    MissingChar(i32),
    MissingCharVisibilityProbe,
    MissingThreatReactionRangeProbe,
    MissingCollectiveProbe,
    MissingClosestObjectProbe,
    MissingVehicleEntityProbe,
    MissingVehicle(i32),
    MissingObjectProbe,
    MissingObject(i32),
    MissingObjectMotionProbe,
    MissingPadProbe,
    MissingWorldStateProbe,
    MissingScriptEntityAllocator,
    ScriptEntityAllocationFailed,
    InvalidAndOrState(u16),
    InvalidMissionScriptIndex(i32),
    MissionScriptOffsetOutOfRange(i32),
    MissionScriptRangeInvalid { offset: i32, end: usize },
    UnsupportedCommand(i32),
    InvalidScriptIndex(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptCommandOutcome {
    pub command: i32,
    pub yielded: bool,
    pub terminated: bool,
    /// Observability for parity transcripts: instruction pointer before this
    /// command was decoded (re3 `m_nIp` at `ProcessOneCommand` entry).
    pub ip_before: u32,
    /// Observability for parity transcripts: instruction pointer after the
    /// command completed (re3 `m_nIp` writeback).
    pub ip_after: u32,
    /// re3 `m_bNotFlag` — the 0x8000 bit of the raw command word.
    pub not_flag: bool,
    /// Observability for parity transcripts: slot of the script that executed
    /// this command. `usize::MAX` when produced outside a `CTheScripts` loop.
    pub script_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptPathNodeSnapshot {
    pub position: CVector,
    pub heading_degrees: f32,
}

/// Frame query of world state that re3 reads synchronously inside
/// `CRunningScript::ProcessCommand`:
/// `FrontEndMenuManager.m_bMenuActive` (Script4.cpp COMMAND_HAS_SAVE_GAME_FINISHED)
/// and `CCranes::HaveAllCarsBeenCollectedByMilitaryCrane()`
/// (Script6.cpp COMMAND_HAS_MILITARY_CRANE_COLLECTED_ALL_CARS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptWorldStateSnapshot {
    pub menu_active: bool,
    pub military_crane_all_cars_collected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptObjectMotionRequest {
    Rotate {
        handle: i32,
        heading_target_degrees: f32,
        rotate_by_degrees: f32,
        check_collision: bool,
    },
    Slide {
        handle: i32,
        target: CVector,
        slide_by: CVector,
        check_collision: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptStatsEffect {
    AddProgress(i32),
    /// re3 Script4.cpp: CStats::IndustrialPassed = true.
    IndustrialPassed,
    /// re3 Script4.cpp: CStats::CommercialPassed = true.
    CommercialPassed,
    /// re3 Script4.cpp: CStats::SuburbanPassed = true.
    SuburbanPassed,
    RegisterJumpDistance(f32),
    RegisterJumpHeight(f32),
    RegisterJumpFlips(i32),
    RegisterJumpSpins(i32),
    RegisterJumpStunt(i32),
    RegisterUniqueJump,
    SetUniqueJumpTotal(i32),
    RegisterTaxiPassenger,
    AddTaxiMoney(i32),
    RegisterMissionGiven,
    RegisterMissionPassed([u8; KEY_LENGTH_IN_SCRIPT]),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptCommandEffect {
    ShakeCamera {
        strength: f32,
        position: Option<CVector>,
    },
    ShakePad {
        pad: i32,
        duration: u32,
        frequency: u32,
    },
    /// re3 Script4.cpp COMMAND_ACTIVATE_SAVE_MENU:
    /// `FrontEndMenuManager.m_bSaveMenuActive = true`.
    ActivateSaveMenu,
    PrintBig {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
        time_ms: i32,
        style: i32,
        queue: bool,
    },
    Print {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
        time_ms: i32,
        flag: i32,
    },
    PrintNow {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
        time_ms: i32,
        flag: i32,
    },
    PrintSoon {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
        time_ms: i32,
        flag: i32,
    },
    PrintBigWithNumbers {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
        time_ms: i32,
        style: i32,
        numbers: [i32; 6],
        queue: bool,
    },
    PrintWithNumbers {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
        time_ms: i32,
        flag: i32,
        numbers: [i32; 6],
        mode: u8,
    },
    PrintStringNow {
        source_key: [u8; KEY_LENGTH_IN_SCRIPT],
        inserted_key: [u8; KEY_LENGTH_IN_SCRIPT],
        time_ms: i32,
        flag: i32,
    },
    PrintHelp {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    ClearHelp,
    ClearThisPrint {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    ClearThisBigPrint {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    AddOnscreenClock {
        variable_offset: u32,
        key: Option<[u8; KEY_LENGTH_IN_SCRIPT]>,
    },
    ClearOnscreenClock {
        variable_offset: u32,
    },
    AddOnscreenCounter {
        variable_offset: u32,
        counter_type: i32,
        key: Option<[u8; KEY_LENGTH_IN_SCRIPT]>,
    },
    ClearOnscreenCounter {
        variable_offset: u32,
    },
    FreezeOnscreenTimer(bool),
    ClearPrints,
    /// MESSAGE_WAIT skip expires only the currently displayed queue heads.
    SkipMessageWait,
    SetTimeScale(f32),
    AddOneOffSound {
        position: CVector,
        sound_id: i32,
    },
    AddContinuousSound {
        handle: i32,
        position: CVector,
        sound_id: i32,
    },
    RemoveSound {
        handle: i32,
    },
    LoadMissionAudio {
        name: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    PlayMissionAudio,
    SetMissionAudioPosition {
        position: CVector,
    },
    ClearMissionAudio,
    PlayMissionPassedTune {
        tune_id: i32,
    },
    /// re3 `DMAudio.PlayRadioAnnouncement` emitted by the island-unlock
    /// progress opcodes.
    PlayRadioAnnouncement {
        announcement: i32,
    },
    SetRadioChannel {
        channel: i32,
        position: i32,
    },
    GrabPhone {
        x: f32,
        y: f32,
    },
    SetPhoneMessages {
        phone_id: i32,
        keys: [[u8; KEY_LENGTH_IN_SCRIPT]; 6],
        count: u8,
        repeated: bool,
    },
    TurnPhoneOff {
        phone_id: i32,
    },
    TurnPhoneOn {
        phone_id: i32,
    },
    StartScriptFire {
        slot: i32,
        position: CVector,
        target: Option<ScriptFireTarget>,
    },
    RemoveScriptFire {
        slot: i32,
    },
    RemoveAllScriptFires,
    SetScriptFireAudio {
        slot: i32,
        enabled: bool,
    },
    ExtinguishPlayerFires {
        player_index: i32,
    },
    SetFreeResprays {
        enabled: bool,
    },
    SetFreeBombs {
        enabled: bool,
    },
    SetCharSay {
        handle: i32,
        sound_id: i32,
    },
    SetTotalNumberOfMissions(i32),
    SetProgressTotal(i32),
    RegisterFastestTime {
        index: i32,
        time: i32,
    },
    RegisterHighestScore {
        index: i32,
        score: i32,
    },
    RegisterElBurroTime(i32),
    Register4x4Time {
        race: usize,
        time: i32,
    },
    RegisterLifeSaved,
    RegisterCriminalCaught,
    RegisterAmbulanceLevel(i32),
    RegisterFireExtinguished,
    RegisterLongestDodoFlight(i32),
    RegisterDefuseBombTime(i32),
    SetTotalKillFrenzies(i32),
    StartCredits,
    StopCredits,
    AddRestartPoint {
        police: bool,
        position: CVector,
        heading: f32,
        needs_ground: bool,
    },
    OverrideNextRestart {
        position: CVector,
        heading: f32,
        needs_ground: bool,
    },
    PlayerFailedCriticalMission,
    CancelOverrideRestart,
    OverrideHospitalLevel(i32),
    OverridePoliceLevel(i32),
    Stats(ScriptStatsEffect),
    RegisterKillFrenzyPassed,
    StoreWeather,
    RestoreWeather,
    StartKillFrenzy {
        weapon_type: i32,
        time_ms: i32,
        kills: u16,
        models: [i32; 4],
        text_key: [u8; KEY_LENGTH_IN_SCRIPT],
        standard_sound: bool,
        need_head_shot: bool,
    },
    ResetModelsKilledByPlayer,
    LoadCutscene {
        name: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    StartCutscene,
    ClearCutscene,
    CreateCutsceneObject {
        handle: i32,
        model_id: i32,
    },
    SetCutsceneAnim {
        object_handle: i32,
        anim_name: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    CreateCutsceneHead {
        handle: i32,
        source_object_handle: i32,
        model_id: i32,
    },
    SetCutsceneHeadAnim {
        object_handle: i32,
        anim_name: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    SetCutsceneOffset {
        position: CVector,
    },
    RequestModel {
        model_id: i32,
    },
    LoadAllModelsNow,
    DoFade {
        time_ms: i32,
        direction: i32,
    },
    SetFadingColour {
        r: i32,
        g: i32,
        b: i32,
    },
    SetPlayerControl {
        player_index: i32,
        enabled: bool,
    },
    EnablePlayerControlCamera,
    GiveRemoteControlledCar {
        player_index: i32,
        position: CVector,
        rotation_radians: f32,
        model_id: i32,
    },
    SetPlayerNeverGetsTired {
        player_index: i32,
        enabled: bool,
    },
    SetPlayerFastReload {
        player_index: i32,
        enabled: bool,
    },
    SetPlayerVisible {
        player_index: i32,
        visible: bool,
    },
    SetGetOutOfJailFree {
        player_index: i32,
        enabled: bool,
    },
    SetFreeHealthCare {
        player_index: i32,
        enabled: bool,
    },
    AddScore {
        player_index: i32,
        amount: i32,
    },
    SetPlayerHealth {
        player_index: i32,
        health: f32,
    },
    AddArmourToPlayer {
        player_index: i32,
        armour: f32,
    },
    SetPlayerAmmo {
        player_index: i32,
        weapon_type: i32,
        ammo: i32,
    },
    SetCurrentPlayerWeapon {
        player_index: i32,
        weapon_type: i32,
    },
    AddAmmoToPlayer {
        player_index: i32,
        weapon_type: i32,
        ammo: i32,
    },
    GiveWeaponToPlayer {
        player_index: i32,
        weapon_type: i32,
        ammo: i32,
    },
    RemoveAllPlayerWeapons { player_index: i32 },
    LoadEndOfGameTune,
    PlayEndOfGameTune,
    StopEndOfGameTune,
    SwitchWidescreen {
        enabled: bool,
    },
    LoadScene {
        position: CVector,
    },
    SetNearClip(f32),
    DrawLight {
        position: CVector,
        color: [u8; 3],
    },
    StartChaseScene(f32),
    StopChaseScene,
    RemoveCarFromChase(i32),
    FlashHudObject(i32),
    ClearSmallPrints,
    AddPagerMessage {
        key: [u8; KEY_LENGTH_IN_SCRIPT],
        speed: u16,
        priority: u16,
        unused: u16,
        numbers: Option<[i32; 6]>,
    },
    LoadSplashScreen([u8; 8]),
    AddMovingParticleEffect {
        object_type: i32,
        position: CVector,
        target: CVector,
        size: f32,
        life_time: u32,
        color: [u8; 4],
        remove: bool,
    },
    StartBoatFoamAnimation,
    UpdateBoatFoamAnimation {
        handle: i32,
    },
    SetMusicDoesFade(bool),
    SetIntroIsPlaying {
        playing: bool,
    },
    /// Script6.cpp SET_INTRO_IS_PLAYING(false)'s call to DMAudio.
    /// Retained until the audio owner applies it, independently of streaming.
    RestoreGameMusicAfterIntro,
    SetFixedCameraPosition {
        position: CVector,
        orientation: CVector,
    },
    PointCameraAtPoint {
        position: CVector,
        switch_style: i32,
    },
    PointCameraAtPlayer {
        player_index: i32,
        mode: i32,
        switch_style: i32,
    },
    PointCameraAtChar {
        handle: i32,
        position: CVector,
        mode: i32,
        switch_style: i32,
    },
    PointCameraAtCar {
        handle: i32,
        position: CVector,
        mode: i32,
        switch_style: i32,
    },
    RestoreCamera,
    RestoreCameraJumpcut,
    SetCameraBehindPlayer,
    SetCameraInFrontOfPlayer,
    SetMotionBlur(i32),
    ForceWeather(i16),
    ForceWeatherNow(i16),
    ReleaseWeather,
    SetScriptsForceRain(bool),
    CreatePlayer {
        index: i32,
        position: CVector,
        needs_ground: bool,
    },
    SetPlayerHeading {
        player_index: i32,
        heading_degrees: f32,
    },
    CreatePickup {
        handle: i32,
        model_id: i32,
        pickup_type: i32,
        quantity: i32,
        position: CVector,
    },
    CreateUntrackedPickup {
        model_id: i32,
        pickup_type: i32,
        quantity: i32,
        position: CVector,
    },
    CreateFloatingPackage {
        handle: i32,
        position: CVector,
    },
    CreatePickupBlip {
        handle: i32,
        pickup_handle: i32,
    },
    CreateSpritePickupBlip {
        handle: i32,
        pickup_handle: i32,
        sprite: i32,
    },
    DrawShadow {
        texture: u8,
        position: CVector,
        front: [f32; 2],
        side: [f32; 2],
        intensity: i16,
        color: [u8; 3],
    },
    CreateObject {
        handle: i32,
        model_id: i32,
        position: CVector,
        add_model_offset: bool,
    },
    DeleteObject {
        handle: i32,
    },
    SetObjectVelocity {
        handle: i32,
        velocity: CVector,
        add: bool,
    },
    SetObjectCollision {
        handle: i32,
        enabled: bool,
    },
    SetObjectDynamic {
        handle: i32,
        dynamic: bool,
    },
    SetObjectDrawLast {
        handle: i32,
        draw_last: bool,
    },
    MakeObjectTargettable {
        handle: i32,
    },
    SetObjectHeading {
        handle: i32,
        heading_degrees: f32,
    },
    SetObjectRotation {
        handle: i32,
        rotation: CVector,
    },
    SetObjectCoordinates {
        handle: i32,
        position: CVector,
    },
    RemovePickup {
        handle: i32,
    },
    SetGangCarModel {
        gang: i32,
        model: i32,
    },
    SetGangWeapons {
        gang: i32,
        weapon1: i32,
        weapon2: i32,
    },
    SetThreatForPedType {
        ped_type: i32,
        threat: i32,
        add: bool,
    },
    SetWantedLevel {
        player_index: i32,
        level: i32,
        no_drop: bool,
    },
    SetMaximumWantedLevel(i32),
    AddGarage {
        handle: i32,
        min: CVector,
        max: CVector,
        garage_type: i32,
        target_model: i32,
    },
    SetGarageCameraFollowsPlayer {
        handle: i32,
    },
    SetGarageActive {
        handle: i32,
        active: bool,
    },
    SetRotatingGarageDoor {
        handle: i32,
    },
    ActivateCrane {
        pickup_min: CVector,
        pickup_max: CVector,
        dropoff: CVector,
        heading_radians: f32,
        crane_position: CVector,
        is_crusher: bool,
        is_military: bool,
    },
    DeactivateCrane {
        position: CVector,
    },
    ChangeGarageType {
        handle: i32,
        garage_type: i32,
        target_model: i32,
    },
    SetTargetCarForMissionGarage {
        handle: i32,
        vehicle_handle: Option<i32>,
    },
    OpenGarage {
        handle: i32,
    },
    CloseGarage {
        handle: i32,
    },
    ClearGarageResprayHappened {
        handle: i32,
    },
    SetZonePedInfo {
        label: [u8; KEY_LENGTH_IN_SCRIPT],
        day: bool,
        ped_density: i32,
        gang_density: [i32; 9],
        cop_density: i32,
    },
    SetZonePedDensity {
        label: [u8; KEY_LENGTH_IN_SCRIPT],
        day_density: i32,
        night_density: i32,
    },
    SetZoneCarInfo {
        label: [u8; KEY_LENGTH_IN_SCRIPT],
        day: bool,
        car_density: i32,
        gang_counts: [i32; 9],
        cop_count: i32,
        car_counts: [i32; 6],
    },
    SetZoneGroup {
        label: [u8; KEY_LENGTH_IN_SCRIPT],
        day: bool,
        ped_group: i32,
    },
    SetCollectableTotal(i32),
    SwitchTaxiTimer {
        enabled: bool,
        time_ms: u32,
    },
    CreateCollectable {
        position: CVector,
    },
    CreateCoordinateBlip {
        handle: i32,
        position: CVector,
        sprite: i32,
        contact_point: bool,
        colour: i32,
        display: i32,
        scale: i32,
    },
    CreateEntityBlip {
        handle: i32,
        entity_handle: i32,
        entity_type: i32,
        colour: i32,
        display: i32,
        sprite: i32,
        scale: i32,
    },
    RemoveBlip {
        handle: i32,
    },
    ChangeBlipColour {
        handle: i32,
        colour: i32,
    },
    ChangeBlipBrightness {
        handle: i32,
        bright: i32,
    },
    ChangeBlipScale {
        handle: i32,
        scale: i32,
    },
    ChangeBlipDisplay {
        handle: i32,
        display: i32,
    },
    SwitchRoadsInArea {
        min: CVector,
        max: CVector,
        pedestrian: bool,
        disable: bool,
    },
    SwitchRoadsInAngledArea {
        p1: CVector,
        p2: CVector,
        length: f32,
        pedestrian: bool,
        enable: bool,
    },
    MarkRoadsBetweenLevels {
        min: CVector,
        max: CVector,
        pedestrian: bool,
    },
    SetStreamingEnabled(bool),
    SetWorldProcessingEnabled(bool),
    SetRubbishVisible(bool),
    MakePlayerSafeForCutscene {
        player_index: i32,
    },
    LoadCollision {
        level: i32,
    },
    MarkModelAsNoLongerNeeded {
        model_id: i32,
    },
    UnloadSpecialCharacter {
        char_id: i32,
    },
    RequestSpecialCharacter {
        slot: usize,
        name: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    RequestSpecialModel {
        model_id: i32,
        name: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    ClearWantedLevel {
        player_index: i32,
    },
    SetWantedMultiplier(f32),
    StartDrugRun,
    StartDrugDropOff,
    StartCatalinaHeli,
    SwitchHelicopter(bool),
    CatalinaHeliTakeOff,
    RemoveCatalinaHeli,
    CatalinaHeliFlyAway,
    StartPacManRace(i32),
    StartPacManRecord,
    ClearPacMan,
    StartPacManScramble {
        position: CVector,
        scramble_mult: f32,
        count: i16,
    },
    ResetPacManPillsCarried,
    ResetPacManPillsEaten,
    AddPowerPill(CVector),
    SetSpecialPoliceRequired {
        kind: u8,
        required: bool,
    },
    DestroyProjectilesInArea {
        min: CVector,
        max: CVector,
    },
    SetCarStatus {
        handle: i32,
        status: i32,
    },
    SetCharIgnoreLevelTransitions {
        handle: i32,
        enabled: bool,
    },
    SetCarIgnoreLevelTransitions {
        handle: i32,
        enabled: bool,
    },
    SetCarMoreResistantToDamage {
        handle: i32,
        enabled: bool,
    },
    /// re3 `COMMAND_CREATE_CAR_GENERATOR` — registers parked spawn slot.
    CreateCarGenerator {
        position: CVector,
        angle_degrees: f32,
        model_index: i32,
        color1: i16,
        color2: i16,
        force_spawn: u8,
        alarm: u8,
        doorlock: u8,
        min_delay: u16,
        max_delay: u16,
    },
    /// re3 `COMMAND_SWITCH_CAR_GENERATOR` — 0 off, ≤100 on+uses, >100 infinite.
    SwitchCarGenerator {
        index: i32,
        count: i32,
    },
    /// re3 `COMMAND_SWAP_NEAREST_BUILDING_MODEL` — map mesh model swap.
    SwapNearestBuildingModel {
        position: CVector,
        radius: f32,
        old_model: i32,
        new_model: i32,
    },
    /// re3 `COMMAND_SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE`.
    SetVisibilityOfClosestObjectOfType {
        position: CVector,
        radius: f32,
        model_id: i32,
        visible: bool,
    },
    SetPoliceIgnorePlayer {
        player_index: i32,
        ignored: bool,
    },
    SetEveryoneIgnorePlayer {
        player_index: i32,
        ignored: bool,
    },
    SetPlayerCoordinates {
        index: i32,
        position: CVector,
        needs_ground: bool,
    },
    DeleteChar {
        handle: i32,
    },
    RemoveCharElegantly {
        handle: i32,
    },
    UndressChar {
        handle: i32,
        model_name: [u8; KEY_LENGTH_IN_SCRIPT],
    },
    DressChar {
        handle: i32,
    },
    CharWanderDir {
        handle: i32,
        path: i32,
    },
    CharFollowPath {
        handle: i32,
        position: CVector,
        needs_ground: bool,
    },
    CharSetIdle {
        handle: i32,
    },
    SetCharWaitState {
        handle: i32,
        wait_state: i32,
        time_ms: Option<i32>,
        now_ms: u32,
    },
    SetCharObjective {
        handle: i32,
        objective: i32,
        target_char: Option<i32>,
        target_car: Option<i32>,
        position: Option<CVector>,
        radius: f32,
        route: Option<(i32, i32)>,
    },
    SetAllOccupantsOfCarLeaveCar {
        handle: i32,
    },
    AddRoutePoint {
        route: i32,
        position: CVector,
    },
    RemoveRoute {
        route: i32,
    },
    ClearCharThreatSearch {
        handle: i32,
    },
    SetCharThreatSearch {
        handle: i32,
        threats: u32,
    },
    SetCharPersonality {
        handle: i32,
        personality: i32,
    },
    MarkCharAsNoLongerNeeded {
        handle: i32,
    },
    MarkCarAsNoLongerNeeded {
        handle: i32,
    },
    MarkObjectAsNoLongerNeeded {
        handle: i32,
    },
    SetPedDensityMultiplier(f32),
    ForceRandomPedType(i32),
    SetCarDensityMultiplier(f32),
    SetCharCantBeDraggedOut {
        handle: i32,
        enabled: bool,
    },
    SetTaxiLights {
        handle: i32,
        enabled: bool,
    },
    SetAllTaxiLights(bool),
    FlashCar {
        handle: i32,
        enabled: bool,
    },
    FlashChar {
        handle: i32,
        enabled: bool,
    },
    FlashObject {
        handle: i32,
        enabled: bool,
    },
    ArmCarWithBomb {
        handle: i32,
        bomb_type: i32,
    },
    ExplodeCar {
        handle: i32,
    },
    CreateSingleParticle {
        particle_type: i32,
        position: CVector,
        direction: CVector,
        size: f32,
    },
    AddExplosion {
        position: CVector,
        explosion_type: i32,
    },
    DrawCorona {
        id: u32,
        position: CVector,
        size: f32,
        texture_type: i32,
        flare_type: i32,
        colour: [u8; 3],
    },
    ClearArea {
        position: CVector,
        radius: f32,
        remove_projectiles_and_tidy_shadows: bool,
    },
    SetCharOnlyDamagedByPlayer {
        handle: i32,
        enabled: bool,
    },
    SetCharProofs {
        handle: i32,
        bullet: bool,
        fire: bool,
        explosion: bool,
        collision: bool,
        melee: bool,
    },
    SetCharUsePedNodeSeek {
        handle: i32,
        enabled: bool,
    },
    SetCharStaysInCurrentLevel {
        handle: i32,
        enabled: bool,
    },
    SetCharsChatting {
        first: i32,
        second: i32,
        duration_ms: u32,
    },
    SetCharBleeding {
        handle: i32,
        enabled: bool,
    },
    SetCharRunning {
        handle: i32,
        enabled: bool,
    },
    SetCharAnimGroup {
        handle: i32,
        anim_group: i32,
    },
    SetCharLookAtChar {
        source: i32,
        target: i32,
    },
    SetCharLookAtPlayer {
        source: i32,
        player_index: i32,
    },
    SetPlayerLookAtChar {
        player_index: i32,
        target: i32,
    },
    StopCharLooking(i32),
    StopPlayerLooking(i32),
    SetPlayerAnimGroup {
        player_index: i32,
        anim_group: i32,
    },
    SetCharStayInSamePlace {
        handle: i32,
        enabled: bool,
    },
    SetCharRespondsToThreats {
        handle: i32,
        enabled: bool,
    },
    SetPlayerRespondsToThreats {
        player_index: i32,
        enabled: bool,
    },
    SetCharVisible {
        handle: i32,
        visible: bool,
    },
    SetCharCoordinates {
        handle: i32,
        position: CVector,
        needs_ground: bool,
    },
    SetCharHealth {
        handle: i32,
        health: f32,
    },
    SetCharAccuracy {
        handle: i32,
        accuracy: i32,
    },
    AddArmourToChar {
        handle: i32,
        armour: f32,
    },
    SetCharAmmo {
        handle: i32,
        weapon_type: i32,
        ammo: i32,
    },
    SetCurrentCharWeapon {
        handle: i32,
        weapon_type: i32,
    },
    AddAmmoToChar {
        handle: i32,
        weapon_type: i32,
        ammo: i32,
    },
    GiveWeaponToChar {
        handle: i32,
        weapon_type: i32,
        ammo: i32,
    },
    ExplodeCharHead {
        handle: i32,
        nasty_game: bool,
    },
    ClearAreaOfChars {
        min: CVector,
        max: CVector,
    },
    ClearAreaOfCars {
        min: CVector,
        max: CVector,
    },
    CreateRandomCarForCarPark {
        model_id: i32,
        position: CVector,
        heading_degrees: f32,
    },
    RemoveParticleEffectsInArea {
        min: CVector,
        max: CVector,
    },
    DeleteCar {
        handle: i32,
    },
    CarGotoCoordinates {
        handle: i32,
        position: CVector,
        needs_ground: bool,
        accurate: bool,
        anti_reverse_timer_ms: u32,
    },
    ApplyBrakesToPlayersCar {
        player_index: i32,
        enabled: bool,
    },
    TurnCarToFaceCoord {
        handle: i32,
        x: f32,
        y: f32,
    },
    PlaceObjectRelativeToCar {
        object_handle: i32,
        vehicle_handle: i32,
        offset: CVector,
        time_step: f32,
    },
    CarWanderRandomly {
        handle: i32,
    },
    CarSetIdle {
        handle: i32,
    },
    SetCarCoordinates {
        handle: i32,
        position: CVector,
        needs_ground: bool,
    },
    SetCarHealth {
        handle: i32,
        health: f32,
    },
    SetCarAmmo {
        handle: i32,
        ammo: i32,
    },
    SetCarOnlyDamagedByPlayer {
        handle: i32,
        enabled: bool,
    },
    SetCarCanBeDamaged {
        handle: i32,
        enabled: bool,
    },
    SetCarLock {
        handle: i32,
        lock: i32,
    },
    SetCarColour {
        handle: i32,
        colour1: i32,
        colour2: i32,
    },
    /// re3 `COMMAND_SET_CAR_HEADING` — degrees about Z.
    SetCarHeading {
        handle: i32,
        heading_degrees: f32,
    },
    /// re3 `COMMAND_SET_CHAR_HEADING` — degrees about Z.
    SetCharHeading {
        handle: i32,
        heading_degrees: f32,
    },
    SetCarVisible {
        handle: i32,
        visible: bool,
    },
    SetCarStrong {
        handle: i32,
        strong: bool,
    },
    SetCarBigWheels {
        handle: i32,
        enabled: bool,
    },
    SetCarCanRespray {
        handle: i32,
        enabled: bool,
    },
    SetCarComedyControls {
        handle: i32,
        enabled: bool,
    },
    SetCarHeavy {
        handle: i32,
        enabled: bool,
    },
    AnchorBoat {
        handle: i32,
        anchored: bool,
    },
    SwitchCarSiren {
        handle: i32,
        enabled: bool,
    },
    SetCarProofs {
        handle: i32,
        bullet: bool,
        fire: bool,
        explosion: bool,
        collision: bool,
        melee: bool,
    },
    SetCarWaterTight {
        handle: i32,
        enabled: bool,
    },
    SetCarAvoidLevelTransitions {
        handle: i32,
        enabled: bool,
    },
    SetCarStaysInCurrentLevel {
        handle: i32,
        enabled: bool,
    },
    SetGenerateCarsAroundCamera(bool),
    SetJamesCarOnPathToPlayer {
        handle: i32,
        player_position: CVector,
    },
    BlowUpRemoteCar,
    WarpCharFromCarToCoord {
        handle: i32,
        position: CVector,
        needs_ground: bool,
    },
    SetAllCarsCanBeDamaged(bool),
    SetUpsideDownCarNotDamaged {
        handle: i32,
        enabled: bool,
    },
    AddAmmoToCar {
        handle: i32,
        ammo: i32,
    },
    SetCarCruiseSpeed {
        handle: i32,
        speed: f32,
    },
    BoatGotoCoordinates {
        handle: i32,
        position: CVector,
        needs_water_level: bool,
        anti_reverse_timer_ms: u32,
    },
    BoatStop {
        handle: i32,
    },
    SetBoatCruiseSpeed {
        handle: i32,
        speed: f32,
    },
    SetCarDrivingStyle {
        handle: i32,
        style: i32,
    },
    SetCarMission {
        handle: i32,
        mission: i32,
        anti_reverse_timer_ms: u32,
    },
    SetCarTarget {
        handle: i32,
        target: i32,
        ram: bool,
    },
    WarpPlayerFromCarToCoord {
        player_index: i32,
        position: CVector,
        needs_ground: bool,
    },
    WarpPlayerIntoCar {
        player_index: i32,
        vehicle_handle: i32,
    },
    WarpCharIntoCar {
        char_handle: i32,
        vehicle_handle: i32,
    },
    StoreCarCharIsIn {
        char_handle: i32,
        vehicle_handle: i32,
    },
    StoreCarPlayerIsIn {
        player_index: i32,
        vehicle_handle: i32,
    },
    HighlightArea {
        script_ip: u32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        z: f32,
    },
    DebugSquare {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    DebugCube {
        x1: f32,
        y1: f32,
        z1: f32,
        x2: f32,
        y2: f32,
        z2: f32,
    },
    MissionCleanupProcessed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptPlayerSnapshot {
    /// Live CPools handle of the player's ped.
    pub ped_handle: i32,
    pub position: CVector,
    pub current_vehicle: Option<i32>,
    pub current_vehicle_model: Option<i32>,
    pub stopped: bool,
    /// Exact `CPed::bIsShooting` flag.
    pub is_shooting: bool,
    pub horn_pressed: bool,
    /// `CPed::IsPedInControl`, supplied by the live player runtime.
    pub is_in_control: bool,
    /// Exact `m_nPedState == PED_DRIVING` predicate; passengers do not qualify.
    pub is_driving: bool,
    /// Exact `m_nPedState == PED_MAKE_CALL` predicate.
    pub is_making_call: bool,
    pub health: f32,
    pub wanted_level: i32,
    pub money: i32,
    pub current_weapon: i32,
    pub weapon_ammo: [i32; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptEntitySnapshot {
    pub position: CVector,
    /// World-space `CEntity::GetBoundCentre`.
    pub bound_center: CVector,
    /// `CEntity::GetBoundRadius`.
    pub bound_radius: f32,
    /// re3 entity heading in degrees (Z rotation) for GET_*_HEADING.
    pub heading_degrees: f32,
    /// Entity matrix up-vector Z component (`CEntity::GetUp().z`).
    pub up_z: f32,
    /// Exact `CEntity::GetModelIndex()` value.
    pub model_index: i32,
    /// Exact `CEntity::GetStatus()` value.
    pub entity_status: i32,
    pub alive: bool,
    pub current_vehicle: Option<i32>,
    pub current_vehicle_model: Option<i32>,
    /// Vehicle palette indices when this snapshot represents a CVehicle.
    pub vehicle_colour1: Option<i32>,
    pub vehicle_colour2: Option<i32>,
    pub vehicle_is_boat: bool,
    pub vehicle_in_water: bool,
    /// `CAutomobile::m_nDriveWheelsOnGround`; absent for non-cars.
    pub automobile_drive_wheels_on_ground: Option<u8>,
    pub stopped: bool,
    /// Exact `CPed::bIsShooting` flag when this snapshot represents a ped.
    pub is_shooting: bool,
    /// `CPhysical::m_aCollisionRecords` kind/generation-qualified references.
    pub collision_records:
        [Option<EntityReference>; crate::entities::Physical::PHYSICAL_MAX_COLLISIONRECORDS],
    /// Magnitude of CPhysical::m_vecMoveSpeed in GTA game-speed units.
    pub speed: f32,
    pub health: f32,
    pub current_weapon: i32,
    pub weapon_ammo: [i32; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
    /// Exact `CPed::bScriptObjectiveCompleted` value.
    pub script_objective_completed: bool,
    /// CPed pool slot referenced by `CPed::m_leader`.
    pub leader_slot: Option<usize>,
    /// `CPed::m_nPedType` when this is a ped.
    pub ped_type: Option<i32>,
    /// `CAutomobile::m_bombType`; boats and non-vehicles have no value.
    pub vehicle_bomb_type: Option<i32>,
    /// `CVehicle::bIsDamaged`.
    pub vehicle_damaged: bool,
    /// `m_lastWepDam` for peds and `m_nLastWeaponDamage` for vehicles.
    pub last_weapon_damage: i32,
    pub vehicle_num_passengers: u8,
    pub vehicle_num_max_passengers: u8,
    pub vehicle_passenger_handles: [Option<i32>; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS],
    /// Bit `eDoors` is set only when the door exists and its hinge is closed.
    pub vehicle_closed_doors: u8,
    /// `CPed::OurPedCanSeeThisOne` for the focused GTA III player.
    pub can_see_player: bool,
}

/// A handle exposed to SCM. It is deliberately distinct from a Bevy
/// `Entity` and from pool slot indices; the latter are implementation details
/// and must never leak into script-facing APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct ScriptHandle(pub i32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptCreateCharRequest {
    pub ped_type: i32,
    pub model_id: i32,
    pub position: CVector,
    pub vehicle_handle: Option<i32>,
    pub passenger_seat: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptCreateCarRequest {
    pub model_id: i32,
    pub position: CVector,
    pub time_ms: u32,
    /// `COMMAND_CREATE_CAR` clears mission-space before CWorld::Add; random
    /// traffic is generated by CCarCtrl and must instead abort when the
    /// authored spawn volume is occupied.
    pub clear_space_for_mission_entity: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptCreateObjectRequest {
    pub model_id: i32,
    pub position: CVector,
    pub add_model_offset: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptEntityMutation {
    ObjectMotion(ScriptObjectMotionRequest),
    CreateChar(ScriptCreateCharRequest),
    CreateCar(ScriptCreateCarRequest),
    CreateObject(ScriptCreateObjectRequest),
    SetAllTaxiLights(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptEntityMutationResult {
    ObjectMotion(bool),
    Created(i32),
    Applied,
}

/// Unified synchronous dynamic-entity boundary for a running SCM command.
///
/// re3 opcodes read and mutate CPools/CWorld in one command stream. Keeping
/// the three entity probes and mutations behind one mutable interface lets a
/// Bevy runtime preserve read-after-write ordering without interior-mutability borrow workarounds or
/// duplicated authoritative state.
pub trait ScriptEntityRuntimeAccess {
    fn supports_mutation(&self, request: &ScriptEntityMutation) -> bool;

    fn mutate(
        &mut self,
        request: ScriptEntityMutation,
        general: Option<&mut CGeneral>,
    ) -> Option<ScriptEntityMutationResult>;

    fn char_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot>;
    fn vehicle_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot>;
    fn object_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot>;

    /// re3 `CPed::OurPedCanSeeThisOne` for two CPed pool handles. Production
    /// resolves both actors through their canonical indexes and checks the
    /// original static-world line of sight.
    fn char_can_see_char(&mut self, _observer: i32, _target: i32) -> Option<bool> {
        None
    }

    /// re3 `COMMAND_CAN_CHAR_SEE_DEAD_CHAR`: inspect the observer's native
    /// ten-entry nearby-ped cache, then apply `OurPedCanSeeThisOne`.
    fn char_can_see_dead_char(&mut self, _observer: i32, _ped_type: i32) -> Option<bool> {
        None
    }

    /// re3 `COMMAND_SET_THREAT_REACTION_RANGE_MULTIPLIER` updates the global
    /// `CPed` list-building range used by `CPed::BuildPedLists`.
    fn set_threat_reaction_range_multiplier(&mut self, _multiplier: f32) -> Option<()> {
        None
    }

    /// re3 `CTheScripts::AddPedsInAreaToCollective` after its CWorld range
    /// query. Returns eligible ped pool handles in query order.
    fn collective_peds_in_area(&mut self, _center: CVector, _radius: f32) -> Option<Vec<i32>> {
        None
    }

    /// re3 `CTheScripts::AddPedsInVehicleToCollective`. Returns eligible
    /// occupants in driver-then-passenger order.
    fn collective_peds_in_vehicle(&mut self, _vehicle: i32) -> Option<Vec<i32>> {
        None
    }

    /// re3 `CTheScripts::SetObjectiveForAllPedsInCollective` applies each
    /// objective directly to its CPed before the next SCM opcode executes.
    /// Returns `Some(())` when the live ECS actor was updated synchronously.
    #[allow(clippy::too_many_arguments)]
    fn set_char_objective_now(
        &mut self,
        _handle: i32,
        _objective: i32,
        _target_char: Option<i32>,
        _target_car: Option<i32>,
        _position: Option<CVector>,
        _radius: f32,
        _route: Option<(i32, i32)>,
    ) -> Option<()> {
        None
    }

    /// re3 `CCarAI::TellCarToBlockOtherCar` / `TellCarToRamOtherCar` mutates
    /// the chasing vehicle before the next SCM opcode and returns the target
    /// Entity that the caller registers with `CReferences`.
    fn set_car_target_now(&mut self, _handle: i32, _target: i32, _ram: bool) -> Option<()> {
        None
    }

    /// re3 `COMMAND_GET_CLOSEST_OBJECT_OF_TYPE` for objects already owned by
    /// the live ECS object pool. Static IPL dummy promotion is a separate
    /// lifecycle operation and must not fabricate a pool handle here.
    fn closest_object_of_type(
        &mut self,
        _position: CVector,
        _range: f32,
        _model_index: i32,
        _general: Option<&mut CGeneral>,
    ) -> Option<i32> {
        None
    }

    /// Shared re3 `gFireManager.m_aFires` occupancy. Production exposes the
    /// live fire pool; lightweight VM probes return None and keep the legacy
    /// script-only reservation fallback.
    fn fire_slot_is_free(&mut self, _slot: usize) -> Option<bool> {
        None
    }

    fn script_fire_is_extinguished(&mut self, _slot: i32) -> Option<bool> {
        None
    }

    /// Live `CWorld::FindObjectsIntersectingCube` adapter. Production returns
    /// `Some` and queries ECS/world state on demand; the default keeps focused
    /// VM tests able to use their explicit snapshot vectors.
    fn any_entity_intersects_cube(
        &mut self,
        _min: CVector,
        _max: CVector,
        _entity_types: [bool; 5],
    ) -> Option<bool> {
        None
    }

    /// Live `CWorld::FindMissionEntitiesIntersectingCube` adapter.
    fn any_mission_entity_intersects_cube(&mut self, _min: CVector, _max: CVector) -> Option<bool> {
        None
    }

    /// Live `CGarages::IsThisCarWithinGarageArea` adapter. Production uses
    /// the ECS vehicle directly instead of rebuilding a garage × vehicle
    /// containment table at the start of every SCM frame.
    fn garage_contains_vehicle(
        &mut self,
        _garage_handle: i32,
        _vehicle_handle: i32,
    ) -> Option<bool> {
        None
    }
}

pub(crate) struct ScriptEntityReadProbeRuntime<'char, 'vehicle, 'object> {
    pub(crate) char_probe: Option<&'char mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
    pub(crate) vehicle_probe: Option<&'vehicle mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
    pub(crate) object_probe: Option<&'object mut dyn FnMut(i32) -> Option<ScriptEntitySnapshot>>,
}

impl ScriptEntityRuntimeAccess for ScriptEntityReadProbeRuntime<'_, '_, '_> {
    fn supports_mutation(&self, _request: &ScriptEntityMutation) -> bool {
        false
    }

    fn mutate(
        &mut self,
        _request: ScriptEntityMutation,
        _general: Option<&mut CGeneral>,
    ) -> Option<ScriptEntityMutationResult> {
        None
    }

    fn char_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        self.char_probe
            .as_deref_mut()
            .and_then(|probe| probe(handle))
    }

    fn vehicle_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        self.vehicle_probe
            .as_deref_mut()
            .and_then(|probe| probe(handle))
    }

    fn object_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        self.object_probe
            .as_deref_mut()
            .and_then(|probe| probe(handle))
    }
}

pub(crate) fn run_script_entity_creation(
    runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    request: ScriptEntityMutation,
    general: Option<&mut CGeneral>,
) -> Result<Option<i32>, ScriptVmError> {
    let Some(runtime) = runtime.as_deref_mut() else {
        return Ok(None);
    };
    if !runtime.supports_mutation(&request) {
        return Ok(None);
    }
    match runtime.mutate(request, general) {
        Some(ScriptEntityMutationResult::Created(handle)) => Ok(Some(handle)),
        _ => Err(ScriptVmError::ScriptEntityAllocationFailed),
    }
}

pub(crate) fn run_script_object_motion(
    runtime: &mut Option<&mut dyn ScriptEntityRuntimeAccess>,
    request: ScriptObjectMotionRequest,
    general: Option<&mut CGeneral>,
) -> Result<bool, ScriptVmError> {
    let handle = match request {
        ScriptObjectMotionRequest::Rotate { handle, .. }
        | ScriptObjectMotionRequest::Slide { handle, .. } => handle,
    };
    let request = ScriptEntityMutation::ObjectMotion(request);
    let runtime = runtime
        .as_deref_mut()
        .ok_or(ScriptVmError::MissingObjectMotionProbe)?;
    if !runtime.supports_mutation(&request) {
        return Err(ScriptVmError::MissingObjectMotionProbe);
    }
    match runtime.mutate(request, general) {
        Some(ScriptEntityMutationResult::ObjectMotion(value)) => Ok(value),
        _ => Err(ScriptVmError::MissingObject(handle)),
    }
}

pub(crate) fn reborrow_script_entity_runtime<'a>(
    runtime: &'a mut Option<&mut dyn ScriptEntityRuntimeAccess>,
) -> Option<&'a mut dyn ScriptEntityRuntimeAccess> {
    match runtime {
        Some(runtime) => Some(&mut **runtime),
        None => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocateMode {
    AnyMeans,
    OnFoot,
    InCar,
}
