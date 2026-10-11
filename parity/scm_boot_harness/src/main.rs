//! Non-production harness: exercise real MAIN.SCM boot path used by load.
//!
//! Mirrors re3 `CGame::Initialise` "Start script":
//!   CTheScripts::StartTestScript();
//!   CTheScripts::Process();  // exactly once during load
//! then drains effects into `ScriptGameplayBridge` via the same
//! `apply_script_effects_to_world` API as production.
//!
//! If CREATE_PLAYER is behind WAIT on the first load Process (re3 wakes later
//! frames in CGame::Process), the harness may advance time and process further
//! **only as harness exercise** — it never invents a multi-frame loop inside
//! the production load function.
//!
//! Usage (from repo root):
//!   cargo run -p re3bevy_scm_boot_harness

use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_core::core::Clock::CClock;
use re3bevy_core::core::General::CGeneral;
use re3bevy_game::animation::CutsceneMgr::CCutsceneMgr;
use re3bevy_game::control::Script::{
    CTheScripts, ScriptCommandEffect, ScriptEntitySnapshot, ScriptPlayerSnapshot,
};
use re3bevy_game::core::Camera::{CCamera, CameraScriptMode};
use re3bevy_game::core::Game::{ScriptGameplayBridge, apply_script_effects_to_world};
use re3bevy_game::core::Stats::CStats;
use re3bevy_game::core::Streaming::CStreaming;
use re3bevy_game::peds::Gangs::CGangs;
use re3bevy_game::peds::PlayerPed::StagedWorldPlayers;
use re3bevy_game::renderer::Weather::CWeather;
use re3bevy_game::vehicles::CarGen::CTheCarGenerators;
use std::collections::BTreeSet;
use std::env;
use std::process::ExitCode;

fn sync_unapplied_collision_mirror(scripts: &mut CTheScripts) {
    if let Some(level) = scripts.pending_effects.iter().rev().find_map(|effect| {
        if let ScriptCommandEffect::LoadCollision { level } = effect {
            Some(*level)
        } else {
            None
        }
    }) {
        scripts.collision_in_memory = level;
    }
}

fn main() -> ExitCode {
    let config = env::args()
        .nth(1)
        .unwrap_or_else(|| "config/original_gta3_path.txt".to_string());

    println!("scm_boot_harness: config={config}");
    let install = match OriginalGtaInstall::from_config_path(&config) {
        Ok(install) => install,
        Err(error) => {
            eprintln!("FAIL: cannot load original GTA install from {config}: {error}");
            return ExitCode::from(2);
        }
    };
    println!(
        "scm_boot_harness: install_root={}",
        install.root().display()
    );

    let mut file_mgr = install.file_mgr();
    let mut scripts = match CTheScripts::init(&mut file_mgr) {
        Ok(scripts) => scripts,
        Err(error) => {
            eprintln!("FAIL: CTheScripts::init (MAIN.SCM): {error}");
            return ExitCode::from(3);
        }
    };
    println!(
        "scm_boot_harness: MAIN.SCM loaded_main_bytes={} variable_space={}",
        scripts.loaded_main_bytes, scripts.variable_space_size
    );

    let mut general = CGeneral::default();
    let mut clock = CClock::initialise(1000, 0);
    let mut bridge = ScriptGameplayBridge::default();
    let mut streaming = CStreaming::default();
    let mut camera = {
        let mut cam = CCamera::default();
        cam.Init();
        cam
    };
    let mut weather = CWeather::init();
    let mut gangs = CGangs::initialise();
    let mut stats = CStats::init();
    let mut cutscene = CCutsceneMgr::initialise();
    let mut world_players = StagedWorldPlayers::default();
    let mut car_generators = CTheCarGenerators::init();

    // --- re3 load shape: StartTestScript + exactly one Process ---
    let active = match scripts.start_test_script() {
        Ok(index) => index,
        Err(error) => {
            eprintln!("FAIL: StartTestScript: {error:?}");
            return ExitCode::from(4);
        }
    };
    println!("scm_boot_harness: StartTestScript active_index={active}");

    if let Err(error) = scripts.process_active_scripts_with_context(
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
    ) {
        eprintln!("FAIL: Process after StartTestScript: {error:?}");
        return ExitCode::from(5);
    }

    let load_commands = scripts.commands_executed;
    let load_effects = scripts.pending_effects.len();
    let create_in_load_effects = scripts
        .pending_effects
        .iter()
        .any(|e| matches!(e, ScriptCommandEffect::CreatePlayer { .. }));
    let load_cutscene_effects = scripts
        .pending_effects
        .iter()
        .filter(|effect| matches!(effect, ScriptCommandEffect::LoadCutscene { .. }))
        .count();
    let start_cutscene_effects = scripts
        .pending_effects
        .iter()
        .filter(|effect| matches!(effect, ScriptCommandEffect::StartCutscene))
        .count();
    println!(
        "scm_boot_harness: load Process once: commands={load_commands} effects={load_effects} create_player_effect={create_in_load_effects} load_cutscene_effects={load_cutscene_effects} start_cutscene_effects={start_cutscene_effects}"
    );
    for script_index in scripts.active_scripts.iter().copied() {
        let script = &scripts.scripts[script_index];
        let ip = script.ip as usize;
        let opcode = scripts
            .script_space
            .get(ip..ip + 2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) & 0x7fff);
        println!(
            "scm_boot_harness: post-load active script index={} name={:?} ip={} wake={} opcode={:?}",
            script_index,
            String::from_utf8_lossy(&script.script_name),
            script.ip,
            script.wake_time,
            opcode
        );
    }

    let create_gen_effects = scripts
        .pending_effects
        .iter()
        .filter(|e| matches!(e, ScriptCommandEffect::CreateCarGenerator { .. }))
        .count();
    let switch_gen_effects = scripts
        .pending_effects
        .iter()
        .filter(|e| matches!(e, ScriptCommandEffect::SwitchCarGenerator { .. }))
        .count();
    let switch_on_count = scripts
        .pending_effects
        .iter()
        .filter(|e| {
            matches!(
                e,
                ScriptCommandEffect::SwitchCarGenerator { count, .. } if *count != 0
            )
        })
        .count();
    println!(
        "scm_boot_harness: pending car_gen effects create={create_gen_effects} switch={switch_gen_effects} switch_on={switch_on_count}"
    );
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
        Some(&mut world_players),
        Some(&mut general),
        Some(&mut car_generators),
        None,
        0,
    );
    println!(
        "scm_boot_harness: car_generators after load count={} active={} provisional={}",
        car_generators.num_of_car_generators,
        car_generators.current_active_count,
        scripts.car_generators_created
    );

    sync_unapplied_collision_mirror(&mut scripts);
    println!(
        "scm_boot_harness: after load drain player_spawned={} index={} pos=({:.3},{:.3},{:.3}) heading={:.3} cutscene='{}' requested={} started={} manager_loaded={} manager_status={:?} camera_mode={:?}",
        bridge.player_spawned,
        bridge.player_index,
        bridge.player_position.x,
        bridge.player_position.y,
        bridge.player_position.z,
        bridge.player_heading_degrees,
        bridge.cutscene_name,
        bridge.cutscene_requested,
        bridge.cutscene_started,
        cutscene.loaded,
        cutscene.cutscene_load_status,
        camera.script_mode
    );

    // Harness-only: multi-frame InGame-shaped Process with the same probe wiring
    // production uses (bridge / camera / pad stand-ins). Load path remains
    // single-Process (verified above). Require CREATE_PLAYER already stored and
    // further frames must not hard-fail with MissingPlayerProbe.
    let mut time_ms: u32 = 0;
    let mut extra_frames = 0u32;
    let mut seen_active: BTreeSet<usize> = scripts.active_scripts.iter().copied().collect();
    let mut seen_inactive: BTreeSet<usize> = BTreeSet::new();
    const MAX_EXTRA_FRAMES: u32 = 6_000;
    /// Minimum multi-frame Process slices after load (InGame-shaped).
    const MIN_INGAME_FRAMES: u32 = 180;
    const STEP_MS: u32 = 16;
    while extra_frames < MAX_EXTRA_FRAMES
        && (extra_frames < MIN_INGAME_FRAMES || !bridge.cutscene_started)
    {
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
            heading_degrees: 0.0,
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
        if let Err(error) = scripts.process_active_scripts_with_context(
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
        ) {
            let msg = format!("{error:?}");
            if msg.contains("MissingPlayerProbe")
                || msg.contains("MissingPadProbe")
                || msg.contains("MissingCharProbe")
                || msg.contains("MissingGroundProbe")
                || msg.contains("MissingCameraProbe")
            {
                eprintln!(
                    "FAIL: probe hard-fail after CREATE_PLAYER frame={extra_frames}: {error:?}"
                );
                return ExitCode::from(6);
            }
            // Other VM errors still fail the harness (honest path).
            eprintln!("FAIL: harness post-load Process frame {extra_frames}: {error:?}");
            return ExitCode::from(6);
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
            Some(&mut world_players),
            Some(&mut general),
            Some(&mut car_generators),
            None,
            time_ms,
        );
        sync_unapplied_collision_mirror(&mut scripts);
        for script_index in scripts.active_scripts.iter().copied() {
            if seen_active.insert(script_index) {
                let script = &scripts.scripts[script_index];
                let ip = script.ip as usize;
                let opcode = scripts
                    .script_space
                    .get(ip..ip + 2)
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) & 0x7fff);
                println!(
                    "scm_boot_harness: new active script frame={} index={} name={:?} ip={} wake={} opcode={:?}",
                    extra_frames,
                    script_index,
                    String::from_utf8_lossy(&script.script_name),
                    script.ip,
                    script.wake_time,
                    opcode
                );
            }
        }
        let current_active: BTreeSet<usize> = scripts.active_scripts.iter().copied().collect();
        for script_index in seen_active.difference(&current_active) {
            if !seen_inactive.insert(*script_index) {
                continue;
            }
            let script = &scripts.scripts[*script_index];
            println!(
                "scm_boot_harness: inactive script frame={} index={} last_name={:?} last_ip={} last_wake={}",
                extra_frames,
                script_index,
                String::from_utf8_lossy(&script.script_name),
                script.ip,
                script.wake_time
            );
        }
    }

    println!(
        "scm_boot_harness: after wake frames={extra_frames} player_spawned={} index={} pos=({:.3},{:.3},{:.3}) heading={:.3} commands_total={} cutscene='{}' requested={} started={} manager_loaded={} manager_status={:?} camera_mode={:?} car_gens={} active={}",
        bridge.player_spawned,
        bridge.player_index,
        bridge.player_position.x,
        bridge.player_position.y,
        bridge.player_position.z,
        bridge.player_heading_degrees,
        scripts.commands_executed,
        bridge.cutscene_name,
        bridge.cutscene_requested,
        bridge.cutscene_started,
        cutscene.loaded,
        cutscene.cutscene_load_status,
        camera.script_mode,
        car_generators.num_of_car_generators,
        car_generators.current_active_count
    );
    for script_index in scripts.active_scripts.iter().copied() {
        let script = &scripts.scripts[script_index];
        let ip = script.ip as usize;
        let opcode = scripts
            .script_space
            .get(ip..ip + 2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) & 0x7fff);
        println!(
            "scm_boot_harness: active script index={} name={:?} ip={} wake={} opcode={:?}",
            script_index,
            String::from_utf8_lossy(&script.script_name),
            script.ip,
            script.wake_time,
            opcode
        );
    }

    if !bridge.player_spawned {
        eprintln!(
            "FAIL: CREATE_PLAYER never stored spawn on ScriptGameplayBridge (extra_frames={extra_frames})"
        );
        return ExitCode::from(7);
    }

    // re3 CREATE_PLAYER always writes index + position; heading may stay 0 until SET_PLAYER_HEADING.
    let pos_ok = bridge.player_position.x.is_finite()
        && bridge.player_position.y.is_finite()
        && bridge.player_position.z.is_finite();
    if !pos_ok {
        eprintln!("FAIL: CREATE_PLAYER position not finite");
        return ExitCode::from(8);
    }

    // Spawn should not be pure origin (MAIN places the player in the city).
    let not_origin = bridge.player_position.x.abs() > 0.01
        || bridge.player_position.y.abs() > 0.01
        || bridge.player_position.z.abs() > 0.01;
    if !not_origin {
        eprintln!(
            "FAIL: CREATE_PLAYER position still origin (0,0,0) — bridge apply path likely broken"
        );
        return ExitCode::from(9);
    }

    if extra_frames < MIN_INGAME_FRAMES {
        eprintln!(
            "FAIL: multi-frame InGame Process only ran {extra_frames} < {MIN_INGAME_FRAMES} frames"
        );
        return ExitCode::from(10);
    }
    // Camera must leave origin when player is spawned (follow snap or fixed).
    let cam_at_origin = camera.position.x.abs() < 0.01
        && camera.position.y.abs() < 0.01
        && camera.position.z.abs() < 0.01;
    if cam_at_origin && camera.script_mode == CameraScriptMode::FollowPlayer {
        // Apply same snap production uses after CREATE_PLAYER.
        re3bevy_game::core::Game::snap_follow_camera_resource(
            &mut camera,
            bridge.player_position,
            bridge.player_heading_degrees,
        );
    }
    let cam_still_origin = camera.position.x.abs() < 0.01
        && camera.position.y.abs() < 0.01
        && camera.position.z.abs() < 0.01;
    if cam_still_origin {
        eprintln!("FAIL: camera still at origin after spawn snap");
        return ExitCode::from(12);
    }

    println!("scm_boot_harness: PASS");
    println!("  load_shape=StartTestScript+single_Process");
    println!("  ingame_frames={extra_frames} (no MissingPlayerProbe)");
    println!("  create_player_index={}", bridge.player_index);
    println!(
        "  create_player_pos=({:.3},{:.3},{:.3})",
        bridge.player_position.x, bridge.player_position.y, bridge.player_position.z
    );
    println!("  create_player_heading={}", bridge.player_heading_degrees);
    println!("  camera_mode={:?}", camera.script_mode);
    println!(
        "  camera_pos=({:.3},{:.3},{:.3})",
        camera.position.x, camera.position.y, camera.position.z
    );
    if camera.script_mode == CameraScriptMode::Fixed {
        println!(
            "  fixed_camera_pos=({:.3},{:.3},{:.3})",
            camera.position.x, camera.position.y, camera.position.z
        );
    }
    ExitCode::SUCCESS
}
