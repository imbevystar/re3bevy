use re3bevy_assets::assets::AnimationAssets::CAnimationAssets;
use re3bevy_assets::install::FileMgr::CFileMgr;
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_assets::streaming::CdStream::CdStream;
use re3bevy_core::core::Clock::CClock;
use re3bevy_core::core::Coordinates::CVector;
use re3bevy_core::core::General::CGeneral;
use re3bevy_core::core::Timer::CTimer;
use re3bevy_formats::formats::AnimManager::{IfpFile, IfpKeyFrame, IfpSequence};
use re3bevy_formats::formats::ClumpRead::{RwClumpSummary, read_chunk_header, read_clump};
use re3bevy_formats::formats::ColRead::{
    GtaColBox, GtaColFile, GtaColLine, GtaColSphere, GtaColTriangle, GtaCompressedVector,
    read_col_file,
};
use re3bevy_formats::formats::DataRead::{
    CarcolsDat, HandlingCfg, PedgrpDat, SurfaceDat, WeaponDat,
};
use re3bevy_formats::formats::Directory::CDirectory;
use re3bevy_formats::formats::FileLoader::{
    GtaDatCommand, GtaDatPlan, GtaIdePlan, GtaIdeRecord, GtaIplPlan, GtaIplRecord,
};
use re3bevy_formats::formats::HAnimRead::{read_hanim_animation, read_hanim_keyframe};
use re3bevy_formats::formats::Script::{
    SIZE_MAIN_SCRIPT, SIZE_MISSION_SCRIPT, ScmFile, USED_OBJECT_NAME_LENGTH,
    scan_instruction_headers,
};
use re3bevy_formats::formats::TexRead::{
    RwTexDictionarySummary, RwTextureNativeDescriptor, decode_txd_texture, read_txd,
};
use re3bevy_formats::formats::Text::GxtFile;
use re3bevy_game::animation::AnimManager::CAnimManager;
use re3bevy_game::animation::CutsceneMgr::{
    CCutsceneMgr, CUTSCENE_AUDIO_TRACKS, CutsceneAudioAction, CutsceneLoadStatus,
};
use re3bevy_game::audio::AudioLogic::{
    BUS, COACH, FIRETRUK, LANDSTAL, VehicleAudioSnapshot, VehicleStatusForAudio,
    process_vehicle_engine, process_vehicle_horn,
};
use re3bevy_game::audio::AudioManager::cAudioManager;
use re3bevy_game::audio::MusicManager::{MusicCommand, MusicServiceContext, cMusicManager};
use re3bevy_game::audio::audio_enums::{
    MUSICMODE_CUTSCENE, MUSICMODE_GAME, NO_TRACK, TOTAL_STREAMED_SOUNDS,
};
use re3bevy_game::collision::ColModel::{CColBox, CColModel, CColPoint, CColSphere, CColVector};
use re3bevy_game::collision::Collision::CCollision;
use re3bevy_game::control::PathFind::{CPathFind, CPathNode, PATH_CAR, PATH_PED};
use re3bevy_game::control::Replay::{
    CAddressInReplayBuffer, CCompressedMatrixNotAlignedReplay, CReplay, ReplayPacket,
    ReplayRuntimeWorldState, ReplayVehicleMutation, ReplayVehicleRuntimeState, TClockPacket,
    TGeneralPacket, TTimerPacket, TVehicleUpdatePacket, TWeatherPacket,
};
use re3bevy_game::control::Script::{
    CTheScripts, ScriptCommandEffect, ScriptEntitySnapshot, ScriptPlayerSnapshot,
};
use re3bevy_game::control::ScriptCommands::{
    COMMAND_LOAD_CUTSCENE, COMMAND_SCRIPT_NAME, COMMAND_SET_INTRO_IS_PLAYING,
    COMMAND_START_CUTSCENE,
};
use re3bevy_game::core::Camera::{CCamera, MAX_CAMERA_SPLINE_PATH_LENGTH, MAX_NUM_OF_SPLINETYPES};
use re3bevy_game::core::FileLoader::CFileLoader;
use re3bevy_game::core::FrontendFlow::{
    FrontendTransitionInput, FrontendTransitionOutcome, apply_frontend_menu_transition,
    boot_to_frontend_state_sequence_snapshot, frontend_main_menu_layout_snapshot,
    frontend_transition_snapshot, initialise_frontend_transition_menu,
};
use re3bevy_game::core::Game::{
    PedAutomobileProfile, ScriptGameplayBridge, WORLD_SPAWN_RADIUS, apply_script_effects_to_world,
    count_world_entities_near,
};
use re3bevy_game::core::Pad::{CPad, controller_state_from_keyboard};
use re3bevy_game::core::Stats::CStats;
use re3bevy_game::core::Streaming::{
    CStreaming, CStreamingModelInfo, STREAM_OFFSET_TXD, StreamLoadState,
};
use re3bevy_game::core::World::CTheZones;
use re3bevy_game::core::{Crime::eCrimeType, Wanted::CWanted};
use re3bevy_game::entities::Entity::eEntityType;
use re3bevy_game::entities::Physical::{
    CPhysical, CPhysicalCollisionOptions, CPhysicalStaticObjectCollisionOptions,
};
use re3bevy_game::math::Matrix::CMatrix;
use re3bevy_game::math::Vector::CVector as GameCVector;
use re3bevy_game::modelinfo::ModelIndices::{MI_AIRTRAIN, MI_DEADDODO};
use re3bevy_game::modelinfo::ModelInfo::CModelInfo;
use re3bevy_game::modelinfo::VehicleModelInfo::{AuthoredVehiclePosition, VehicleDamageNodes};
use re3bevy_game::objects::ParticleObject::CRGBA;
use re3bevy_game::peds::Gangs::CGangs;
use re3bevy_game::peds::Ped::{CPed, PedState};
use re3bevy_game::peds::PlayerPed::CPlayerPed;
use re3bevy_game::renderer::Font::{CFont, FONT_BANK, FONT_HEADING};
use re3bevy_game::renderer::Timecycle::{CTimeCycle, TimeCycleUpdateContext};
use re3bevy_game::renderer::WaterLevel::CWaterLevel;
use re3bevy_game::renderer::Weather::{CWeather, WeatherUpdateContext};
use re3bevy_game::save::GenericGameStorage::{
    BLOCK_COUNT, CGenericGameStorage, SIZE_OF_ONE_GAME_IN_BYTES, SIZE_OF_SIMPLEVARS, SaveBlock,
    SaveBlockKind, SaveSimpleVars, SaveSystemTime, WORK_BUFF_SIZE, build_generic_save_blocks,
    encode_pc_save_image, split_generic_load_blocks,
};
use re3bevy_game::save::PCSave::C_PcSave;
use re3bevy_game::vehicles::Automobile::{
    AutomobileDamageOrigin, CARWHEEL_FRONT_LEFT, CARWHEEL_FRONT_RIGHT, CAutomobile,
    CAutomobileWheelProcessInput, HANDLING_1G_BOOST, HANDLING_2G_BOOST, HANDLING_NEUTRALHANDLING,
    NUM_CAR_WHEELS, eCarPiece,
};
use re3bevy_game::vehicles::DamageManager::{CDamageManager, tComponent};
use re3bevy_game::vehicles::HandlingMgr::{cHandlingDataMgr, tVehicleType};
use re3bevy_game::vehicles::Plane::{
    CPlane, CPlaneInterpolationLine, CPlaneNode, CPlaneRuntime, CesnaMissionStatus,
};
use re3bevy_game::vehicles::Transmission::cTransmission;
use re3bevy_game::vehicles::Vehicle::{CVehicle, MISSION_VEHICLE, PERMANENT_VEHICLE, tWheelState};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct CaseConfig {
    name: String,
    seed: u32,
    count: usize,
    elapsed_ms: Vec<f64>,
    timer_ms: Vec<u32>,
    events: Vec<String>,
    slot_states: Vec<i32>,
    milliseconds_per_game_minute: u32,
    asset_root_config: String,
    relative_path: String,
    default_relative_path: String,
    gta3_relative_path: String,
    record_playback: bool,
    chase_recording: bool,
    debug_fast_forward: bool,
    forced_weather_type: i16,
    image_file: String,
    time_step_ms: u32,
    case_dir: PathBuf,
}

impl Default for CaseConfig {
    fn default() -> Self {
        Self {
            name: "random_sequence".to_string(),
            seed: 1,
            count: 16,
            elapsed_ms: Vec::new(),
            timer_ms: Vec::new(),
            events: Vec::new(),
            slot_states: Vec::new(),
            milliseconds_per_game_minute: 1000,
            asset_root_config: "config/original_gta3_path.txt".to_string(),
            relative_path: String::new(),
            default_relative_path: String::new(),
            gta3_relative_path: String::new(),
            record_playback: false,
            chase_recording: false,
            debug_fast_forward: false,
            forced_weather_type: -1,
            image_file: String::new(),
            time_step_ms: 16,
            case_dir: PathBuf::new(),
        }
    }
}

fn trim_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn read_case_config(path: &Path) -> Result<CaseConfig, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut config = CaseConfig::default();

    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = trim_value(value);
        match key {
            "name" => config.name = value,
            "seed" => {
                config.seed = value
                    .parse()
                    .map_err(|err| format!("invalid seed in {}: {err}", path.display()))?
            }
            "count" => {
                config.count = value
                    .parse()
                    .map_err(|err| format!("invalid count in {}: {err}", path.display()))?
            }
            "elapsed_ms" => config.elapsed_ms = parse_f64_array(&value)?,
            "timer_ms" => config.timer_ms = parse_u32_array(&value)?,
            "events" => config.events = parse_string_array(&value),
            "slot_states" => config.slot_states = parse_i32_array(&value)?,
            "milliseconds_per_game_minute" => {
                config.milliseconds_per_game_minute = value.parse().map_err(|err| {
                    format!(
                        "invalid milliseconds_per_game_minute in {}: {err}",
                        path.display()
                    )
                })?
            }
            "asset_root_config" => config.asset_root_config = value,
            "relative_path" => config.relative_path = value,
            "default_relative_path" => config.default_relative_path = value,
            "gta3_relative_path" => config.gta3_relative_path = value,
            "record_playback" => config.record_playback = parse_bool(&value),
            "chase_recording" => config.chase_recording = parse_bool(&value),
            "debug_fast_forward" => config.debug_fast_forward = parse_bool(&value),
            "forced_weather_type" => {
                config.forced_weather_type = value.parse().map_err(|err| {
                    format!("invalid forced_weather_type in {}: {err}", path.display())
                })?
            }
            "image_file" => config.image_file = value,
            "time_step_ms" => {
                config.time_step_ms = value
                    .parse()
                    .map_err(|err| format!("invalid time_step_ms in {}: {err}", path.display()))?
            }
            _ => {}
        }
    }

    config.case_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    Ok(config)
}

fn parse_f64_array(value: &str) -> Result<Vec<f64>, String> {
    let body = value.trim().trim_start_matches('[').trim_end_matches(']');
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }
    body.split(',')
        .map(|part| {
            trim_value(part)
                .parse()
                .map_err(|err| format!("invalid f64 array value `{part}`: {err}"))
        })
        .collect()
}

fn parse_u32_array(value: &str) -> Result<Vec<u32>, String> {
    let body = value.trim().trim_start_matches('[').trim_end_matches(']');
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }
    body.split(',')
        .map(|part| {
            trim_value(part)
                .parse()
                .map_err(|err| format!("invalid u32 array value `{part}`: {err}"))
        })
        .collect()
}

fn parse_i32_array(value: &str) -> Result<Vec<i32>, String> {
    let body = value.trim().trim_start_matches('[').trim_end_matches(']');
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }
    body.split(',')
        .map(|part| {
            trim_value(part)
                .parse()
                .map_err(|err| format!("invalid i32 array value `{part}`: {err}"))
        })
        .collect()
}

fn parse_string_array(value: &str) -> Vec<String> {
    let body = value.trim().trim_start_matches('[').trim_end_matches(']');
    if body.trim().is_empty() {
        return Vec::new();
    }
    body.split(',')
        .map(trim_value)
        .filter(|part| !part.is_empty())
        .collect()
}

fn parse_bool(value: &str) -> bool {
    matches!(trim_value(value).as_str(), "true" | "1")
}

fn parse_args() -> Result<(PathBuf, PathBuf), String> {
    let mut args = env::args().skip(1);
    let mut case_file = None;
    let mut out_file = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case-file" => case_file = args.next().map(PathBuf::from),
            "--out" => out_file = args.next().map(PathBuf::from),
            _ => {}
        }
    }

    let case_file = case_file.ok_or("missing --case-file")?;
    let out_file = out_file.ok_or("missing --out")?;
    Ok((case_file, out_file))
}

fn main() -> Result<(), String> {
    let (case_file, out_file) = parse_args()?;
    let config = read_case_config(&case_file)?;

    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    let json = match config.name.as_str() {
        "random_sequence" => run_random_sequence(&config),
        "timer_update" => run_timer_update(&config),
        "clock_update" => run_clock_update(&config),
        "dat_default_gta3" => run_dat_default_gta3(&config)?,
        "handling_cfg" => run_handling_cfg(&config)?,
        "surface_dat" => run_surface_dat(&config)?,
        "timecyc_dat" => run_timecyc_dat(&config)?,
        "waterpro_dat" => run_waterpro_dat(&config)?,
        "carcols_dat" => run_carcols_dat(&config)?,
        "pedgrp_dat" => run_pedgrp_dat(&config)?,
        "scm_raw_script_space" => run_scm_raw_script_space(&config)?,
        "gxt_text" => run_gxt_text(&config)?,
        "img_dir" => run_img_dir(&config)?,
        "ide_modelinfo" => run_ide_modelinfo(&config)?,
        "ipl_world_instances" => run_ipl_world_instances(&config)?,
        "mapzone" => run_mapzone(&config)?,
        "txd_metadata" => run_txd_metadata(&config)?,
        "txd_raster_decode_stats" => run_txd_raster_decode_stats(&config)?,
        "dff_clump_metadata" => run_dff_clump_metadata(&config)?,
        "dff_geometry_metadata" => run_dff_geometry_metadata(&config)?,
        "model_file_atomic_binding" => run_model_file_atomic_binding(&config)?,
        "col_model_metadata" => run_col_model_metadata(&config)?,
        "ifp_anim_metadata" => run_ifp_anim_metadata(&config)?,
        "weapon_dat" => run_weapon_dat(&config)?,
        "weather_update" => run_weather_update(&config),
        "timecycle_update" => run_timecycle_update(&config)?,
        "cutscene_audio_tracks" => run_cutscene_audio_tracks(&config),
        "cutscene_music_lifecycle" => run_cutscene_music_lifecycle(&config),
        "frontend_menu_transition" => run_frontend_menu_transition(&config),
        "frontend_start_new_game" => run_frontend_start_new_game(&config),
        "streaming_initial_requests" => run_streaming_initial_requests(&config)?,
        "font_layout" => run_font_layout(&config),
        "frontend_main_menu_layout" => run_frontend_main_menu_layout(&config),
        "boot_to_frontend_state_sequence" => run_boot_to_frontend_state_sequence(&config),
        "transmission_accel" => run_transmission_accel(&config),
        "simple_model_lod" => run_simple_model_lod(&config),
        "taxi_full_throttle_90_frames"
        | "taxi_brake_from_speed_60_frames"
        | "taxi_steer_left_constant_speed_60_frames"
        | "taxi_handbrake_turn_60_frames" => run_taxi_full_throttle_90_frames(&config)?,
        "save_load_slot_roundtrip" => run_save_load_slot_roundtrip(&config, &out_file)?,
        "replay_vehicle_frame" => run_replay_vehicle_frame(&config)?,
        "intro_cutscene_state" => run_intro_cutscene_state(&config),
        "cutscene_load_intro" => run_cutscene_load_intro(&config)?,
        "cutscene_load_bet" => run_cutscene_load_bet(&config)?,
        "cutscene_head_anm_bet_cat" => run_cutscene_head_anm_bet_cat(&config)?,
        "cutscene_hanim_archive_inventory" => run_cutscene_hanim_archive_inventory(&config)?,
        "scm_intro_mission_launch" => run_scm_intro_mission_launch(&config)?,
        "scm_vm_core" => run_scm_vm_core(&config)?,
        "scm_intro_cutscene_runtime" => run_scm_intro_cutscene_runtime(&config)?,
        "scm_intro_cutscene_objects" => run_scm_intro_cutscene_objects(&config)?,
        "world_spawn_ipl_occupancy" => run_world_spawn_ipl_occupancy(&config)?,
        "playerped_constructor_defaults" => run_playerped_constructor_defaults(&config),
        "follow_ped_camera_placement" => run_follow_ped_camera_placement(&config),
        "player_zelda_control_step" => run_player_zelda_control_step(&config),
        "physical_force_integration" => run_physical_force_integration(&config),
        "physical_environment_forces" => run_physical_environment_forces(&config),
        "physical_collision_alt" => run_physical_collision_alt(&config),
        "physical_collision_dynamic" => run_physical_collision_dynamic(&config),
        "physical_collision_ped_vehicle" => run_physical_collision_ped_vehicle(&config),
        "ped_kill_with_car_threshold" => run_ped_kill_with_car_threshold(&config),
        "ped_car_player_fall" => run_ped_car_player_fall(&config),
        "collision_process_col_models_spheres" => run_collision_process_col_models_spheres(&config),
        "physical_process_shift" => run_physical_process_shift(&config),
        "object_damage_smash_cardboard" => run_object_damage_smash_cardboard(&config),
        "object_world_control_step" => run_object_world_control_step(&config),
        "object_vehicle_collision_lock" => run_object_vehicle_collision_lock(&config),
        "physical_static_object_damage" => run_physical_static_object_damage(&config),
        "ped_animation_root_velocity" => run_ped_animation_root_velocity(&config),
        "wanted_chaos_thresholds" => run_wanted_chaos_thresholds(&config),
        "wanted_update_decay_slots" => run_wanted_update_decay_slots(&config),
        "wanted_crime_queue_timing" => run_wanted_crime_queue_timing(&config),
        "vehicle_component_damage_thresholds" => run_vehicle_component_damage_thresholds(&config),
        "automobile_collision_piece_damage" => run_automobile_collision_piece_damage(&config),
        "vehicle_audio_model_selection" => run_vehicle_audio_model_selection(&config),
        "automobile_fire_particle_origin" => run_automobile_fire_particle_origin(&config),
        "ped_vehicle_special_animation_paths" => run_ped_vehicle_special_animation_paths(&config),
        "pathfind_closest_node" => run_pathfind_closest_node(&config),
        "pathfind_favour_direction" => run_pathfind_favour_direction(&config),
        "pathfind_node_orientation" => run_pathfind_node_orientation(&config),
        "mission_audio_lifecycle" => run_mission_audio_lifecycle(&config),
        "mission_audio_uninitialised_api" => run_mission_audio_uninitialised_api(&config),
        "plane_path_process_control" => run_plane_path_process_control(&config),
        other => return Err(format!("unsupported case: {other}")),
    };

    fs::write(&out_file, json)
        .map_err(|err| format!("failed to write {}: {err}", out_file.display()))?;

    Ok(())
}

fn run_plane_path_process_control(config: &CaseConfig) -> String {
    let nodes = vec![
        CPlaneNode {
            p: CVector::new(0.0, 0.0, 10.0),
            t: 0.0,
            on_ground: true,
        },
        CPlaneNode {
            p: CVector::new(100.0, 0.0, 10.0),
            t: 100.0,
            on_ground: true,
        },
        CPlaneNode {
            p: CVector::new(100.0, 100.0, 30.0),
            t: 200.0,
            on_ground: false,
        },
        CPlaneNode {
            p: CVector::new(0.0, 100.0, 30.0),
            t: 300.0,
            on_ground: false,
        },
    ];
    let runtime = CPlaneRuntime {
        path_nodes: nodes.clone(),
        path2_nodes: nodes.clone(),
        path3_nodes: nodes.clone(),
        path4_nodes: nodes,
        total_length: [400.0; 4],
        total_duration: [8.0; 4],
        landing_point: 300.0,
        take_off_point: 100.0,
        interpolation_lines: [CPlaneInterpolationLine::default(); 6],
        plane_path_position: [130.0, 15.0, 0.0],
        old_plane_path_position: [299.0, 0.0, 0.0],
        plane_path_speed: [48.0, 12.0, 0.0],
        plane_path2_position: [260.0, 0.0, 0.0],
        plane_path2_speed: [25.0, 0.0, 0.0],
        plane_path3_position: 260.0,
        plane_path3_speed: 50.0,
        plane_path4_position: 260.0,
        plane_path4_speed: 50.0,
        cesna_mission_status: CesnaMissionStatus::Flying,
        cesna_mission_start_time: 0,
        drop_off_mission_status: CesnaMissionStatus::Flying,
        drop_off_mission_start_time: 0,
    };
    let mut general = CGeneral::with_seed(7);
    let mut cases = [
        CPlane::new(MI_AIRTRAIN as i32, PERMANENT_VEHICLE, 0, &mut general),
        CPlane::new(MI_AIRTRAIN as i32, PERMANENT_VEHICLE, 0, &mut general),
        CPlane::new(MI_DEADDODO as i32, PERMANENT_VEHICLE, 0, &mut general),
    ];
    cases[0].plane_id = 0;
    cases[1].plane_id = 1;
    cases[2].plane_id = 0;
    let labels = ["airtrain_curve", "airtrain_wrap", "deaddodo_curve"];
    let cameras = [
        CVector::new(100.0, 50.0, 20.0),
        CVector::new(1000.0, 1000.0, 0.0),
        CVector::new(1000.0, 1000.0, 0.0),
    ];

    let mut snapshots = Vec::new();
    for index in 0..cases.len() {
        let result = cases[index].process_path_control(&runtime, cameras[index], 8);
        let physical = &cases[index].vehicle.physical;
        let matrix = physical.entity.matrix;
        snapshots.push(format!(
            concat!(
                "    {{\"label\":\"{}\",\"updated\":{},\"landing_sound\":{},",
                "\"cur_node\":{},\"far\":{},\"speed\":{:.9},",
                "\"position\":[{:.9},{:.9},{:.9}],",
                "\"right\":[{:.9},{:.9},{:.9}],",
                "\"forward\":[{:.9},{:.9},{:.9}],",
                "\"up\":[{:.9},{:.9},{:.9}],",
                "\"move_speed\":[{:.9},{:.9},{:.9}]}}"
            ),
            labels[index],
            result.transform_updated,
            result.plane_on_ground_sound,
            cases[index].cur_path_node,
            cases[index].is_far_away,
            cases[index].speed,
            matrix.position.x,
            matrix.position.y,
            matrix.position.z,
            matrix.right.x,
            matrix.right.y,
            matrix.right.z,
            matrix.forward.x,
            matrix.forward.y,
            matrix.forward.z,
            matrix.up.x,
            matrix.up.y,
            matrix.up.z,
            physical.move_speed.x,
            physical.move_speed.y,
            physical.move_speed.z,
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_mission_audio_uninitialised_api(config: &CaseConfig) -> String {
    let mut audio = cAudioManager::default();
    audio.PreloadMissionAudio(123);
    audio.SetMissionAudioLocation(1.0, 2.0, 3.0);
    audio.PlayLoadedMissionAudio();
    audio.ProcessMissionAudio();
    audio.ClearMissionAudio();
    let pulse_count = (0..64)
        .filter(|_| audio.IsMissionAudioSampleFinished())
        .count();
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"sample\": {},\n  \"loading_field\": {},\n  \"loading_query\": {},\n  \"play\": {},\n  \"allowed\": {},\n  \"playing\": {},\n  \"is_2d\": {},\n  \"pretend_finished_pulses\": {}\n}}\n",
        config.name,
        audio.m_nMissionAudioSampleIndex,
        audio.m_nMissionAudioLoadingStatus,
        audio.GetMissionAudioLoadingStatus(),
        audio.m_nMissionAudioPlayStatus,
        audio.m_bIsMissionAudioAllowedToPlay,
        audio.m_bIsMissionAudioPlaying,
        audio.m_bIsMissionAudio2D,
        pulse_count,
    )
}

fn run_mission_audio_lifecycle(config: &CaseConfig) -> String {
    let mut audio = cAudioManager::default();
    audio.m_bIsInitialised = true;
    let mut rows = Vec::new();
    let mut capture = |index: usize, audio: &cAudioManager| {
        rows.push(format!(
        "    {{\"index\": {index}, \"sample\": {}, \"loading\": {}, \"play\": {}, \"allowed\": {}, \"playing\": {}, \"is_2d\": {}}}",
        audio.m_nMissionAudioSampleIndex, audio.m_nMissionAudioLoadingStatus,
        audio.m_nMissionAudioPlayStatus, audio.m_bIsMissionAudioAllowedToPlay,
        audio.m_bIsMissionAudioPlaying, audio.m_bIsMissionAudio2D,
    ))
    };
    capture(0, &audio);
    audio.PreloadMissionAudio(123);
    capture(1, &audio);
    audio.ProcessMissionAudio();
    capture(2, &audio);
    audio.SetMissionAudioLocation(1.0, 2.0, 3.0);
    audio.PlayLoadedMissionAudio();
    capture(3, &audio);
    audio.ProcessMissionAudio();
    capture(4, &audio);
    audio.ProcessMissionAudio();
    // re3's second PLAYING service observes SampleManager::IsStreamPlaying.
    // The Bevy adapter reports the equivalent backend fact explicitly.
    audio.MarkMissionAudioBackendPlaying();
    capture(5, &audio);
    audio.ClearMissionAudio();
    capture(6, &audio);
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        rows.join(",\n")
    )
}

fn run_pathfind_node_orientation(config: &CaseConfig) -> String {
    let mut paths = CPathFind::init();
    paths.path_nodes[0] = CPathNode {
        pos: CVector::ZERO,
        first_link: 0,
        num_links: 3,
        unk_bits: 1,
        ..Default::default()
    };
    paths.path_nodes[1] = CPathNode {
        pos: CVector::new(5.0, 0.0, 0.0),
        unk_bits: 1,
        ..Default::default()
    };
    paths.path_nodes[2] = CPathNode {
        pos: CVector::new(0.0, 5.0, 0.0),
        unk_bits: 1,
        ..Default::default()
    };
    paths.path_nodes[3] = CPathNode {
        pos: CVector::new(-5.0, 0.0, 0.0),
        unk_bits: 1,
        ..Default::default()
    };
    paths.path_nodes[4] = CPathNode {
        pos: CVector::new(1.0, 1.0, 0.0),
        unk_bits: 1,
        ..Default::default()
    };
    paths.connections = vec![1, 2, 3];
    let snapshots = [
        (0, paths.find_node_orientation_for_car_placement(4)),
        (1, paths.find_node_orientation_for_car_placement(0)),
        (
            2,
            paths.find_node_orientation_for_car_placement_facing_destination(0, 0.0, 10.0, true),
        ),
        (
            3,
            paths.find_node_orientation_for_car_placement_facing_destination(0, 0.0, 10.0, false),
        ),
        (
            4,
            paths.find_node_orientation_for_car_placement_facing_destination(0, -10.0, 0.0, true),
        ),
    ];
    let rows: Vec<String> = snapshots
        .into_iter()
        .map(|(index, heading)| {
            format!("    {{\"index\": {index}, \"heading_degrees\": {heading}}}")
        })
        .collect();
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        rows.join(",\n")
    )
}

fn run_pathfind_favour_direction(config: &CaseConfig) -> String {
    let mut paths = CPathFind::init();
    let node = |pos: CVector, unk_bits: u8| CPathNode {
        pos,
        unk_bits,
        ..Default::default()
    };
    paths.path_nodes[0] = node(CVector::new(5.0, 0.0, 0.0), 1);
    paths.path_nodes[1] = node(CVector::new(-2.0, 0.0, 0.0), 2);
    paths.path_nodes[2] = node(CVector::new(0.0, 3.0, 0.0), 1);
    paths.path_nodes[3] = node(CVector::new(20.0, 0.0, 0.0), 1);
    paths.path_nodes[4] = node(CVector::new(1.0, 0.0, 0.0), 0);
    paths.num_car_path_nodes = 3;
    paths.num_ped_path_nodes = 2;
    paths.num_path_nodes = 5;
    let queries = [
        (PATH_CAR, 1.0, 0.0),
        (PATH_CAR, -1.0, 0.0),
        (PATH_CAR, 0.0, 1.0),
        (PATH_PED, 1.0, 0.0),
    ];
    let snapshots: Vec<String> = queries.into_iter().enumerate().map(|(index, (path_type, dir_x, dir_y))| {
        let result = paths.find_node_closest_to_coors_favour_direction(CVector::ZERO, path_type, dir_x, dir_y);
        format!("    {{\"index\": {index}, \"path_type\": {path_type}, \"direction\": [{dir_x}, {dir_y}], \"result\": {result}}}")
    }).collect();
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_pathfind_closest_node(config: &CaseConfig) -> String {
    let mut paths = CPathFind::init();
    let node = |pos: CVector, unk_bits: u8, disabled: bool, between_levels: bool| CPathNode {
        pos,
        unk_bits,
        disabled,
        between_levels,
        ..Default::default()
    };
    paths.path_nodes[0] = node(CVector::new(0.0, 0.0, 0.0), 1, false, false);
    paths.path_nodes[1] = node(CVector::new(2.0, 0.0, 0.0), 2, true, false);
    paths.path_nodes[2] = node(CVector::new(0.0, 0.0, 1.0), 1, false, true);
    paths.path_nodes[3] = node(CVector::new(10.0, 0.0, 0.0), 1, false, false);
    paths.path_nodes[4] = node(CVector::new(9.0, 0.0, 0.0), 0, false, false);
    paths.num_car_path_nodes = 3;
    paths.num_ped_path_nodes = 2;
    paths.num_path_nodes = 5;
    let queries = [
        (CVector::new(1.5, 0.0, 0.0), PATH_CAR, 10.0, false, false),
        (CVector::new(1.5, 0.0, 0.0), PATH_CAR, 10.0, true, false),
        (CVector::new(0.0, 0.0, 0.9), PATH_CAR, 10.0, false, false),
        (CVector::new(0.0, 0.0, 0.9), PATH_CAR, 10.0, false, true),
        (CVector::new(0.0, 0.0, 0.0), PATH_CAR, 0.0, false, false),
        (CVector::new(9.0, 0.0, 0.0), PATH_PED, 2.0, false, false),
    ];
    let snapshots: Vec<String> = queries.into_iter().enumerate().map(|(index, (coors, path_type, limit, ignore_disabled, ignore_between))| {
        let result = paths.find_node_closest_to_coors(coors, path_type, limit, ignore_disabled, ignore_between);
        format!("    {{\"index\": {index}, \"path_type\": {path_type}, \"ignore_disabled\": {ignore_disabled}, \"ignore_between\": {ignore_between}, \"result\": {result}}}")
    }).collect();
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_automobile_collision_piece_damage(config: &CaseConfig) -> String {
    let absent_damage_nodes = VehicleDamageNodes::default();
    let scenarios = [
        (eCarPiece::CAR_PIECE_BUMP_FRONT, 20.0, false, false),
        (eCarPiece::CAR_PIECE_BUMP_FRONT, 40.0, false, false),
        (eCarPiece::CAR_PIECE_BUMP_FRONT, 40.0, true, false),
        (eCarPiece::CAR_PIECE_DOOR_LF, 40.0, false, true),
        (eCarPiece::CAR_PIECE_DOOR_LF, 40.0, false, false),
        (eCarPiece::CAR_PIECE_WING_LF, 30.0, false, false),
        (eCarPiece::CAR_PIECE_WHEEL_LF, 100.0, false, false),
    ];
    let mut snapshots = Vec::new();
    for (index, (piece, impulse, resistant, locked)) in scenarios.into_iter().enumerate() {
        let mut general = CGeneral::default();
        let mut car = CAutomobile::new(0, MISSION_VEHICLE, 0, &mut general, None, None, false);
        car.more_resistant_to_damage = resistant;
        car.vehicle.flags.is_locked = locked;
        let applied = car.apply_collision_piece_damage(impulse, piece, 1.0, &mut general, Some(&absent_damage_nodes));
        snapshots.push(format!(
            "    {{\"index\": {index}, \"piece\": {}, \"impulse\": {impulse}, \"resistant\": {resistant}, \"locked\": {locked}, \"applied\": {applied}, \"door0\": {}, \"door2\": {}, \"panel0\": {}, \"panel5\": {}, \"light0\": {}}}",
            piece as u16, car.damage.door_status[0], car.damage.door_status[2],
            car.damage.get_panel_status(0), car.damage.get_panel_status(5),
            car.damage.get_light_status(0),
        ));
    }
    let mut general = CGeneral::default();
    let mut car = CAutomobile::new(0, MISSION_VEHICLE, 0, &mut general, None, None, false);
    let damage_nodes = VehicleDamageNodes {
        bonnet: AuthoredVehiclePosition::Position(CVector::ZERO),
        ..Default::default()
    };
    let mut applied = false;
    for _ in 0..3 {
        applied |= car.apply_collision_piece_damage(
            40.0,
            eCarPiece::CAR_PIECE_BUMP_FRONT,
            1.0,
            &mut general,
            Some(&damage_nodes),
        );
    }
    snapshots.push(format!(
        "    {{\"index\": 7, \"piece\": 3, \"impulse\": 40, \"resistant\": false, \"locked\": false, \"applied\": {applied}, \"door0\": {}, \"door2\": 0, \"panel0\": 0, \"panel5\": {}, \"light0\": 0}}",
        car.damage.door_status[0], car.damage.get_panel_status(5),
    ));
    let mut general = CGeneral::default();
    let mut car = CAutomobile::new(0, MISSION_VEHICLE, 0, &mut general, None, None, false);
    car.vehicle.physical.entity.flags.collision_proof = true;
    let applied =
        car.apply_collision_piece_damage(100.0, eCarPiece::CAR_PIECE_BUMP_FRONT, 1.0, &mut general, Some(&absent_damage_nodes));
    snapshots.push(format!(
        "    {{\"index\": 8, \"piece\": 3, \"impulse\": 100, \"resistant\": false, \"locked\": false, \"applied\": {applied}, \"door0\": 0, \"door2\": 0, \"panel0\": 0, \"panel5\": {}, \"light0\": 0}}",
        car.damage.get_panel_status(5),
    ));
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_vehicle_audio_model_selection(config: &CaseConfig) -> String {
    let models = [
        ("landstal", LANDSTAL),
        ("firetruck", FIRETRUK),
        ("bus", BUS),
        ("coach", COACH),
    ];
    let snapshots = models
        .into_iter()
        .enumerate()
        .map(|(index, (name, model_index))| {
            let mut audio = cAudioManager::default();
            let snapshot = VehicleAudioSnapshot {
                audio_entity: 3,
                model_index,
                status: VehicleStatusForAudio::Player,
                horn_timer: 1,
                engine_on: true,
                gas_pedal: 0.8,
                gas_pedal_audio: 0.75,
                velocity_change: 0.2,
                max_velocity: 0.4,
                current_gear: 1,
                gear_shift_down_velocity: 0.05,
                first_gear_shift_up_velocity: 0.15,
                max_reverse_velocity: -0.2,
                ..Default::default()
            };
            process_vehicle_horn(&mut audio, &snapshot);
            let mut gas_pedal_audio = snapshot.gas_pedal_audio;
            process_vehicle_engine(&mut audio, &snapshot, &mut gas_pedal_audio);
            let queue = audio.m_nActiveQueue;
            let mut sounds = audio.m_aRequestedQueue[queue]
                [..audio.m_nRequestedCount[queue]]
                .iter()
                .map(|sound| {
                    (
                        sound.m_nCounter,
                        sound.m_nSampleIndex,
                        sound.m_nFrequency,
                        sound.m_nLoopCount,
                        sound.m_nVolume,
                    )
                })
                .collect::<Vec<_>>();
            sounds.sort_unstable_by_key(|sound| sound.0);
            let sounds = sounds
                .into_iter()
                .map(|(counter, sample, frequency, loop_count, volume)| {
                    format!(
                        "{{\"counter\": {counter}, \"sample\": {sample}, \"frequency\": {frequency}, \"loop_count\": {loop_count}, \"volume\": {volume}}}"
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "    {{\"index\": {index}, \"model\": \"{name}\", \"reverse_warning\": {}, \"air_brakes\": {}, \"sounds\": [{sounds}]}}",
                re3bevy_game::audio::AudioLogic::uses_reverse_warning(model_index),
                re3bevy_game::audio::AudioLogic::has_air_brakes(model_index),
            )
        })
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_automobile_fire_particle_origin(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut car = CAutomobile::new(0, MISSION_VEHICLE, 0, &mut general, None, None, false);
    car.vehicle
        .physical
        .entity
        .set_position(CVector::new(10.0, 20.0, 3.0));
    let headlights = CVector::new(1.0, 2.0, 0.5);
    car.damage.set_door_status(0, 0);
    let bonnet_present = car.damage_particle_position(headlights, AutomobileDamageOrigin::Fire { looking_forward_first_person: false });
    car.damage.set_door_status(0, 2);
    let bonnet_missing = car.damage_particle_position(headlights, AutomobileDamageOrigin::Fire { looking_forward_first_person: false });
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n    {{\"bonnet_status\": 0, \"position\": [{:.6}, {:.6}, {:.6}]}},\n    {{\"bonnet_status\": 2, \"position\": [{:.6}, {:.6}, {:.6}]}}\n  ]\n}}\n",
        config.name,
        bonnet_present.x,
        bonnet_present.y,
        bonnet_present.z,
        bonnet_missing.x,
        bonnet_missing.y,
        bonnet_missing.z,
    )
}

fn run_ped_vehicle_special_animation_paths(config: &CaseConfig) -> String {
    let scenarios = [
        (
            "standard_lf",
            PedAutomobileProfile::from_flags(false, false, false),
            2,
        ),
        (
            "low_rf",
            PedAutomobileProfile::from_flags(false, false, true),
            3,
        ),
        (
            "bus_lf",
            PedAutomobileProfile::from_flags(false, true, false),
            2,
        ),
        (
            "van_lr",
            PedAutomobileProfile::from_flags(true, false, false),
            4,
        ),
        (
            "van_rr",
            PedAutomobileProfile::from_flags(true, false, false),
            5,
        ),
    ];
    let snapshots = scenarios
        .into_iter()
        .enumerate()
        .map(|(index, (name, profile, door))| {
            format!(
                "    {{\"index\": {index}, \"name\": \"{name}\", \"door\": {door}, \"open\": {}, \"get_in\": {}, \"close_enter\": {}, \"exit\": {}, \"close_exit\": {}}}",
                profile.open_animation(door),
                profile.get_in_animation(door),
                profile.close_enter_animation(door),
                profile.exit_animation(door),
                profile.close_exit_animation(door),
            )
        })
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_vehicle_component_damage_thresholds(config: &CaseConfig) -> String {
    let samples = [
        (tComponent::COMPONENT_WHEEL_FRONT_LEFT, 120.0_f32),
        (tComponent::COMPONENT_WHEEL_FRONT_LEFT, 120.01),
        (tComponent::COMPONENT_DOOR_FRONT_LEFT, 46.875),
        (tComponent::COMPONENT_DOOR_FRONT_LEFT, 46.88),
        (tComponent::COMPONENT_DOOR_BONNET, 107.142_86),
        (tComponent::COMPONENT_DOOR_BONNET, 107.15),
        (tComponent::COMPONENT_PANEL_FRONT_LEFT, 53.571_43),
        (tComponent::COMPONENT_PANEL_FRONT_LEFT, 53.58),
        (tComponent::COMPONENT_BUMPER_FRONT, 60.0),
        (tComponent::COMPONENT_BUMPER_FRONT, 60.01),
        (tComponent::COMPONENT_DOOR_BONNET, 160.0),
    ];
    let mut snapshots = Vec::new();
    for (index, (component, input_damage)) in samples.into_iter().enumerate() {
        let mut manager = CDamageManager::new();
        let mut general = CGeneral::default();
        let applied = manager.apply_damage(component, input_damage, 1.0, &mut general);
        snapshots.push(format!(
            "    {{\"index\": {index}, \"component\": {}, \"input_damage\": {input_damage}, \"applied\": {applied}, \"wheel0\": {}, \"door0\": {}, \"door2\": {}, \"panel0\": {}, \"panel5\": {}, \"light0\": {}, \"engine\": {}}}",
            component as u8,
            manager.wheel_status[0], manager.door_status[0], manager.door_status[2],
            manager.get_panel_status(0), manager.get_panel_status(5),
            manager.get_light_status(0), manager.engine_status,
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_wanted_crime_queue_timing(config: &CaseConfig) -> String {
    let mut wanted = CWanted::initialise();
    wanted.register_crime(
        eCrimeType::CRIME_HIT_COP,
        CVector::new(1.0, 2.0, 3.0),
        77,
        true,
        0,
    );
    let mut snapshots = Vec::new();
    for (index, now_ms) in [500_u32, 501, 10_001].into_iter().enumerate() {
        wanted.update_crimes_queue(now_ms, false, 0);
        let crime = wanted.m_aCrimes[0];
        snapshots.push(format!(
            "    {{\"index\": {index}, \"now_ms\": {now_ms}, \"chaos\": {}, \"level\": {}, \"crime_type\": {}, \"reported\": {}}}",
            wanted.m_nChaos,
            wanted.m_nWantedLevel,
            crime.crime_type as u8,
            crime.reported,
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_wanted_update_decay_slots(config: &CaseConfig) -> String {
    let mut snapshots = Vec::new();
    let scenarios = [
        ("one_star_no_police", 40, 1, 0),
        ("one_star_police_present", 40, 1, 1),
        ("two_star_no_decay", 200, 2, 0),
    ];
    for (index, (name, chaos, level, police_presence)) in scenarios.into_iter().enumerate() {
        let mut wanted = CWanted::initialise();
        wanted.m_nChaos = chaos;
        wanted.m_nWantedLevel = level;
        wanted.m_pCops[0] = Some(10);
        wanted.m_pCops[2] = Some(20);
        wanted.m_pCops[4] = Some(30);
        wanted.update(1_001, police_presence, false, 0);
        let slots: Vec<String> = wanted
            .m_pCops
            .iter()
            .map(|slot| slot.map_or("null".to_string(), |value| value.to_string()))
            .collect();
        snapshots.push(format!(
            "    {{\"index\": {index}, \"scenario\": \"{name}\", \"chaos\": {}, \"level\": {}, \"last_update\": {}, \"last_change_ms\": {}, \"current_cops\": {}, \"cop_slots\": [{}]}}",
            wanted.m_nChaos,
            wanted.m_nWantedLevel,
            wanted.m_nLastUpdateTime,
            wanted.m_nLastWantedLevelChange,
            wanted.m_CurrentCops,
            slots.join(", "),
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_wanted_chaos_thresholds(config: &CaseConfig) -> String {
    let chaos_values = [
        -1, 0, 39, 40, 199, 200, 399, 400, 799, 800, 1599, 1600, 3199, 3200, 6401,
    ];
    let mut snapshots = Vec::new();
    for (index, chaos) in chaos_values.into_iter().enumerate() {
        let mut wanted = CWanted::initialise();
        wanted.m_nChaos = chaos;
        wanted.update_wanted_level_at(12_345);
        snapshots.push(format!(
            "    {{\"index\": {index}, \"input_chaos\": {chaos}, \"chaos\": {}, \"level\": {}, \"vehicles\": {}, \"max_cops\": {}, \"roadblock_density\": {}, \"last_change_ms\": {}}}",
            wanted.m_nChaos,
            wanted.m_nWantedLevel,
            wanted.m_MaximumLawEnforcerVehicles,
            wanted.m_MaxCops,
            wanted.m_RoadblockDensity,
            wanted.m_nLastWantedLevelChange,
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_ped_animation_root_velocity(config: &CaseConfig) -> String {
    let mut snapshots = Vec::new();
    for (index, attacking) in [false, true].into_iter().enumerate() {
        let mut general = CGeneral::default();
        let mut player = CPlayerPed::new(&mut general, None);
        player.locomotion.rotation_cur = 3.0;
        player.locomotion.rotation_dest = -3.0;
        player.locomotion.anim_move_delta = re3bevy_core::core::Coordinates::CVector2D::new(0.12, 0.25);
        if attacking {
            player.ped.ped_state = PedState::PED_ATTACK;
        }
        let moved = player.ped.calculate_new_velocity(&mut player.locomotion, &player.vitality, 0.5, attacking);
        snapshots.push(format!(
            "    {{\"index\": {index}, \"attacking\": {attacking}, \"rotation_cur\": {}, \"moved\": [{}, {}]}}",
            player.locomotion.rotation_cur, moved.x, moved.y
        ));
    }
    // Exercise re3's unusual sub-0.01 timestep fallback separately.
    let mut general = CGeneral::default();
    let mut player = CPlayerPed::new(&mut general, None);
    player.locomotion.rotation_cur = 0.2;
    player.locomotion.rotation_dest = 0.2;
    player.locomotion.anim_move_delta = re3bevy_core::core::Coordinates::CVector2D::new(0.12, 0.25);
    let moved = player.ped.calculate_new_velocity(&mut player.locomotion, &player.vitality, 0.005, false);
    snapshots.push(format!(
        "    {{\"index\": 2, \"attacking\": false, \"rotation_cur\": {}, \"moved\": [{}, {}]}}",
        player.locomotion.rotation_cur, moved.x, moved.y
    ));
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_physical_static_object_damage(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut physical = CPhysical::new(&mut general);
    physical.mass = 100.0;
    physical.turn_mass = 200.0;
    physical.elasticity = 0.5;
    physical.move_speed = CVector::new(-0.5, 0.0, 0.0);
    let mut point = CColPoint::new();
    point.point = CColVector::new(0.0, 1.0, 0.0);
    point.normal = CColVector::new(1.0, 0.0, 0.0);
    let result = physical.apply_collision_with_static_object(
        &point,
        CPhysicalStaticObjectCollisionOptions {
            object_uproot_limit: 100.0,
            object_collision_damage_effect: 1,
            object_uses_collision_after_damage: false,
            object_infinite_mass: false,
            object_is_glass: false,
            no_more_collision_torque: false,
        },
    );
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"applied\": {},\n  \"impulse\": {},\n  \"should_damage_object\": {},\n",
            "  \"damage_impulse\": {},\n  \"should_uproot_object\": {},\n",
            "  \"object_collision_disabled_by_damage\": {},\n",
            "  \"move_speed\": [{}, {}, {}],\n  \"turn_speed\": [{}, {}, {}]\n}}\n"
        ),
        config.name,
        result.applied,
        result.impulse,
        result.should_damage_object,
        result.damage_impulse,
        result.should_uproot_object,
        result.object_collision_disabled_by_damage,
        physical.move_speed.x,
        physical.move_speed.y,
        physical.move_speed.z,
        physical.turn_speed.x,
        physical.turn_speed.y,
        physical.turn_speed.z,
    )
}

fn run_physical_collision_alt(config: &CaseConfig) -> String {
    let mut snapshots = Vec::new();
    for (index, entity_type) in [
        eEntityType::ENTITY_TYPE_NOTHING,
        eEntityType::ENTITY_TYPE_VEHICLE,
    ]
    .into_iter()
    .enumerate()
    {
        let mut general = CGeneral::default();
        let mut physical = CPhysical::new(&mut general);
        physical.set_entity_type(entity_type);
        physical.mass = 100.0;
        physical.turn_mass = 200.0;
        physical.elasticity = 0.5;
        physical.move_speed = CVector::new(-0.5, 0.0, 0.0);
        let mut point = CColPoint::new();
        point.point = CColVector::new(0.0, 1.0, 0.0);
        point.normal = CColVector::new(1.0, 0.0, 0.0);
        let result = physical.apply_collision_alt(&point, true, 1.0);
        snapshots.push(format!(
            "    {{\"index\": {index}, \"vehicle\": {}, \"applied\": {}, \"impulse\": {}, \"move_speed\": [{}, {}, {}], \"turn_speed\": [{}, {}, {}]}}",
            entity_type == eEntityType::ENTITY_TYPE_VEHICLE,
            result.applied,
            result.impulse,
            result.move_speed.x, result.move_speed.y, result.move_speed.z,
            result.turn_speed.x, result.turn_speed.y, result.turn_speed.z,
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_physical_collision_dynamic(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut first = CPhysical::new(&mut general);
    first.set_entity_type(eEntityType::ENTITY_TYPE_VEHICLE);
    first.mass = 100.0;
    first.turn_mass = 200.0;
    first.elasticity = 0.5;
    first.move_speed = CVector::new(-0.5, 0.0, 0.0);

    let mut second = CPhysical::new(&mut general);
    second.set_entity_type(eEntityType::ENTITY_TYPE_VEHICLE);
    second.mass = 150.0;
    second.turn_mass = 300.0;
    second.elasticity = 0.25;

    let mut point = CColPoint::new();
    point.point = CColVector::new(0.0, 1.0, 0.0);
    point.normal = CColVector::new(1.0, 0.0, 0.0);
    let result =
        first.apply_collision_dynamic(&mut second, &point, CPhysicalCollisionOptions::default());
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"applied\": {},\n  \"impulse_a\": {},\n  \"impulse_b\": {},\n",
            "  \"move_a\": [{}, {}, {}],\n  \"turn_a\": [{}, {}, {}],\n",
            "  \"move_b\": [{}, {}, {}],\n  \"turn_b\": [{}, {}, {}]\n}}\n"
        ),
        config.name,
        result.applied,
        result.impulse_a,
        result.impulse_b,
        first.move_speed.x,
        first.move_speed.y,
        first.move_speed.z,
        first.turn_speed.x,
        first.turn_speed.y,
        first.turn_speed.z,
        second.move_speed.x,
        second.move_speed.y,
        second.move_speed.z,
        second.turn_speed.x,
        second.turn_speed.y,
        second.turn_speed.z,
    )
}

fn run_physical_collision_ped_vehicle(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut vehicle = CPhysical::new(&mut general);
    vehicle.set_entity_type(eEntityType::ENTITY_TYPE_VEHICLE);
    vehicle.mass = 100.0;
    vehicle.turn_mass = 200.0;
    vehicle.elasticity = 0.5;
    vehicle.move_speed = CVector::new(-0.5, 0.0, 0.0);

    let mut ped = CPhysical::new(&mut general);
    ped.set_entity_type(eEntityType::ENTITY_TYPE_PED);
    ped.entity.flags.ped_physics = true;
    ped.mass = 70.0;
    ped.turn_mass = 100.0;
    ped.elasticity = 0.05;

    let mut point = CColPoint::new();
    point.point = CColVector::new(0.0, 1.0, 0.0);
    point.normal = CColVector::new(1.0, 0.0, 0.0);
    let result =
        vehicle.apply_collision_dynamic(&mut ped, &point, CPhysicalCollisionOptions::default());
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"applied\": {},\n  \"impulse_vehicle\": {},\n",
            "  \"impulse_ped\": {},\n",
            "  \"vehicle_move\": [{}, {}, {}],\n",
            "  \"ped_move\": [{}, {}, {}],\n",
            "  \"ped_turn\": [{}, {}, {}]\n}}\n"
        ),
        config.name,
        result.applied,
        result.impulse_a,
        result.impulse_b,
        vehicle.move_speed.x,
        vehicle.move_speed.y,
        vehicle.move_speed.z,
        ped.move_speed.x,
        ped.move_speed.y,
        ped.move_speed.z,
        ped.turn_speed.x,
        ped.turn_speed.y,
        ped.turn_speed.z,
    )
}

fn run_ped_kill_with_car_threshold(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut ped = CPed::new(
        re3bevy_game::peds::PedType::ePedType::PEDTYPE_GANG1,
        &mut general,
    );
    let locomotion = re3bevy_game::peds::PedLocomotion::PedLocomotion::default();
    let mut vitality = re3bevy_game::peds::PedVitality::PedVitality::default();
    let transitioned = ped.kill_ped_with_car(&locomotion, &mut vitality, 17, 13.0, false);
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"transitioned\": {},\n  \"health\": {},\n",
            "  \"state\": \"{:?}\",\n  \"colliding_entity\": {}\n}}\n"
        ),
        config.name,
        transitioned,
        vitality.health,
        ped.ped_state,
        ped.colliding_entity.is_some(),
    )
}

fn run_ped_car_player_fall(config: &CaseConfig) -> String {
    use re3bevy_game::peds::Ped::{PedCarCollisionInput, PedCarCollisionOutcome};
    use re3bevy_game::peds::PedType::ePedType;

    let mut general = CGeneral::default();
    let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
    let locomotion = re3bevy_game::peds::PedLocomotion::PedLocomotion::default();
    let mut vitality = re3bevy_game::peds::PedVitality::PedVitality::default();
    ped.physical
        .entity
        .set_position(CVector::new(1.0, 0.0, 0.0));
    let result = ped.kill_ped_with_car_detailed(
        &locomotion,
        &mut vitality,
        PedCarCollisionInput {
            vehicle_slot: 17,
            impulse: 7.0,
            vehicle_move_speed: CVector::new(0.0, 2.0, 0.0),
            damage_normal: CVector::new(0.0, 0.0, -1.0),
            now_ms: 1_000,
            ..Default::default()
        },
        &mut general,
    );
    // The C++ comparison input provides a resident 1.25 s fall association.
    // Feed that same input to the production animation command consumer.
    let mut animation = re3bevy_game::peds::PedAnimation::PedAnimation::default();
    animation.movement_anim_clump.set_number_of_frames(1);
    animation.movement_anim_clump.frames[0].frame_name = "comparison_ped_frame".into();
    let mut association = re3bevy_game::animation::AnimBlendAssociation::CAnimBlendAssociation::new();
    association.init_with_hierarchy(
        re3bevy_game::animation::AnimBlendHierarchy::CAnimBlendHierarchy {
            total_length: 1.25,
            ..Default::default()
        },
        0,
    );
    association.anim_id = result.animation_id.expect("comparison fall animation");
    animation.movement_anim_clump.associations.push(association);
    assert!(re3bevy_game::peds::apply_pending_fall_animation(
        &mut ped,
        &mut animation,
        &CAnimManager::initialise(),
        &CFileLoader::default(),
    ));
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"fell\": {},\n  \"health\": {},\n  \"state\": \"{:?}\",\n",
            "  \"damage_direction\": {},\n  \"animation_id\": {},\n",
            "  \"get_up_timer\": {},\n  \"colliding_entity\": {},\n",
            "  \"ped_move\": [{}, {}, {}],\n",
            "  \"vehicle_move_force\": [{}, {}, {}]\n}}\n"
        ),
        config.name,
        result.outcome == PedCarCollisionOutcome::Fell,
        vitality.health,
        ped.ped_state,
        result.damage_direction,
        result.animation_id.unwrap_or(-1),
        ped.get_up_timer,
        ped.colliding_entity.is_some(),
        ped.physical.move_speed.x,
        ped.physical.move_speed.y,
        ped.physical.move_speed.z,
        result.vehicle_move_force.x,
        result.vehicle_move_force.y,
        result.vehicle_move_force.z,
    )
}

fn run_collision_process_col_models_spheres(config: &CaseConfig) -> String {
    let bounds = CColBox::new(
        CColVector::new(-1.0, -1.0, -1.0),
        CColVector::new(1.0, 1.0, 1.0),
        0,
        0,
    );
    let model_a = CColModel::new(
        CColSphere::new(1.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
        bounds.clone(),
        vec![
            CColSphere::new(0.75, CColVector::new(0.0, 0.0, 0.0), 10, 3),
            CColSphere::new(0.25, CColVector::new(3.0, 0.0, 0.0), 10, 9),
        ],
    );
    let model_b = CColModel::new(
        CColSphere::new(1.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
        bounds,
        vec![CColSphere::new(0.75, CColVector::new(0.0, 0.0, 0.0), 12, 4)],
    );
    let mut matrix_a = CMatrix::unity();
    matrix_a.position = CVector::new(0.5, 0.0, 0.0);
    let matrix_b = CMatrix::unity();
    let contacts = CCollision::process_col_models_spheres(&matrix_a, &model_a, &matrix_b, &model_b);
    let rows = contacts
        .iter()
        .enumerate()
        .map(|(index, point)| {
            format!(
                "    {{\"index\": {index}, \"point\": [{}, {}, {}], \"normal\": [{}, {}, {}], \"depth\": {}, \"surface_a\": {}, \"piece_a\": {}, \"surface_b\": {}, \"piece_b\": {}}}",
                point.point.x,
                point.point.y,
                point.point.z,
                point.normal.x,
                point.normal.y,
                point.normal.z,
                point.depth,
                point.surface_a,
                point.piece_a,
                point.surface_b,
                point.piece_b,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"count\": {},\n  \"contacts\": [\n{}\n  ]\n}}\n",
        config.name,
        contacts.len(),
        rows,
    )
}

fn run_physical_process_shift(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut physical = CPhysical::new(&mut general);
    let shifted =
        physical.apply_process_shift(re3bevy_core::core::Coordinates::CVector::new(0.0, 0.0, 1.2));
    let position = physical.entity.get_position();
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"position\": [{}, {}, {}],\n",
            "  \"distance_travelled\": {},\n",
            "  \"is_in_safe_position\": {}\n}}\n"
        ),
        config.name,
        position.x,
        position.y,
        position.z,
        physical.distance_travelled,
        shifted && physical.entity.flags.is_in_safe_position,
    )
}

fn run_object_damage_smash_cardboard(config: &CaseConfig) -> String {
    use re3bevy_game::objects::Object::{
        CObject, CObjectDamageContext, CObjectGlobalState, CollisionDamageEffect,
    };

    let mut general = CGeneral::default();
    let mut object = CObject::new_with_model(0, false, &mut general);
    object.collision_damage_multiplier = 1.0;
    object.collision_damage_effect =
        CollisionDamageEffect::DAMAGE_EFFECT_SMASH_CARDBOARD_COMPLETELY as u8;
    let mut globals = CObjectGlobalState::default();
    let result = object.object_damage(
        200.0,
        &mut globals,
        CObjectDamageContext {
            body_cast_model_index: None,
        },
        &mut general,
    );
    let sound = result
        .script_sound
        .map(|sound| format!("\"{sound:?}\""))
        .unwrap_or_else(|| "null".to_owned());
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"applied\": {},\n  \"smashed\": {},\n",
            "  \"visible\": {},\n  \"uses_collision\": {},\n",
            "  \"explosion_proof\": {},\n  \"frame_gen\": {},\n",
            "  \"debris_particles\": {},\n  \"script_sound\": {}\n}}\n"
        ),
        config.name,
        result.applied,
        result.smashed,
        object.physical.entity.flags.is_visible,
        object.physical.entity.flags.uses_collision,
        object.physical.entity.flags.explosion_proof,
        globals.frame_gen,
        result.debris_particles,
        sound,
    )
}

fn run_object_world_control_step(config: &CaseConfig) -> String {
    use re3bevy_game::objects::Object::CObject;

    let mut general = CGeneral::default();
    let mut object = CObject::new_with_model(0, false, &mut general);
    object.physical.entity.set_is_static(false);
    object.physical.mass = 100.0;
    object.physical.turn_mass = 200.0;
    object.physical.air_resistance = 0.85;
    object.physical.move_speed = CVector::new(1.0, 0.0, 0.2);
    let mut collision = re3bevy_game::objects::Object::ObjectCollisionState::default();
    let moved = object.process_world_control_step(1.25, &mut collision);
    let position = object.physical.entity.get_position();
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"moved\": {},\n  \"position\": [{}, {}, {}],\n",
            "  \"move_speed\": [{}, {}, {}],\n",
            "  \"distance_travelled\": {},\n  \"is_static\": {}\n}}\n"
        ),
        config.name,
        moved,
        position.x,
        position.y,
        position.z,
        object.physical.move_speed.x,
        object.physical.move_speed.y,
        object.physical.move_speed.z,
        object.physical.distance_travelled,
        object.physical.entity.flags.is_static,
    )
}

fn run_object_vehicle_collision_lock(config: &CaseConfig) -> String {
    use re3bevy_game::objects::Object::CObject;

    let mut general = CGeneral::default();
    let mut object = CObject::new_with_model(0, false, &mut general);
    object.physical.entity.set_is_static(false);
    object.colliding_entity = Some(7);
    let skip_while_touching = object.update_vehicle_collision_lock(7, true);
    let retained_slot = object.colliding_entity;
    let skip_after_separation = object.update_vehicle_collision_lock(7, false);
    let released_slot = object.colliding_entity;
    format!(
        concat!(
            "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n",
            "  \"skip_while_touching\": {},\n  \"retained_slot\": {},\n",
            "  \"skip_after_separation\": {},\n  \"released\": {}\n}}\n"
        ),
        config.name,
        skip_while_touching,
        retained_slot.unwrap_or(usize::MAX),
        skip_after_separation,
        released_slot.is_none(),
    )
}

fn run_physical_environment_forces(config: &CaseConfig) -> String {
    let mut snapshots = Vec::new();
    for (index, air_resistance) in [0.85_f32, 0.05_f32].into_iter().enumerate() {
        let mut general = CGeneral::default();
        let mut physical = CPhysical::new(&mut general);
        physical.flags.affected_by_gravity = true;
        physical.air_resistance = air_resistance;
        physical.move_speed = CVector::new(1.2, -0.8, 0.4);
        physical.turn_speed = CVector::new(0.3, -0.2, 0.1);
        physical.move_friction = CVector::new(-0.1, 0.05, -0.02);
        physical.turn_friction = CVector::new(0.01, -0.03, 0.02);
        physical.apply_gravity(1.25);
        physical.apply_friction();
        physical.apply_air_resistance(1.25);
        snapshots.push(format!(
            "    {{\"index\": {index}, \"air_resistance\": {air_resistance}, \"move_speed\": [{}, {}, {}], \"turn_speed\": [{}, {}, {}], \"move_friction\": [{}, {}, {}], \"turn_friction\": [{}, {}, {}]}}",
            physical.move_speed.x, physical.move_speed.y, physical.move_speed.z,
            physical.turn_speed.x, physical.turn_speed.y, physical.turn_speed.z,
            physical.move_friction.x, physical.move_friction.y, physical.move_friction.z,
            physical.turn_friction.x, physical.turn_friction.y, physical.turn_friction.z,
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"time_step\": 1.25,\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_physical_force_integration(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut physical = CPhysical::new(&mut general);
    physical.mass = 20.0;
    physical.turn_mass = 40.0;
    physical.set_center_of_mass(0.2, -0.1, 0.3);
    physical.entity.set_position(CVector::new(10.0, -5.0, 2.0));
    physical.apply_move_force(CVector::new(4.0, -2.0, 6.0));
    physical.apply_turn_force(CVector::new(4.0, -2.0, 6.0), CVector::new(1.5, 0.5, -0.25));
    physical.apply_turn_speed(0.75);
    physical.apply_move_speed(0.75);
    let matrix = physical.entity.matrix;
    let position = physical.entity.get_position();
    format!(
        concat!(
            "{{\n",
            "  \"case\": \"{}\",\n",
            "  \"engine\": \"rust-re3bevy\",\n",
            "  \"move_speed\": [{}, {}, {}],\n",
            "  \"turn_speed\": [{}, {}, {}],\n",
            "  \"position\": [{}, {}, {}],\n",
            "  \"right\": [{}, {}, {}],\n",
            "  \"forward\": [{}, {}, {}],\n",
            "  \"up\": [{}, {}, {}]\n",
            "}}\n"
        ),
        config.name,
        physical.move_speed.x,
        physical.move_speed.y,
        physical.move_speed.z,
        physical.turn_speed.x,
        physical.turn_speed.y,
        physical.turn_speed.z,
        position.x,
        position.y,
        position.z,
        matrix.right.x,
        matrix.right.y,
        matrix.right.z,
        matrix.forward.x,
        matrix.forward.y,
        matrix.forward.z,
        matrix.up.x,
        matrix.up.y,
        matrix.up.z,
    )
}

fn run_player_zelda_control_step(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let mut player = CPlayerPed::new(&mut general, None);
    player.active_in_world = true;
    let mut pad = CPad::default();
    pad.Update(controller_state_from_keyboard(
        true, false, false, true, true, false, false, false,
    ));
    let mut snapshots = Vec::new();
    for frame in 1..=10 {
        // C++ case uses CTimer::GetTimeStep() = 0.6, therefore
        // GetTimeStepInSeconds() is 0.6 / 50 = 0.012 seconds.
        let moved = player.apply_on_foot_pad_step_camera(&pad, 0.012, true, Some(0.3), false);
        snapshots.push(format!(
            "    {{\"frame\": {frame}, \"moved\": {moved}, \"move_speed\": {}, \"rotation_dest\": {}, \"rotation_cur\": {}, \"move_state\": \"{:?}\"}}",
            player.move_speed,
            player.locomotion.rotation_dest,
            player.locomotion.rotation_cur,
            player.ped.move_state,
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"input\": {{\"left_right\": 128, \"up_down\": -128, \"camera_orientation\": 0.3, \"time_step\": 0.6}},\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_follow_ped_camera_placement(config: &CaseConfig) -> String {
    let target = CVector::new(811.875, -939.938, 35.75);
    let states = [(-0.22_f32, 0.0_f32), (0.5, 1.0), (1.0, -2.5), (-2.0, 3.5)];
    let mut snapshots = Vec::new();
    for (index, (alpha, beta)) in states.into_iter().enumerate() {
        let mut camera = CCamera::default();
        camera.Init();
        camera.follow_initialised = true;
        camera.follow_alpha = alpha;
        camera.follow_beta = beta;
        camera.process_follow_ped_with_mouse(target, 0.0, |_target, source| {
            (index == 3).then_some(CVector::new(
                source.x + 0.25,
                source.y - 0.1,
                source.z + 0.05,
            ))
        });
        let front = camera.orientation;
        let dx = camera.look_at.x - camera.position.x;
        let dy = camera.look_at.y - camera.position.y;
        let dz = camera.look_at.z - camera.position.z;
        snapshots.push(format!(
            "    {{\"index\": {index}, \"alpha\": {}, \"beta\": {}, \"target\": [{}, {}, {}], \"source\": [{}, {}, {}], \"front\": [{}, {}, {}], \"distance\": {}}}",
            camera.follow_alpha,
            camera.follow_beta,
            camera.look_at.x,
            camera.look_at.y,
            camera.look_at.z,
            camera.position.x,
            camera.position.y,
            camera.position.z,
            front.x,
            front.y,
            front.z,
            (dx * dx + dy * dy + dz * dz).sqrt(),
        ));
    }
    format!(
        "{{\n  \"case\": \"{}\",\n  \"engine\": \"rust-re3bevy\",\n  \"snapshots\": [\n{}\n  ]\n}}\n",
        config.name,
        snapshots.join(",\n")
    )
}

fn run_playerped_constructor_defaults(config: &CaseConfig) -> String {
    let mut general = CGeneral::default();
    let player = CPlayerPed::new(&mut general, None);
    let safe_positions_zero = player
        .safe_pos
        .iter()
        .all(|position| *position == CVector::ZERO);
    let safe_peds_null = player.ped_at_safe_pos.iter().all(Option::is_none);
    format!(
        concat!(
            "{{\n",
            "  \"case\": \"{}\",\n",
            "  \"engine\": \"rust-re3bevy\",\n",
            "  \"model_id\": {},\n",
            "  \"move_speed\": {},\n",
            "  \"current_stamina\": {},\n",
            "  \"max_stamina\": {},\n",
            "  \"current_weapon\": \"unarmed\",\n",
            "  \"selected_weapon_slot\": \"unarmed\",\n",
            "  \"ped_state\": \"idle\",\n",
            "  \"can_be_damaged\": {},\n",
            "  \"targettable_objects\": {:?},\n",
            "  \"safe_positions_zero\": {},\n",
            "  \"safe_peds_null\": {},\n",
            "  \"active_in_world\": {}\n",
            "}}\n"
        ),
        config.name,
        player.ped.physical.entity.model_index,
        player.move_speed,
        player.current_stamina,
        player.max_stamina,
        player.can_be_damaged,
        player.targettable_objects,
        safe_positions_zero,
        safe_peds_null,
        player.active_in_world,
    )
}

fn run_world_spawn_ipl_occupancy(config: &CaseConfig) -> Result<String, String> {
    let install = OriginalGtaInstall::from_config_path(&config.asset_root_config)
        .map_err(|error| format!("install discovery failed: {error}"))?;
    let mut file_mgr = install.file_mgr();
    let mut cd_stream = CdStream::init(5);
    let mut file_loader = CFileLoader::default();
    file_loader
        .load_default_and_dat_plans(&install, &mut file_mgr, &mut cd_stream, "DATA\\GTA3.DAT")
        .map_err(|error| format!("world plan load failed: {error}"))?;

    let spawn = CVector::new(811.875, -939.938, 35.75);
    let total = file_loader.world.entities.len();
    let near = count_world_entities_near(&file_loader.world, spawn, WORLD_SPAWN_RADIUS);
    Ok(format!(
        concat!(
            "{{\n",
            "  \"case\": \"{}\",\n",
            "  \"engine\": \"rust-re3bevy\",\n",
            "  \"spawn\": [811.875, -939.938, 35.75],\n",
            "  \"radius\": 250.0,\n",
            "  \"ipl_entity_count\": {},\n",
            "  \"near_spawn_count\": {}\n",
            "}}\n"
        ),
        config.name, total, near
    ))
}

fn camera_spline_bits_checksum(values: &[f32]) -> u64 {
    values
        .iter()
        .enumerate()
        .fold(0u64, |checksum, (index, value)| {
            checksum.wrapping_add(u64::from(value.to_bits()).wrapping_mul(index as u64 + 1))
        })
}

fn camera_spline_stored_values(bytes: &[u8]) -> [usize; MAX_NUM_OF_SPLINETYPES] {
    let mut counts = [0usize; MAX_NUM_OF_SPLINETYPES];
    let mut spline = 0usize;
    for byte in bytes {
        if *byte == 0 || spline >= MAX_NUM_OF_SPLINETYPES {
            break;
        }
        if matches!(*byte, b',' | b';') {
            counts[spline] += 1;
            if *byte == b';' {
                spline += 1;
            }
        }
    }
    counts
}

fn run_cutscene_load_intro(config: &CaseConfig) -> Result<String, String> {
    run_cutscene_load(config, "INTRO")
}

fn run_cutscene_load_bet(config: &CaseConfig) -> Result<String, String> {
    run_cutscene_load(config, "BET")
}

fn run_cutscene_head_anm_bet_cat(config: &CaseConfig) -> Result<String, String> {
    const RW_ID_HANIMANIMATION: u32 = 0x1B;

    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|error| format!("failed to read {}: {error}", config.asset_root_config))?;
    let mut file_mgr = CFileMgr::initialise(PathBuf::from(root_text.trim()));
    let (directory_entry_count, entry) =
        CAnimationAssets::load_cutscene_archive_file(&mut file_mgr, "BET_CAT.ANM")
            .map_err(|error| format!("failed to load BET_CAT.ANM: {error}"))?;
    let entry = entry.ok_or("BET_CAT.ANM missing from CUTS.DIR")?;
    let chunk = read_chunk_header(&entry.bytes, 0).ok_or("BET_CAT.ANM missing RW chunk header")?;
    let hanim_found = chunk.id == RW_ID_HANIMANIMATION && chunk.data_end <= entry.bytes.len();
    let payload = if hanim_found {
        &entry.bytes[chunk.data_start..chunk.data_end]
    } else {
        &[]
    };
    let sector_padding_bytes = entry.bytes.len().saturating_sub(chunk.data_end);
    let first_payload_words = [
        read_u32_le(payload, 0).unwrap_or(0),
        read_u32_le(payload, 4).unwrap_or(0),
        read_u32_le(payload, 8).unwrap_or(0),
        read_u32_le(payload, 12).unwrap_or(0),
    ];
    let hanim = read_hanim_animation(&entry.bytes)?;
    let first_keyframe = payload.get(20..20 + hanim.keyframe_size).unwrap_or(&[]);
    let last_keyframe_start = 20usize
        .saturating_add(hanim.num_keyframes.saturating_sub(1) as usize * hanim.keyframe_size);
    let last_keyframe = payload
        .get(last_keyframe_start..last_keyframe_start + hanim.keyframe_size)
        .unwrap_or(&[]);
    let sample_indices = [
        0usize,
        1,
        2,
        100,
        200,
        hanim.num_keyframes.saturating_sub(1) as usize,
    ];
    let sampled_keyframes = sample_indices
        .iter()
        .map(|index| read_hanim_keyframe(&entry.bytes, *index))
        .collect::<Result<Vec<_>, _>>()?;

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_assets/src/assets/AnimationAssets.rs\",\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/ClumpRead.rs\",\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/HAnimRead.rs\",\n");
    json.push_str("    \"references/re-GTA/src/objects/CutsceneHead.cpp\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"archive\": {{\"directory_entry_count\": {}, \"anm_found\": true, \"anm_offset_sectors\": {}, \"anm_size_sectors\": {}, \"anm_byte_len\": {}, \"anm_checksum\": {}}},\n",
        directory_entry_count,
        entry.offset_sectors,
        entry.size_sectors,
        entry.bytes.len(),
        byte_checksum(&entry.bytes)
    ));
    json.push_str(&format!(
        "  \"chunk\": {{\"hanim_found\": {}, \"id\": {}, \"length\": {}, \"version\": {}, \"data_start\": {}, \"data_end\": {}, \"payload_checksum\": {}, \"sector_padding_bytes\": {}, \"first_payload_words\": [{}, {}, {}, {}]}},\n",
        hanim_found,
        chunk.id,
        chunk.length,
        chunk.version,
        chunk.data_start,
        chunk.data_end,
        byte_checksum(payload),
        sector_padding_bytes,
        first_payload_words[0],
        first_payload_words[1],
        first_payload_words[2],
        first_payload_words[3]
    ));
    json.push_str(&format!(
        "  \"animation\": {{\"type_id\": {}, \"flags\": {}, \"num_keyframes\": {}, \"custom_data_size\": {}, \"duration_bits\": {}, \"duration_seconds\": {:.6}, \"keyframe_size\": {}, \"keyframe_bytes\": {}, \"payload_bytes\": {}, \"expected_payload_bytes\": {}, \"payload_size_matches\": {}, \"first_keyframe_checksum\": {}, \"last_keyframe_checksum\": {}}},\n",
        hanim.type_id,
        hanim.flags,
        hanim.num_keyframes,
        hanim.custom_data_size,
        hanim.duration_seconds.to_bits(),
        hanim.duration_seconds,
        hanim.keyframe_size,
        hanim.keyframe_bytes,
        hanim.payload_bytes,
        hanim.expected_payload_bytes,
        hanim.payload_size_matches,
        byte_checksum(first_keyframe),
        byte_checksum(last_keyframe)
    ));
    json.push_str("  \"sampled_keyframes\": [\n");
    for (index, frame) in sampled_keyframes.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"time_bits\": {}, \"time_seconds\": {:.6}, \"quaternion_bits\": [{}, {}, {}, {}], \"translation_bits\": [{}, {}, {}], \"frame_tail_raw\": {}}}{}\n",
            frame.index,
            frame.time_bits,
            frame.time_seconds,
            frame.quaternion_bits[0],
            frame.quaternion_bits[1],
            frame.quaternion_bits[2],
            frame.quaternion_bits[3],
            frame.translation_bits[0],
            frame.translation_bits[1],
            frame.translation_bits[2],
            frame.frame_tail_raw,
            if index + 1 == sampled_keyframes.len() { "" } else { "," }
        ));
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_cutscene_hanim_archive_inventory(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|error| format!("failed to read {}: {error}", config.asset_root_config))?;
    let mut file_mgr = CFileMgr::initialise(PathBuf::from(root_text.trim()));
    let listings = CAnimationAssets::list_cutscene_archive(&mut file_mgr)
        .map_err(|error| format!("failed to list CUTS.DIR: {error}"))?;
    let anm_listings = listings
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.filename.to_ascii_lowercase().ends_with(".anm"))
        .collect::<Vec<_>>();

    let mut entries = Vec::new();
    let mut total_keyframes = 0u64;
    let mut total_size_sectors = 0u64;
    let mut file_checksum_accumulator = 0u64;
    let mut duration_bits_accumulator = 0u64;

    for (anm_index, (directory_index, listing)) in anm_listings.iter().enumerate() {
        let archive_entry = CAnimationAssets::load_cutscene_archive_listing(&mut file_mgr, listing)
            .map_err(|error| {
                format!(
                    "failed to load CUTS.IMG range for {} at sector {}: {error}",
                    listing.filename, listing.offset_sectors
                )
            })?;
        let chunk = read_chunk_header(&archive_entry.bytes, 0)
            .ok_or_else(|| format!("{} missing RW chunk header", listing.filename))?;
        let hanim = read_hanim_animation(&archive_entry.bytes)
            .map_err(|error| format!("failed to parse {} HAnim: {error}", listing.filename))?;
        let first_keyframe = archive_entry
            .bytes
            .get(chunk.data_start + 20..chunk.data_start + 20 + hanim.keyframe_size)
            .unwrap_or(&[]);
        let last_keyframe_start = chunk.data_start
            + 20usize.saturating_add(
                hanim.num_keyframes.saturating_sub(1) as usize * hanim.keyframe_size,
            );
        let last_keyframe = archive_entry
            .bytes
            .get(last_keyframe_start..last_keyframe_start + hanim.keyframe_size)
            .unwrap_or(&[]);
        let file_checksum = byte_checksum(&archive_entry.bytes);
        let first_keyframe_checksum = byte_checksum(first_keyframe);
        let last_keyframe_checksum = byte_checksum(last_keyframe);

        total_keyframes = total_keyframes.wrapping_add(u64::from(hanim.num_keyframes));
        total_size_sectors = total_size_sectors.wrapping_add(u64::from(archive_entry.size_sectors));
        file_checksum_accumulator = file_checksum_accumulator
            .wrapping_add(file_checksum.wrapping_mul(anm_index as u64 + 1));
        duration_bits_accumulator = duration_bits_accumulator.wrapping_add(
            u64::from(hanim.duration_seconds.to_bits()).wrapping_mul(anm_index as u64 + 1),
        );

        entries.push((
            anm_index,
            *directory_index,
            listing.filename.clone(),
            archive_entry.offset_sectors,
            archive_entry.size_sectors,
            archive_entry.bytes.len(),
            file_checksum,
            hanim,
            first_keyframe_checksum,
            last_keyframe_checksum,
        ));
    }

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_assets/src/assets/AnimationAssets.rs\",\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/HAnimRead.rs\",\n");
    json.push_str("    \"references/re-GTA/src/objects/CutsceneHead.cpp\",\n");
    json.push_str("    \"references/re-GTA/src/core/Directory.cpp\",\n");
    json.push_str("    \"references/re-GTA/src/fakerw/rphanim.h\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"summary\": {{\"directory_entry_count\": {}, \"anm_count\": {}, \"total_size_sectors\": {}, \"total_keyframes\": {}, \"file_checksum_accumulator\": {}, \"duration_bits_accumulator\": {}}},\n",
        listings.len(),
        entries.len(),
        total_size_sectors,
        total_keyframes,
        file_checksum_accumulator,
        duration_bits_accumulator
    ));
    json.push_str("  \"entries\": [\n");
    for (index, entry) in entries.iter().enumerate() {
        let (
            anm_index,
            directory_index,
            filename,
            offset_sectors,
            size_sectors,
            byte_len,
            file_checksum,
            hanim,
            first_keyframe_checksum,
            last_keyframe_checksum,
        ) = entry;
        json.push_str(&format!(
            "    {{\"anm_index\": {}, \"directory_index\": {}, \"name\": \"{}\", \"offset_sectors\": {}, \"size_sectors\": {}, \"byte_len\": {}, \"file_checksum\": {}, \"chunk_length\": {}, \"chunk_version\": {}, \"type_id\": {}, \"flags\": {}, \"num_keyframes\": {}, \"custom_data_size\": {}, \"duration_bits\": {}, \"duration_seconds\": {:.6}, \"keyframe_size\": {}, \"payload_bytes\": {}, \"expected_payload_bytes\": {}, \"payload_size_matches\": {}, \"first_keyframe_checksum\": {}, \"last_keyframe_checksum\": {}}}{}\n",
            anm_index,
            directory_index,
            json_escape(filename),
            offset_sectors,
            size_sectors,
            byte_len,
            file_checksum,
            hanim.chunk_length,
            hanim.chunk_version,
            hanim.type_id,
            hanim.flags,
            hanim.num_keyframes,
            hanim.custom_data_size,
            hanim.duration_seconds.to_bits(),
            hanim.duration_seconds,
            hanim.keyframe_size,
            hanim.payload_bytes,
            hanim.expected_payload_bytes,
            hanim.payload_size_matches,
            first_keyframe_checksum,
            last_keyframe_checksum,
            if index + 1 == entries.len() { "" } else { "," }
        ));
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_cutscene_load(config: &CaseConfig, cutscene_name: &str) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|error| format!("failed to read {}: {error}", config.asset_root_config))?;
    let mut file_mgr = CFileMgr::initialise(PathBuf::from(root_text.trim()));
    let loaded = CAnimationAssets::load_cutscene(&mut file_mgr, cutscene_name)
        .map_err(|error| format!("failed to load {cutscene_name} cutscene assets: {error}"))?;
    let camera_entry = loaded
        .camera_entry
        .as_ref()
        .ok_or_else(|| format!("{cutscene_name} camera entry missing from CUTS.DIR"))?;
    let mut camera = CCamera::default();
    camera.load_path_splines(&camera_entry.bytes)?;
    let stored_values = camera_spline_stored_values(&camera_entry.bytes);
    let mut anim_manager = CAnimManager::initialise();
    if let Some(animation_file) = loaded.animation_file.as_ref() {
        anim_manager.load_anim_file(animation_file);
    }
    let cutscene_hierarchies = anim_manager
        .get_animation_block(cutscene_name)
        .map(|block| {
            anim_manager
                .animations
                .iter()
                .skip(block.first_index)
                .take(block.num_anims)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut manager = CCutsceneMgr::initialise();
    manager.load_cutscene_data_with_hierarchies(cutscene_name, &cutscene_hierarchies);

    let ifp_summary = loaded.animation_file.as_ref().map(|file| {
        let animation_count = file
            .blocks
            .iter()
            .map(|block| block.animations.len())
            .sum::<usize>();
        let sequence_count = file
            .blocks
            .iter()
            .flat_map(|block| block.animations.iter())
            .map(|animation| animation.sequences.len())
            .sum::<usize>();
        let frame_count = file
            .blocks
            .iter()
            .flat_map(|block| block.animations.iter())
            .flat_map(|animation| animation.sequences.iter())
            .map(|sequence| sequence.frames.len())
            .sum::<usize>();
        let translated_sequence_count = file
            .blocks
            .iter()
            .flat_map(|block| block.animations.iter())
            .flat_map(|animation| animation.sequences.iter())
            .filter(|sequence| ifp_sequence_has_translation(sequence))
            .count();
        (
            file.blocks.len(),
            animation_count,
            sequence_count,
            frame_count,
            translated_sequence_count,
            file.blocks.first().map(|block| block.name.as_str()),
        )
    });

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_assets/src/assets/AnimationAssets.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Camera.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/animation/AnimManager.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/animation/CutsceneMgr.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"archive\": {{\"directory_entry_count\": {}, \"animation_found\": {}, \"animation_offset_sectors\": {}, \"animation_size_sectors\": {}, \"animation_byte_len\": {}, \"animation_checksum\": {}, \"camera_found\": true, \"camera_offset_sectors\": {}, \"camera_size_sectors\": {}, \"camera_byte_len\": {}, \"camera_checksum\": {}}},\n",
        loaded.directory_entry_count,
        loaded.animation_entry.is_some(),
        json_optional_u32(loaded.animation_entry.as_ref().map(|entry| entry.offset_sectors)),
        json_optional_u32(loaded.animation_entry.as_ref().map(|entry| entry.size_sectors)),
        json_optional_usize(
            loaded
                .animation_entry
                .as_ref()
                .map(|entry| entry.bytes.len())
        ),
        json_optional_u64(
            loaded
                .animation_entry
                .as_ref()
                .map(|entry| byte_checksum(&entry.bytes))
        ),
        camera_entry.offset_sectors,
        camera_entry.size_sectors,
        camera_entry.bytes.len(),
        byte_checksum(&camera_entry.bytes),
    ));
    if let Some((
        block_count,
        animation_count,
        sequence_count,
        frame_count,
        translated_sequence_count,
        first_block_name,
    )) = ifp_summary
    {
        json.push_str(&format!(
            "  \"ifp\": {{\"block_count\": {}, \"animation_count\": {}, \"sequence_count\": {}, \"frame_count\": {}, \"translated_sequence_count\": {}, \"first_block_name\": {}}},\n",
            block_count,
            animation_count,
            sequence_count,
            frame_count,
            translated_sequence_count,
            json_optional_string(first_block_name),
        ));
    } else {
        json.push_str("  \"ifp\": null,\n");
    }
    json.push_str("  \"splines\": [\n");
    for index in 0..MAX_NUM_OF_SPLINETYPES {
        json.push_str(&format!(
            "    {{\"index\": {}, \"stored_values\": {}, \"frame_count_bits\": {}, \"float_bits_checksum\": {}}}",
            index,
            stored_values[index],
            camera.path_splines[index][0].to_bits(),
            camera_spline_bits_checksum(
                &camera.path_splines[index][..MAX_CAMERA_SPLINE_PATH_LENGTH]
            ),
        ));
        if index + 1 < MAX_NUM_OF_SPLINETYPES {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    let sample_times = [0.0f32, 8_100.0, 16_200.0, 32_400.0, 64_800.0];
    json.push_str("  \"samples\": [\n");
    for (index, time_ms) in sample_times.iter().enumerate() {
        let sample = camera.sample_cutscene_spline(*time_ms)?;
        json.push_str(&format!(
            "    {{\"time_ms\": {}, \"source_bits\": [{}, {}, {}], \"target_bits\": [{}, {}, {}], \"up_bits\": [{}, {}, {}], \"fov_bits\": {}, \"position_along_bits\": {}, \"finish_time_ms\": {}}}",
            *time_ms as u32,
            sample.source.x.to_bits(),
            sample.source.y.to_bits(),
            sample.source.z.to_bits(),
            sample.target.x.to_bits(),
            sample.target.y.to_bits(),
            sample.target.z.to_bits(),
            sample.up.x.to_bits(),
            sample.up.y.to_bits(),
            sample.up.z.to_bits(),
            sample.fov.to_bits(),
            sample.position_along_spline.to_bits(),
            sample.finish_time_ms,
        ));
        if index + 1 < sample_times.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"manager\": {{\"loaded\": {}, \"running\": {}, \"anim_loaded\": {}, \"processing\": {}, \"name\": \"{}\", \"association_count\": {}, \"audio_track_id\": {}}}\n",
        manager.loaded,
        manager.running,
        manager.anim_loaded,
        manager.cutscene_processing,
        manager.cutscene_name,
        manager.cutscene_associations.num_associations(),
        CCutsceneMgr::find_cutscene_audio_track_id(cutscene_name),
    ));
    json.push_str("}\n");
    Ok(json)
}

fn cutscene_status_name(status: CutsceneLoadStatus) -> &'static str {
    match status {
        CutsceneLoadStatus::Loading0 => "Loading0",
        CutsceneLoadStatus::LoadingAudio => "LoadingAudio",
        CutsceneLoadStatus::Loading2 => "Loading2",
        CutsceneLoadStatus::Loading3 => "Loading3",
        CutsceneLoadStatus::Loading4 => "Loading4",
    }
}

fn cutscene_audio_track_name_checksum() -> u64 {
    CUTSCENE_AUDIO_TRACKS
        .iter()
        .enumerate()
        .fold(0u64, |sum, (index, track)| {
            sum.wrapping_add(
                string_checksum(track.name)
                    .wrapping_mul((index as u64 + 1).wrapping_mul(track.track_id as u64 + 1)),
            )
        })
}

fn cutscene_audio_track_id_checksum() -> u64 {
    CUTSCENE_AUDIO_TRACKS
        .iter()
        .enumerate()
        .fold(0u64, |sum, (index, track)| {
            sum.wrapping_add((index as u64 + 1).wrapping_mul(track.track_id as u64 + 1))
        })
}

fn run_cutscene_audio_tracks(config: &CaseConfig) -> String {
    let lookups = [
        "JB", "bet", "A1_SS0", "a1_ss0", "A1_SSO", "A7_ETG", "K4_SHI2", "MT_PH4", "missing",
    ];
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/animation/CutsceneMgr.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"summary\": {{\"track_count\": {}, \"first_name\": \"{}\", \"first_track_id\": {}, \"last_name\": \"{}\", \"last_track_id\": {}, \"name_checksum\": {}, \"id_checksum\": {}}},\n",
        CUTSCENE_AUDIO_TRACKS.len(),
        CUTSCENE_AUDIO_TRACKS.first().map(|track| track.name).unwrap_or(""),
        CUTSCENE_AUDIO_TRACKS.first().map(|track| track.track_id).unwrap_or(-1),
        CUTSCENE_AUDIO_TRACKS.last().map(|track| track.name).unwrap_or(""),
        CUTSCENE_AUDIO_TRACKS.last().map(|track| track.track_id).unwrap_or(-1),
        cutscene_audio_track_name_checksum(),
        cutscene_audio_track_id_checksum(),
    ));
    json.push_str("  \"lookups\": [\n");
    for (index, name) in lookups.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"name\": \"{}\", \"track_id\": {}}}",
            json_escape(name),
            CCutsceneMgr::find_cutscene_audio_track_id(name)
        ));
        if index + 1 < lookups.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"tracks\": [\n");
    for (index, track) in CUTSCENE_AUDIO_TRACKS.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"name\": \"{}\", \"track_id\": {}}}",
            index,
            json_escape(track.name),
            track.track_id
        ));
        if index + 1 < CUTSCENE_AUDIO_TRACKS.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn write_cutscene_music_command_json(json: &mut String, command: MusicCommand) {
    match command {
        MusicCommand::StopStreamedFile => {
            json.push_str("{\"kind\": \"StopStreamedFile\", \"track\": null, \"volume\": null, \"pan\": null, \"front_end\": null}");
        }
        MusicCommand::PreloadStreamedFile { track } => {
            json.push_str(&format!("{{\"kind\": \"PreloadStreamedFile\", \"track\": {track}, \"volume\": null, \"pan\": null, \"front_end\": null}}"));
        }
        MusicCommand::StartPreloadedStreamedFile => {
            json.push_str("{\"kind\": \"StartPreloadedStreamedFile\", \"track\": null, \"volume\": null, \"pan\": null, \"front_end\": null}");
        }
        MusicCommand::StartStreamedFile { track, .. } => {
            json.push_str(&format!("{{\"kind\": \"StartStreamedFile\", \"track\": {track}, \"volume\": null, \"pan\": null, \"front_end\": null}}"));
        }
        MusicCommand::SetStreamVolumeAndPan {
            volume,
            pan,
            front_end,
        } => {
            json.push_str(&format!("{{\"kind\": \"SetStreamVolumeAndPan\", \"track\": null, \"volume\": {volume}, \"pan\": {pan}, \"front_end\": {front_end}}}"));
        }
    }
}

struct CutsceneMusicSnapshot {
    label: &'static str,
    music_mode: u8,
    next_track: u8,
    playing_track: u8,
    saved_track_position: u32,
    commands: Vec<MusicCommand>,
}

fn cutscene_music_snapshot(
    label: &'static str,
    manager: &cMusicManager,
    commands: Vec<MusicCommand>,
) -> CutsceneMusicSnapshot {
    CutsceneMusicSnapshot {
        label,
        music_mode: manager.m_nMusicMode,
        next_track: manager.m_nNextTrack,
        playing_track: manager.m_nPlayingTrack,
        saved_track_position: manager.m_aTracks[10].m_nPosition,
        commands,
    }
}

fn apply_cutscene_music_actions(
    manager: &mut cMusicManager,
    actions: Vec<CutsceneAudioAction>,
    stream_playing: &mut bool,
    context: &MusicServiceContext,
) {
    for action in actions {
        match action {
            CutsceneAudioAction::EnterMusicModeAndPreload { track_id } => {
                manager.ChangeMusicMode(
                    MUSICMODE_CUTSCENE as u8,
                    &MusicServiceContext {
                        stream_playing: *stream_playing,
                        ..*context
                    },
                );
                if *stream_playing {
                    *stream_playing = false;
                }
                if let Some(track_id) = track_id {
                    manager.PreloadCutSceneMusic(track_id, *stream_playing);
                }
            }
            CutsceneAudioAction::PlayPreloaded => {
                manager.PlayPreloadedCutSceneMusic();
                *stream_playing = true;
            }
            CutsceneAudioAction::Stop { restore_game_mode } => {
                manager.StopCutSceneMusic();
                *stream_playing = false;
                if restore_game_mode {
                    manager.ChangeMusicMode(
                        MUSICMODE_GAME as u8,
                        &MusicServiceContext {
                            stream_playing: *stream_playing,
                            ..*context
                        },
                    );
                }
            }
        }
    }
}

fn write_cutscene_music_snapshot_json(
    json: &mut String,
    snapshot: CutsceneMusicSnapshot,
    trailing_comma: bool,
) {
    json.push_str(&format!(
        "    {{\"label\": \"{}\", \"music_mode\": {}, \"next_track\": {}, \"playing_track\": {}, \"saved_track_position\": {}, \"commands\": [",
        json_escape(snapshot.label),
        snapshot.music_mode,
        snapshot.next_track,
        snapshot.playing_track,
        snapshot.saved_track_position,
    ));
    let command_count = snapshot.commands.len();
    for (index, command) in snapshot.commands.into_iter().enumerate() {
        write_cutscene_music_command_json(json, command);
        if index + 1 < command_count {
            json.push(',');
        }
    }
    json.push_str("]}");
    if trailing_comma {
        json.push(',');
    }
    json.push('\n');
}

fn run_cutscene_music_lifecycle(config: &CaseConfig) -> String {
    let mut manager = cMusicManager::new();
    let lengths = vec![10_000; TOTAL_STREAMED_SOUNDS as usize];
    manager.Initialise(&lengths, 100, 0);
    manager.m_nMusicMode = MUSICMODE_GAME as u8;
    manager.m_nNextTrack = 10;
    manager.m_nPlayingTrack = 10;
    let context = MusicServiceContext {
        time_ms_pause_mode: 777,
        streamed_file_position: 321,
        ..Default::default()
    };
    let mut stream_playing = true;
    let mut cutscene = CCutsceneMgr::initialise();
    cutscene.load_cutscene_data("BET", false);
    apply_cutscene_music_actions(
        &mut manager,
        cutscene.drain_audio_actions(),
        &mut stream_playing,
        &context,
    );
    let commands = std::mem::take(&mut manager.emitted_commands);
    let load_bet_snapshot = cutscene_music_snapshot("load_bet", &manager, commands);
    cutscene.cutscene_load_status = CutsceneLoadStatus::LoadingAudio;
    cutscene.update(0.0, false);
    apply_cutscene_music_actions(
        &mut manager,
        cutscene.drain_audio_actions(),
        &mut stream_playing,
        &context,
    );
    let commands = std::mem::take(&mut manager.emitted_commands);
    let start_bet_snapshot = cutscene_music_snapshot("start_bet", &manager, commands);
    cutscene.delete_cutscene_data();
    apply_cutscene_music_actions(
        &mut manager,
        cutscene.drain_audio_actions(),
        &mut stream_playing,
        &context,
    );
    let commands = std::mem::take(&mut manager.emitted_commands);
    let stop_bet_snapshot = cutscene_music_snapshot("clear_bet", &manager, commands);
    cutscene.load_cutscene_data("JB", false);
    apply_cutscene_music_actions(
        &mut manager,
        cutscene.drain_audio_actions(),
        &mut stream_playing,
        &context,
    );
    let commands = std::mem::take(&mut manager.emitted_commands);
    let load_jb_snapshot = cutscene_music_snapshot("load_jb", &manager, commands);
    cutscene.cutscene_load_status = CutsceneLoadStatus::LoadingAudio;
    cutscene.update(0.0, false);
    apply_cutscene_music_actions(
        &mut manager,
        cutscene.drain_audio_actions(),
        &mut stream_playing,
        &context,
    );
    let commands = std::mem::take(&mut manager.emitted_commands);
    let start_jb_snapshot = cutscene_music_snapshot("start_jb", &manager, commands);
    cutscene.delete_cutscene_data();
    apply_cutscene_music_actions(
        &mut manager,
        cutscene.drain_audio_actions(),
        &mut stream_playing,
        &context,
    );
    let commands = std::mem::take(&mut manager.emitted_commands);
    let clear_jb_snapshot = cutscene_music_snapshot("clear_jb_restore_game", &manager, commands);

    let snapshots = [
        load_bet_snapshot,
        start_bet_snapshot,
        stop_bet_snapshot,
        load_jb_snapshot,
        start_jb_snapshot,
        clear_jb_snapshot,
    ];
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/audio/MusicManager.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/animation/CutsceneMgr.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"summary\": {{\"bet_track\": 16, \"jb_track\": 15, \"no_track\": {}, \"snapshot_count\": {}}},\n",
        NO_TRACK,
        snapshots.len()
    ));
    json.push_str("  \"snapshots\": [\n");
    let snapshot_count = snapshots.len();
    for (index, snapshot) in snapshots.into_iter().enumerate() {
        write_cutscene_music_snapshot_json(&mut json, snapshot, index + 1 < snapshot_count);
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn push_cutscene_snapshot(json: &mut String, label: &str, cutscene: &CCutsceneMgr, comma: bool) {
    json.push_str(&format!(
        "    {{\"label\": \"{}\", \"loaded\": {}, \"running\": {}, \"anim_loaded\": {}, \"processing\": {}, \"use_lod_multiplier\": {}, \"name\": \"{}\", \"load_status\": \"{}\", \"timer_ms\": {}, \"offset\": [{:.1}, {:.1}, {:.1}], \"object_count\": {}}}{}\n",
        label,
        cutscene.loaded,
        cutscene.running,
        cutscene.anim_loaded,
        cutscene.cutscene_processing,
        cutscene.use_lod_multiplier,
        cutscene.cutscene_name,
        cutscene_status_name(cutscene.cutscene_load_status),
        cutscene.get_cutscene_time_in_milliseconds(),
        cutscene.cutscene_offset.x,
        cutscene.cutscene_offset.y,
        cutscene.cutscene_offset.z,
        cutscene.num_cutscene_objs,
        if comma { "," } else { "" },
    ));
}

fn run_intro_cutscene_state(config: &CaseConfig) -> String {
    let mut cutscene = CCutsceneMgr::initialise();
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/animation/CutsceneMgr.rs\"\n");
    json.push_str("  ],\n");
    json.push_str("  \"snapshots\": [\n");

    push_cutscene_snapshot(&mut json, "initialise", &cutscene, true);
    cutscene.load_cutscene_data("INTRO", true);
    push_cutscene_snapshot(&mut json, "load_intro", &cutscene, true);
    cutscene.set_cutscene_offset(GameCVector::new(10.0, 20.0, 30.0));
    cutscene.create_cutscene_object(100);
    cutscene.create_cutscene_object(101);
    cutscene.cutscene_load_status = CutsceneLoadStatus::LoadingAudio;
    cutscene.update(0.02, false);
    push_cutscene_snapshot(&mut json, "loading_audio", &cutscene, true);
    cutscene.update(0.02, false);
    push_cutscene_snapshot(&mut json, "loading_2", &cutscene, true);
    cutscene.update(0.02, false);
    push_cutscene_snapshot(&mut json, "loading_3", &cutscene, true);
    cutscene.update(0.02, false);
    push_cutscene_snapshot(&mut json, "loading_4", &cutscene, true);
    cutscene.finish_cutscene(12_500.0);
    push_cutscene_snapshot(&mut json, "finish", &cutscene, true);
    cutscene.delete_cutscene_data();
    push_cutscene_snapshot(&mut json, "delete", &cutscene, false);

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

#[derive(Debug, Clone)]
struct ScmMissionOpcodeEvent {
    kind: &'static str,
    opcode: i32,
    mission_offset: usize,
    script_space_offset: usize,
    key: Option<String>,
    raw_key: Option<String>,
    argument_type: Option<u8>,
    argument_value: Option<i32>,
}

fn scm_read_u16(bytes: &[u8], offset: usize) -> u16 {
    let lo = bytes.get(offset).copied().unwrap_or(0);
    let hi = bytes.get(offset + 1).copied().unwrap_or(0);
    u16::from_le_bytes([lo, hi])
}

fn scm_read_i16(bytes: &[u8], offset: usize) -> i16 {
    scm_read_u16(bytes, offset) as i16
}

fn scm_read_i32(bytes: &[u8], offset: usize) -> i32 {
    let b0 = bytes.get(offset).copied().unwrap_or(0);
    let b1 = bytes.get(offset + 1).copied().unwrap_or(0);
    let b2 = bytes.get(offset + 2).copied().unwrap_or(0);
    let b3 = bytes.get(offset + 3).copied().unwrap_or(0);
    i32::from_le_bytes([b0, b1, b2, b3])
}

fn scm_key_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

fn scm_find_opcode(bytes: &[u8], opcode: i32) -> Option<usize> {
    let opcode = opcode as u16;
    bytes
        .windows(2)
        .position(|window| u16::from_le_bytes([window[0], window[1]]) == opcode)
}

fn scm_read_int_argument(bytes: &[u8], offset: usize) -> (u8, i32) {
    let argument_type = bytes.get(offset).copied().unwrap_or(0);
    let value = match argument_type {
        1 => scm_read_i32(bytes, offset + 1),
        4 => i32::from(bytes.get(offset + 1).copied().unwrap_or(0) as i8),
        5 => i32::from(scm_read_i16(bytes, offset + 1)),
        _ => 0,
    };
    (argument_type, value)
}

fn scm_mission_opcode_events(mission_space: &[u8]) -> Vec<ScmMissionOpcodeEvent> {
    let mut events = Vec::new();

    if let Some(offset) = scm_find_opcode(mission_space, COMMAND_SCRIPT_NAME) {
        let key_bytes = &mission_space[offset + 2..offset + 10];
        let raw_key = scm_key_string(key_bytes);
        events.push(ScmMissionOpcodeEvent {
            kind: "SCRIPT_NAME",
            opcode: COMMAND_SCRIPT_NAME,
            mission_offset: offset,
            script_space_offset: SIZE_MAIN_SCRIPT + offset,
            key: Some(raw_key.to_ascii_lowercase()),
            raw_key: Some(raw_key),
            argument_type: None,
            argument_value: None,
        });
    }

    if let Some(offset) = scm_find_opcode(mission_space, COMMAND_SET_INTRO_IS_PLAYING) {
        let (argument_type, argument_value) = scm_read_int_argument(mission_space, offset + 2);
        events.push(ScmMissionOpcodeEvent {
            kind: "SET_INTRO_IS_PLAYING",
            opcode: COMMAND_SET_INTRO_IS_PLAYING,
            mission_offset: offset,
            script_space_offset: SIZE_MAIN_SCRIPT + offset,
            key: None,
            raw_key: None,
            argument_type: Some(argument_type),
            argument_value: Some(argument_value),
        });
    }

    if let Some(offset) = scm_find_opcode(mission_space, COMMAND_LOAD_CUTSCENE) {
        let key_bytes = &mission_space[offset + 2..offset + 10];
        let raw_key = scm_key_string(key_bytes);
        events.push(ScmMissionOpcodeEvent {
            kind: "LOAD_CUTSCENE",
            opcode: COMMAND_LOAD_CUTSCENE,
            mission_offset: offset,
            script_space_offset: SIZE_MAIN_SCRIPT + offset,
            key: Some(raw_key.clone()),
            raw_key: Some(raw_key),
            argument_type: None,
            argument_value: None,
        });
    }

    if let Some(offset) = scm_find_opcode(mission_space, COMMAND_START_CUTSCENE) {
        events.push(ScmMissionOpcodeEvent {
            kind: "START_CUTSCENE",
            opcode: COMMAND_START_CUTSCENE,
            mission_offset: offset,
            script_space_offset: SIZE_MAIN_SCRIPT + offset,
            key: None,
            raw_key: None,
            argument_type: None,
            argument_value: None,
        });
    }

    events.sort_by_key(|event| event.mission_offset);
    events
}

fn push_scm_mission_events_json(json: &mut String, events: &[ScmMissionOpcodeEvent]) {
    json.push_str("  \"opcode_events\": [\n");
    for (index, event) in events.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"kind\": \"{}\", \"opcode\": {}, \"mission_offset\": {}, \"script_space_offset\": {}",
            event.kind, event.opcode, event.mission_offset, event.script_space_offset
        ));
        if let Some(raw_key) = &event.raw_key {
            json.push_str(&format!(", \"raw_key\": \"{}\"", json_escape(raw_key)));
        }
        if let Some(key) = &event.key {
            json.push_str(&format!(", \"key\": \"{}\"", json_escape(key)));
        }
        if let Some(argument_type) = event.argument_type {
            json.push_str(&format!(", \"argument_type\": {}", argument_type));
        }
        if let Some(argument_value) = event.argument_value {
            json.push_str(&format!(", \"argument_value\": {}", argument_value));
        }
        json.push('}');
        if index + 1 < events.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
}

fn run_scm_intro_mission_launch(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|error| format!("failed to read {}: {error}", config.asset_root_config))?;
    let mut file_mgr = CFileMgr::initialise(PathBuf::from(root_text.trim()));
    let mut scripts =
        CTheScripts::init(&mut file_mgr).map_err(|error| format!("script init failed: {error}"))?;
    let scm = ScmFile::parse_main_scm(&scripts.main_scm_bytes);
    let mission_index = 0usize;
    let source_offset = scripts.multi_script_array[mission_index] as usize;
    let script_index = scripts
        .load_and_launch_mission_internal(mission_index as i32)
        .map_err(|error| format!("mission launch failed: {error:?}"))?;
    let mission_space =
        &scripts.script_space[SIZE_MAIN_SCRIPT..SIZE_MAIN_SCRIPT + SIZE_MISSION_SCRIPT];
    let events = scm_mission_opcode_events(mission_space);
    let launched = &scripts.scripts[script_index];

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/Script.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/control/Script.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"main_scm_size\": {}, \"loaded_main_bytes\": {}, \"main_checksum\": {}, \"mission_checksum\": {}}},\n",
        scripts.main_scm_bytes.len(),
        scripts.loaded_main_bytes,
        byte_checksum(&scripts.script_space[..scripts.loaded_main_bytes]),
        byte_checksum(mission_space)
    ));
    json.push_str(&format!(
        "  \"header\": {{\"main_script_size\": {}, \"largest_mission_script_size\": {}, \"number_of_mission_scripts\": {}}},\n",
        scm.multiscript.main_script_size,
        scm.multiscript.largest_mission_script_size,
        scm.multiscript.number_of_mission_scripts
    ));
    json.push_str(&format!(
        "  \"mission\": {{\"index\": {}, \"source_offset\": {}, \"script_space_ip\": {}, \"copy_len\": {}, \"first_opcode\": {}, \"first_24_bytes_hex\": \"{}\"}},\n",
        mission_index,
        source_offset,
        SIZE_MAIN_SCRIPT,
        SIZE_MISSION_SCRIPT,
        scm_read_u16(mission_space, 0) & 0x7FFF,
        bytes_hex(&mission_space[..24])
    ));
    push_scm_mission_events_json(&mut json, &events);
    json.push_str(&format!(
        "  \"launch_state\": {{\"script_index\": {}, \"active_script_count\": {}, \"active_contains_script\": {}, \"active_ip\": {}, \"script_name\": \"{}\", \"is_mission_script\": {}, \"mission_flag\": {}, \"already_running_a_mission_script\": {}}}\n",
        script_index,
        scripts.active_script_count,
        scripts.active_scripts.contains(&script_index),
        launched.ip,
        json_escape(&scm_key_string(&launched.script_name)),
        launched.is_mission_script,
        launched.mission_flag,
        scripts.already_running_a_mission_script
    ));
    json.push_str("}\n");
    Ok(json)
}

fn read_hex_image(path: &Path) -> Result<Vec<u8>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    for token in text.split_whitespace() {
        let byte = u8::from_str_radix(token, 16).map_err(|error| {
            format!("invalid hex byte `{token}` in {}: {error}", path.display())
        })?;
        bytes.push(byte);
    }
    if bytes.is_empty() {
        return Err(format!("empty image in {}", path.display()));
    }
    Ok(bytes)
}

/// re3 VM core parity over a synthetic SCM image (see
/// parity/cases/step/scm_vm_core/build_image.py): instruction fetch/arity,
/// locals/globals, compare flag + NOT + AND/OR chains, GOTO/GOTO_IF_FALSE,
/// GOSUB/RETURN stack, WAIT wake gating, per-frame timers, START_NEW_SCRIPT
/// deferral, LAUNCH_MISSION and TERMINATE_THIS_SCRIPT list moves.
///
/// References:
///   references/re-GTA/src/control/Script.cpp (CTheScripts::Process,
///     CRunningScript::Process/ProcessOneCommand/CollectParameters/StoreParameters)
///   references/re-GTA/src/control/Script5.cpp (UpdateCompareFlag)
///   references/re-GTA/src/control/Script.h (CRunningScript fields, arg types)
fn run_scm_vm_core(config: &CaseConfig) -> Result<String, String> {
    let image = read_hex_image(&config.case_dir.join(&config.image_file))?;

    // Boot the production VM from the synthetic image through the same
    // CTheScripts::init path real main.scm uses (CFileMgr + data\main.scm).
    let temp_root = env::temp_dir().join("re3bevy_scm_vm_core");
    let data_dir = temp_root.join("data");
    fs::create_dir_all(&data_dir)
        .map_err(|error| format!("failed to create {}: {error}", data_dir.display()))?;
    let main_path = data_dir.join("main.scm");
    fs::write(&main_path, &image)
        .map_err(|error| format!("failed to write {}: {error}", main_path.display()))?;

    let mut file_mgr = CFileMgr::initialise(temp_root);
    let mut scripts =
        CTheScripts::init(&mut file_mgr).map_err(|error| format!("script init failed: {error}"))?;
    let mut general = CGeneral::default();
    let mut clock = CClock::initialise(1000, 0);
    scripts
        .start_test_script()
        .map_err(|error| format!("StartTestScript failed: {error:?}"))?;

    let mut trace_json = String::new();
    let mut frames_json = String::new();
    let mut order = 0usize;
    let time_step_ms = config.time_step_ms;

    for (frame_index, &time_ms) in config.timer_ms.iter().enumerate() {
        let outcomes = scripts
            .process_active_scripts_with_context(
                time_ms,
                time_step_ms as f32,
                1.0,
                &mut general,
                &mut clock,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                false,
            )
            .map_err(|error| format!("frame {frame_index} process failed: {error:?}"))?;

        for outcome in &outcomes {
            if order > 0 {
                trace_json.push_str(",\n");
            }
            trace_json.push_str(&format!(
                "    {{\"frame\": {frame_index}, \"order\": {order}, \"slot\": {}, \"ip_before\": {}, \"opcode\": {}, \"not_flag\": {}, \"ip_after\": {}, \"yield\": {}}}",
                outcome.script_index,
                outcome.ip_before,
                outcome.command,
                outcome.not_flag,
                outcome.ip_after,
                outcome.yielded
            ));
            order += 1;
        }

        if frame_index > 0 {
            frames_json.push_str(",\n");
        }
        frames_json.push_str(&format!(
            "    {{\"index\": {frame_index}, \"time_ms\": {time_ms}, \"commands_executed\": {}, \"scripts_updated\": {}, \"active_order\": [{}], \"idle_count\": {},\n",
            scripts.commands_executed,
            scripts.scripts_updated,
            scripts
                .active_scripts
                .iter()
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            scripts.idle_scripts.len()
        ));
        frames_json.push_str("     \"scripts\": [");
        for (script_pos, script_index) in scripts.active_scripts.iter().copied().enumerate() {
            let script = &scripts.scripts[script_index];
            if script_pos > 0 {
                frames_json.push_str(", ");
            }
            let stack: Vec<String> = script.stack[..script.stack_pointer as usize]
                .iter()
                .map(|value| value.to_string())
                .collect();
            let locals: Vec<String> = script.local_variables[..8]
                .iter()
                .map(|value| value.to_string())
                .collect();
            frames_json.push_str(&format!(
                "{{\"slot\": {script_index}, \"name\": \"{}\", \"ip\": {}, \"wake\": {}, \"stack_pointer\": {}, \"stack\": [{}], \"cond\": {}, \"and_or\": {}, \"not_flag\": {}, \"locals_0_to_7\": [{}], \"timer_local_16\": {}, \"timer_local_17\": {}, \"is_mission_script\": {}, \"mission_flag\": {}, \"skip_wake_time\": {}}}",
                json_escape(&scm_key_string(&script.script_name)),
                script.ip,
                script.wake_time,
                script.stack_pointer,
                stack.join(", "),
                script.cond_result,
                script.and_or_state,
                script.not_flag,
                locals.join(", "),
                script.local_variables[16],
                script.local_variables[17],
                script.is_mission_script,
                script.mission_flag,
                script.skip_wake_time
            ));
        }
        frames_json.push_str("],\n     \"globals\": [");
        for (global_pos, offset) in [8usize, 12, 16, 20].iter().copied().enumerate() {
            if global_pos > 0 {
                frames_json.push_str(", ");
            }
            let value = i32::from_le_bytes([
                scripts.script_space[offset],
                scripts.script_space[offset + 1],
                scripts.script_space[offset + 2],
                scripts.script_space[offset + 3],
            ]);
            frames_json.push_str(&format!("{{\"offset\": {offset}, \"value\": {value}}}"));
        }
        frames_json.push_str("]}");
    }

    let frame_ms: Vec<String> = config
        .timer_ms
        .iter()
        .map(|value| value.to_string())
        .collect();
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/control/Script.rs\",\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/Script.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"image_checksum\": {}, \"image_len\": {}, \"time_step_ms\": {time_step_ms}, \"frame_ms\": [{}]}},\n",
        byte_checksum(&image),
        image.len(),
        frame_ms.join(", ")
    ));
    json.push_str("  \"command_trace\": [\n");
    json.push_str(&trace_json);
    json.push_str("\n  ],\n");
    json.push_str("  \"frames\": [\n");
    json.push_str(&frames_json);
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn script_next_opcode(scripts: &CTheScripts, script_index: usize) -> u16 {
    let ip = scripts.scripts[script_index].ip as usize;
    scripts
        .script_space
        .get(ip..ip + 2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) & 0x7fff)
        .unwrap_or(0)
}

fn find_active_script_by_name(scripts: &CTheScripts, name: &str) -> Option<usize> {
    scripts.active_scripts.iter().copied().find(|script_index| {
        scm_key_string(&scripts.scripts[*script_index].script_name).eq_ignore_ascii_case(name)
    })
}

fn find_active_mission_script(scripts: &CTheScripts) -> Option<usize> {
    scripts
        .active_scripts
        .iter()
        .copied()
        .find(|script_index| scripts.scripts[*script_index].is_mission_script)
}

fn effect_count<F>(effects: &[ScriptCommandEffect], mut predicate: F) -> usize
where
    F: FnMut(&ScriptCommandEffect) -> bool,
{
    effects.iter().filter(|effect| predicate(effect)).count()
}

fn run_scm_intro_cutscene_runtime(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|error| format!("failed to read {}: {error}", config.asset_root_config))?;
    let mut file_mgr = CFileMgr::initialise(PathBuf::from(root_text.trim()));
    let mut scripts =
        CTheScripts::init(&mut file_mgr).map_err(|error| format!("script init failed: {error}"))?;
    let scm = ScmFile::parse_main_scm(&scripts.main_scm_bytes);
    let mission_source_offset = scripts.multi_script_array[0] as usize;
    let mission_space = scripts.main_scm_bytes
        [mission_source_offset..mission_source_offset + SIZE_MISSION_SCRIPT]
        .to_vec();

    scripts
        .start_test_script()
        .map_err(|error| format!("StartTestScript failed: {error:?}"))?;
    let mut general = CGeneral::default();
    let mut clock = CClock::initialise(1000, 0);
    scripts
        .process_active_scripts_with_context(
            0,
            0.0,
            1.0,
            &mut general,
            &mut clock,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        )
        .map_err(|error| format!("load Process failed: {error:?}"))?;

    let load_commands = scripts.commands_executed;
    let load_create_player_effects = effect_count(&scripts.pending_effects, |effect| {
        matches!(effect, ScriptCommandEffect::CreatePlayer { .. })
    });
    let load_cutscene_effects = effect_count(&scripts.pending_effects, |effect| {
        matches!(effect, ScriptCommandEffect::LoadCutscene { .. })
    });
    let load_start_cutscene_effects = effect_count(&scripts.pending_effects, |effect| {
        matches!(effect, ScriptCommandEffect::StartCutscene)
    });

    let mut bridge = ScriptGameplayBridge::default();
    let mut streaming = CStreaming::default();
    let mut camera = CCamera::default();
    camera.Init();
    let mut weather = CWeather::init();
    let mut gangs = CGangs::initialise();
    let mut stats = CStats::init();
    let mut cutscene = CCutsceneMgr::initialise();
    apply_script_effects_to_world(
        &mut scripts,
        &mut bridge,
        None,
        Some(&mut stats),
        Some(&mut cutscene),
        Some(&mut streaming),
        Some(&mut camera),
        Some(&mut weather),
        Some(&mut gangs),
        None,
        None,
        None,
        None,
        0,
    );

    let player_x_milli = (bridge.player_position.x * 1000.0).round() as i32;
    let player_y_milli = (bridge.player_position.y * 1000.0).round() as i32;
    let player_z_milli = (bridge.player_position.z * 1000.0).round() as i32;
    let player_heading_milli = (bridge.player_heading_degrees * 1000.0).round() as i32;

    let mut time_ms = 0u32;
    let mut extra_frames = 0u32;
    let mut cutscene_effect_frame = 0u32;
    let mut cutscene_effect_time_ms = 0u32;
    let mut frame_load_cutscene_effects = 0usize;
    let mut frame_start_cutscene_effects = 0usize;
    let mut frame_set_intro_true_effects = 0usize;
    let mut mission_launch_frame = 0u32;
    let mut mission_script_index = None;
    const STEP_MS: u32 = 16;
    const MAX_EXTRA_FRAMES: u32 = 6_000;

    while !bridge.cutscene_started && extra_frames < MAX_EXTRA_FRAMES {
        time_ms = time_ms.saturating_add(STEP_MS);
        extra_frames += 1;
        let player_snapshot = ScriptPlayerSnapshot {
            ped_handle: bridge.player_index.saturating_add(1),
            position: bridge.player_position,
            current_vehicle: None,
            current_vehicle_model: None,
            stopped: true,
            is_shooting: false,
            horn_pressed: false,
            is_in_control: true,
            is_driving: false,
            is_making_call: false,
            health: 100.0,
            wanted_level: 0,
            money: 0,
            current_weapon: 0,
            weapon_ammo: [0; 13],
        };
        let entity_snapshot = ScriptEntitySnapshot {
            position: bridge.player_position,
            bound_center: bridge.player_position,
            bound_radius: 0.8,
            heading_degrees: bridge.player_heading_degrees,
            up_z: 1.0,
            model_index: 0,
            entity_status: 0,
            alive: true,
            current_vehicle: None,
            current_vehicle_model: None,
            vehicle_colour1: None,
            vehicle_colour2: None,
            vehicle_is_boat: false,
            vehicle_in_water: false,
            stopped: true,
            is_shooting: false,
            collision_records: [None; 6],
            speed: 0.0,
            health: 100.0,
            current_weapon: 0,
            weapon_ammo: [0; 13],
            script_objective_completed: false,
            leader_slot: None,
            ped_type: Some(0),
            vehicle_bomb_type: None,
            vehicle_damaged: false,
            last_weapon_damage: 0,
            vehicle_num_passengers: 0,
            vehicle_num_max_passengers: 0,
            vehicle_passenger_handles: [None; 8],
            vehicle_closed_doors: 0,
            can_see_player: true,
            automobile_drive_wheels_on_ground: None,
        };
        let player_spawned = bridge.player_spawned;
        let player_index = bridge.player_index;
        let player_handles = scripts.player_char_handles;
        let mut player_probe = |index: i32| {
            (player_spawned && (index == 0 || index == player_index)).then_some(player_snapshot)
        };
        let mut char_probe = |handle: i32| {
            (player_spawned
                && player_handles
                    .iter()
                    .any(|player_handle| *player_handle == handle))
            .then_some(entity_snapshot)
        };
        let mut ground_probe = |_x: f32, _y: f32| bridge.player_position.z;
        let mut camera_probe = |_position, _radius| true;
        let mut pad_probe = |_pad: i32, _button| 0i16;

        scripts
            .process_active_scripts_with_context(
                time_ms,
                STEP_MS as f32,
                1.0,
                &mut general,
                &mut clock,
                Some(&mut camera_probe),
                Some(&mut ground_probe),
                Some(&mut player_probe),
                Some(&mut char_probe),
                None,
                None,
                Some(&mut pad_probe),
                false,
            )
            .map_err(|error| format!("runtime Process frame {extra_frames} failed: {error:?}"))?;

        if mission_launch_frame == 0 {
            if let Some(index) = find_active_mission_script(&scripts) {
                mission_launch_frame = extra_frames;
                mission_script_index = Some(index);
                // This case's C++ harness replaces the platform streamer with
                // an immediate-success stub. Prime the equivalent observable
                // slots before the mission's first Process call.
                scripts.special_char_loaded.fill(true);
                scripts.model_loaded.fill(true);
            }
        }

        let load_cutscene_effects = effect_count(&scripts.pending_effects, |effect| {
            matches!(effect, ScriptCommandEffect::LoadCutscene { .. })
        });
        let start_cutscene_effects = effect_count(&scripts.pending_effects, |effect| {
            matches!(effect, ScriptCommandEffect::StartCutscene)
        });
        let set_intro_true_effects = effect_count(&scripts.pending_effects, |effect| {
            matches!(
                effect,
                ScriptCommandEffect::SetIntroIsPlaying { playing: true }
            )
        });
        if load_cutscene_effects > 0 || start_cutscene_effects > 0 {
            cutscene_effect_frame = extra_frames;
            cutscene_effect_time_ms = time_ms;
            frame_load_cutscene_effects = load_cutscene_effects;
            frame_start_cutscene_effects = start_cutscene_effects;
            frame_set_intro_true_effects = set_intro_true_effects;
        }

        apply_script_effects_to_world(
            &mut scripts,
            &mut bridge,
            None,
            Some(&mut stats),
            Some(&mut cutscene),
            Some(&mut streaming),
            Some(&mut camera),
            Some(&mut weather),
            Some(&mut gangs),
            None,
            None,
            None,
            None,
            0,
        );
    }

    let mission_script_index = mission_script_index
        .or_else(|| find_active_script_by_name(&scripts, "intro"))
        .ok_or("intro mission script not active after runtime frames")?;
    let mission_script = &scripts.scripts[mission_script_index];

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/control/Script.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Game.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"main_scm_size\": {}, \"loaded_main_bytes\": {}, \"mission_checksum\": {}}},\n",
        scripts.main_scm_bytes.len(),
        scripts.loaded_main_bytes,
        byte_checksum(&mission_space)
    ));
    json.push_str(&format!(
        "  \"header\": {{\"main_script_size\": {}, \"largest_mission_script_size\": {}, \"number_of_mission_scripts\": {}}},\n",
        scm.multiscript.main_script_size,
        scm.multiscript.largest_mission_script_size,
        scm.multiscript.number_of_mission_scripts
    ));
    json.push_str(&format!(
        "  \"load_process\": {{\"commands\": {}, \"create_player_effects\": {}, \"load_cutscene_effects\": {}, \"start_cutscene_effects\": {}}},\n",
        load_commands,
        load_create_player_effects,
        load_cutscene_effects,
        load_start_cutscene_effects
    ));
    json.push_str(&format!(
        "  \"player_after_load\": {{\"spawned\": {}, \"index\": {}, \"position_milli\": [{}, {}, {}], \"heading_milli\": {}}},\n",
        bridge.player_spawned,
        bridge.player_index,
        player_x_milli,
        player_y_milli,
        player_z_milli,
        player_heading_milli
    ));
    json.push_str(&format!(
        "  \"cutscene_effect_frame\": {{\"frame\": {}, \"time_ms\": {}, \"load_cutscene_effects\": {}, \"start_cutscene_effects\": {}, \"set_intro_true_effects\": {}}},\n",
        cutscene_effect_frame,
        cutscene_effect_time_ms,
        frame_load_cutscene_effects,
        frame_start_cutscene_effects,
        frame_set_intro_true_effects
    ));
    json.push_str(&format!(
        "  \"runtime\": {{\"extra_frames_to_cutscene\": {}, \"time_ms\": {}, \"cutscene_name\": \"{}\", \"requested\": {}, \"started\": {}, \"playing_intro\": {}, \"player_visible\": {}, \"player_control_enabled\": {}, \"manager_loaded\": {}, \"manager_status\": \"{}\", \"camera_mode\": \"{:?}\"}},\n",
        extra_frames,
        time_ms,
        json_escape(&bridge.cutscene_name),
        bridge.cutscene_requested,
        bridge.cutscene_started,
        bridge.playing_intro,
        bridge.player_visible,
        bridge.player_control_enabled,
        cutscene.loaded,
        cutscene_status_name(cutscene.cutscene_load_status),
        camera.script_mode
    ));
    json.push_str(&format!(
        "  \"mission_script\": {{\"launch_frame\": {}, \"script_index\": {}, \"name\": \"{}\", \"ip\": {}, \"wake_time\": {}, \"next_opcode\": {}, \"is_mission_script\": {}, \"mission_flag\": {}}}\n",
        mission_launch_frame,
        mission_script_index,
        json_escape(&scm_key_string(&mission_script.script_name)),
        mission_script.ip,
        mission_script.wake_time,
        script_next_opcode(&scripts, mission_script_index),
        mission_script.is_mission_script,
        mission_script.mission_flag
    ));
    json.push_str("}\n");
    Ok(json)
}

#[derive(Debug, Clone)]
struct CutsceneScriptEvent {
    frame: u32,
    time_ms: u32,
    kind: &'static str,
    handle: Option<i32>,
    object_handle: Option<i32>,
    source_object_handle: Option<i32>,
    model_id: Option<i32>,
    name: Option<String>,
    anim_name: Option<String>,
}

fn cutscene_script_event_from_effect(
    effect: &ScriptCommandEffect,
    frame: u32,
    time_ms: u32,
) -> Option<CutsceneScriptEvent> {
    match effect {
        ScriptCommandEffect::LoadCutscene { name } => Some(CutsceneScriptEvent {
            frame,
            time_ms,
            kind: "LOAD_CUTSCENE",
            handle: None,
            object_handle: None,
            source_object_handle: None,
            model_id: None,
            name: Some(scm_key_string(name)),
            anim_name: None,
        }),
        ScriptCommandEffect::CreateCutsceneObject { handle, model_id } => {
            Some(CutsceneScriptEvent {
                frame,
                time_ms,
                kind: "CREATE_CUTSCENE_OBJECT",
                handle: Some(*handle),
                object_handle: None,
                source_object_handle: None,
                model_id: Some(*model_id),
                name: None,
                anim_name: None,
            })
        }
        ScriptCommandEffect::SetCutsceneAnim {
            object_handle,
            anim_name,
        } => Some(CutsceneScriptEvent {
            frame,
            time_ms,
            kind: "SET_CUTSCENE_ANIM",
            handle: None,
            object_handle: Some(*object_handle),
            source_object_handle: None,
            model_id: None,
            name: None,
            anim_name: Some(scm_key_string(anim_name)),
        }),
        ScriptCommandEffect::CreateCutsceneHead {
            handle,
            source_object_handle,
            model_id,
        } => Some(CutsceneScriptEvent {
            frame,
            time_ms,
            kind: "CREATE_CUTSCENE_HEAD",
            handle: Some(*handle),
            object_handle: None,
            source_object_handle: Some(*source_object_handle),
            model_id: Some(*model_id),
            name: None,
            anim_name: None,
        }),
        ScriptCommandEffect::SetCutsceneHeadAnim {
            object_handle,
            anim_name,
        } => Some(CutsceneScriptEvent {
            frame,
            time_ms,
            kind: "SET_CUTSCENE_HEAD_ANIM",
            handle: None,
            object_handle: Some(*object_handle),
            source_object_handle: None,
            model_id: None,
            name: None,
            anim_name: Some(scm_key_string(anim_name)),
        }),
        ScriptCommandEffect::StartCutscene => Some(CutsceneScriptEvent {
            frame,
            time_ms,
            kind: "START_CUTSCENE",
            handle: None,
            object_handle: None,
            source_object_handle: None,
            model_id: None,
            name: None,
            anim_name: None,
        }),
        _ => None,
    }
}

fn write_cutscene_script_events_json(json: &mut String, events: &[CutsceneScriptEvent]) {
    json.push_str("  \"events\": [\n");
    for (index, event) in events.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"frame\": {}, \"time_ms\": {}, \"kind\": \"{}\", \"handle\": {}, \"object_handle\": {}, \"source_object_handle\": {}, \"model_id\": {}, \"name\": {}, \"anim_name\": {}}}{}\n",
            index,
            event.frame,
            event.time_ms,
            event.kind,
            json_optional_i32(event.handle),
            json_optional_i32(event.object_handle),
            json_optional_i32(event.source_object_handle),
            json_optional_i32(event.model_id),
            json_optional_string(event.name.as_deref()),
            json_optional_string(event.anim_name.as_deref()),
            if index + 1 == events.len() { "" } else { "," }
        ));
    }
    json.push_str("  ],\n");
}

fn cutscene_event_count(events: &[CutsceneScriptEvent], kind: &str) -> usize {
    events.iter().filter(|event| event.kind == kind).count()
}

fn run_scm_intro_cutscene_objects(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|error| format!("failed to read {}: {error}", config.asset_root_config))?;
    let mut file_mgr = CFileMgr::initialise(PathBuf::from(root_text.trim()));
    let mut scripts =
        CTheScripts::init(&mut file_mgr).map_err(|error| format!("script init failed: {error}"))?;
    let scm = ScmFile::parse_main_scm(&scripts.main_scm_bytes);
    let mission_source_offset = scripts.multi_script_array[0] as usize;
    let mission_space = scripts.main_scm_bytes
        [mission_source_offset..mission_source_offset + SIZE_MISSION_SCRIPT]
        .to_vec();

    scripts
        .start_test_script()
        .map_err(|error| format!("StartTestScript failed: {error:?}"))?;
    let mut general = CGeneral::default();
    let mut clock = CClock::initialise(1000, 0);
    scripts
        .process_active_scripts_with_context(
            0,
            0.0,
            1.0,
            &mut general,
            &mut clock,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        )
        .map_err(|error| format!("load Process failed: {error:?}"))?;

    let mut bridge = ScriptGameplayBridge::default();
    let mut streaming = CStreaming::default();
    let mut camera = CCamera::default();
    camera.Init();
    let mut weather = CWeather::init();
    let mut gangs = CGangs::initialise();
    let mut stats = CStats::init();
    let mut cutscene = CCutsceneMgr::initialise();
    apply_script_effects_to_world(
        &mut scripts,
        &mut bridge,
        None,
        Some(&mut stats),
        Some(&mut cutscene),
        Some(&mut streaming),
        Some(&mut camera),
        Some(&mut weather),
        Some(&mut gangs),
        None,
        None,
        None,
        None,
        0,
    );

    let mut time_ms = 0u32;
    let mut extra_frames = 0u32;
    let mut captured_events = Vec::new();
    let mut capture_frame = 0u32;
    let mut capture_time_ms = 0u32;
    const STEP_MS: u32 = 16;
    const MAX_EXTRA_FRAMES: u32 = 6_000;

    while captured_events.is_empty() && extra_frames < MAX_EXTRA_FRAMES {
        time_ms = time_ms.saturating_add(STEP_MS);
        extra_frames += 1;
        if find_active_mission_script(&scripts).is_some() {
            // Equivalent to this case's immediate-success C++ streaming stub.
            scripts.special_char_loaded.fill(true);
            scripts.model_loaded.fill(true);
        }
        let player_snapshot = ScriptPlayerSnapshot {
            ped_handle: bridge.player_index.saturating_add(1),
            position: bridge.player_position,
            current_vehicle: None,
            current_vehicle_model: None,
            stopped: true,
            is_shooting: false,
            horn_pressed: false,
            is_in_control: true,
            is_driving: false,
            is_making_call: false,
            health: 100.0,
            wanted_level: 0,
            money: 0,
            current_weapon: 0,
            weapon_ammo: [0; 13],
        };
        let entity_snapshot = ScriptEntitySnapshot {
            position: bridge.player_position,
            bound_center: bridge.player_position,
            bound_radius: 0.8,
            heading_degrees: bridge.player_heading_degrees,
            up_z: 1.0,
            model_index: 0,
            entity_status: 0,
            alive: true,
            current_vehicle: None,
            current_vehicle_model: None,
            vehicle_colour1: None,
            vehicle_colour2: None,
            vehicle_is_boat: false,
            vehicle_in_water: false,
            stopped: true,
            is_shooting: false,
            collision_records: [None; 6],
            speed: 0.0,
            health: 100.0,
            current_weapon: 0,
            weapon_ammo: [0; 13],
            script_objective_completed: false,
            leader_slot: None,
            ped_type: Some(0),
            vehicle_bomb_type: None,
            vehicle_damaged: false,
            last_weapon_damage: 0,
            vehicle_num_passengers: 0,
            vehicle_num_max_passengers: 0,
            vehicle_passenger_handles: [None; 8],
            vehicle_closed_doors: 0,
            can_see_player: true,
            automobile_drive_wheels_on_ground: None,
        };
        let player_spawned = bridge.player_spawned;
        let player_index = bridge.player_index;
        let player_handles = scripts.player_char_handles;
        let mut player_probe = |index: i32| {
            (player_spawned && (index == 0 || index == player_index)).then_some(player_snapshot)
        };
        let mut char_probe = |handle: i32| {
            (player_spawned
                && player_handles
                    .iter()
                    .any(|player_handle| *player_handle == handle))
            .then_some(entity_snapshot)
        };
        let mut ground_probe = |_x: f32, _y: f32| bridge.player_position.z;
        let mut camera_probe = |_position, _radius| true;
        let mut pad_probe = |_pad: i32, _button: i32| 0i16;

        scripts
            .process_active_scripts_with_context(
                time_ms,
                STEP_MS as f32,
                1.0,
                &mut general,
                &mut clock,
                Some(&mut camera_probe),
                Some(&mut ground_probe),
                Some(&mut player_probe),
                Some(&mut char_probe),
                None,
                None,
                Some(&mut pad_probe),
                false,
            )
            .map_err(|error| format!("runtime Process frame {extra_frames} failed: {error:?}"))?;

        let frame_events: Vec<_> = scripts
            .pending_effects
            .iter()
            .filter_map(|effect| cutscene_script_event_from_effect(effect, extra_frames, time_ms))
            .collect();
        let saw_start = frame_events
            .iter()
            .any(|event| event.kind == "START_CUTSCENE");

        if saw_start {
            capture_frame = extra_frames;
            capture_time_ms = time_ms;
            captured_events = frame_events;
        }

        apply_script_effects_to_world(
            &mut scripts,
            &mut bridge,
            None,
            Some(&mut stats),
            Some(&mut cutscene),
            Some(&mut streaming),
            Some(&mut camera),
            Some(&mut weather),
            Some(&mut gangs),
            None,
            None,
            None,
            None,
            0,
        );
    }

    if captured_events.is_empty() {
        return Err("intro mission did not emit START_CUTSCENE before frame limit".to_string());
    }

    let head_count = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| object.is_head)
        .count();
    let animated_object_count = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| object.animation.is_some())
        .count();
    let head_animated_count = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| object.is_head && object.animation.is_some())
        .count();
    let head_animation_intent_count = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| object.is_head && object.head_animation_name.is_some())
        .count();
    let first_head_animation_name = cutscene
        .cutscene_objects
        .iter()
        .find_map(|object| object.head_animation_name.clone());
    let in_world_count = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| object.in_world)
        .count();
    let head_in_world = cutscene
        .cutscene_objects
        .iter()
        .any(|object| object.is_head && object.in_world);
    cutscene.update(STEP_MS as f32 * 0.001, false);
    let running_animation_count = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| {
            object
                .animation
                .as_ref()
                .is_some_and(|animation| animation.is_running())
        })
        .count();
    let in_world_after_setup = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| object.in_world)
        .count();
    let animated_in_world_after_setup = cutscene
        .cutscene_objects
        .iter()
        .filter(|object| object.in_world && object.animation.is_some())
        .count();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/control/Script.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Game.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"main_scm_size\": {}, \"loaded_main_bytes\": {}, \"mission_checksum\": {}}},\n",
        scripts.main_scm_bytes.len(),
        scripts.loaded_main_bytes,
        byte_checksum(&mission_space)
    ));
    json.push_str(&format!(
        "  \"header\": {{\"main_script_size\": {}, \"largest_mission_script_size\": {}, \"number_of_mission_scripts\": {}}},\n",
        scm.multiscript.main_script_size,
        scm.multiscript.largest_mission_script_size,
        scm.multiscript.number_of_mission_scripts
    ));
    json.push_str(&format!(
        "  \"capture_frame\": {{\"frame\": {}, \"time_ms\": {}}},\n",
        capture_frame, capture_time_ms
    ));
    json.push_str(&format!(
        "  \"summary\": {{\"event_count\": {}, \"create_object_count\": {}, \"set_anim_count\": {}, \"create_head_count\": {}, \"set_head_anim_count\": {}, \"start_count\": {}}},\n",
        captured_events.len(),
        cutscene_event_count(&captured_events, "CREATE_CUTSCENE_OBJECT"),
        cutscene_event_count(&captured_events, "SET_CUTSCENE_ANIM"),
        cutscene_event_count(&captured_events, "CREATE_CUTSCENE_HEAD"),
        cutscene_event_count(&captured_events, "SET_CUTSCENE_HEAD_ANIM"),
        cutscene_event_count(&captured_events, "START_CUTSCENE")
    ));
    write_cutscene_script_events_json(&mut json, &captured_events);
    json.push_str(&format!(
        "  \"post_apply\": {{\"object_count\": {}, \"head_count\": {}, \"animated_object_count\": {}, \"head_animated_count\": {}, \"head_animation_intent_count\": {}, \"first_head_animation_name\": {}, \"in_world_count\": {}, \"head_in_world\": {}}},\n",
        cutscene.num_cutscene_objs,
        head_count,
        animated_object_count,
        head_animated_count,
        head_animation_intent_count,
        json_optional_string(first_head_animation_name.as_deref()),
        in_world_count,
        head_in_world
    ));
    json.push_str(&format!(
        "  \"after_first_update\": {{\"running\": {}, \"status\": \"{}\", \"timer_ms\": {}, \"running_animation_count\": {}, \"in_world_count\": {}, \"animated_in_world_count\": {}}}\n",
        cutscene.running,
        cutscene_status_name(cutscene.cutscene_load_status),
        cutscene.get_cutscene_time_in_milliseconds(),
        running_animation_count,
        in_world_after_setup,
        animated_in_world_after_setup
    ));
    json.push_str("}\n");
    Ok(json)
}

fn save_align4(size: usize) -> usize {
    (size + 3) & !3
}

fn save_block_kind_name(kind: SaveBlockKind) -> &'static str {
    match kind {
        SaveBlockKind::SimpleVarsAndScripts => "SimpleVarsAndScripts",
        SaveBlockKind::PedPool => "PedPool",
        SaveBlockKind::Garages => "Garages",
        SaveBlockKind::VehiclePool => "VehiclePool",
        SaveBlockKind::ObjectPool => "ObjectPool",
        SaveBlockKind::Paths => "Paths",
        SaveBlockKind::Cranes => "Cranes",
        SaveBlockKind::Pickups => "Pickups",
        SaveBlockKind::PhoneInfo => "PhoneInfo",
        SaveBlockKind::Restart => "Restart",
        SaveBlockKind::RadarBlips => "RadarBlips",
        SaveBlockKind::Zones => "Zones",
        SaveBlockKind::GangData => "GangData",
        SaveBlockKind::CarGenerators => "CarGenerators",
        SaveBlockKind::Particles => "Particles",
        SaveBlockKind::AudioScriptObjects => "AudioScriptObjects",
        SaveBlockKind::PlayerInfo => "PlayerInfo",
        SaveBlockKind::Stats => "Stats",
        SaveBlockKind::Streaming => "Streaming",
        SaveBlockKind::PedType => "PedType",
    }
}

fn save_level_number(level: re3bevy_core::core::GameState::ELevelName) -> i32 {
    level as i32
}

fn build_roundtrip_simple_vars() -> SaveSimpleVars {
    SaveSimpleVars {
        save_name: "PORTLAND TEST".to_string(),
        save_time: SaveSystemTime {
            year: 2026,
            month: 7,
            day_of_week: 3,
            day: 8,
            hour: 21,
            minute: 30,
            second: 15,
            milliseconds: 0,
        },
        save_size: SIZE_OF_ONE_GAME_IN_BYTES as i32,
        curr_level: re3bevy_core::core::GameState::ELevelName::Industrial,
        camera_position: CVector {
            x: 100.25,
            y: -42.5,
            z: 13.75,
        },
        milliseconds_per_game_minute: 1000,
        last_clock_tick: 123456,
        game_clock_hours: 14,
        game_clock_minutes: 37,
        pad_mode: 0,
        timer_ms: 987654,
        time_scale: 1.0,
        time_step: 1.5,
        time_step_non_clipped: 1.25,
        frame_counter: 77,
        timestep_time_step: 1.0,
        frames_per_update: 1.0,
        timestep_time_scale: 1.0,
        old_weather_type: 0,
        new_weather_type: 1,
        forced_weather_type: -1,
        weather_interpolation: 0.25,
        compile_date: re3bevy_game::save::Date::CDate {
            m_nSecond: 15,
            m_nMinute: 30,
            m_nHour: 21,
            m_nDay: 8,
            m_nMonth: 7,
            m_nYear: 2026,
        },
        weather_type_in_list: 2,
        car_zoom_indicator: 1.2,
        ped_zoom_indicator: 0.8,
    }
}

fn build_roundtrip_scripts() -> Vec<u8> {
    vec![
        0x53, 0x43, 0x4D, 0x01, 0x10, 0x20, 0x30, 0x40, 0x05, 0x06, 0x07, 0x08, 0x09,
    ]
}

fn build_roundtrip_subsystem_blocks() -> Vec<SaveBlock> {
    SaveBlockKind::RE3_ORDER
        .iter()
        .copied()
        .enumerate()
        .skip(1)
        .map(|(index, kind)| {
            let payload_size = 5 + index * 7;
            let mut data = Vec::new();
            data.extend_from_slice(&(payload_size as u32).to_le_bytes());
            for i in 0..payload_size {
                data.push(((index * 31 + i * 17 + 3) & 0xFF) as u8);
            }
            data.resize(4 + save_align4(payload_size), 0);
            SaveBlock { kind, data }
        })
        .collect()
}

fn run_save_load_slot_roundtrip(config: &CaseConfig, out_file: &Path) -> Result<String, String> {
    let simple = build_roundtrip_simple_vars();
    let scripts = build_roundtrip_scripts();
    let subsystem_blocks = build_roundtrip_subsystem_blocks();
    let blocks = build_generic_save_blocks(&simple, &scripts, subsystem_blocks)
        .map_err(|err| format!("failed to build save blocks: {err}"))?;
    let (expected_image, expected_checksum) = encode_pc_save_image(&blocks)
        .map_err(|err| format!("failed to encode save image: {err}"))?;

    let slot_dir = out_file
        .parent()
        .ok_or_else(|| "missing parity output directory".to_string())?
        .join("slot_roundtrip_rust");
    if slot_dir.exists() {
        fs::remove_dir_all(&slot_dir)
            .map_err(|err| format!("failed to clear {}: {err}", slot_dir.display()))?;
    }
    fs::create_dir_all(&slot_dir)
        .map_err(|err| format!("failed to create {}: {err}", slot_dir.display()))?;

    let mut storage = CGenericGameStorage::default();
    let mut pc_save = C_PcSave::default();
    C_PcSave::SetSaveDirectory(&mut storage, &slot_dir);
    pc_save
        .SaveSlotBlocks(&mut storage, 2, &blocks)
        .map_err(|err| format!("failed to save slot blocks: {err}"))?;
    pc_save
        .PopulateSlotInfo(&mut storage, &slot_dir)
        .map_err(|err| format!("failed to populate slot info: {err}"))?;
    let loaded = pc_save
        .LoadSlotBlocks(&mut storage, 2)
        .map_err(|err| format!("failed to load slot blocks: {err}"))?;
    let generic = split_generic_load_blocks(&loaded)
        .map_err(|err| format!("failed to split generic load blocks: {err}"))?;
    let slot_path = slot_dir.join("GTA3sf3.b");
    let loaded_image = fs::read(&slot_path)
        .map_err(|err| format!("failed to read {}: {err}", slot_path.display()))?;
    let generic_blocks: Vec<SaveBlock> = loaded.blocks.iter().take(BLOCK_COUNT).cloned().collect();
    let (reencoded_image, _) = encode_pc_save_image(&generic_blocks)
        .map_err(|err| format!("failed to re-encode loaded save blocks: {err}"))?;

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/save/GenericGameStorage.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/save/PCSave.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/save/SaveBuf.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"slot\": 2, \"slot_file\": \"GTA3sf3.b\", \"generic_block_count\": {}, \"save_size_field\": {}, \"work_buff_size\": {}}},\n",
        BLOCK_COUNT, SIZE_OF_ONE_GAME_IN_BYTES, WORK_BUFF_SIZE
    ));
    json.push_str("  \"blocks\": [\n");
    for (index, block) in loaded.blocks.iter().enumerate() {
        let kind = if index < BLOCK_COUNT {
            save_block_kind_name(block.kind)
        } else {
            "Padding"
        };
        json.push_str(&format!(
            "    {{\"index\": {}, \"kind\": \"{}\", \"payload_size\": {}, \"aligned_size\": {}}}",
            index,
            kind,
            block.data.len(),
            save_align4(block.data.len())
        ));
        if index + 1 < loaded.blocks.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"roundtrip\": {{\"image_len\": {}, \"checksum\": {}, \"parsed_checksum\": {}, \"bytes_processed\": {}, \"parsed_block_count\": {}, \"padding_block_count\": {}, \"level_to_load\": {}, \"loaded_block_count\": {}, \"loaded_checksum\": {}, \"byte_equal\": {}}},\n",
        loaded_image.len(),
        expected_checksum,
        loaded.checksum,
        loaded.bytes_processed,
        loaded.blocks.len(),
        loaded.blocks.len().saturating_sub(BLOCK_COUNT),
        save_level_number(loaded.level_to_load),
        loaded.blocks.len(),
        loaded.checksum,
        loaded_image == expected_image && loaded_image == reencoded_image
    ));
    json.push_str(&format!(
        "  \"slot_info\": {{\"state\": {}, \"name\": \"{}\", \"date\": \"{}\", \"level_to_load\": {}}},\n",
        storage.Slots[3].as_re3_pc_slot_value(),
        json_escape(&storage.SlotFileName[2]),
        json_escape(&storage.SlotSaveDate[2]),
        save_level_number(storage.m_LevelToLoad)
    ));
    json.push_str(&format!(
        "  \"simple_vars\": {{\"save_name\": \"{}\", \"save_size\": {}, \"curr_level\": {}, \"camera_position\": [{}, {}, {}], \"clock\": {{\"milliseconds_per_game_minute\": {}, \"last_clock_tick\": {}, \"hours\": {}, \"minutes\": {}}}, \"timer_ms\": {}, \"frame_counter\": {}, \"weather\": {{\"old\": {}, \"new\": {}, \"forced\": {}, \"interpolation\": {}, \"type_in_list\": {}}}, \"compile_date\": {{\"second\": {}, \"minute\": {}, \"hour\": {}, \"day\": {}, \"month\": {}, \"year\": {}}}, \"zoom\": {{\"car\": {}, \"ped\": {}}}}},\n",
        json_escape(&generic.simple_vars.save_name),
        generic.simple_vars.save_size,
        save_level_number(generic.simple_vars.curr_level),
        generic.simple_vars.camera_position.x,
        generic.simple_vars.camera_position.y,
        generic.simple_vars.camera_position.z,
        generic.simple_vars.milliseconds_per_game_minute,
        generic.simple_vars.last_clock_tick,
        generic.simple_vars.game_clock_hours,
        generic.simple_vars.game_clock_minutes,
        generic.simple_vars.timer_ms,
        generic.simple_vars.frame_counter,
        generic.simple_vars.old_weather_type,
        generic.simple_vars.new_weather_type,
        generic.simple_vars.forced_weather_type,
        generic.simple_vars.weather_interpolation,
        generic.simple_vars.weather_type_in_list,
        generic.simple_vars.compile_date.m_nSecond,
        generic.simple_vars.compile_date.m_nMinute,
        generic.simple_vars.compile_date.m_nHour,
        generic.simple_vars.compile_date.m_nDay,
        generic.simple_vars.compile_date.m_nMonth,
        generic.simple_vars.compile_date.m_nYear,
        generic.simple_vars.car_zoom_indicator,
        generic.simple_vars.ped_zoom_indicator
    ));
    json.push_str(&format!(
        "  \"scripts\": {{\"payload_size\": {}, \"aligned_size\": {}}}\n",
        generic.scripts.len(),
        SIZE_OF_SIMPLEVARS + 4 + save_align4(generic.scripts.len()) - SIZE_OF_SIMPLEVARS - 4
    ));
    json.push_str("}\n");
    Ok(json)
}

fn replay_vehicle_matrix(
    position: [f32; 3],
    right: [i8; 3],
    up: [i8; 3],
) -> CCompressedMatrixNotAlignedReplay {
    CCompressedMatrixNotAlignedReplay {
        position,
        right,
        up,
    }
}

fn replay_vehicle_update_one() -> TVehicleUpdatePacket {
    TVehicleUpdatePacket {
        index: 1,
        health: 200,
        acceleration: 64,
        matrix: replay_vehicle_matrix([10.0, -5.5, 2.25], [127, 0, 0], [0, 127, 0]),
        door_angles: [-12, 34],
        model_index: 101,
        panels: 0x0102_0304,
        velocity: [-3, 4, 5],
        car_gun_or_wheel_state: -7,
        wheel_susp_dist: [11, 22, 33, 44],
        wheel_rotation: [5, 15, 25, 35],
        door_status: 3,
        primary_color: 12,
        secondary_color: 34,
    }
}

fn replay_vehicle_update_two() -> TVehicleUpdatePacket {
    TVehicleUpdatePacket {
        index: 3,
        health: 180,
        acceleration: 90,
        matrix: replay_vehicle_matrix([14.5, 3.0, 2.5], [0, 127, 0], [0, 0, 127]),
        door_angles: [8, -9],
        model_index: 202,
        panels: 0x0A0B_0C0D,
        velocity: [6, -7, 8],
        car_gun_or_wheel_state: 9,
        wheel_susp_dist: [55, 66, 77, 88],
        wheel_rotation: [95, 85, 75, 65],
        door_status: 4,
        primary_color: 56,
        secondary_color: 78,
    }
}

fn replay_dummy_vehicle_update(index: u8) -> TVehicleUpdatePacket {
    TVehicleUpdatePacket {
        index,
        health: 1,
        acceleration: 0,
        matrix: replay_vehicle_matrix([0.0, 0.0, 0.0], [127, 0, 0], [0, 127, 0]),
        door_angles: [0, 0],
        model_index: 900 + index as u16,
        panels: 0,
        velocity: [0, 0, 0],
        car_gun_or_wheel_state: 0,
        wheel_susp_dist: [0, 0, 0, 0],
        wheel_rotation: [0, 0, 0, 0],
        door_status: 0,
        primary_color: 0,
        secondary_color: 0,
    }
}

fn replay_empty_vehicle_update(index: u8, model_index: u16) -> TVehicleUpdatePacket {
    TVehicleUpdatePacket {
        index,
        health: 0,
        acceleration: 0,
        matrix: CCompressedMatrixNotAlignedReplay {
            position: [0.0; 3],
            right: [0; 3],
            up: [0; 3],
        },
        door_angles: [0; 2],
        model_index,
        panels: 0,
        velocity: [0; 3],
        car_gun_or_wheel_state: 0,
        wheel_susp_dist: [0; 4],
        wheel_rotation: [0; 4],
        door_status: 0,
        primary_color: 0,
        secondary_color: 0,
    }
}

fn replay_vehicle_packets() -> Vec<ReplayPacket> {
    vec![
        ReplayPacket::General(TGeneralPacket {
            in_rcvehicle: false,
            camera_matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 100.0, 200.0, 5.0, 1.0,
            ],
            player_pos: [100.0, 200.0, 5.0],
        }),
        ReplayPacket::Clock(TClockPacket {
            hours: 14,
            minutes: 37,
        }),
        ReplayPacket::Weather(TWeatherPacket {
            old_weather: 1,
            new_weather: 2,
            interpolation: 0.25,
        }),
        ReplayPacket::Timer(TTimerPacket { timer: 123456 }),
        ReplayPacket::Vehicle(replay_vehicle_update_one()),
        ReplayPacket::Vehicle(replay_vehicle_update_two()),
        ReplayPacket::EndOfFrame,
    ]
}

fn replay_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn replay_packet_kind_name(packet: &ReplayPacket) -> &'static str {
    match packet {
        ReplayPacket::End => "End",
        ReplayPacket::Vehicle(_) => "Vehicle",
        ReplayPacket::PedHeader(_) => "PedHeader",
        ReplayPacket::PedUpdate(_) => "PedUpdate",
        ReplayPacket::General(_) => "General",
        ReplayPacket::Clock(_) => "Clock",
        ReplayPacket::Weather(_) => "Weather",
        ReplayPacket::EndOfFrame => "EndOfFrame",
        ReplayPacket::Timer(_) => "Timer",
        ReplayPacket::BulletTraces(_) => "BulletTraces",
    }
}

fn replay_vehicle_json(vehicle: TVehicleUpdatePacket) -> String {
    format!(
        "{{\"index\": {}, \"health\": {}, \"acceleration\": {}, \"position\": [{}, {}, {}], \"right\": [{}, {}, {}], \"up\": [{}, {}, {}], \"door_angles\": [{}, {}], \"model_index\": {}, \"panels\": {}, \"velocity\": [{}, {}, {}], \"wheel_state\": {}, \"wheel_susp_dist\": [{}, {}, {}, {}], \"wheel_rotation\": [{}, {}, {}, {}], \"door_status\": {}, \"primary_color\": {}, \"secondary_color\": {}}}",
        vehicle.index,
        vehicle.health,
        vehicle.acceleration,
        vehicle.matrix.position[0],
        vehicle.matrix.position[1],
        vehicle.matrix.position[2],
        vehicle.matrix.right[0],
        vehicle.matrix.right[1],
        vehicle.matrix.right[2],
        vehicle.matrix.up[0],
        vehicle.matrix.up[1],
        vehicle.matrix.up[2],
        vehicle.door_angles[0],
        vehicle.door_angles[1],
        vehicle.model_index,
        vehicle.panels,
        vehicle.velocity[0],
        vehicle.velocity[1],
        vehicle.velocity[2],
        vehicle.car_gun_or_wheel_state,
        vehicle.wheel_susp_dist[0],
        vehicle.wheel_susp_dist[1],
        vehicle.wheel_susp_dist[2],
        vehicle.wheel_susp_dist[3],
        vehicle.wheel_rotation[0],
        vehicle.wheel_rotation[1],
        vehicle.wheel_rotation[2],
        vehicle.wheel_rotation[3],
        vehicle.door_status,
        vehicle.primary_color,
        vehicle.secondary_color
    )
}

fn replay_vehicle_op_json(op: &ReplayVehicleMutation) -> String {
    match *op {
        ReplayVehicleMutation::RemoveMissingBefore {
            start,
            end_exclusive,
        } => format!(
            "{{\"op\": \"remove_missing_before\", \"start\": {}, \"end_exclusive\": {}}}",
            start, end_exclusive
        ),
        ReplayVehicleMutation::EnsureExists { index, model_index } => format!(
            "{{\"op\": \"ensure_exists\", \"index\": {}, \"model_index\": {}}}",
            index, model_index
        ),
        ReplayVehicleMutation::ApplyUpdate(update) => format!(
            "{{\"op\": \"apply_update\", \"index\": {}, \"model_index\": {}, \"health\": {}, \"position\": [{}, {}, {}]}}",
            update.index,
            update.model_index,
            update.health,
            update.matrix.position[0],
            update.matrix.position[1],
            update.matrix.position[2]
        ),
        ReplayVehicleMutation::RemoveMissingAfter { start } => {
            format!("{{\"op\": \"remove_missing_after\", \"start\": {}}}", start)
        }
    }
}

fn run_replay_vehicle_frame(config: &CaseConfig) -> Result<String, String> {
    let packets = replay_vehicle_packets();
    let encoded_packets: Vec<Vec<u8>> = packets
        .iter()
        .cloned()
        .map(CReplay::encode_packet)
        .collect();
    let mut replay = CReplay::init();
    for bytes in &encoded_packets {
        replay.record_bytes(bytes);
    }
    let end_packet = CReplay::encode_packet(ReplayPacket::End);
    let stream_end = replay.record.offset as usize + end_packet.len();
    let stream = replay.buffers[0][0..stream_end].to_vec();
    let mut address = CAddressInReplayBuffer {
        offset: 0,
        slot: 0,
        has_base: true,
    };
    let frame = replay
        .collect_playback_frame(&mut address, 0.5)
        .ok_or_else(|| "failed to collect replay frame".to_string())?;
    let plan = frame.build_world_mutation_plan();

    let mut runtime = ReplayRuntimeWorldState::default();
    runtime.camera_focus = [80.0, 160.0, 2.0];
    let mut vehicle_states = BTreeMap::<u8, ReplayVehicleRuntimeState>::new();
    for index in 0..5_u8 {
        let update = replay_dummy_vehicle_update(index);
        vehicle_states.insert(
            index,
            ReplayVehicleRuntimeState {
                replay_index: index,
                model_index: update.model_index,
                update,
            },
        );
    }
    plan.apply_to_runtime_state(&mut runtime);
    for op in &plan.vehicle_ops {
        match *op {
            ReplayVehicleMutation::RemoveMissingBefore {
                start,
                end_exclusive,
            } => {
                vehicle_states.retain(|index, _| *index < start || *index >= end_exclusive);
            }
            ReplayVehicleMutation::EnsureExists { index, model_index } => {
                let state = vehicle_states.get(&index).map_or_else(
                    || ReplayVehicleRuntimeState {
                        replay_index: index,
                        model_index,
                        update: replay_empty_vehicle_update(index, model_index),
                    },
                    |current| ReplayVehicleRuntimeState {
                        replay_index: index,
                        model_index,
                        update: current.update,
                    },
                );
                vehicle_states.insert(index, state);
            }
            ReplayVehicleMutation::ApplyUpdate(update) => {
                vehicle_states.insert(
                    update.index,
                    ReplayVehicleRuntimeState {
                        replay_index: update.index,
                        model_index: update.model_index,
                        update,
                    },
                );
            }
            ReplayVehicleMutation::RemoveMissingAfter { start } => {
                vehicle_states.retain(|index, _| *index < start);
            }
        }
    }
    let overlay = vehicle_states.values().copied().collect::<Vec<_>>();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/control/Replay.rs\"\n");
    json.push_str("  ],\n");
    json.push_str("  \"setup\": {\"buffer_count\": 8, \"buffer_size\": 100000, \"interpolation\": 0.5, \"initial_vehicle_indices\": [0, 1, 2, 3, 4]},\n");
    json.push_str(&format!(
        "  \"stream\": {{\"len\": {}, \"end_marker_offset\": {}, \"hex\": \"{}\"}},\n",
        stream.len(),
        replay.record.offset,
        replay_hex(&stream)
    ));
    json.push_str("  \"packets\": [\n");
    let mut offset = 0usize;
    for (index, (packet, bytes)) in packets.iter().zip(encoded_packets.iter()).enumerate() {
        json.push_str(&format!(
            "    {{\"offset\": {}, \"kind\": \"{}\", \"size\": {}, \"hex\": \"{}\"}}",
            offset,
            replay_packet_kind_name(packet),
            bytes.len(),
            replay_hex(bytes)
        ));
        offset += bytes.len();
        if index + 1 < packets.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"frame\": {{\"address_after\": {{\"slot\": {}, \"offset\": {}}}, \"wrapped_slots\": {}, \"reached_record_slot\": {}, \"clock\": {{\"hours\": {}, \"minutes\": {}}}, \"weather\": {{\"old\": {}, \"new\": {}, \"interpolation\": {}}}, \"timer\": {}, \"camera_focus\": [{}, {}, {}], \"player_in_rcvehicle\": {}, \"vehicle_count\": {}}},\n",
        address.slot,
        address.offset,
        frame.wrapped_slots,
        frame.reached_record_slot,
        frame.clock.unwrap().hours,
        frame.clock.unwrap().minutes,
        frame.weather.unwrap().old_weather,
        frame.weather.unwrap().new_weather,
        frame.weather.unwrap().interpolation,
        frame.timer.unwrap().timer,
        frame.general.unwrap().player_pos[0],
        frame.general.unwrap().player_pos[1],
        frame.general.unwrap().player_pos[2],
        frame.general.unwrap().in_rcvehicle,
        frame.vehicle_updates.len()
    ));
    json.push_str("  \"vehicles\": [\n");
    for (index, vehicle) in frame.vehicle_updates.iter().enumerate() {
        json.push_str("    ");
        json.push_str(&replay_vehicle_json(*vehicle));
        if index + 1 < frame.vehicle_updates.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"mutation_ops\": [\n");
    for (index, op) in plan.vehicle_ops.iter().enumerate() {
        json.push_str("    ");
        json.push_str(&replay_vehicle_op_json(op));
        if index + 1 < plan.vehicle_ops.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"runtime_overlay\": [\n");
    for (index, vehicle) in overlay.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"replay_index\": {}, \"model_index\": {}, \"health\": {}, \"position\": [{}, {}, {}], \"velocity\": [{}, {}, {}]}}",
            vehicle.replay_index,
            vehicle.model_index,
            vehicle.update.health,
            vehicle.update.matrix.position[0],
            vehicle.update.matrix.position[1],
            vehicle.update.matrix.position[2],
            vehicle.update.velocity[0],
            vehicle.update.velocity[1],
            vehicle.update.velocity[2]
        ));
        if index + 1 < overlay.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_random_sequence(config: &CaseConfig) -> String {
    let mut general = CGeneral::with_seed(config.seed);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_core/src/core/General.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"seed\": {}, \"count\": {}}},\n",
        config.seed, config.count
    ));
    json.push_str("  \"snapshots\": [\n");

    for index in 0..config.count {
        let value = general.get_random_number();
        json.push_str(&format!(
            "    {{\"index\": {}, \"value\": {}, \"seed_after\": {}}}",
            index,
            value,
            general.seed()
        ));
        if index + 1 < config.count {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn run_timer_update(config: &CaseConfig) -> String {
    let mut timer = CTimer::initialise();
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_core/src/core/Timer.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"record_playback\": {}, \"chase_recording\": {}}},\n",
        config.record_playback, config.chase_recording
    ));
    json.push_str("  \"snapshots\": [\n");

    for (index, elapsed_ms) in config.elapsed_ms.iter().enumerate() {
        timer.update_from_elapsed_milliseconds(
            *elapsed_ms,
            config.record_playback,
            config.chase_recording,
        );
        json.push_str(&format!(
            "    {{\"index\": {}, \"elapsed_ms\": {}, \"time_in_milliseconds\": {}, \"time_in_milliseconds_pause_mode\": {}, \"time_in_milliseconds_non_clipped\": {}, \"previous_time_in_milliseconds\": {}, \"frame_counter\": {}, \"time_step\": {}, \"time_step_non_clipped\": {}}}",
            index,
            elapsed_ms,
            timer.time_in_milliseconds(),
            timer.time_in_milliseconds_pause_mode(),
            timer.time_in_milliseconds_non_clipped(),
            timer.previous_time_in_milliseconds(),
            timer.frame_counter(),
            timer.time_step(),
            timer.time_step_non_clipped()
        ));
        if index + 1 < config.elapsed_ms.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn run_clock_update(config: &CaseConfig) -> String {
    let mut clock = CClock::initialise(config.milliseconds_per_game_minute, 0);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_core/src/core/Clock.rs\",\n");
    json.push_str("    \"crates/re3bevy_core/src/core/Timer.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"milliseconds_per_game_minute\": {}, \"debug_fast_forward\": {}}},\n",
        config.milliseconds_per_game_minute, config.debug_fast_forward
    ));
    json.push_str("  \"snapshots\": [\n");

    for (index, timer_ms) in config.timer_ms.iter().enumerate() {
        clock.update(*timer_ms, config.debug_fast_forward);
        json.push_str(&format!(
            "    {{\"index\": {}, \"timer_ms\": {}, \"hours\": {}, \"minutes\": {}, \"seconds\": {}, \"days_passed\": {}}}",
            index,
            timer_ms,
            clock.hours(),
            clock.minutes(),
            clock.seconds(),
            clock.days_passed()
        ));
        if index + 1 < config.timer_ms.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn run_weather_update(config: &CaseConfig) -> String {
    let mut general = CGeneral::with_seed(config.seed);
    let mut weather = CWeather::init();
    weather.forced_weather_type = config.forced_weather_type;
    let mut mm_rain_total: u16 = 0;

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/renderer/Weather.rs\",\n");
    json.push_str("    \"crates/re3bevy_core/src/core/General.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"seed\": {}, \"milliseconds_per_game_minute\": {}, \"forced_weather_type\": {}}},\n",
        config.seed, config.milliseconds_per_game_minute, config.forced_weather_type
    ));
    json.push_str("  \"snapshots\": [\n");

    for (index, timer_ms) in config.timer_ms.iter().enumerate() {
        let total_minutes = *timer_ms / config.milliseconds_per_game_minute;
        let context = WeatherUpdateContext {
            minute: (total_minutes % 60) as u8,
            hour: ((12 + (total_minutes / 60)) % 24) as u8,
            time_ms: *timer_ms,
            frame_counter: index as u32,
            time_step: config.elapsed_ms.get(index).copied().unwrap_or(1.0) as f32,
            right_shock_just_down: false,
            cam_no_rain: true,
            player_no_rain: false,
            looking_lrb_first_person: false,
            player_vehicle_has_roof: false,
        };
        let result = weather.update(context, &mut general);
        mm_rain_total = mm_rain_total.wrapping_add(result.mm_rain_delta);
        json.push_str(&format!(
            "    {{\"index\": {}, \"timer_ms\": {}, \"hour\": {}, \"minute\": {}, \"time_step\": {}, \"seed_after\": {}, \"weather_type_in_list\": {}, \"old_weather_type\": {}, \"new_weather_type\": {}, \"forced_weather_type\": {}, \"interpolation_value\": {}, \"wet_roads\": {}, \"rain\": {}, \"cloud_coverage\": {}, \"foggyness\": {}, \"rainbow\": {}, \"wind\": {}, \"lightning_flash\": {}, \"lightning_burst\": {}, \"lightning_start\": {}, \"lightning_flash_last_change\": {}, \"when_to_play_lightning_sound\": {}, \"lightning_duration\": {}, \"mm_rain_delta\": {}, \"mm_rain_total\": {}, \"lightning_sound\": ",
            index,
            timer_ms,
            context.hour,
            context.minute,
            context.time_step,
            general.seed(),
            weather.weather_type_in_list,
            weather.old_weather_type,
            weather.new_weather_type,
            weather.forced_weather_type,
            weather.interpolation_value,
            weather.wet_roads,
            weather.rain,
            weather.cloud_coverage,
            weather.foggyness,
            weather.rainbow,
            weather.wind,
            weather.lightning_flash,
            weather.lightning_burst,
            weather.lightning_start,
            weather.lightning_flash_last_change,
            weather.when_to_play_lightning_sound,
            weather.lightning_duration,
            result.mm_rain_delta,
            mm_rain_total
        ));
        if let Some(sound) = result.lightning_sound {
            json.push_str(&format!(
                "{{\"sound_handle\": {}, \"sound\": {}, \"duration\": {}}}",
                sound.sound_handle, sound.sound, sound.duration
            ));
        } else {
            json.push_str("null");
        }
        json.push_str(", \"pad_shake\": ");
        if let Some(pad) = result.pad_shake {
            json.push_str(&format!(
                "{{\"duration\": {}, \"frequency\": {}}}",
                pad.duration, pad.frequency
            ));
        } else {
            json.push_str("null");
        }
        json.push_str(", \"rain_intent\": ");
        if let Some(rain) = result.rain_intent {
            json.push_str(&format!(
                "{{\"rain\": {}, \"current_frame\": {}, \"num_drops\": {}, \"num_splash_attempts\": {}, \"num_splashes\": {}}}",
                rain.rain,
                rain.current_frame,
                rain.num_drops,
                rain.num_splash_attempts,
                rain.num_splashes
            ));
        } else {
            json.push_str("null");
        }
        json.push_str(&format!(
            ", \"remove_2d_raindrops\": {}}}",
            result.remove_2d_raindrops
        ));
        if index + 1 < config.timer_ms.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

#[derive(Clone, Copy)]
struct TimeCycleStepInput {
    hour: u8,
    minute: u8,
    old_weather: i16,
    new_weather: i16,
    interpolation: f32,
    camera_forward_z: f32,
    motion_blur_light_scene: bool,
    no_rain: bool,
    scripts_force_rain: bool,
    cutscene_running: bool,
}

fn timecycle_step_inputs() -> Vec<TimeCycleStepInput> {
    let mut steps = vec![
        TimeCycleStepInput {
            hour: 0,
            minute: 0,
            old_weather: 0,
            new_weather: 0,
            interpolation: 0.0,
            camera_forward_z: 0.0,
            motion_blur_light_scene: true,
            no_rain: false,
            scripts_force_rain: false,
            cutscene_running: false,
        },
        TimeCycleStepInput {
            hour: 5,
            minute: 17,
            old_weather: 0,
            new_weather: 1,
            interpolation: 0.25,
            camera_forward_z: -1.0,
            motion_blur_light_scene: true,
            no_rain: false,
            scripts_force_rain: false,
            cutscene_running: false,
        },
        TimeCycleStepInput {
            hour: 12,
            minute: 30,
            old_weather: 1,
            new_weather: 2,
            interpolation: 0.5,
            camera_forward_z: 0.0,
            motion_blur_light_scene: false,
            no_rain: true,
            scripts_force_rain: false,
            cutscene_running: false,
        },
        TimeCycleStepInput {
            hour: 18,
            minute: 45,
            old_weather: 2,
            new_weather: 3,
            interpolation: 0.75,
            camera_forward_z: 0.0,
            motion_blur_light_scene: true,
            no_rain: true,
            scripts_force_rain: true,
            cutscene_running: false,
        },
        TimeCycleStepInput {
            hour: 23,
            minute: 59,
            old_weather: 3,
            new_weather: 0,
            interpolation: 1.0 / 3.0,
            camera_forward_z: 0.0,
            motion_blur_light_scene: true,
            no_rain: false,
            scripts_force_rain: false,
            cutscene_running: true,
        },
    ];

    let fog_ramp = TimeCycleStepInput {
        hour: 5,
        minute: 17,
        old_weather: 0,
        new_weather: 1,
        interpolation: 0.25,
        camera_forward_z: -1.0,
        motion_blur_light_scene: true,
        no_rain: false,
        scripts_force_rain: false,
        cutscene_running: false,
    };
    steps.extend(std::iter::repeat_n(fog_ramp, 64));

    let fog_decay = TimeCycleStepInput {
        camera_forward_z: 0.0,
        ..fog_ramp
    };
    steps.extend(std::iter::repeat_n(fog_decay, 3));
    steps
}

fn run_timecycle_update(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let install = OriginalGtaInstall::from_root(root_text.trim())
        .map_err(|err| format!("invalid original GTA III install: {err}"))?;
    let mut file_mgr = install.file_mgr();
    let mut timecycle = CTimeCycle::initialise(&mut file_mgr)
        .map_err(|err| format!("failed to initialise CTimeCycle: {err}"))?;
    let steps = timecycle_step_inputs();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/renderer/Timecycle.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/renderer/Weather.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"asset_root\": \"configured\", \"scripted_steps\": {}}},\n",
        steps.len()
    ));
    json.push_str("  \"snapshots\": [\n");

    for (index, step) in steps.iter().copied().enumerate() {
        let mut weather = CWeather::init();
        weather.old_weather_type = step.old_weather;
        weather.new_weather_type = step.new_weather;
        weather.interpolation_value = step.interpolation;
        weather.scripts_force_rain = step.scripts_force_rain;
        let result = timecycle.update(
            &weather,
            TimeCycleUpdateContext {
                hour: step.hour,
                minute: step.minute,
                camera_forward_z: step.camera_forward_z,
                camera_motion_blur_light_scene: step.motion_blur_light_scene,
                no_rain_from_cullzone: step.no_rain,
                cutscene_running: step.cutscene_running,
            },
        );
        let slot = timecycle.current_stored_value as usize;
        let current = timecycle.current;
        let sun = timecycle.vector_to_sun[slot];

        json.push_str(&format!(
            "    {{\"index\": {index}, \"hour\": {}, \"minute\": {}, \"old_weather\": {}, \"new_weather\": {}, \"interpolation\": {}, \"fog_reduction\": {}, \"stored_value\": {}, \"ambient\": [{}, {}, {}], \"directional\": [{}, {}, {}], \"sky_top\": [{}, {}, {}], \"sky_bottom\": [{}, {}, {}], \"sun_core\": [{}, {}, {}], \"sun_corona\": [{}, {}, {}], \"sun_size\": {}, \"sprite_size\": {}, \"sprite_brightness\": {}, \"shadow_strength\": {}, \"light_shadow_strength\": {}, \"tree_shadow_strength\": {}, \"far_clip\": {}, \"fog_start\": {}, \"lights_on_ground_brightness\": {}, \"low_clouds\": [{}, {}, {}], \"fluffy_clouds_top\": [{}, {}, {}], \"fluffy_clouds_bottom\": [{}, {}, {}], \"blur\": [{}, {}, {}, {}], \"fog_colour\": [{}, {}, {}], \"sun_direction\": [{}, {}, {}], \"shadow_front\": [{}, {}], \"shadow_side\": [{}, {}], \"shadow_displacement\": [{}, {}], \"motion_blur\": ",
            step.hour,
            step.minute,
            step.old_weather,
            step.new_weather,
            step.interpolation,
            timecycle.fog_reduction,
            timecycle.current_stored_value,
            current.ambient[0],
            current.ambient[1],
            current.ambient[2],
            current.directional[0],
            current.directional[1],
            current.directional[2],
            current.sky_top[0],
            current.sky_top[1],
            current.sky_top[2],
            current.sky_bottom[0],
            current.sky_bottom[1],
            current.sky_bottom[2],
            current.sun_core[0],
            current.sun_core[1],
            current.sun_core[2],
            current.sun_corona[0],
            current.sun_corona[1],
            current.sun_corona[2],
            current.sun_size,
            current.sprite_size,
            current.sprite_brightness,
            current.shadow_strength,
            current.light_shadow_strength,
            current.tree_shadow_strength,
            current.far_clip,
            current.fog_start,
            current.lights_on_ground_brightness,
            current.low_clouds[0],
            current.low_clouds[1],
            current.low_clouds[2],
            current.fluffy_clouds_top[0],
            current.fluffy_clouds_top[1],
            current.fluffy_clouds_top[2],
            current.fluffy_clouds_bottom[0],
            current.fluffy_clouds_bottom[1],
            current.fluffy_clouds_bottom[2],
            current.blur[0],
            current.blur[1],
            current.blur[2],
            current.blur[3],
            current.fog_colour[0],
            current.fog_colour[1],
            current.fog_colour[2],
            sun.x,
            sun.y,
            sun.z,
            timecycle.shadow_front_x[slot],
            timecycle.shadow_front_y[slot],
            timecycle.shadow_side_x[slot],
            timecycle.shadow_side_y[slot],
            timecycle.shadow_displacement_x[slot],
            timecycle.shadow_displacement_y[slot],
        ));
        if let Some(blur) = result.motion_blur {
            json.push_str(&format!(
                "[{}, {}, {}, {}]",
                blur[0], blur[1], blur[2], blur[3]
            ));
        } else {
            json.push_str("null");
        }
        json.push('}');
        if index + 1 < steps.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_frontend_menu_transition(config: &CaseConfig) -> String {
    let mut menu = initialise_frontend_transition_menu(true);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Frontend.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/FrontendFlow.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/MenuScreens.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"event_count\": {}, \"slot_states\": [",
        config.events.len()
    ));
    for (index, state) in config.slot_states.iter().enumerate() {
        json.push_str(&state.to_string());
        if index + 1 < config.slot_states.len() {
            json.push_str(", ");
        }
    }
    json.push_str("]},\n");
    json.push_str("  \"snapshots\": [\n");
    write_frontend_snapshot(&mut json, 0, "initial", "none", &menu);
    if !config.events.is_empty() {
        json.push_str(",\n");
    } else {
        json.push('\n');
    }

    for (index, event) in config.events.iter().enumerate() {
        let input = parse_frontend_input(event);
        let outcome = apply_frontend_menu_transition(&mut menu, input, &config.slot_states);
        write_frontend_snapshot(
            &mut json,
            index + 1,
            event,
            frontend_outcome_name(outcome),
            &menu,
        );
        if index + 1 < config.events.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn run_frontend_start_new_game(config: &CaseConfig) -> String {
    let mut menu = initialise_frontend_transition_menu(true);
    let default_events = ["enter".to_string(), "enter".to_string()];
    let events = if config.events.is_empty() {
        default_events.as_slice()
    } else {
        config.events.as_slice()
    };
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Frontend.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/FrontendFlow.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/MenuScreens.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"event_count\": {}, \"route\": \"start_menu_single_game\", \"game_not_loaded\": true, \"slot_states\": [",
        events.len()
    ));
    for (index, state) in config.slot_states.iter().enumerate() {
        json.push_str(&state.to_string());
        if index + 1 < config.slot_states.len() {
            json.push_str(", ");
        }
    }
    json.push_str("]},\n");
    json.push_str("  \"snapshots\": [\n");
    write_frontend_snapshot(&mut json, 0, "initial", "none", &menu);
    if !events.is_empty() {
        json.push_str(",\n");
    } else {
        json.push('\n');
    }

    for (index, event) in events.iter().enumerate() {
        let input = parse_frontend_input(event);
        let outcome = apply_frontend_menu_transition(&mut menu, input, &config.slot_states);
        write_frontend_snapshot(
            &mut json,
            index + 1,
            event,
            frontend_outcome_name(outcome),
            &menu,
        );
        if index + 1 < events.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn run_streaming_initial_requests(config: &CaseConfig) -> Result<String, String> {
    let install = OriginalGtaInstall::from_config_path(&config.asset_root_config)
        .map_err(|err| format!("failed to read GTA install config: {err}"))?;
    let mut file_mgr = install.file_mgr();
    let mut cd_stream = CdStream::init(2);
    let mut file_loader = CFileLoader::new();
    let mut streaming = CStreaming::default();
    let dat_file = config.gta3_relative_path.replace('/', "\\");

    let load_result = file_loader
        .load_default_and_dat_plans(&install, &mut file_mgr, &mut cd_stream, &dat_file)
        .and_then(|_| streaming.load_cd_directory(&cd_stream, &mut file_loader));
    if let Err(error) = load_result {
        cd_stream.shutdown();
        return Err(format!(
            "failed to prepare streaming startup state: {error}"
        ));
    }

    file_loader.load_initial_vehicles(&mut streaming);
    file_loader.load_initial_peds(&mut streaming);
    file_loader.request_initial_big_buildings(&mut streaming);

    let default_path =
        install.resolve_existing_case_path(config.default_relative_path.replace('/', "\\"));
    let gta3_path =
        install.resolve_existing_case_path(config.gta3_relative_path.replace('/', "\\"));
    let model_info_count = (0..STREAM_OFFSET_TXD)
        .filter(|id| file_loader.model_name(*id).is_some())
        .count();
    let big_building_count = file_loader.world.big_buildings_by_level[0].len();
    let requests = streaming.requested_list().collect::<Vec<_>>();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Game.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Streaming.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/FileLoader.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/modelinfo/ModelIndices.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"default_dat_size\": {}, \"gta3_dat_size\": {}, \"ide_file_count\": {}, \"ipl_file_count\": {}, \"model_info_count\": {}, \"big_building_count\": {}, \"requested_count\": {}, \"priority_count\": {}, \"has_loaded_lods\": {}, \"num_vehicles_loaded\": {}, \"last_vehicle_deleted\": {}}},\n",
        fs::metadata(&default_path)
            .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
            .len(),
        fs::metadata(&gta3_path)
            .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
            .len(),
        file_loader.loaded_ide_plans.len(),
        file_loader.loaded_ipl_plans.len(),
        model_info_count,
        big_building_count,
        requests.len(),
        streaming.num_priority_requests,
        streaming.has_loaded_lods,
        streaming.num_vehicles_loaded,
        streaming.last_vehicle_deleted
    ));
    json.push_str("  \"requests\": [\n");
    for (index, stream_id) in requests.iter().enumerate() {
        let info = &streaming.info_for_model[*stream_id];
        let (kind, name, txd_slot, txd_name) = if *stream_id >= STREAM_OFFSET_TXD {
            let txd_id = *stream_id - STREAM_OFFSET_TXD;
            (
                "txd",
                file_loader
                    .txd_store
                    .get_txd_name(txd_id)
                    .unwrap_or("")
                    .to_string(),
                "null".to_string(),
                file_loader
                    .txd_store
                    .get_txd_name(txd_id)
                    .unwrap_or("")
                    .to_string(),
            )
        } else {
            let txd_slot = file_loader.txd_slot_for_model(*stream_id);
            (
                "model",
                file_loader.model_name(*stream_id).unwrap_or_default(),
                txd_slot.map_or("null".to_string(), |slot| slot.to_string()),
                txd_slot
                    .and_then(|slot| file_loader.txd_store.get_txd_name(slot))
                    .unwrap_or("")
                    .to_string(),
            )
        };
        json.push_str(&format!(
            "    {{\"index\": {}, \"stream_id\": {}, \"kind\": \"{}\", \"name\": \"{}\", \"load_state\": {}, \"flags\": {}, \"dont_remove\": {}, \"priority\": {}, \"txd_slot\": {}, \"txd_name\": \"{}\"}}",
            index,
            stream_id,
            kind,
            json_escape(&name),
            stream_load_state_id(info.load_state),
            info.flags,
            info.flags & re3bevy_game::core::Streaming::STREAMFLAGS_DONT_REMOVE != 0,
            info.flags & re3bevy_game::core::Streaming::STREAMFLAGS_PRIORITY != 0,
            txd_slot,
            json_escape(&txd_name)
        ));
        if index + 1 < requests.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    cd_stream.shutdown();
    Ok(json)
}

fn stream_load_state_id(state: StreamLoadState) -> u8 {
    match state {
        StreamLoadState::NotLoaded => 0,
        StreamLoadState::Loaded => 1,
        StreamLoadState::InQueue => 2,
        StreamLoadState::Reading => 3,
        StreamLoadState::Started => 4,
    }
}

fn parse_frontend_input(event: &str) -> FrontendTransitionInput {
    match event {
        "up" => FrontendTransitionInput::Up,
        "down" => FrontendTransitionInput::Down,
        "enter" | "select" => FrontendTransitionInput::Select,
        "back" => FrontendTransitionInput::Back,
        "reset" => FrontendTransitionInput::ResetStart,
        _ => FrontendTransitionInput::ResetStart,
    }
}

fn frontend_outcome_name(outcome: FrontendTransitionOutcome) -> &'static str {
    match outcome {
        FrontendTransitionOutcome::None => "none",
        FrontendTransitionOutcome::ChangeScreen => "change_screen",
        FrontendTransitionOutcome::PopulateSlots => "populate_slots",
        FrontendTransitionOutcome::CheckSave => "check_save",
        FrontendTransitionOutcome::BlockedEmptySlot => "blocked_empty_slot",
        FrontendTransitionOutcome::BlockedCorruptedSlot => "blocked_corrupted_slot",
        FrontendTransitionOutcome::NewGame => "new_game",
        FrontendTransitionOutcome::Exit => "exit",
    }
}

fn write_frontend_snapshot(
    json: &mut String,
    index: usize,
    event: &str,
    outcome: &str,
    menu: &re3bevy_game::core::Frontend::CMenuManager,
) {
    let snapshot = frontend_transition_snapshot(menu);
    json.push_str(&format!(
        "    {{\"index\": {}, \"event\": \"{}\", \"outcome\": \"{}\", \"curr_screen\": {}, \"prev_screen\": {}, \"curr_option\": {}, \"curr_save_slot\": {}, \"menu_active\": {}, \"game_not_loaded\": {}, \"want_to_restart\": {}, \"entry_action\": {}, \"entry_key\": \"{}\", \"entry_target\": {}, \"entry_save_slot\": {}}}",
        index,
        json_escape(event),
        outcome,
        snapshot.curr_screen,
        snapshot.prev_screen,
        snapshot.curr_option,
        snapshot.curr_save_slot,
        snapshot.menu_active,
        snapshot.game_not_loaded,
        snapshot.want_to_restart,
        snapshot.entry_action,
        json_escape(snapshot.entry_key),
        snapshot.entry_target,
        snapshot.entry_save_slot
    ));
}

fn run_frontend_main_menu_layout(config: &CaseConfig) -> String {
    let count = config.count.max(1).min(3);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/core/FrontendFlow.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Frontend.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/MenuScreens.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"screen\": 51, \"screen_width\": 640, \"screen_height\": 448, \"count\": {}}},\n",
        count
    ));
    json.push_str("  \"snapshots\": [\n");
    for selected in 0..count {
        let items = frontend_main_menu_layout_snapshot(selected as i32);
        json.push_str(&format!(
            "    {{\"selected_option\": {}, \"items\": [\n",
            selected
        ));
        for (index, item) in items.iter().enumerate() {
            write_frontend_layout_item(&mut json, item);
            if index + 1 < items.len() {
                json.push(',');
            }
            json.push('\n');
        }
        json.push_str("    ]}");
        if selected + 1 < count {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn write_frontend_layout_item(
    json: &mut String,
    item: &re3bevy_game::core::FrontendFlow::FrontendLayoutItem,
) {
    json.push_str(&format!(
        "      {{\"order\": {}, \"kind\": \"{}\", \"texture\": \"{}\", \"text_key\": \"{}\", \"entry_index\": {}, \"selected\": {}, \"left\": {}, \"top\": {}, \"right\": {}, \"bottom\": {}, \"x\": {}, \"y\": {}, \"style\": {}, \"scale_x\": {}, \"scale_y\": {}, \"align\": \"{}\", \"colour\": [{}, {}, {}, {}], \"shadow_colour\": [{}, {}, {}, {}], \"foreground_offset_x\": {}, \"foreground_offset_y\": {}}}",
        item.order,
        json_escape(item.kind),
        json_escape(item.texture),
        json_escape(item.text_key),
        item.entry_index,
        item.selected,
        item.left,
        item.top,
        item.right,
        item.bottom,
        item.x,
        item.y,
        item.style,
        item.scale_x,
        item.scale_y,
        json_escape(item.align),
        item.colour[0],
        item.colour[1],
        item.colour[2],
        item.colour[3],
        item.shadow_colour[0],
        item.shadow_colour[1],
        item.shadow_colour[2],
        item.shadow_colour[3],
        item.foreground_offset_x,
        item.foreground_offset_y
    ));
}

fn run_boot_to_frontend_state_sequence(config: &CaseConfig) -> String {
    let snapshot = boot_to_frontend_state_sequence_snapshot(false);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_core/src/core/GameState.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Game.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Movie.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/Frontend.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/FrontendFlow.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/audio/audio_enums.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/audio/soundlist.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"movies_enabled\": {}, \"language\": \"{}\", \"logo_movie\": \"{}\", \"intro_movie\": \"{}\", \"game_initialise_before_frontend\": {}, \"level_load_deferred_until\": \"{}\", \"dat_file\": \"{}\"}},\n",
        snapshot.movies_enabled,
        json_escape(snapshot.language),
        json_escape(snapshot.logo_movie),
        json_escape(snapshot.intro_movie),
        snapshot.game_initialise_before_frontend,
        json_escape(snapshot.level_load_deferred_until),
        json_escape(snapshot.dat_file)
    ));
    json.push_str("  \"states\": [\n");
    for (index, step) in snapshot.states.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"cpp_state\": \"{}\", \"rust_state\": \"{}\", \"event\": \"{}\", \"action\": \"{}\", \"movie\": \"{}\", \"next_cpp_state\": \"{}\", \"next_rust_state\": \"{}\"}}",
            step.index,
            json_escape(step.cpp_state),
            json_escape(step.rust_state),
            json_escape(step.event),
            json_escape(step.action),
            json_escape(step.movie),
            json_escape(step.next_cpp_state),
            json_escape(step.next_rust_state)
        ));
        if index + 1 < snapshot.states.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"frontend_idle_calls\": [");
    for (index, call) in snapshot.frontend_idle_calls.iter().enumerate() {
        json.push_str(&format!("\"{}\"", json_escape(call)));
        if index + 1 < snapshot.frontend_idle_calls.len() {
            json.push_str(", ");
        }
    }
    json.push_str("],\n");
    json.push_str("  \"load_all_textures_intents\": [\n");
    for (index, intent) in snapshot.load_all_textures_intents.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"order\": {}, \"intent\": \"{}\", \"value\": \"{}\", \"numeric\": {}}}",
            intent.order,
            json_escape(intent.intent),
            json_escape(intent.value),
            intent.numeric
        ));
        if index + 1 < snapshot.load_all_textures_intents.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"frontend\": {{\"curr_screen\": {}, \"prev_screen\": {}, \"curr_option\": {}, \"curr_save_slot\": {}, \"menu_active\": {}, \"game_not_loaded\": {}, \"want_to_restart\": {}, \"entry_action\": {}, \"entry_key\": \"{}\", \"entry_target\": {}, \"entry_save_slot\": {}}}\n",
        snapshot.frontend.curr_screen,
        snapshot.frontend.prev_screen,
        snapshot.frontend.curr_option,
        snapshot.frontend.curr_save_slot,
        snapshot.frontend.menu_active,
        snapshot.frontend.game_not_loaded,
        snapshot.frontend.want_to_restart,
        snapshot.frontend.entry_action,
        json_escape(snapshot.frontend.entry_key),
        snapshot.frontend.entry_target,
        snapshot.frontend.entry_save_slot
    ));
    json.push_str("}\n");
    json
}

#[derive(Clone, Copy)]
struct FontLayoutCase {
    label: &'static str,
    text: &'static str,
    style: i16,
    scale_x: f32,
    scale_y: f32,
    proportional: bool,
    justify: bool,
    centre: bool,
    right_justify: bool,
    wrap_x: f32,
    centre_size: f32,
    right_justify_wrap: f32,
    background_only_text: bool,
    alpha_fade: f32,
}

fn font_layout_cases() -> [FontLayoutCase; 4] {
    [
        FontLayoutCase {
            label: "bank_left_wrap",
            text: "PORTLAND HARBOR TAXI",
            style: FONT_BANK,
            scale_x: 0.8,
            scale_y: 1.0,
            proportional: true,
            justify: false,
            centre: false,
            right_justify: false,
            wrap_x: 360.0,
            centre_size: 640.0,
            right_justify_wrap: 0.0,
            background_only_text: false,
            alpha_fade: 255.0,
        },
        FontLayoutCase {
            label: "bank_nonprop_spaces",
            text: "ABC xyz",
            style: FONT_BANK,
            scale_x: 0.65,
            scale_y: 0.75,
            proportional: false,
            justify: false,
            centre: false,
            right_justify: false,
            wrap_x: 180.0,
            centre_size: 640.0,
            right_justify_wrap: 0.0,
            background_only_text: false,
            alpha_fade: 128.0,
        },
        FontLayoutCase {
            label: "heading_centre_rect",
            text: "LOAD GAME NOW",
            style: FONT_HEADING,
            scale_x: 0.72,
            scale_y: 0.9,
            proportional: true,
            justify: false,
            centre: true,
            right_justify: false,
            wrap_x: 640.0,
            centre_size: 130.0,
            right_justify_wrap: 0.0,
            background_only_text: true,
            alpha_fade: 255.0,
        },
        FontLayoutCase {
            label: "right_justify_wrap",
            text: "OPTIONS DISPLAY AUDIO",
            style: FONT_BANK,
            scale_x: 0.7,
            scale_y: 0.85,
            proportional: true,
            justify: false,
            centre: false,
            right_justify: true,
            wrap_x: 640.0,
            centre_size: 640.0,
            right_justify_wrap: 50.0,
            background_only_text: false,
            alpha_fade: 200.0,
        },
    ]
}

fn run_font_layout(config: &CaseConfig) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/renderer/Font.rs\"\n");
    json.push_str("  ],\n");
    json.push_str("  \"snapshots\": [\n");

    let cases = font_layout_cases();
    for (index, case) in cases.iter().enumerate() {
        let mut font = CFont::initialise();
        font.set_font_style(case.style);
        font.set_scale(case.scale_x, case.scale_y);
        font.set_wrapx(case.wrap_x);
        font.set_centre_size(case.centre_size);
        font.set_right_justify_wrap(case.right_justify_wrap);
        font.set_alpha_fade(case.alpha_fade);
        font.set_color(CRGBA::new(10, 20, 30, 240));
        font.set_drop_color(CRGBA::new(2, 4, 6, 220));
        if case.proportional {
            font.set_prop_on();
        } else {
            font.set_prop_off();
        }
        if case.justify {
            font.set_justify_on();
        }
        if case.centre {
            font.set_centre_on();
        }
        if case.right_justify {
            font.set_right_justify_on();
        }
        if case.background_only_text {
            font.set_background_only_text_on();
        }

        let text: Vec<u16> = case.text.encode_utf16().chain(std::iter::once(0)).collect();
        let rect = font.get_text_rect(100.0, 50.0, &text);
        json.push_str(&format!(
            "    {{\"index\": {}, \"label\": \"{}\", \"style\": {}, \"scale_x\": {:.6}, \"scale_y\": {:.6}, \"proportional\": {}, \"justify\": {}, \"centre\": {}, \"right_justify\": {}, \"width_no_spaces\": {:.6}, \"width_spaces\": {:.6}, \"char_a\": {:.6}, \"space\": {:.6}, \"lines\": {}, \"rect_left\": {:.6}, \"rect_top\": {:.6}, \"rect_right\": {:.6}, \"rect_bottom\": {:.6}, \"color_alpha\": {}, \"drop_alpha\": {}}}",
            index,
            case.label,
            font.details.style,
            font.details.scale_x,
            font.details.scale_y,
            font.details.proportional,
            font.details.justify,
            font.details.centre,
            font.details.right_justify,
            font.get_string_width(&text, false),
            font.get_string_width(&text, true),
            font.get_character_size(('A' as u16) - (' ' as u16)),
            font.get_character_size(0),
            font.get_number_lines(100.0, 50.0, &text),
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            font.details.color.alpha,
            font.details.drop_color.alpha
        ));
        if index + 1 < cases.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[derive(Debug, Clone, Copy)]
struct TransmissionAccelCase {
    label: &'static str,
    number_of_gears: i8,
    flags: u8,
    engine_acceleration: f32,
    max_velocity: f32,
    max_reverse_velocity: f32,
    gas_pedal: f32,
    gear: u8,
    time: f32,
    velocity: f32,
    cheat: bool,
    time_step: f32,
}

fn transmission_accel_cases() -> [TransmissionAccelCase; 8] {
    [
        TransmissionAccelCase {
            label: "first_gear_pull",
            number_of_gears: 5,
            flags: 0,
            engine_acceleration: 0.0008,
            max_velocity: 0.24,
            max_reverse_velocity: -0.2,
            gas_pedal: 1.0,
            gear: 1,
            time: 0.0,
            velocity: 0.015,
            cheat: false,
            time_step: 1.0,
        },
        TransmissionAccelCase {
            label: "upshift_resets_cheat",
            number_of_gears: 5,
            flags: 0,
            engine_acceleration: 0.0008,
            max_velocity: 0.24,
            max_reverse_velocity: -0.2,
            gas_pedal: 1.0,
            gear: 1,
            time: 0.0,
            velocity: 0.05,
            cheat: true,
            time_step: 1.0,
        },
        TransmissionAccelCase {
            label: "boosted_first_gear",
            number_of_gears: 5,
            flags: (HANDLING_1G_BOOST | HANDLING_2G_BOOST) as u8,
            engine_acceleration: 0.0008,
            max_velocity: 0.24,
            max_reverse_velocity: -0.2,
            gas_pedal: 0.75,
            gear: 1,
            time: 0.0,
            velocity: 0.01,
            cheat: false,
            time_step: 1.25,
        },
        TransmissionAccelCase {
            label: "reverse_boost",
            number_of_gears: 5,
            flags: HANDLING_2G_BOOST as u8,
            engine_acceleration: 0.0008,
            max_velocity: 0.24,
            max_reverse_velocity: -0.2,
            gas_pedal: -0.8,
            gear: 0,
            time: 0.0,
            velocity: -0.05,
            cheat: false,
            time_step: 1.0,
        },
        TransmissionAccelCase {
            label: "gear_change_delay",
            number_of_gears: 5,
            flags: 0,
            engine_acceleration: 0.0008,
            max_velocity: 0.24,
            max_reverse_velocity: -0.2,
            gas_pedal: 1.0,
            gear: 2,
            time: 0.04,
            velocity: 0.04,
            cheat: false,
            time_step: 1.5,
        },
        TransmissionAccelCase {
            label: "downshift_to_reverse",
            number_of_gears: 5,
            flags: 0,
            engine_acceleration: 0.0008,
            max_velocity: 0.24,
            max_reverse_velocity: -0.2,
            gas_pedal: -1.0,
            gear: 1,
            time: 0.0,
            velocity: -0.05,
            cheat: false,
            time_step: 1.0,
        },
        TransmissionAccelCase {
            label: "single_forward_gear",
            number_of_gears: 1,
            flags: 0,
            engine_acceleration: 0.0011,
            max_velocity: 0.16,
            max_reverse_velocity: -0.2,
            gas_pedal: 0.9,
            gear: 1,
            time: 0.0,
            velocity: 0.03,
            cheat: false,
            time_step: 1.0,
        },
        TransmissionAccelCase {
            label: "velocity_clamped_high",
            number_of_gears: 5,
            flags: 0,
            engine_acceleration: 0.0008,
            max_velocity: 0.24,
            max_reverse_velocity: -0.2,
            gas_pedal: 1.0,
            gear: 5,
            time: 0.0,
            velocity: 0.3,
            cheat: false,
            time_step: 1.0,
        },
    ]
}

fn run_transmission_accel(config: &CaseConfig) -> String {
    let cases = transmission_accel_cases();
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/vehicles/Transmission.rs\"\n");
    json.push_str("  ],\n");
    json.push_str("  \"snapshots\": [\n");

    for (index, case) in cases.iter().enumerate() {
        let mut transmission = cTransmission {
            number_of_gears: case.number_of_gears,
            flags: case.flags,
            engine_acceleration: case.engine_acceleration,
            max_velocity: case.max_velocity,
            max_cruise_velocity: case.max_velocity,
            max_reverse_velocity: case.max_reverse_velocity,
            ..cTransmission::default()
        };
        transmission.init_gear_ratios();

        let gear_before = case.gear;
        let time_before = case.time;
        let time_step_seconds = case.time_step / 50.0;
        let mut gear = case.gear;
        let mut time = case.time;
        let acceleration = transmission.calculate_drive_acceleration(
            case.gas_pedal,
            &mut gear,
            &mut time,
            case.velocity,
            case.cheat,
            case.time_step,
            time_step_seconds,
        );
        let gear_index = gear as usize;
        json.push_str(&format!(
            "    {{\"index\": {}, \"label\": \"{}\", \"number_of_gears\": {}, \"flags\": {}, \"gas_pedal\": {:.6}, \"velocity\": {:.6}, \"gear_before\": {}, \"gear_after\": {}, \"time_before\": {:.6}, \"time_after\": {:.6}, \"time_step\": {:.6}, \"time_step_seconds\": {:.6}, \"cheat\": {}, \"acceleration\": {:.9}, \"cur_velocity\": {:.6}, \"gear_max_velocity\": {:.6}, \"gear_shift_up\": {:.6}, \"gear_shift_down\": {:.6}}}",
            index,
            case.label,
            transmission.number_of_gears,
            transmission.flags,
            case.gas_pedal,
            case.velocity,
            gear_before,
            gear,
            time_before,
            time,
            case.time_step,
            time_step_seconds,
            case.cheat,
            acceleration,
            transmission.cur_velocity,
            transmission.gears[gear_index].max_velocity,
            transmission.gears[gear_index].shift_up_velocity,
            transmission.gears[gear_index].shift_down_velocity
        ));
        if index + 1 < cases.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn run_simple_model_lod(config: &CaseConfig) -> String {
    let mut model_info = CModelInfo::initialise_empty();
    {
        let model = model_info.add_simple_model(0);
        model.num_atomics = 3;
        model.first_damaged = 2;
        model.lod_distances = [40.0, 80.0, 120.0];
    }
    {
        let model = model_info.add_simple_model(1);
        model.num_atomics = 2;
        model.first_damaged = 1;
        model.lod_distances = [50.0, 100.0, 0.0];
    }
    model_info.add_simple_model(2);

    let queries = [
        ("normal_before_lod0", 0usize, 47.999_f32, 1.2_f32, false),
        ("normal_at_lod0", 0, 48.0, 1.2, false),
        ("normal_at_lod1", 0, 96.0, 1.2, false),
        ("normal_at_lod2", 0, 144.0, 1.2, false),
        ("damaged_near", 0, 0.0, 1.2, true),
        ("damaged_at_lod2", 0, 144.0, 1.2, true),
        ("normal_damage_split", 1, 59.999, 1.2, false),
        ("normal_damage_split_at", 1, 60.0, 1.2, false),
        ("damaged_split", 1, 60.0, 1.2, true),
        ("negative_distance", 0, -20.0, 0.75, false),
        ("empty_model", 2, 0.0, 1.0, false),
    ];

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/modelinfo/SimpleModelInfo.rs\"\n");
    json.push_str("  ],\n");
    json.push_str("  \"snapshots\": [\n");
    for (index, (label, model_id, distance, multiplier, damaged)) in queries.iter().enumerate() {
        let model = model_info.simple_model_info(*model_id).unwrap();
        let selected = model.atomic_index_from_distance(*distance, *multiplier, *damaged);
        let largest = model.largest_lod_distance(*multiplier, *damaged);
        json.push_str(&format!(
            "    {{\"index\": {}, \"label\": \"{}\", \"model_id\": {}, \"num_atomics\": {}, \"first_damaged\": {}, \"distance_bits\": {}, \"lod_multiplier_bits\": {}, \"damaged\": {}, \"selected_atomic\": {}, \"largest_lod_distance_bits\": {}}}",
            index,
            label,
            model_id,
            model.num_atomics,
            model.first_damaged,
            distance.to_bits(),
            multiplier.to_bits(),
            damaged,
            selected
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            largest.to_bits()
        ));
        if index + 1 < queries.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    json
}

fn run_taxi_full_throttle_90_frames(config: &CaseConfig) -> Result<String, String> {
    let text = read_asset_text(config, &config.relative_path)?;
    let handling_cfg = HandlingCfg::parse(&text);
    let mut handling_mgr = cHandlingDataMgr::new();
    handling_mgr.initialise_from_cfg(&handling_cfg)?;
    let handling_id = tVehicleType::HANDLING_TAXI as usize;
    let handling = *handling_mgr
        .get_handling_data(handling_id)
        .ok_or("missing TAXI handling data")?;
    let wheel_handling = handling_mgr
        .wheel_handling(handling_id)
        .ok_or("missing TAXI wheel handling")?;

    let mut general = CGeneral::with_seed(config.seed);
    let mut car = CAutomobile::new(
        0,
        MISSION_VEHICLE,
        0,
        &mut general,
        handling_mgr.automobile_snapshot(handling_id),
        Some((0, 0)),
        false,
    );
    car.vehicle
        .physical
        .entity
        .set_position(CVector::new(0.0, 0.0, 10.0));
    car.vehicle.physical.entity.set_heading(0.0);
    let brake_from_speed = config.name == "taxi_brake_from_speed_60_frames";
    let steer_at_speed = config.name == "taxi_steer_left_constant_speed_60_frames";
    let handbrake_turn = config.name == "taxi_handbrake_turn_60_frames";
    if brake_from_speed {
        car.vehicle.physical.move_speed = CVector::new(0.0, 0.4, 0.0);
        car.vehicle.drive.gas_pedal = 0.0;
        car.vehicle.drive.brake_pedal = 1.0;
        car.vehicle.drive.steer_angle = 0.0;
    } else if steer_at_speed {
        car.vehicle.physical.move_speed = CVector::new(0.0, 0.25, 0.0);
        car.vehicle.drive.gas_pedal = 0.0;
        car.vehicle.drive.brake_pedal = 0.0;
        car.vehicle.drive.steer_angle = 0.35;
    } else if handbrake_turn {
        car.vehicle.physical.move_speed = CVector::new(0.0, 0.25, 0.0);
        car.vehicle.drive.gas_pedal = 0.0;
        car.vehicle.drive.brake_pedal = 0.0;
        car.vehicle.drive.steer_angle = 0.35;
        car.vehicle.drive.is_handbrake_on = true;
    } else {
        car.vehicle.drive.gas_pedal = 1.0;
        car.vehicle.drive.brake_pedal = 0.0;
        car.vehicle.drive.steer_angle = 0.0;
    }
    car.suspension_spring_ratio = [0.5; NUM_CAR_WHEELS];
    car.suspension_spring_ratio_prev = [0.5; NUM_CAR_WHEELS];
    car.wheel_timer = [4.0; NUM_CAR_WHEELS];
    car.suspension_spring_length = [1.0; NUM_CAR_WHEELS];

    let time_step = 1.0;
    let time_step_seconds = time_step / 50.0;
    let frames = config.count;
    let mut transmission = handling.transmission;
    let mut wheel_state_scratch = [tWheelState::WHEEL_STATE_NORMAL; NUM_CAR_WHEELS];
    let mut already_skidding = false;
    let contact_points = taxi_contact_points(handling.dimension);
    let mut snapshots = Vec::with_capacity(frames + 1);
    snapshots.push(taxi_snapshot(0, &car, &transmission));

    for frame in 1..=frames {
        let contact_speeds =
            contact_points.map(|point| car.vehicle.physical.get_speed_at_point(point));
        let forward = car.vehicle.physical.entity.get_forward();
        let fwd_speed = dot_vec3(car.vehicle.physical.move_speed, forward);
        let drive = &mut *car.vehicle.drive;
        let acceleration = transmission.calculate_drive_acceleration(
            drive.gas_pedal,
            &mut drive.current_gear,
            &mut drive.change_gear_time,
            fwd_speed,
            false,
            time_step,
            time_step_seconds,
        ) / car.vehicle.physical.force_multiplier;
        let brake = car.vehicle.drive.brake_pedal * handling.brake_deceleration * time_step;
        let neutral = handling.flags & HANDLING_NEUTRALHANDLING != 0;
        let brake_bias_front = if neutral {
            1.0
        } else {
            2.0 * handling.brake_bias
        };
        let brake_bias_rear = if neutral {
            1.0
        } else {
            2.0 * (1.0 - handling.brake_bias)
        };
        let traction_bias_front = if neutral {
            1.0
        } else {
            2.0 * handling.traction_bias
        };
        let traction_bias_rear = if neutral {
            1.0
        } else {
            2.0 - traction_bias_front
        };
        car.wheels_on_ground = NUM_CAR_WHEELS as u8;
        car.drive_wheels_on_ground_prev = car.drive_wheels_on_ground;
        car.drive_wheels_on_ground = match handling.transmission.drive_type {
            '4' => 4,
            'F' | 'R' => 2,
            _ => 0,
        };
        let traction = 0.004 * car.traction * handling.traction_multiplier
            / 4.0
            / car.vehicle.physical.force_multiplier;

        let wheel_result = car.process_wheels(
            CAutomobileWheelProcessInput {
                contact_speeds,
                contact_points,
                adhesive_limits: [1.0; NUM_CAR_WHEELS],
                acceleration,
                brake,
                traction,
                traction_bias_front,
                traction_bias_rear,
                brake_bias_front,
                brake_bias_rear,
                front_wheel_drive: handling_mgr.has_front_wheel_drive(handling_id),
                rear_wheel_drive: handling_mgr.has_rear_wheel_drive(handling_id),
                wheel_state_scratch,
                already_skidding,
                time_step,
                handling: wheel_handling,
            },
            &mut general,
        );
        wheel_state_scratch = wheel_result.wheel_state_scratch;
        already_skidding = wheel_result.already_skidding;

        apply_taxi_wheel_rotation(&mut car, contact_speeds, 0.35, time_step);
        car.vehicle.physical.apply_turn_speed(time_step);
        car.vehicle.physical.entity.matrix.reorthogonalise();
        car.vehicle.physical.apply_move_speed(time_step);
        snapshots.push(taxi_snapshot(frame, &car, &transmission));
    }

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/vehicles/Automobile.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/vehicles/Vehicle.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/vehicles/Transmission.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/vehicles/HandlingMgr.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"setup\": {{\"model\": \"taxi\", \"handling\": \"TAXI\", \"frames\": {}, \"throttle\": {}, \"brake\": {}, \"steer\": {}, \"handbrake\": {}}},\n",
        frames, car.vehicle.drive.gas_pedal, car.vehicle.drive.brake_pedal, car.vehicle.drive.steer_angle, car.vehicle.drive.is_handbrake_on
    ));
    json.push_str("  \"snapshots\": [\n");
    for (index, snapshot) in snapshots.iter().enumerate() {
        json.push_str(snapshot);
        if index + 1 < snapshots.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn taxi_contact_points(dimension: CVector) -> [CVector; NUM_CAR_WHEELS] {
    let x = dimension.x * 0.5;
    let y = dimension.y * 0.5;
    let z = -dimension.z * 0.35;
    [
        CVector::new(-x, y, z),
        CVector::new(-x, -y, z),
        CVector::new(x, y, z),
        CVector::new(x, -y, z),
    ]
}

fn apply_taxi_wheel_rotation(
    car: &mut CAutomobile,
    contact_speeds: [CVector; NUM_CAR_WHEELS],
    radius: f32,
    time_step: f32,
) {
    let (sin_steer, cos_steer) = car.vehicle.drive.steer_angle.sin_cos();
    let front_fwd = car
        .vehicle
        .physical
        .entity
        .transform_vector(CVector::new(-sin_steer, cos_steer, 0.0));
    let rear_fwd = car.vehicle.physical.entity.get_forward();
    for wheel in 0..NUM_CAR_WHEELS {
        let fwd = if wheel == CARWHEEL_FRONT_LEFT || wheel == CARWHEEL_FRONT_RIGHT {
            front_fwd
        } else {
            rear_fwd
        };
        car.wheel_speed[wheel] = CVehicle::process_wheel_rotation(
            car.wheel_state[wheel],
            fwd,
            contact_speeds[wheel],
            radius,
            time_step,
        );
        car.wheel_rotation[wheel] += car.wheel_speed[wheel];
    }
}

fn taxi_snapshot(frame: usize, car: &CAutomobile, transmission: &cTransmission) -> String {
    let pos = car.vehicle.physical.entity.get_position();
    let right = car.vehicle.physical.entity.get_right();
    let forward = car.vehicle.physical.entity.get_forward();
    let up = car.vehicle.physical.entity.get_up();
    let move_speed = car.vehicle.physical.move_speed;
    let turn_speed = car.vehicle.physical.turn_speed;
    format!(
        "    {{\"frame\": {}, \"position\": [{:.9}, {:.9}, {:.9}], \"right\": [{:.9}, {:.9}, {:.9}], \"forward\": [{:.9}, {:.9}, {:.9}], \"up\": [{:.9}, {:.9}, {:.9}], \"move_speed\": [{:.9}, {:.9}, {:.9}], \"turn_speed\": [{:.9}, {:.9}, {:.9}], \"gear\": {}, \"engine_revs\": {:.9}, \"steer_angle\": {:.9}, \"wheel_rotation\": [{:.9}, {:.9}, {:.9}, {:.9}], \"wheel_suspension\": [{:.9}, {:.9}, {:.9}, {:.9}], \"health\": {:.6}, \"damage\": {{\"engine\": {}, \"wheel\": [{}, {}, {}, {}]}}}}",
        frame,
        pos.x,
        pos.y,
        pos.z,
        right.x,
        right.y,
        right.z,
        forward.x,
        forward.y,
        forward.z,
        up.x,
        up.y,
        up.z,
        move_speed.x,
        move_speed.y,
        move_speed.z,
        turn_speed.x,
        turn_speed.y,
        turn_speed.z,
        car.vehicle.drive.current_gear,
        transmission.cur_velocity,
        car.vehicle.drive.steer_angle,
        car.wheel_rotation[0],
        car.wheel_rotation[1],
        car.wheel_rotation[2],
        car.wheel_rotation[3],
        car.suspension_spring_ratio[0],
        car.suspension_spring_ratio[1],
        car.suspension_spring_ratio[2],
        car.suspension_spring_ratio[3],
        car.vehicle.vitality.health,
        car.damage.engine_status,
        car.damage.get_wheel_status(0),
        car.damage.get_wheel_status(1),
        car.damage.get_wheel_status(2),
        car.damage.get_wheel_status(3)
    )
}

fn run_handling_cfg(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let text = fs::read_to_string(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let parsed = HandlingCfg::parse(&text);
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/DataRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"handling_cfg_size\": {}}},\n",
        size
    ));
    json.push_str("  \"records\": [\n");

    for (index, record) in parsed.records.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"name\": \"{}\", \"field_count\": {}, \"fields\": [",
            index,
            json_escape(&record.name),
            record.fields.len()
        ));
        for (field_index, field) in record.fields.iter().enumerate() {
            json.push_str(&format!("\"{}\"", json_escape(field)));
            if field_index + 1 < record.fields.len() {
                json.push_str(", ");
            }
        }
        json.push_str("]}");
        if index + 1 < parsed.records.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_carcols_dat(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let text = fs::read_to_string(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let parsed = CarcolsDat::parse(&text);
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();
    let pair_count: usize = parsed.cars.iter().map(|car| car.pairs.len()).sum();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/DataRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"carcols_dat_size\": {}}},\n",
        size
    ));
    json.push_str(&format!(
        "  \"summary\": {{\"colour_count\": {}, \"car_count\": {}, \"colour_pair_count\": {}}},\n",
        parsed.colours.len(),
        parsed.cars.len(),
        pair_count
    ));
    json.push_str("  \"colours\": [\n");
    for (index, colour) in parsed.colours.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"red\": {}, \"green\": {}, \"blue\": {}, \"alpha\": {}}}",
            index, colour.red, colour.green, colour.blue, colour.alpha
        ));
        if index + 1 < parsed.colours.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"cars\": [\n");
    for (index, car) in parsed.cars.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"name\": \"{}\", \"pair_count\": {}, \"pairs\": [",
            index,
            json_escape(&car.name),
            car.pairs.len()
        ));
        for (pair_index, (primary, secondary)) in car.pairs.iter().enumerate() {
            json.push_str(&format!("[{}, {}]", primary, secondary));
            if pair_index + 1 < car.pairs.len() {
                json.push_str(", ");
            }
        }
        json.push_str("]}");
        if index + 1 < parsed.cars.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_pedgrp_dat(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let text = fs::read_to_string(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let parsed = PedgrpDat::parse(&text);
    let model_ids = collect_model_id_entries(&gta_root, &default_path, &gta3_path)?;
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();
    let unresolved_count = parsed
        .groups
        .iter()
        .flat_map(|group| group.models.iter())
        .filter(|name| lookup_model_id(&model_ids, name).is_none())
        .count();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/DataRead.rs\",\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/FileLoader.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"pedgrp_dat_size\": {}, \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"model_name_count\": {}}},\n",
        size,
        fs::metadata(&default_path)
            .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
            .len(),
        fs::metadata(&gta3_path)
            .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
            .len(),
        model_ids.len()
    ));
    json.push_str(&format!(
        "  \"summary\": {{\"group_count\": {}, \"models_per_group\": 8, \"unresolved_model_count\": {}}},\n",
        parsed.groups.len(),
        unresolved_count
    ));
    json.push_str("  \"groups\": [\n");
    for (group_index, group) in parsed.groups.iter().enumerate() {
        json.push_str(&format!("    {{\"index\": {}, \"models\": [", group_index));
        for (slot, name) in group.models.iter().enumerate() {
            let model_id = lookup_model_id(&model_ids, name).unwrap_or(-1);
            json.push_str(&format!(
                "{{\"slot\": {}, \"name\": \"{}\", \"model_id\": {}}}",
                slot,
                json_escape(name),
                model_id
            ));
            if slot + 1 < group.models.len() {
                json.push_str(", ");
            }
        }
        json.push_str("]}");
        if group_index + 1 < parsed.groups.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_scm_raw_script_space(config: &CaseConfig) -> Result<String, String> {
    let path = asset_path(config, &config.relative_path)?;
    let bytes =
        fs::read(&path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let scm = ScmFile::parse_main_scm(&bytes);
    let script_space_tail_zero = scm.script_space[scm.loaded_main_bytes..]
        .iter()
        .filter(|byte| **byte == 0)
        .count();
    let object_table_offset = (scm.variable_space_size.max(0) as usize) + 8;
    let mission_header_offset = (scm.multiscript.object_size.max(0) as usize) + 8;

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/Script.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/control/Script.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"main_scm_size\": {}, \"loaded_main_bytes\": {}, \"script_space_len\": {}, \"main_checksum\": {}, \"script_space_checksum\": {}, \"script_space_tail_zero_bytes\": {}}},\n",
        bytes.len(),
        scm.loaded_main_bytes,
        scm.script_space.len(),
        byte_checksum(&scm.script_space[..scm.loaded_main_bytes]),
        byte_checksum(&scm.script_space),
        script_space_tail_zero
    ));
    json.push_str(&format!(
        "  \"header\": {{\"variable_space_size\": {}, \"object_table_offset\": {}, \"used_object_count\": {}, \"object_size\": {}, \"mission_header_offset\": {}, \"main_script_size\": {}, \"largest_mission_script_size\": {}, \"number_of_mission_scripts\": {}}},\n",
        scm.variable_space_size,
        object_table_offset,
        scm.used_objects.len(),
        scm.multiscript.object_size,
        mission_header_offset,
        scm.multiscript.main_script_size,
        scm.multiscript.largest_mission_script_size,
        scm.multiscript.number_of_mission_scripts
    ));
    json.push_str("  \"used_objects\": [\n");
    for (index, object) in scm.used_objects.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"model_index\": {}, \"name\": \"{}\", \"raw_hex\": \"{}\"}}",
            index,
            object.index,
            json_escape(&script_name_string(&object.name)),
            bytes_hex(&object.name)
        ));
        if index + 1 < scm.used_objects.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"mission_offsets\": [");
    for (index, offset) in scm.multiscript.offsets.iter().enumerate() {
        json.push_str(&offset.to_string());
        if index + 1 < scm.multiscript.offsets.len() {
            json.push_str(", ");
        }
    }
    json.push_str("],\n");
    json.push_str("  \"word_probe\": [\n");
    let headers = scan_instruction_headers(&scm.script_space, 16);
    for (index, header) in headers.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"offset\": {}, \"opcode\": {}, \"not_flag\": {}}}",
            header.offset, header.opcode, header.not_flag
        ));
        if index + 1 < headers.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn byte_checksum(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .enumerate()
        .map(|(index, byte)| (index as u64 + 1) * (*byte as u64 + 1))
        .sum()
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn script_name_string(bytes: &[u8; USED_OBJECT_NAME_LENGTH]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(USED_OBJECT_NAME_LENGTH);
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

fn collect_model_id_entries(
    gta_root: &Path,
    default_path: &Path,
    gta3_path: &Path,
) -> Result<Vec<(String, usize)>, String> {
    let default_text = fs::read_to_string(default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;
    let mut ide_paths = collect_ide_paths_from_dat(&default_text);
    ide_paths.extend(collect_ide_paths_from_dat(&gta3_text));
    let mut entries = Vec::new();

    for relative_path in ide_paths {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let text = fs::read_to_string(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let plan = GtaIdePlan::parse(&text);
        for record in plan.records {
            match record {
                GtaIdeRecord::Object(object) => entries.push((object.model, object.id)),
                GtaIdeRecord::TimeObject(object) => {
                    entries.push((object.object.model, object.object.id))
                }
                GtaIdeRecord::Clump(clump) => entries.push((clump.model, clump.id)),
                GtaIdeRecord::Vehicle(vehicle) => entries.push((vehicle.model, vehicle.id)),
                GtaIdeRecord::Ped(ped) => entries.push((ped.model, ped.id)),
                _ => {}
            }
        }
    }

    entries.sort_by_key(|(_, id)| *id);
    Ok(entries)
}

fn lookup_model_id(entries: &[(String, usize)], name: &str) -> Option<i32> {
    entries
        .iter()
        .find(|(model_name, _)| model_name.eq_ignore_ascii_case(name))
        .map(|(_, id)| *id as i32)
}

fn read_asset_text(config: &CaseConfig, relative_path: &str) -> Result<String, String> {
    let path = asset_path(config, relative_path)?;
    fs::read_to_string(&path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn asset_path(config: &CaseConfig, relative_path: &str) -> Result<PathBuf, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    Ok(PathBuf::from(root_text.trim()).join(relative_path.replace('/', "\\")))
}

fn dot_vec3(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn run_dat_default_gta3(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;
    let default_plan = GtaDatPlan::parse(&default_text);
    let gta3_plan = GtaDatPlan::parse(&gta3_text);
    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/FileLoader.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}}},\n",
        default_size, gta3_size
    ));
    json.push_str("  \"files\": [\n");
    push_dat_file_json(
        &mut json,
        "default",
        &config.default_relative_path,
        &default_plan.commands,
    );
    json.push_str(",\n");
    push_dat_file_json(
        &mut json,
        "gta3",
        &config.gta3_relative_path,
        &gta3_plan.commands,
    );
    json.push_str("\n  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn push_dat_file_json(json: &mut String, role: &str, path: &str, commands: &[GtaDatCommand]) {
    json.push_str(&format!(
        "    {{\"role\": \"{}\", \"path\": \"{}\", \"commands\": [\n",
        json_escape(role),
        json_escape(path)
    ));
    for (index, command) in commands.iter().enumerate() {
        let (kind, level, path) = dat_command_parts(command);
        json.push_str(&format!(
            "      {{\"index\": {}, \"kind\": \"{}\", \"level\": {}, \"path\": \"{}\"}}",
            index,
            kind,
            level.map_or("null".to_string(), |value| value.to_string()),
            json_escape(path.unwrap_or(""))
        ));
        if index + 1 < commands.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("    ]}");
}

fn dat_command_parts(command: &GtaDatCommand) -> (&'static str, Option<i32>, Option<&str>) {
    match command {
        GtaDatCommand::ImagePath(path) => ("IMAGEPATH", None, Some(path)),
        GtaDatCommand::TexDictionary(path) => ("TEXDICTION", None, Some(path)),
        GtaDatCommand::ColFile { level, path } => ("COLFILE", Some(*level), Some(path)),
        GtaDatCommand::ModelFile(path) => ("MODELFILE", None, Some(path)),
        GtaDatCommand::HierFile(path) => ("HIERFILE", None, Some(path)),
        GtaDatCommand::Ide(path) => ("IDE", None, Some(path)),
        GtaDatCommand::Ipl(path) => ("IPL", None, Some(path)),
        GtaDatCommand::MapZone(path) => ("MAPZONE", None, Some(path)),
        GtaDatCommand::Splash => ("SPLASH", None, None),
        GtaDatCommand::CdImage(path) => ("CDIMAGE", None, Some(path)),
        GtaDatCommand::Exit => ("EXIT", None, None),
        GtaDatCommand::Unknown(line) => ("UNKNOWN", None, Some(line)),
    }
}

fn run_ide_modelinfo(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut ide_paths = collect_ide_paths_from_dat(&default_text);
    ide_paths.extend(collect_ide_paths_from_dat(&gta3_text));

    let mut files = Vec::new();
    let mut records = Vec::new();
    for (file_index, relative_path) in ide_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let text = fs::read_to_string(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let before = records.len();
        let plan = GtaIdePlan::parse(&text);
        for (local_index, record) in plan.records.iter().enumerate() {
            records.push((
                file_index,
                relative_path.clone(),
                local_index,
                record.clone(),
            ));
        }
        let size = fs::metadata(&asset_path)
            .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
            .len();
        files.push((
            file_index,
            relative_path.clone(),
            size,
            records.len() - before,
        ));
    }

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/FileLoader.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"ide_file_count\": {}, \"record_count\": {}}},\n",
        fs::metadata(&default_path)
            .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
            .len(),
        fs::metadata(&gta3_path)
            .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
            .len(),
        files.len(),
        records.len()
    ));
    json.push_str("  \"files\": [\n");
    for (index, (file_index, path, size, record_count)) in files.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"path\": \"{}\", \"size\": {}, \"record_count\": {}}}",
            file_index,
            json_escape(path),
            size,
            record_count
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"records\": [\n");
    for (index, (file_index, file_path, local_index, record)) in records.iter().enumerate() {
        push_ide_record_json(
            &mut json,
            index,
            *file_index,
            file_path,
            *local_index,
            record,
        );
        if index + 1 < records.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn collect_ide_paths_from_dat(text: &str) -> Vec<String> {
    GtaDatPlan::parse(text)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            GtaDatCommand::Ide(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn push_ide_record_json(
    json: &mut String,
    index: usize,
    file_index: usize,
    file_path: &str,
    local_index: usize,
    record: &GtaIdeRecord,
) {
    let kind = ide_record_kind(record);
    json.push_str(&format!(
        "    {{\"index\": {}, \"file_index\": {}, \"file_path\": \"{}\", \"local_index\": {}, \"kind\": \"{}\"",
        index,
        file_index,
        json_escape(file_path),
        local_index,
        kind
    ));

    match record {
        GtaIdeRecord::Object(object) => {
            push_ide_object_fields(json, object, false);
        }
        GtaIdeRecord::TimeObject(object) => {
            push_ide_object_fields(json, &object.object, false);
            json.push_str(&format!(
                ", \"time_on\": {}, \"time_off\": {}",
                object.time_on, object.time_off
            ));
        }
        GtaIdeRecord::Clump(clump) => {
            json.push_str(&format!(
                ", \"id\": {}, \"model\": \"{}\", \"txd\": \"{}\"",
                clump.id,
                json_escape(&clump.model),
                json_escape(&clump.txd)
            ));
        }
        GtaIdeRecord::Vehicle(vehicle) => {
            json.push_str(&format!(
                ", \"id\": {}, \"model\": \"{}\", \"txd\": \"{}\", \"vehicle_type\": \"{}\", \"handling_id\": \"{}\", \"game_name\": \"{}\", \"vehicle_class\": \"{}\", \"frequency\": {}, \"level\": {}, \"component_rules\": {}, \"misc\": {}, \"wheel_scale\": {}",
                vehicle.id,
                json_escape(&vehicle.model),
                json_escape(&vehicle.txd),
                json_escape(&vehicle.vehicle_type),
                json_escape(&vehicle.handling_id),
                json_escape(&vehicle.game_name),
                json_escape(&vehicle.vehicle_class),
                vehicle.frequency,
                vehicle.level,
                vehicle.component_rules,
                vehicle.misc,
                vehicle.wheel_scale
            ));
        }
        GtaIdeRecord::Ped(ped) => {
            json.push_str(&format!(
                ", \"id\": {}, \"model\": \"{}\", \"txd\": \"{}\", \"ped_type\": \"{}\", \"ped_stats\": \"{}\", \"anim_group\": \"{}\", \"cars_can_drive\": {}",
                ped.id,
                json_escape(&ped.model),
                json_escape(&ped.txd),
                json_escape(&ped.ped_type),
                json_escape(&ped.ped_stats),
                json_escape(&ped.anim_group),
                ped.cars_can_drive
            ));
        }
        GtaIdeRecord::PathHeader(header) => {
            json.push_str(&format!(
                ", \"id\": {}, \"path_type\": \"{}\", \"model_name\": \"{}\"",
                header.id,
                json_escape(&header.path_type),
                json_escape(&header.model_name)
            ));
        }
        GtaIdeRecord::PathNode(node) => {
            json.push_str(&format!(
                ", \"node_type\": {}, \"next\": {}, \"cross\": {}, \"position\": [{}, {}, {}], \"width\": {}, \"num_left\": {}, \"num_right\": {}",
                node.node_type,
                node.next,
                node.cross,
                node.position[0],
                node.position[1],
                node.position[2],
                node.width,
                node.num_left.map_or("null".to_string(), |value| value.to_string()),
                node.num_right.map_or("null".to_string(), |value| value.to_string())
            ));
        }
        GtaIdeRecord::TwoDfx(effect) => {
            json.push_str(&format!(
                ", \"id\": {}, \"position\": [{}, {}, {}], \"color\": [{}, {}, {}, {}], \"effect_type\": {}",
                effect.id,
                effect.position[0],
                effect.position[1],
                effect.position[2],
                effect.color[0],
                effect.color[1],
                effect.color[2],
                effect.color[3],
                effect.effect_type
            ));
        }
        GtaIdeRecord::MloHeader(header) => {
            json.push_str(&format!(
                ", \"id\": {}, \"model\": \"{}\", \"draw_dist\": {}",
                header.id,
                json_escape(&header.name),
                header.draw_dist
            ));
        }
        GtaIdeRecord::MloInstance(instance) => {
            json.push_str(&format!(
                ", \"id\": {}, \"model\": \"{}\", \"position\": [{}, {}, {}]",
                instance.mlo_id,
                json_escape(&instance.name),
                instance.position[0],
                instance.position[1],
                instance.position[2]
            ));
        }
        GtaIdeRecord::Unknown(line) => {
            json.push_str(&format!(", \"line\": \"{}\"", json_escape(line)));
        }
    }
    json.push('}');
}

fn push_ide_object_fields(
    json: &mut String,
    object: &re3bevy_formats::formats::FileLoader::GtaIdeObject,
    include_kind_specific: bool,
) {
    let _ = include_kind_specific;
    json.push_str(&format!(
        ", \"id\": {}, \"model\": \"{}\", \"txd\": \"{}\", \"num_objs\": {}, \"distances\": [{}, {}, {}], \"flags\": {}, \"first_damaged\": {}",
        object.id,
        json_escape(&object.model),
        json_escape(&object.txd),
        object.num_objs,
        object.distances[0],
        object.distances[1],
        object.distances[2],
        object.flags,
        object.first_damaged
    ));
}

fn ide_record_kind(record: &GtaIdeRecord) -> &'static str {
    match record {
        GtaIdeRecord::Object(_) => "object",
        GtaIdeRecord::TimeObject(_) => "time_object",
        GtaIdeRecord::Clump(_) => "clump",
        GtaIdeRecord::Vehicle(_) => "vehicle",
        GtaIdeRecord::Ped(_) => "ped",
        GtaIdeRecord::MloHeader(_) => "mlo_header",
        GtaIdeRecord::MloInstance(_) => "mlo_instance",
        GtaIdeRecord::PathHeader(_) => "path_header",
        GtaIdeRecord::PathNode(_) => "path_node",
        GtaIdeRecord::TwoDfx(_) => "2dfx",
        GtaIdeRecord::Unknown(_) => "unknown",
    }
}

fn run_ipl_world_instances(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut ipl_paths = collect_ipl_paths_from_dat(&default_text);
    ipl_paths.extend(collect_ipl_paths_from_dat(&gta3_text));

    let mut files = Vec::new();
    let mut records = Vec::new();
    for (file_index, relative_path) in ipl_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let text = fs::read_to_string(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let before = records.len();
        let plan = GtaIplPlan::parse(&text);
        let mut local_index = 0usize;
        for record in plan.records {
            if ipl_record_is_world_surface(&record) {
                records.push((file_index, relative_path.clone(), local_index, record));
                local_index += 1;
            }
        }
        let size = fs::metadata(&asset_path)
            .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
            .len();
        files.push((
            file_index,
            relative_path.clone(),
            size,
            records.len() - before,
        ));
    }

    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/FileLoader.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"ipl_file_count\": {}, \"record_count\": {}}},\n",
        default_size,
        gta3_size,
        files.len(),
        records.len()
    ));
    json.push_str("  \"files\": [\n");
    for (index, (file_index, path, size, record_count)) in files.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"path\": \"{}\", \"size\": {}, \"record_count\": {}}}",
            file_index,
            json_escape(path),
            size,
            record_count
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"records\": [\n");
    for (index, (file_index, file_path, local_index, record)) in records.iter().enumerate() {
        push_ipl_record_json(
            &mut json,
            index,
            *file_index,
            file_path,
            *local_index,
            record,
        );
        if index + 1 < records.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_mapzone(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut mapzone_paths = collect_mapzone_paths_from_dat(&default_text);
    mapzone_paths.extend(collect_mapzone_paths_from_dat(&gta3_text));

    let mut zones = CTheZones::init();
    let mut files = Vec::new();
    let mut source_records = Vec::new();
    for (file_index, relative_path) in mapzone_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let text = fs::read_to_string(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let plan = GtaIplPlan::parse(&text);
        let mut zone_count = 0usize;
        for record in plan.records {
            if let GtaIplRecord::Zone(zone) = record {
                let local_index = zone_count;
                zones.create_map_zone(&zone);
                source_records.push((file_index, relative_path.clone(), local_index, zone));
                zone_count += 1;
            }
        }
        let size = fs::metadata(&asset_path)
            .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
            .len();
        files.push((file_index, relative_path.clone(), size, zone_count));
    }

    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/FileLoader.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/World.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"mapzone_file_count\": {}, \"source_zone_count\": {}, \"runtime_mapzone_count\": {}}},\n",
        default_size,
        gta3_size,
        files.len(),
        source_records.len(),
        zones.map_zones.len()
    ));
    json.push_str("  \"files\": [\n");
    for (index, (file_index, path, size, zone_count)) in files.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"path\": \"{}\", \"size\": {}, \"zone_count\": {}}}",
            file_index,
            json_escape(path),
            size,
            zone_count
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"source_zones\": [\n");
    for (index, (file_index, file_path, local_index, zone)) in source_records.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"file_index\": {}, \"file_path\": \"{}\", \"local_index\": {}, \"name\": \"{}\", \"zone_type\": {}, \"min\": [{}, {}, {}], \"max\": [{}, {}, {}], \"level\": {}}}",
            index,
            file_index,
            json_escape(file_path),
            local_index,
            json_escape(&zone.name),
            zone.zone_type,
            zone.min[0],
            zone.min[1],
            zone.min[2],
            zone.max[0],
            zone.max[1],
            zone.max[2],
            zone.level
        ));
        if index + 1 < source_records.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"runtime_map_zones\": [\n");
    for (index, zone) in zones.map_zones.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"name\": \"{}\", \"zone_type\": {}, \"min\": [{}, {}, {}], \"max\": [{}, {}, {}], \"level\": {}}}",
            index,
            json_escape(&zone.name),
            zone.zone_type,
            zone.min[0],
            zone.min[1],
            zone.min[2],
            zone.max[0],
            zone.max[1],
            zone.max[2],
            zone.level
        ));
        if index + 1 < zones.map_zones.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn collect_ipl_paths_from_dat(text: &str) -> Vec<String> {
    GtaDatPlan::parse(text)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            GtaDatCommand::Ipl(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn collect_mapzone_paths_from_dat(text: &str) -> Vec<String> {
    GtaDatPlan::parse(text)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            GtaDatCommand::MapZone(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn collect_col_paths_from_dat(text: &str) -> Vec<(i32, String)> {
    GtaDatPlan::parse(text)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            GtaDatCommand::ColFile { level, path } => Some((level, path)),
            _ => None,
        })
        .collect()
}

fn ipl_record_is_world_surface(record: &GtaIplRecord) -> bool {
    matches!(
        record,
        GtaIplRecord::Instance(_)
            | GtaIplRecord::Zone(_)
            | GtaIplRecord::Cull(_)
            | GtaIplRecord::Pickup(_)
    )
}

fn push_ipl_record_json(
    json: &mut String,
    index: usize,
    file_index: usize,
    file_path: &str,
    local_index: usize,
    record: &GtaIplRecord,
) {
    let kind = match record {
        GtaIplRecord::Instance(_) => "instance",
        GtaIplRecord::Zone(_) => "zone",
        GtaIplRecord::Cull(_) => "cull",
        GtaIplRecord::Pickup(_) => "pickup",
        GtaIplRecord::PathHeader(_) => "path_header",
        GtaIplRecord::PathNode(_) => "path_node",
        GtaIplRecord::Unknown(_) => "unknown",
    };
    json.push_str(&format!(
        "    {{\"index\": {}, \"file_index\": {}, \"file_path\": \"{}\", \"local_index\": {}, \"kind\": \"{}\"",
        index,
        file_index,
        json_escape(file_path),
        local_index,
        kind
    ));

    match record {
        GtaIplRecord::Instance(instance) => {
            json.push_str(&format!(
                ", \"id\": {}, \"name\": \"{}\", \"position\": [{}, {}, {}], \"scale\": [{}, {}, {}], \"rotation_axis\": [{}, {}, {}], \"angle\": {}",
                instance.id,
                json_escape(&instance.name),
                instance.position[0],
                instance.position[1],
                instance.position[2],
                instance.scale[0],
                instance.scale[1],
                instance.scale[2],
                instance.rotation_axis[0],
                instance.rotation_axis[1],
                instance.rotation_axis[2],
                instance.angle
            ));
        }
        GtaIplRecord::Zone(zone) => {
            json.push_str(&format!(
                ", \"name\": \"{}\", \"zone_type\": {}, \"min\": [{}, {}, {}], \"max\": [{}, {}, {}], \"level\": {}",
                json_escape(&zone.name),
                zone.zone_type,
                zone.min[0],
                zone.min[1],
                zone.min[2],
                zone.max[0],
                zone.max[1],
                zone.max[2],
                zone.level
            ));
        }
        GtaIplRecord::Cull(cull) => {
            json.push_str(&format!(
                ", \"position\": [{}, {}, {}], \"min\": [{}, {}, {}], \"max\": [{}, {}, {}], \"flags\": {}, \"wanted_level_drop\": {}",
                cull.position[0],
                cull.position[1],
                cull.position[2],
                cull.min[0],
                cull.min[1],
                cull.min[2],
                cull.max[0],
                cull.max[1],
                cull.max[2],
                cull.flags,
                cull.wanted_level_drop
            ));
        }
        GtaIplRecord::Pickup(pickup) => {
            json.push_str(&format!(
                ", \"id\": {}, \"position\": [{}, {}, {}]",
                pickup.id, pickup.position[0], pickup.position[1], pickup.position[2]
            ));
        }
        GtaIplRecord::PathHeader(_) | GtaIplRecord::PathNode(_) | GtaIplRecord::Unknown(_) => {}
    }
    json.push('}');
}

fn run_txd_metadata(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut txd_paths = collect_txd_paths_from_dat(&default_text);
    txd_paths.extend(collect_txd_paths_from_dat(&gta3_text));

    let mut files = Vec::new();
    let mut total_textures = 0usize;
    for (file_index, relative_path) in txd_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let bytes = fs::read(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let summary = read_txd(&bytes)
            .ok_or_else(|| format!("failed to parse TXD {}", asset_path.display()))?;
        total_textures += summary.textures.len();
        let size = fs::metadata(&asset_path)
            .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
            .len();
        files.push((file_index, relative_path.clone(), size, summary));
    }

    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/TexRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"txd_file_count\": {}, \"texture_count\": {}}},\n",
        default_size,
        gta3_size,
        files.len(),
        total_textures
    ));
    json.push_str("  \"files\": [\n");
    for (index, (file_index, path, size, summary)) in files.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"path\": \"{}\", \"size\": {}, \"num_textures\": {}, \"first_phase_textures\": {}, \"texture_native_chunks\": {}}}",
            file_index,
            json_escape(path),
            size,
            summary.num_textures,
            summary.first_phase_textures,
            summary.texture_native_chunks
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"textures\": [\n");
    let mut written = 0usize;
    for (file_index, _, _, summary) in &files {
        push_txd_textures_json(
            &mut json,
            *file_index,
            summary,
            &mut written,
            total_textures,
        );
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn collect_txd_paths_from_dat(text: &str) -> Vec<String> {
    GtaDatPlan::parse(text)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            GtaDatCommand::TexDictionary(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn push_txd_textures_json(
    json: &mut String,
    file_index: usize,
    summary: &RwTexDictionarySummary,
    written: &mut usize,
    total_textures: usize,
) {
    for (texture_index, texture) in summary.textures.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"file_index\": {}, \"texture_index\": {}, \"platform_id\": {}, \"chunk_version\": {}, \"chunk_length\": {}, \"filter_addressing\": {}, \"name\": \"{}\", \"mask\": \"{}\", \"raster_format\": {}, \"d3d_format\": {}, \"has_alpha\": {}, \"width\": {}, \"height\": {}, \"depth\": {}, \"num_levels\": {}, \"raster_type\": {}, \"compression_or_flags\": {}, \"palette_len\": {}, \"mip_level_sizes\": [",
            texture_index,
            file_index,
            texture_index,
            texture.platform_id,
            texture.chunk_version,
            texture.chunk_length,
            texture.filter_addressing,
            json_escape(&texture.name),
            json_escape(&texture.mask),
            texture.raster_format,
            texture.d3d_format.map_or("null".to_string(), |value| value.to_string()),
            texture.has_alpha,
            texture.width,
            texture.height,
            texture.depth,
            texture.num_levels,
            texture.raster_type,
            texture.compression_or_flags,
            texture.palette.len()
        ));
        for (index, size) in texture.mip_level_sizes.iter().enumerate() {
            json.push_str(&size.to_string());
            if index + 1 < texture.mip_level_sizes.len() {
                json.push_str(", ");
            }
        }
        json.push_str("]}");
        *written += 1;
        if *written < total_textures {
            json.push(',');
        }
        json.push('\n');
    }
}

fn run_txd_raster_decode_stats(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut txd_paths = collect_txd_paths_from_dat(&default_text);
    txd_paths.extend(collect_txd_paths_from_dat(&gta3_text));

    let mut files = Vec::new();
    let mut total_textures = 0usize;
    let mut decoded_textures = 0usize;
    for (file_index, relative_path) in txd_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let bytes = fs::read(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let summary = read_txd(&bytes)
            .ok_or_else(|| format!("failed to parse TXD {}", asset_path.display()))?;
        total_textures += summary.textures.len();
        decoded_textures += summary
            .textures
            .iter()
            .filter(|texture| decode_txd_texture(texture).is_some())
            .count();
        let size = fs::metadata(&asset_path)
            .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
            .len();
        files.push((file_index, relative_path.clone(), size, summary));
    }

    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/TexRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"txd_file_count\": {}, \"texture_count\": {}, \"decoded_texture_count\": {}}},\n",
        default_size,
        gta3_size,
        files.len(),
        total_textures,
        decoded_textures
    ));
    json.push_str("  \"files\": [\n");
    for (index, (file_index, path, size, summary)) in files.iter().enumerate() {
        let file_decoded = summary
            .textures
            .iter()
            .filter(|texture| decode_txd_texture(texture).is_some())
            .count();
        json.push_str(&format!(
            "    {{\"index\": {}, \"path\": \"{}\", \"size\": {}, \"num_textures\": {}, \"decoded_textures\": {}}}",
            file_index,
            json_escape(path),
            size,
            summary.textures.len(),
            file_decoded
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"textures\": [\n");
    let mut written = 0usize;
    for (file_index, _, _, summary) in &files {
        for (texture_index, texture) in summary.textures.iter().enumerate() {
            push_txd_decode_stats_json(&mut json, *file_index, texture_index, texture);
            written += 1;
            if written < total_textures {
                json.push(',');
            }
            json.push('\n');
        }
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn push_txd_decode_stats_json(
    json: &mut String,
    file_index: usize,
    texture_index: usize,
    texture: &RwTextureNativeDescriptor,
) {
    let first_mip = texture
        .mip_level_payloads
        .first()
        .map_or(&[][..], Vec::as_slice);
    let decoded = decode_txd_texture(texture);
    let (decoded_flag, rgba_len, rgba_checksum, alpha_zero_count, alpha_nonzero_count, head, tail) =
        if let Some(decoded) = decoded {
            let alpha_zero = decoded
                .rgba
                .chunks_exact(4)
                .filter(|pixel| pixel[3] == 0)
                .count();
            let alpha_nonzero = decoded
                .rgba
                .chunks_exact(4)
                .filter(|pixel| pixel[3] != 0)
                .count();
            (
                true,
                decoded.rgba.len(),
                byte_checksum(&decoded.rgba),
                alpha_zero,
                alpha_nonzero,
                bytes_hex(&decoded.rgba[..decoded.rgba.len().min(16)]),
                bytes_hex(&decoded.rgba[decoded.rgba.len().saturating_sub(16)..]),
            )
        } else {
            (false, 0, 0, 0, 0, String::new(), String::new())
        };

    json.push_str(&format!(
        "    {{\"index\": {}, \"file_index\": {}, \"texture_index\": {}, \"platform_id\": {}, \"name\": \"{}\", \"mask\": \"{}\", \"raster_format\": {}, \"d3d_format\": {}, \"width\": {}, \"height\": {}, \"depth\": {}, \"num_levels\": {}, \"palette_len\": {}, \"source_mip_size\": {}, \"source_mip_checksum\": {}, \"decoded\": {}, \"rgba_len\": {}, \"rgba_checksum\": {}, \"alpha_zero_count\": {}, \"alpha_nonzero_count\": {}, \"rgba_head\": \"{}\", \"rgba_tail\": \"{}\"}}",
        texture_index,
        file_index,
        texture_index,
        texture.platform_id,
        json_escape(&texture.name),
        json_escape(&texture.mask),
        texture.raster_format,
        texture.d3d_format.map_or("null".to_string(), |value| value.to_string()),
        texture.width,
        texture.height,
        texture.depth,
        texture.num_levels,
        texture.palette.len(),
        first_mip.len(),
        byte_checksum(first_mip),
        decoded_flag,
        rgba_len,
        rgba_checksum,
        alpha_zero_count,
        alpha_nonzero_count,
        head,
        tail
    ));
}

fn run_dff_geometry_metadata(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut dff_paths = collect_model_paths_from_dat(&default_text);
    dff_paths.extend(collect_model_paths_from_dat(&gta3_text));

    let mut files = Vec::new();
    let mut total_geometries = 0usize;
    let mut total_atomics = 0usize;
    let mut total_meshes = 0usize;
    for (file_index, relative_path) in dff_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let bytes = fs::read(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let summary = read_clump(&bytes)
            .ok_or_else(|| format!("failed to parse DFF {}", asset_path.display()))?;
        total_geometries += summary.geometries.len();
        total_atomics += summary.atomics.len();
        total_meshes += summary.bevy_meshes.len();
        let size = fs::metadata(&asset_path)
            .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
            .len();
        files.push((file_index, relative_path.clone(), size, summary));
    }

    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/ClumpRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"dff_file_count\": {}, \"geometry_count\": {}, \"atomic_count\": {}, \"mesh_count\": {}}},\n",
        default_size,
        gta3_size,
        files.len(),
        total_geometries,
        total_atomics,
        total_meshes
    ));
    json.push_str("  \"files\": [\n");
    for (index, (file_index, path, size, summary)) in files.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"path\": \"{}\", \"size\": {}, \"num_atomics\": {}, \"num_lights\": {}, \"num_cameras\": {}, \"num_frames\": {}, \"num_geometries\": {}, \"first_phase_geometries\": {}, \"atomic_chunks\": {}, \"root_frame_name\": {}, \"mesh_count\": {}}}",
            file_index,
            json_escape(path),
            size,
            summary.num_atomics,
            summary.num_lights,
            summary.num_cameras,
            summary.num_frames,
            summary.num_geometries,
            summary.first_phase_geometries,
            summary.atomic_chunks,
            json_optional_string(summary.root_frame_name.as_deref()),
            summary.bevy_meshes.len()
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"geometries\": [\n");
    let mut written_geometries = 0usize;
    for (file_index, _, _, summary) in &files {
        push_dff_geometries_json(
            &mut json,
            *file_index,
            summary,
            &mut written_geometries,
            total_geometries,
        );
    }
    json.push_str("  ],\n");
    json.push_str("  \"atomics\": [\n");
    let mut written_atomics = 0usize;
    for (file_index, _, _, summary) in &files {
        push_dff_atomics_json(
            &mut json,
            *file_index,
            summary,
            &mut written_atomics,
            total_atomics,
        );
    }
    json.push_str("  ],\n");
    json.push_str("  \"meshes\": [\n");
    let mut written_meshes = 0usize;
    for (file_index, _, _, summary) in &files {
        push_dff_meshes_json(
            &mut json,
            *file_index,
            summary,
            &mut written_meshes,
            total_meshes,
        );
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_dff_clump_metadata(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut dff_paths = collect_model_paths_from_dat(&default_text);
    dff_paths.extend(collect_model_paths_from_dat(&gta3_text));

    let mut files = Vec::new();
    let mut total_frames = 0usize;
    let mut total_atomics = 0usize;
    let mut named_frames = 0usize;
    let mut root_frames = 0usize;
    for (file_index, relative_path) in dff_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let bytes = fs::read(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let summary = read_clump(&bytes)
            .ok_or_else(|| format!("failed to parse DFF {}", asset_path.display()))?;
        total_frames += summary.frames.len();
        total_atomics += summary.atomics.len();
        named_frames += summary
            .frames
            .iter()
            .filter(|frame| frame.name.is_some())
            .count();
        root_frames += summary
            .frames
            .iter()
            .filter(|frame| frame.parent_index < 0)
            .count();
        let size = fs::metadata(&asset_path)
            .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
            .len();
        files.push((file_index, relative_path.clone(), size, summary));
    }

    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/ClumpRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"dff_file_count\": {}, \"frame_count\": {}, \"named_frame_count\": {}, \"root_frame_count\": {}, \"atomic_count\": {}}},\n",
        default_size,
        gta3_size,
        files.len(),
        total_frames,
        named_frames,
        root_frames,
        total_atomics
    ));
    json.push_str("  \"files\": [\n");
    for (index, (file_index, path, size, summary)) in files.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"path\": \"{}\", \"size\": {}, \"num_atomics\": {}, \"num_lights\": {}, \"num_cameras\": {}, \"num_frames\": {}, \"parsed_frame_count\": {}, \"num_geometries\": {}, \"atomic_chunks\": {}, \"root_frame_name\": {}}}",
            file_index,
            json_escape(path),
            size,
            summary.num_atomics,
            summary.num_lights,
            summary.num_cameras,
            summary.num_frames,
            summary.frames.len(),
            summary.num_geometries,
            summary.atomic_chunks,
            json_optional_string(summary.root_frame_name.as_deref())
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"frames\": [\n");
    let mut written_frames = 0usize;
    for (file_index, _, _, summary) in &files {
        for (frame_index, frame) in summary.frames.iter().enumerate() {
            let child_count = summary
                .frames
                .iter()
                .filter(|candidate| candidate.parent_index == frame_index as i32)
                .count();
            let atomic_count = summary
                .atomics
                .iter()
                .filter(|atomic| atomic.frame_index == frame_index as i32)
                .count();
            json.push_str(&format!(
                "    {{\"index\": {}, \"file_index\": {}, \"frame_index\": {}, \"parent_index\": {}, \"matrix_flags\": {}, \"name\": {}, \"right_bits\": [{}, {}, {}], \"up_bits\": [{}, {}, {}], \"at_bits\": [{}, {}, {}], \"position_bits\": [{}, {}, {}], \"child_count\": {}, \"atomic_count\": {}}}",
                written_frames,
                file_index,
                frame_index,
                frame.parent_index,
                frame.matrix_flags,
                json_optional_string(frame.name.as_deref()),
                frame.right[0].to_bits(),
                frame.right[1].to_bits(),
                frame.right[2].to_bits(),
                frame.up[0].to_bits(),
                frame.up[1].to_bits(),
                frame.up[2].to_bits(),
                frame.at[0].to_bits(),
                frame.at[1].to_bits(),
                frame.at[2].to_bits(),
                frame.position[0].to_bits(),
                frame.position[1].to_bits(),
                frame.position[2].to_bits(),
                child_count,
                atomic_count
            ));
            written_frames += 1;
            if written_frames < total_frames {
                json.push(',');
            }
            json.push('\n');
        }
    }
    json.push_str("  ],\n");
    json.push_str("  \"atomics\": [\n");
    let mut written_atomics = 0usize;
    for (file_index, _, _, summary) in &files {
        push_dff_atomics_json(
            &mut json,
            *file_index,
            summary,
            &mut written_atomics,
            total_atomics,
        );
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_model_file_atomic_binding(config: &CaseConfig) -> Result<String, String> {
    let install = OriginalGtaInstall::from_config_path(&config.asset_root_config)
        .map_err(|err| format!("failed to read GTA install config: {err}"))?;
    let mut file_mgr = install.file_mgr();
    let mut cd_stream = CdStream::init(2);
    let mut file_loader = CFileLoader::new();
    let dat_file = config.default_relative_path.replace('/', "\\");
    file_loader
        .load_level_plan(&install, &mut file_mgr, &mut cd_stream, &dat_file)
        .map_err(|err| format!("failed to load {dat_file}: {err}"))?;
    cd_stream.shutdown();

    let default_path = install.resolve_existing_case_path(&dat_file);
    let mut records = Vec::new();
    let mut missing_count = 0usize;
    for (file_index, loaded) in file_loader.loaded_model_files.iter().enumerate() {
        missing_count += loaded.missing_atomics.len();
        for bound in &loaded.bound_atomics {
            let simple = file_loader
                .model_info
                .simple_model_info(bound.model_id)
                .ok_or_else(|| format!("bound model {} is not simple", bound.model_id))?;
            let binding = simple
                .atomic_bindings
                .iter()
                .find(|binding| binding.lod_index == bound.lod_index)
                .ok_or_else(|| {
                    format!(
                        "missing runtime binding for model {} lod {}",
                        bound.model_id, bound.lod_index
                    )
                })?;
            let txd_slot = usize::try_from(simple.base.txd_slot)
                .map_err(|_| format!("model {} has no TXD slot", bound.model_id))?;
            let txd_name = file_loader
                .txd_store
                .get_txd_name(txd_slot)
                .unwrap_or_default();
            records.push((
                file_index,
                loaded.filename.as_str(),
                bound,
                binding,
                txd_name,
                file_loader.txd_store.get_num_refs(txd_slot),
                simple.base.ref_count,
                simple.base.rw_object_loaded,
            ));
        }
    }

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/core/FileLoader.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/modelinfo/ModelInfo.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"model_file_count\": {}, \"binding_count\": {}, \"missing_count\": {}}},\n",
        fs::metadata(&default_path)
            .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
            .len(),
        file_loader.loaded_model_files.len(),
        records.len(),
        missing_count
    ));
    json.push_str("  \"bindings\": [\n");
    for (index, (file_index, file_path, bound, binding, txd_name, txd_refs, model_refs, loaded)) in
        records.iter().enumerate()
    {
        json.push_str(&format!(
            "    {{\"index\": {}, \"file_index\": {}, \"file_path\": \"{}\", \"model_id\": {}, \"model_name\": \"{}\", \"txd_name\": \"{}\", \"lod_index\": {}, \"frame_name\": \"{}\", \"geometry_index\": {}, \"num_vertices\": {}, \"num_triangles\": {}, \"model_ref_count\": {}, \"txd_ref_count\": {}, \"rw_object_loaded\": {}}}",
            index,
            file_index,
            json_escape(file_path),
            bound.model_id,
            json_escape(&bound.model_name),
            json_escape(txd_name),
            bound.lod_index,
            json_escape(&bound.frame_name),
            binding.atomic.geometry_index,
            binding.geometry.num_vertices,
            binding.geometry.num_triangles,
            model_refs,
            txd_refs,
            loaded
        ));
        if index + 1 < records.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

struct ColFileSummary {
    file_index: usize,
    level: i32,
    path: String,
    size: u64,
    checksum: u64,
    col: GtaColFile,
}

fn run_col_model_metadata(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let default_path = gta_root.join(config.default_relative_path.replace('/', "\\"));
    let gta3_path = gta_root.join(config.gta3_relative_path.replace('/', "\\"));
    let default_text = fs::read_to_string(&default_path)
        .map_err(|err| format!("failed to read {}: {err}", default_path.display()))?;
    let gta3_text = fs::read_to_string(&gta3_path)
        .map_err(|err| format!("failed to read {}: {err}", gta3_path.display()))?;

    let mut col_paths = collect_col_paths_from_dat(&default_text);
    col_paths.extend(collect_col_paths_from_dat(&gta3_text));

    let mut files = Vec::new();
    let mut total_models = 0usize;
    let mut total_bytes = 0u64;
    for (file_index, (level, relative_path)) in col_paths.iter().enumerate() {
        let asset_path = gta_root.join(relative_path.replace('/', "\\"));
        let bytes = fs::read(&asset_path)
            .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
        let col = read_col_file(&bytes)
            .map_err(|err| format!("failed to parse COL {}: {err}", asset_path.display()))?;
        let size = bytes.len() as u64;
        total_models += col.models.len();
        total_bytes += size;
        files.push(ColFileSummary {
            file_index,
            level: *level,
            path: relative_path.clone(),
            size,
            checksum: byte_checksum(&bytes),
            col,
        });
    }

    let default_size = fs::metadata(&default_path)
        .map_err(|err| format!("failed to stat {}: {err}", default_path.display()))?
        .len();
    let gta3_size = fs::metadata(&gta3_path)
        .map_err(|err| format!("failed to stat {}: {err}", gta3_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/ColRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"default_dat_size\": {}, \"gta3_dat_size\": {}, \"col_file_count\": {}, \"col_total_bytes\": {}, \"model_count\": {}}},\n",
        default_size,
        gta3_size,
        files.len(),
        total_bytes,
        total_models
    ));
    json.push_str("  \"files\": [\n");
    for (index, file) in files.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"level\": {}, \"path\": \"{}\", \"size\": {}, \"checksum\": {}, \"model_count\": {}}}",
            file.file_index,
            file.level,
            json_escape(&file.path),
            file.size,
            file.checksum,
            file.col.models.len()
        ));
        if index + 1 < files.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"models\": [\n");
    let mut written_models = 0usize;
    for file in &files {
        for (local_index, named_model) in file.col.models.iter().enumerate() {
            let model = &named_model.model;
            json.push_str(&format!(
                "    {{\"index\": {}, \"file_index\": {}, \"file_path\": \"{}\", \"level\": {}, \"local_index\": {}, \"name\": \"{}\", \"bounds\": {{\"sphere_radius_bits\": {}, \"sphere_center_bits\": [{}, {}, {}], \"box_min_bits\": [{}, {}, {}], \"box_max_bits\": [{}, {}, {}]}}, \"counts\": {{\"spheres\": {}, \"lines\": {}, \"boxes\": {}, \"vertices\": {}, \"triangles\": {}}}, \"checksums\": {{\"spheres\": {}, \"lines\": {}, \"boxes\": {}, \"vertices\": {}, \"triangles\": {}}}}}",
                written_models,
                file.file_index,
                json_escape(&file.path),
                file.level,
                local_index,
                json_escape(&named_model.name),
                f32_bits(model.bounding_sphere.radius),
                f32_bits(model.bounding_sphere.center.x),
                f32_bits(model.bounding_sphere.center.y),
                f32_bits(model.bounding_sphere.center.z),
                f32_bits(model.bounding_box.min.x),
                f32_bits(model.bounding_box.min.y),
                f32_bits(model.bounding_box.min.z),
                f32_bits(model.bounding_box.max.x),
                f32_bits(model.bounding_box.max.y),
                f32_bits(model.bounding_box.max.z),
                model.spheres.len(),
                model.lines.len(),
                model.boxes.len(),
                model.vertices.len(),
                model.triangles.len(),
                col_sphere_checksum(&model.spheres),
                col_line_checksum(&model.lines),
                col_box_checksum(&model.boxes),
                col_vertex_checksum(&model.vertices),
                col_triangle_checksum(&model.triangles)
            ));
            written_models += 1;
            if written_models < total_models {
                json.push(',');
            }
            json.push('\n');
        }
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_ifp_anim_metadata(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let bytes = fs::read(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let ifp = IfpFile::parse_with_error(&bytes)
        .map_err(|err| format!("failed to parse IFP {}: {err}", asset_path.display()))?;

    let mut animation_count = 0usize;
    let mut sequence_count = 0usize;
    let mut frame_count = 0usize;
    let mut translated_sequence_count = 0usize;
    for block in &ifp.blocks {
        animation_count += block.animations.len();
        for animation in &block.animations {
            sequence_count += animation.sequences.len();
            for sequence in &animation.sequences {
                frame_count += sequence.frames.len();
                if ifp_sequence_has_translation(sequence) {
                    translated_sequence_count += 1;
                }
            }
        }
    }

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/AnimManager.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"ifp_size\": {}, \"checksum\": {}, \"block_count\": {}, \"animation_count\": {}, \"sequence_count\": {}, \"frame_count\": {}, \"translated_sequence_count\": {}}},\n",
        bytes.len(),
        byte_checksum(&bytes),
        ifp.blocks.len(),
        animation_count,
        sequence_count,
        frame_count,
        translated_sequence_count
    ));
    json.push_str("  \"blocks\": [\n");
    let mut first_animation_index = 0usize;
    for (block_index, block) in ifp.blocks.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"name\": \"{}\", \"first_animation_index\": {}, \"animation_count\": {}}}",
            block_index,
            json_escape(&block.name),
            first_animation_index,
            block.animations.len()
        ));
        first_animation_index += block.animations.len();
        if block_index + 1 < ifp.blocks.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"animations\": [\n");
    let mut animation_index = 0usize;
    for (block_index, block) in ifp.blocks.iter().enumerate() {
        for (local_index, animation) in block.animations.iter().enumerate() {
            let anim_frame_count: usize = animation
                .sequences
                .iter()
                .map(|sequence| sequence.frames.len())
                .sum();
            let anim_translated_sequences = animation
                .sequences
                .iter()
                .filter(|sequence| ifp_sequence_has_translation(sequence))
                .count();
            json.push_str(&format!(
                "    {{\"index\": {}, \"block_index\": {}, \"local_index\": {}, \"name\": \"{}\", \"sequence_count\": {}, \"frame_count\": {}, \"translated_sequence_count\": {}, \"total_length_bits\": {}, \"sequence_name_checksum\": {}, \"keyframe_checksum\": {}, \"compressed_checksum\": {}}}",
                animation_index,
                block_index,
                local_index,
                json_escape(&animation.name),
                animation.sequences.len(),
                anim_frame_count,
                anim_translated_sequences,
                f32_bits(ifp_animation_total_length(&animation.sequences)),
                ifp_sequence_name_checksum(&animation.sequences),
                ifp_animation_keyframe_checksum(&animation.sequences),
                ifp_animation_compressed_checksum(&animation.sequences)
            ));
            animation_index += 1;
            if animation_index < animation_count {
                json.push(',');
            }
            json.push('\n');
        }
    }
    json.push_str("  ],\n");
    json.push_str("  \"sequences\": [\n");
    let mut global_sequence_index = 0usize;
    animation_index = 0;
    for (block_index, block) in ifp.blocks.iter().enumerate() {
        for animation in &block.animations {
            for (local_index, sequence) in animation.sequences.iter().enumerate() {
                json.push_str(&format!(
                    "    {{\"index\": {}, \"animation_index\": {}, \"block_index\": {}, \"local_index\": {}, \"name\": \"{}\", \"bone_tag\": {}, \"frame_count\": {}, \"has_translation\": {}, \"sequence_time_bits\": {}, \"keyframe_checksum\": {}, \"compressed_checksum\": {}}}",
                    global_sequence_index,
                    animation_index,
                    block_index,
                    local_index,
                    json_escape(&sequence.name),
                    json_optional_i32(sequence.bone_tag),
                    sequence.frames.len(),
                    ifp_sequence_has_translation(sequence),
                    f32_bits(ifp_sequence_time(sequence)),
                    ifp_sequence_keyframe_checksum(sequence),
                    ifp_sequence_compressed_checksum(sequence)
                ));
                global_sequence_index += 1;
                if global_sequence_index < sequence_count {
                    json.push(',');
                }
                json.push('\n');
            }
            animation_index += 1;
        }
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn collect_model_paths_from_dat(text: &str) -> Vec<String> {
    GtaDatPlan::parse(text)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            GtaDatCommand::ModelFile(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn f32_bits(value: f32) -> u32 {
    value.to_bits()
}

fn json_optional_i32(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn ifp_sequence_has_translation(sequence: &IfpSequence) -> bool {
    sequence
        .frames
        .iter()
        .any(|frame| frame.translation.is_some())
}

fn ifp_sequence_time(sequence: &IfpSequence) -> f32 {
    sequence
        .frames
        .iter()
        .fold(0.0f32, |total, frame| total + frame.delta_time)
}

fn ifp_animation_total_length(sequences: &[IfpSequence]) -> f32 {
    sequences
        .iter()
        .map(ifp_sequence_time)
        .fold(0.0f32, |longest, sequence_time| {
            if sequence_time > longest {
                sequence_time
            } else {
                longest
            }
        })
}

fn ifp_sequence_name_checksum(sequences: &[IfpSequence]) -> u64 {
    sequences
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, sequence)| {
            acc.wrapping_add(string_checksum(&sequence.name).wrapping_mul(index as u64 + 1))
        })
}

fn ifp_animation_keyframe_checksum(sequences: &[IfpSequence]) -> u64 {
    sequences
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, sequence)| {
            acc.wrapping_add(
                ifp_sequence_keyframe_checksum(sequence).wrapping_mul(index as u64 + 1),
            )
        })
}

fn ifp_animation_compressed_checksum(sequences: &[IfpSequence]) -> u64 {
    sequences
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, sequence)| {
            acc.wrapping_add(
                ifp_sequence_compressed_checksum(sequence).wrapping_mul(index as u64 + 1),
            )
        })
}

fn ifp_sequence_keyframe_checksum(sequence: &IfpSequence) -> u64 {
    sequence
        .frames
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, frame)| {
            let weight = index as u64 + 1;
            acc.wrapping_add(ifp_frame_bits_checksum(frame).wrapping_mul(weight))
        })
}

fn ifp_frame_bits_checksum(frame: &IfpKeyFrame) -> u64 {
    let mut sum = 0u64;
    for (index, component) in frame.rotation.iter().enumerate() {
        sum = sum.wrapping_add((component.to_bits() as u64).wrapping_mul(67 + index as u64));
    }
    if let Some(translation) = frame.translation {
        for (index, component) in translation.iter().enumerate() {
            sum = sum.wrapping_add((component.to_bits() as u64).wrapping_mul(73 + index as u64));
        }
    }
    sum.wrapping_add((frame.delta_time.to_bits() as u64).wrapping_mul(79))
}

fn ifp_sequence_compressed_checksum(sequence: &IfpSequence) -> u64 {
    sequence
        .frames
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, frame)| {
            let weight = index as u64 + 1;
            acc.wrapping_add(ifp_frame_compressed_checksum(frame).wrapping_mul(weight))
        })
}

fn ifp_frame_compressed_checksum(frame: &IfpKeyFrame) -> u64 {
    let mut sum = 0u64;
    for (index, component) in frame.rotation.iter().enumerate() {
        let compressed = (*component * 4096.0) as i16;
        sum = sum.wrapping_add(signed_i16_to_positive(compressed).wrapping_mul(83 + index as u64));
    }
    let delta = (frame.delta_time * 60.0 + 0.5) as i16;
    sum = sum.wrapping_add(signed_i16_to_positive(delta).wrapping_mul(89));
    if let Some(translation) = frame.translation {
        for (index, component) in translation.iter().enumerate() {
            let compressed = (*component * 128.0) as i16;
            sum = sum
                .wrapping_add(signed_i16_to_positive(compressed).wrapping_mul(97 + index as u64));
        }
    }
    sum
}

fn string_checksum(value: &str) -> u64 {
    value.bytes().enumerate().fold(0u64, |acc, (index, byte)| {
        acc.wrapping_add((index as u64 + 1).wrapping_mul(byte as u64 + 1))
    })
}

fn col_sphere_checksum(values: &[GtaColSphere]) -> u64 {
    values
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, sphere)| {
            acc.wrapping_add(col_vec_checksum(sphere.center, index as u64 + 1))
                .wrapping_add((sphere.radius.to_bits() as u64).wrapping_mul(17))
                .wrapping_add((sphere.surface as u64).wrapping_mul(19))
                .wrapping_add((sphere.piece as u64).wrapping_mul(23))
        })
}

fn col_line_checksum(values: &[GtaColLine]) -> u64 {
    values.iter().enumerate().fold(0u64, |acc, (index, line)| {
        acc.wrapping_add(col_vec_checksum(line.p0, index as u64 + 3))
            .wrapping_add(col_vec_checksum(line.p1, index as u64 + 5))
    })
}

fn col_box_checksum(values: &[GtaColBox]) -> u64 {
    values.iter().enumerate().fold(0u64, |acc, (index, item)| {
        acc.wrapping_add(col_vec_checksum(item.min, index as u64 + 7))
            .wrapping_add(col_vec_checksum(item.max, index as u64 + 11))
            .wrapping_add((item.surface as u64).wrapping_mul(29))
            .wrapping_add((item.piece as u64).wrapping_mul(31))
    })
}

fn col_vertex_checksum(values: &[GtaCompressedVector]) -> u64 {
    values
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, vertex)| {
            let weight = index as u64 + 1;
            acc.wrapping_add(signed_i16_to_positive(vertex.x).wrapping_mul(37 * weight))
                .wrapping_add(signed_i16_to_positive(vertex.y).wrapping_mul(41 * weight))
                .wrapping_add(signed_i16_to_positive(vertex.z).wrapping_mul(43 * weight))
        })
}

fn col_triangle_checksum(values: &[GtaColTriangle]) -> u64 {
    values
        .iter()
        .enumerate()
        .fold(0u64, |acc, (index, triangle)| {
            let weight = index as u64 + 1;
            acc.wrapping_add((triangle.a as u64).wrapping_mul(47 * weight))
                .wrapping_add((triangle.b as u64).wrapping_mul(53 * weight))
                .wrapping_add((triangle.c as u64).wrapping_mul(59 * weight))
                .wrapping_add((triangle.surface as u64).wrapping_mul(61 * weight))
        })
}

fn col_vec_checksum(value: re3bevy_formats::formats::ColRead::GtaColVector, salt: u64) -> u64 {
    (value.x.to_bits() as u64)
        .wrapping_mul(3 * salt)
        .wrapping_add((value.y.to_bits() as u64).wrapping_mul(5 * salt))
        .wrapping_add((value.z.to_bits() as u64).wrapping_mul(7 * salt))
}

fn signed_i16_to_positive(value: i16) -> u64 {
    (i64::from(value) + 32769) as u64
}

fn json_optional_string(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn push_dff_geometries_json(
    json: &mut String,
    file_index: usize,
    summary: &RwClumpSummary,
    written: &mut usize,
    total_geometries: usize,
) {
    for (geometry_index, geometry) in summary.geometries.iter().enumerate() {
        let (
            decoded,
            position_count,
            normal_count,
            prelit_color_count,
            tex_coord_set_count,
            index_count,
            triangle_material_count,
            has_bounding_sphere,
        ) = geometry
            .decoded
            .as_ref()
            .map(|decoded| {
                (
                    true,
                    decoded.positions.len(),
                    decoded.normals.len(),
                    decoded.prelit_colors.len(),
                    decoded.tex_coords.len(),
                    decoded.indices.len(),
                    decoded.triangle_materials.len(),
                    decoded.bounding_sphere.is_some(),
                )
            })
            .unwrap_or((false, 0, 0, 0, 0, 0, 0, false));

        json.push_str(&format!(
            "    {{\"index\": {}, \"file_index\": {}, \"geometry_index\": {}, \"flags\": {}, \"raw_flags\": {}, \"num_triangles\": {}, \"num_vertices\": {}, \"num_morph_targets\": {}, \"num_materials\": {}, \"num_tex_coord_sets\": {}, \"chunk_version\": {}, \"chunk_length\": {}, \"decoded\": {}, \"position_count\": {}, \"normal_count\": {}, \"prelit_color_count\": {}, \"tex_coord_set_count\": {}, \"index_count\": {}, \"triangle_material_count\": {}, \"has_bounding_sphere\": {}}}",
            *written,
            file_index,
            geometry_index,
            geometry.flags,
            geometry.raw_flags,
            geometry.num_triangles,
            geometry.num_vertices,
            geometry.num_morph_targets,
            geometry.num_materials,
            geometry.num_tex_coord_sets,
            geometry.chunk_version,
            geometry.chunk_length,
            decoded,
            position_count,
            normal_count,
            prelit_color_count,
            tex_coord_set_count,
            index_count,
            triangle_material_count,
            has_bounding_sphere
        ));
        *written += 1;
        if *written < total_geometries {
            json.push(',');
        }
        json.push('\n');
    }
}

fn push_dff_atomics_json(
    json: &mut String,
    file_index: usize,
    summary: &RwClumpSummary,
    written: &mut usize,
    total_atomics: usize,
) {
    for (atomic_index, atomic) in summary.atomics.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"file_index\": {}, \"atomic_index\": {}, \"frame_index\": {}, \"geometry_index\": {}, \"flags\": {}, \"frame_name\": {}}}",
            *written,
            file_index,
            atomic_index,
            atomic.frame_index,
            atomic.geometry_index,
            atomic.flags,
            json_optional_string(atomic.frame_name.as_deref())
        ));
        *written += 1;
        if *written < total_atomics {
            json.push(',');
        }
        json.push('\n');
    }
}

fn push_dff_meshes_json(
    json: &mut String,
    file_index: usize,
    summary: &RwClumpSummary,
    written: &mut usize,
    total_meshes: usize,
) {
    for mesh in &summary.bevy_meshes {
        json.push_str(&format!(
            "    {{\"index\": {}, \"file_index\": {}, \"geometry_index\": {}, \"vertex_count\": {}, \"triangle_count\": {}, \"index_count\": {}, \"material_slots\": {}, \"has_positions\": {}, \"has_normals\": {}, \"has_prelit_colors\": {}, \"tex_coord_sets\": {}}}",
            *written,
            file_index,
            mesh.geometry_index,
            mesh.vertex_count,
            mesh.triangle_count,
            mesh.index_count,
            mesh.material_slots,
            mesh.has_positions,
            mesh.has_normals,
            mesh.has_prelit_colors,
            mesh.tex_coord_sets
        ));
        *written += 1;
        if *written < total_meshes {
            json.push(',');
        }
        json.push('\n');
    }
}

fn run_weapon_dat(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let text = fs::read_to_string(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let parsed = WeaponDat::parse(&text);
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/DataRead.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"weapon_dat_size\": {}}},\n",
        size
    ));
    json.push_str("  \"records\": [\n");

    for (index, record) in parsed.records.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"weapon_name\": \"{}\", \"fire_type\": \"{}\", \"range\": {}, \"firing_rate\": {}, \"reload\": {}, \"ammo_amount\": {}, \"damage\": {}, \"speed\": {}, \"radius\": {}, \"life_span\": {}, \"spread\": {}, \"fire_offset\": [{}, {}, {}], \"anim_to_play\": \"{}\", \"anim2_to_play\": \"{}\", \"anim_loop_start\": {}, \"anim_loop_end\": {}, \"delay_between_anim_and_fire\": {}, \"delay_between_anim2_and_fire\": {}, \"model_id\": {}, \"flags\": {}}}",
            index,
            json_escape(&record.weapon_name),
            json_escape(&record.fire_type),
            record.range,
            record.firing_rate,
            record.reload,
            record.ammo_amount,
            record.damage,
            record.speed,
            record.radius,
            record.life_span,
            record.spread,
            record.fire_offset[0],
            record.fire_offset[1],
            record.fire_offset[2],
            json_escape(&record.anim_to_play),
            json_escape(&record.anim2_to_play),
            record.anim_loop_start,
            record.anim_loop_end,
            record.delay_between_anim_and_fire,
            record.delay_between_anim2_and_fire,
            record.model_id,
            record.flags
        ));
        if index + 1 < parsed.records.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_surface_dat(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let text = fs::read_to_string(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let parsed = SurfaceDat::parse(&text)?;
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();

    let mut matrix = [[0.0f32; 5]; 5];
    for (row_index, row) in parsed.rows.iter().enumerate().take(5) {
        for (field_index, value) in row.adhesive_limits.iter().enumerate().take(row_index + 1) {
            let adhesive_limit = value.unwrap_or(0.0);
            matrix[row_index][field_index] = adhesive_limit;
            matrix[field_index][row_index] = adhesive_limit;
        }
    }

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/DataRead.rs\",\n");
    json.push_str("    \"crates/re3bevy_game/src/core/SurfaceTable.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"surface_dat_size\": {}}},\n",
        size
    ));
    json.push_str("  \"rows\": [\n");
    for (index, row) in parsed.rows.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"name\": \"{}\"}}",
            index,
            json_escape(&row.name)
        ));
        if index + 1 < parsed.rows.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"matrix\": [\n");
    for (row_index, row) in matrix.iter().enumerate() {
        json.push_str("    [");
        for (col_index, value) in row.iter().enumerate() {
            json.push_str(&value.to_string());
            if col_index + 1 < row.len() {
                json.push_str(", ");
            }
        }
        json.push(']');
        if row_index + 1 < matrix.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

#[derive(Debug, Clone)]
struct TimecycRecord {
    ambient: [i32; 3],
    directional: [i32; 3],
    sky_top: [i32; 3],
    sky_bottom: [i32; 3],
    sun_core: [i32; 3],
    sun_corona: [i32; 3],
    sun_size: f32,
    sprite_size: f32,
    sprite_brightness: f32,
    shadow_strength: i32,
    light_shadow_strength: i32,
    tree_shadow_strength: i32,
    far_clip: f32,
    fog_start: f32,
    lights_on_ground_brightness: f32,
    low_clouds: [i32; 3],
    fluffy_clouds_top: [i32; 3],
    fluffy_clouds_bottom: [i32; 3],
    blur: [f32; 4],
}

fn run_timecyc_dat(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let text = fs::read_to_string(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let records = parse_timecyc_records(&text)?;
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/renderer/Timecycle.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"timecyc_dat_size\": {}}},\n",
        size
    ));
    json.push_str("  \"records\": [\n");

    for (index, record) in records.iter().enumerate() {
        let weather = index / 24;
        let hour = index % 24;
        json.push_str(&format!(
            "    {{\"index\": {}, \"weather\": {}, \"hour\": {}, \"ambient\": [{}, {}, {}], \"directional\": [{}, {}, {}], \"sky_top\": [{}, {}, {}], \"sky_bottom\": [{}, {}, {}], \"sun_core\": [{}, {}, {}], \"sun_corona\": [{}, {}, {}], \"sun_size\": {}, \"sprite_size\": {}, \"sprite_brightness\": {}, \"shadow_strength\": {}, \"light_shadow_strength\": {}, \"tree_shadow_strength\": {}, \"far_clip\": {}, \"fog_start\": {}, \"lights_on_ground_brightness\": {}, \"low_clouds\": [{}, {}, {}], \"fluffy_clouds_top\": [{}, {}, {}], \"fluffy_clouds_bottom\": [{}, {}, {}], \"blur\": [{}, {}, {}, {}]}}",
            index,
            weather,
            hour,
            record.ambient[0],
            record.ambient[1],
            record.ambient[2],
            record.directional[0],
            record.directional[1],
            record.directional[2],
            record.sky_top[0],
            record.sky_top[1],
            record.sky_top[2],
            record.sky_bottom[0],
            record.sky_bottom[1],
            record.sky_bottom[2],
            record.sun_core[0],
            record.sun_core[1],
            record.sun_core[2],
            record.sun_corona[0],
            record.sun_corona[1],
            record.sun_corona[2],
            record.sun_size,
            record.sprite_size,
            record.sprite_brightness,
            record.shadow_strength,
            record.light_shadow_strength,
            record.tree_shadow_strength,
            record.far_clip,
            record.fog_start,
            record.lights_on_ground_brightness,
            record.low_clouds[0],
            record.low_clouds[1],
            record.low_clouds[2],
            record.fluffy_clouds_top[0],
            record.fluffy_clouds_top[1],
            record.fluffy_clouds_top[2],
            record.fluffy_clouds_bottom[0],
            record.fluffy_clouds_bottom[1],
            record.fluffy_clouds_bottom[2],
            record.blur[0],
            record.blur[1],
            record.blur[2],
            record.blur[3]
        ));
        if index + 1 < records.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_waterpro_dat(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let bytes = fs::read(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let water = CWaterLevel::from_waterpro_bytes(&bytes)
        .map_err(|err| format!("failed to parse {}: {err}", asset_path.display()))?;
    let active_count = water.no_of_water_levels.clamp(0, 48) as usize;
    let block_non_water = water
        .water_block_list
        .iter()
        .filter(|block| **block >= 0)
        .count();
    let fine_non_water = water
        .water_fine_block_list
        .iter()
        .filter(|block| **block >= 0)
        .count();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_game/src/renderer/WaterLevel.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"waterpro_size\": {}, \"expected_layout_size\": {}}},\n",
        bytes.len(),
        waterpro_expected_layout_size()
    ));
    json.push_str(&format!(
        "  \"summary\": {{\"no_of_water_levels\": {}, \"water_z_slots\": {}, \"rect_slots\": {}, \"block_list_len\": {}, \"fine_block_list_len\": {}, \"non_water_blocks\": {}, \"non_water_fine_blocks\": {}, \"block_checksum\": {}, \"fine_checksum\": {}}},\n",
        water.no_of_water_levels,
        water.water_zs.len(),
        water.water_rects.len(),
        water.water_block_list.len(),
        water.water_fine_block_list.len(),
        block_non_water,
        fine_non_water,
        signed_i8_checksum(&water.water_block_list),
        signed_i8_checksum(&water.water_fine_block_list)
    ));
    json.push_str("  \"levels\": [\n");
    for index in 0..active_count {
        let rect = water.water_rects[index];
        json.push_str(&format!(
            "    {{\"index\": {}, \"z\": {:.9}, \"left\": {:.9}, \"bottom\": {:.9}, \"right\": {:.9}, \"top\": {:.9}}}",
            index,
            water.water_zs[index],
            rect.left,
            rect.bottom,
            rect.right,
            rect.top
        ));
        if index + 1 < active_count {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");
    json.push_str("  \"samples\": [\n");
    let samples = waterpro_query_samples();
    for (index, sample) in samples.iter().enumerate() {
        let rect_level = water.water_level_according_to_rectangles(sample.x, sample.y);
        let no_waves = water.get_water_level_no_waves(sample.x, sample.y, sample.z);
        json.push_str(&format!(
            "    {{\"label\": \"{}\", \"x\": {:.9}, \"y\": {:.9}, \"z\": {:.9}, \"rect_found\": {}, \"rect_level\": {:.9}, \"no_waves_found\": {}, \"no_waves_level\": {:.9}, \"no_waves_block\": {}, \"distance_to_water\": {:.9}}}",
            json_escape(sample.label),
            sample.x,
            sample.y,
            sample.z,
            rect_level.is_some(),
            rect_level.unwrap_or(0.0),
            no_waves.is_some(),
            no_waves.map(|sample| sample.level).unwrap_or(0.0),
            no_waves.map(|sample| sample.block).unwrap_or(-128),
            water.calc_distance_to_water(sample.x, sample.y)
        ));
        if index + 1 < samples.len() {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

#[derive(Debug, Clone, Copy)]
struct WaterproQuerySample {
    label: &'static str,
    x: f32,
    y: f32,
    z: f32,
}

fn waterpro_query_samples() -> [WaterproQuerySample; 5] {
    [
        WaterproQuerySample {
            label: "origin",
            x: 0.0,
            y: 0.0,
            z: 100.0,
        },
        WaterproQuerySample {
            label: "south_west_edge",
            x: -2048.0,
            y: -2048.0,
            z: 100.0,
        },
        WaterproQuerySample {
            label: "north_east_inside",
            x: 2047.0,
            y: 2047.0,
            z: 100.0,
        },
        WaterproQuerySample {
            label: "east_south",
            x: 1000.0,
            y: -1000.0,
            z: 100.0,
        },
        WaterproQuerySample {
            label: "west_mid",
            x: -1300.0,
            y: 500.0,
            z: 100.0,
        },
    ]
}

fn waterpro_expected_layout_size() -> usize {
    4 + 48 * 4 + 48 * 4 * 4 + 64 * 64 + 128 * 128
}

fn signed_i8_checksum(values: &[i8]) -> i64 {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| (index as i64 + 1) * (*value as i64 + 129))
        .sum()
}

fn run_gxt_text(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let bytes = fs::read(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let gxt = GxtFile::parse(&bytes)
        .ok_or_else(|| format!("failed to parse GXT {}", asset_path.display()))?;
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();

    let mut tkey_bytes = 0usize;
    let mut tdat_bytes = 0usize;
    collect_gxt_section_lengths(&bytes, &mut tkey_bytes, &mut tdat_bytes)?;

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/Text.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"gxt_size\": {}, \"tkey_bytes\": {}, \"tdat_bytes\": {}, \"tdat_code_units\": {}}},\n",
        size,
        tkey_bytes,
        tdat_bytes,
        gxt.data.len()
    ));
    json.push_str("  \"entries\": [\n");

    for (index, entry) in gxt.entries.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"key\": \"{}\", \"value_offset\": {}, \"text_units_hex\": \"{}\"}}",
            index,
            json_escape(&entry.key),
            entry.value_offset,
            text_units_hex(&gxt.data, entry.value_offset)
        ));
        if index + 1 < gxt.entries.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn run_img_dir(config: &CaseConfig) -> Result<String, String> {
    let root_text = fs::read_to_string(&config.asset_root_config)
        .map_err(|err| format!("failed to read {}: {err}", config.asset_root_config))?;
    let gta_root = PathBuf::from(root_text.trim());
    let asset_path = gta_root.join(config.relative_path.replace('/', "\\"));
    let bytes = fs::read(&asset_path)
        .map_err(|err| format!("failed to read {}: {err}", asset_path.display()))?;
    let directory = CDirectory::read_from(Cursor::new(&bytes), bytes.len() / 32 + 1)
        .map_err(|err| format!("failed to parse {}: {err}", asset_path.display()))?;
    let size = fs::metadata(&asset_path)
        .map_err(|err| format!("failed to stat {}: {err}", asset_path.display()))?
        .len();

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"case\": \"{}\",\n", config.name));
    json.push_str("  \"engine\": \"rust-re3bevy\",\n");
    json.push_str("  \"reference_files\": [\n");
    json.push_str("    \"crates/re3bevy_formats/src/formats/Directory.rs\"\n");
    json.push_str("  ],\n");
    json.push_str(&format!(
        "  \"asset_fingerprint\": {{\"gta_root\": \"configured\", \"dir_size\": {}, \"entry_byte_len\": 32, \"entry_count\": {}}},\n",
        size,
        directory.len()
    ));
    json.push_str("  \"entries\": [\n");

    for (index, entry) in directory.entries().iter().enumerate() {
        json.push_str(&format!(
            "    {{\"index\": {}, \"offset\": {}, \"size\": {}, \"name\": \"{}\"}}",
            index,
            entry.offset,
            entry.size,
            json_escape(&entry.name_string())
        ));
        if index + 1 < directory.len() {
            json.push(',');
        }
        json.push('\n');
    }

    json.push_str("  ]\n");
    json.push_str("}\n");
    Ok(json)
}

fn collect_gxt_section_lengths(
    bytes: &[u8],
    tkey_bytes: &mut usize,
    tdat_bytes: &mut usize,
) -> Result<(), String> {
    let mut offset = 0usize;
    while offset + 8 <= bytes.len() {
        let section = bytes
            .get(offset..offset + 4)
            .ok_or("truncated GXT section name")?;
        offset += 4;
        let section_len = read_u32_le(bytes, offset)? as usize;
        offset += 4;
        let section_end = offset
            .checked_add(section_len)
            .ok_or("GXT section length overflow")?;
        if section_end > bytes.len() {
            return Err("GXT section overruns file".to_string());
        }
        if section == b"TKEY" {
            *tkey_bytes = section_len;
        } else if section == b"TDAT" {
            *tdat_bytes = section_len;
        }
        offset = section_end;
    }
    Ok(())
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| format!("missing u32 at byte offset {offset}"))?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn text_units_hex(data: &[u16], value_offset: u32) -> String {
    let start = (value_offset / 2) as usize;
    data.get(start..)
        .unwrap_or(&[])
        .iter()
        .take_while(|unit| **unit != 0)
        .map(|unit| format!("{unit:04x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn parse_timecyc_records(text: &str) -> Result<Vec<TimecycRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('/') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 40 {
            return Err(format!("TIMECYC.DAT row has fewer than 40 fields: {line}"));
        }
        records.push(TimecycRecord {
            ambient: [
                parse_i32_part(&parts, 0)?,
                parse_i32_part(&parts, 1)?,
                parse_i32_part(&parts, 2)?,
            ],
            directional: [
                parse_i32_part(&parts, 3)?,
                parse_i32_part(&parts, 4)?,
                parse_i32_part(&parts, 5)?,
            ],
            sky_top: [
                parse_i32_part(&parts, 6)?,
                parse_i32_part(&parts, 7)?,
                parse_i32_part(&parts, 8)?,
            ],
            sky_bottom: [
                parse_i32_part(&parts, 9)?,
                parse_i32_part(&parts, 10)?,
                parse_i32_part(&parts, 11)?,
            ],
            sun_core: [
                parse_i32_part(&parts, 12)?,
                parse_i32_part(&parts, 13)?,
                parse_i32_part(&parts, 14)?,
            ],
            sun_corona: [
                parse_i32_part(&parts, 15)?,
                parse_i32_part(&parts, 16)?,
                parse_i32_part(&parts, 17)?,
            ],
            sun_size: parse_f32_part(&parts, 18)?,
            sprite_size: parse_f32_part(&parts, 19)?,
            sprite_brightness: parse_f32_part(&parts, 20)?,
            shadow_strength: parse_i32_part(&parts, 21)?,
            light_shadow_strength: parse_i32_part(&parts, 22)?,
            tree_shadow_strength: parse_i32_part(&parts, 23)?,
            far_clip: parse_f32_part(&parts, 24)?,
            fog_start: parse_f32_part(&parts, 25)?,
            lights_on_ground_brightness: parse_f32_part(&parts, 26)?,
            low_clouds: [
                parse_i32_part(&parts, 27)?,
                parse_i32_part(&parts, 28)?,
                parse_i32_part(&parts, 29)?,
            ],
            fluffy_clouds_top: [
                parse_i32_part(&parts, 30)?,
                parse_i32_part(&parts, 31)?,
                parse_i32_part(&parts, 32)?,
            ],
            fluffy_clouds_bottom: [
                parse_i32_part(&parts, 33)?,
                parse_i32_part(&parts, 34)?,
                parse_i32_part(&parts, 35)?,
            ],
            blur: [
                parse_f32_part(&parts, 36)?,
                parse_f32_part(&parts, 37)?,
                parse_f32_part(&parts, 38)?,
                parse_f32_part(&parts, 39)?,
            ],
        });
    }
    if records.len() < 96 {
        return Err(format!(
            "TIMECYC.DAT produced {} records, expected 96",
            records.len()
        ));
    }
    records.truncate(96);
    Ok(records)
}

fn parse_i32_part(parts: &[&str], index: usize) -> Result<i32, String> {
    parts[index]
        .parse()
        .map_err(|err| format!("invalid i32 `{}` at {index}: {err}", parts[index]))
}

fn parse_f32_part(parts: &[&str], index: usize) -> Result<f32, String> {
    parts[index]
        .parse()
        .map_err(|err| format!("invalid f32 `{}` at {index}: {err}", parts[index]))
}
