//! Non-production harness: CREATE_PLAYER → real CWorldPlayers ped + pad-driven on-foot motion.
//!
//! Uses production apply path and CPlayerPed::apply_on_foot_pad_step (not a re-implementation).
//!
//! Usage: cargo run -p re3bevy_player_control_harness

use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_core::core::Clock::CClock;
use re3bevy_core::core::General::CGeneral;
use re3bevy_game::control::Script::{CTheScripts, ScriptEntitySnapshot, ScriptPlayerSnapshot};
use re3bevy_game::core::Camera::CCamera;
use re3bevy_game::core::Game::{ScriptGameplayBridge, apply_script_effects_to_world};
use re3bevy_game::core::Pad::{CPad, CPads, controller_state_from_keyboard};
use re3bevy_game::core::Stats::CStats;
use re3bevy_game::core::Streaming::CStreaming;
use re3bevy_game::peds::Gangs::CGangs;
use re3bevy_game::peds::{
    PedType::ePedType,
    PlayerPed::{CPlayerPed, StagedWorldPlayers, WorldPlayerAccess},
};
use re3bevy_game::renderer::Weather::CWeather;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let config = env::args()
        .nth(1)
        .unwrap_or_else(|| "config/original_gta3_path.txt".to_string());
    println!("player_control_harness: config={config}");

    let install = match OriginalGtaInstall::from_config_path(&config) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("FAIL: install: {e}");
            return ExitCode::from(2);
        }
    };
    let mut file_mgr = install.file_mgr();
    let mut scripts = match CTheScripts::init(&mut file_mgr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FAIL: MAIN.SCM init: {e}");
            return ExitCode::from(3);
        }
    };

    let mut general = CGeneral::default();
    let mut clock = CClock::initialise(1000, 0);
    let mut bridge = ScriptGameplayBridge::default();
    let mut streaming = CStreaming::default();
    let mut camera = {
        let mut c = CCamera::default();
        c.Init();
        c
    };
    let mut weather = CWeather::init();
    let mut gangs = CGangs::initialise();
    let mut stats = CStats::init();
    let mut world_players = StagedWorldPlayers::default();
    let pads = CPads::default();

    if scripts.start_test_script().is_err() {
        eprintln!("FAIL: StartTestScript");
        return ExitCode::from(4);
    }
    if let Err(e) = scripts.process_active_scripts_with_context(
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
        eprintln!("FAIL: load Process: {e:?}");
        return ExitCode::from(5);
    }
    apply_script_effects_to_world(
        &mut scripts,
        &mut bridge,
        None,
        Some(&mut stats),
        None,
        Some(&mut streaming),
        Some(&mut camera),
        Some(&mut weather),
        Some(&mut gangs),
        Some(&mut world_players),
        Some(&mut general),
        None,
        None,
        0,
    );

    if !bridge.player_spawned {
        eprintln!("FAIL: CREATE_PLAYER not on bridge");
        return ExitCode::from(6);
    }
    let Some(player) = world_players.player(bridge.player_index) else {
        eprintln!("FAIL: no CPlayerPed after CREATE_PLAYER apply");
        return ExitCode::from(7);
    };
    let spawn = player.position();
    let not_origin = spawn.x.abs() > 0.01 || spawn.y.abs() > 0.01 || spawn.z.abs() > 0.01;
    if !not_origin {
        eprintln!("FAIL: ped still at origin");
        return ExitCode::from(8);
    }
    if !player.active_in_world {
        eprintln!("FAIL: ped not active_in_world");
        return ExitCode::from(9);
    }
    println!(
        "player_control_harness: ped pool_handle={} pos=({:.3},{:.3},{:.3}) heading={:.1} active={}",
        player.pool_handle,
        spawn.x,
        spawn.y,
        spawn.z,
        player.heading_degrees(),
        player.active_in_world
    );

    // Multi-frame Process with live probes (no MissingPlayerProbe).
    let mut time_ms = 0u32;
    const FRAMES: u32 = 90;
    const STEP_MS: u32 = 16;
    for frame in 1..=FRAMES {
        time_ms = time_ms.saturating_add(STEP_MS);
        let pos = world_players
            .player(bridge.player_index)
            .map(|p| p.position())
            .unwrap_or(bridge.player_position);
        let stopped = world_players
            .player(bridge.player_index)
            .map(|p| p.is_stopped())
            .unwrap_or(true);
        let handle = world_players
            .player(bridge.player_index)
            .map(|p| p.pool_handle)
            .unwrap_or(1);
        let pad0 = pads.pads[0];
        let mut player_probe = |index: i32| {
            (bridge.player_spawned && (index == 0 || index == bridge.player_index)).then_some(
                ScriptPlayerSnapshot {
                    ped_handle: handle,
                    position: pos,
                    current_vehicle: None,
                    current_vehicle_model: None,
                    stopped,
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
                },
            )
        };
        let mut char_probe = |h: i32| {
            (bridge.player_spawned && h == handle).then_some(ScriptEntitySnapshot {
                position: pos,
                bound_center: pos,
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
                stopped,
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
            })
        };
        let mut ground_probe = |_: f32, _: f32| pos.z;
        let mut camera_probe = |_: _, _: _| true;
        let mut pad_probe = |pad: i32, button: i32| {
            let p = if pad <= 0 { &pad0 } else { &pads.pads[1] };
            p.get_pad_state_button(button)
        };
        if let Err(e) = scripts.process_active_scripts_with_context(
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
            let msg = format!("{e:?}");
            if msg.contains("MissingPlayerProbe") || msg.contains("MissingPadProbe") {
                eprintln!("FAIL: probe hard-fail frame={frame}: {e:?}");
                return ExitCode::from(10);
            }
            eprintln!("FAIL: Process frame={frame}: {e:?}");
            return ExitCode::from(11);
        }
        apply_script_effects_to_world(
            &mut scripts,
            &mut bridge,
            None,
            Some(&mut stats),
            None,
            Some(&mut streaming),
            Some(&mut camera),
            Some(&mut weather),
            Some(&mut gangs),
            Some(&mut world_players),
            Some(&mut general),
            None,
            None,
            0,
        );
    }
    println!("player_control_harness: {FRAMES} Process frames OK (no MissingPlayerProbe)");

    // Harness-forced control enable + synthetic pad (MAIN may leave control off in intro).
    bridge.player_control_enabled = true;
    bridge.playing_intro = false;
    let before = world_players
        .player(bridge.player_index)
        .map(|p| p.position())
        .expect("ped");
    let mut force_pad = CPad::default();
    force_pad.Update(controller_state_from_keyboard(
        true, // W forward
        false, false, false, true, // sprint
        false, false, false,
    ));
    let dt = STEP_MS as f32 / 1000.0;
    let mut moved_frames = 0u32;
    for _ in 0..60 {
        let mut player = world_players.player_mut(bridge.player_index).unwrap();
        if player.apply_on_foot_pad_step(&force_pad, dt, true, false) {
            moved_frames += 1;
        }
    }
    let after = world_players
        .player(bridge.player_index)
        .map(|p| p.position())
        .expect("ped");
    let dx = after.x - before.x;
    let dy = after.y - before.y;
    let dist = (dx * dx + dy * dy).sqrt();
    println!(
        "player_control_harness: forced motion moved_frames={moved_frames} dist={dist:.3} before=({:.3},{:.3}) after=({:.3},{:.3})",
        before.x, before.y, after.x, after.y
    );
    if dist < 0.5 || moved_frames == 0 {
        eprintln!("FAIL: on-foot pad step did not move ped (dist={dist})");
        return ExitCode::from(12);
    }

    // Control disabled → no free motion.
    let freeze = after;
    for _ in 0..30 {
        let mut player = world_players.player_mut(bridge.player_index).unwrap();
        player.apply_on_foot_pad_step(&force_pad, dt, false, false);
    }
    let frozen = world_players
        .player(bridge.player_index)
        .unwrap()
        .position();
    let freeze_dist = ((frozen.x - freeze.x).powi(2) + (frozen.y - freeze.y).powi(2)).sqrt();
    if freeze_dist > 0.01 {
        eprintln!("FAIL: ped moved while control disabled dist={freeze_dist}");
        return ExitCode::from(13);
    }
    println!("player_control_harness: control-disabled freeze OK");

    // Mod-extension invariant: the second local ped uses the identical
    // production Zelda-control step without taking SCM/PlayerInFocus ownership.
    CPlayerPed::setup_player_ped(&mut world_players, 1, &mut general, None);
    if world_players.player_in_focus != 0 {
        eprintln!("FAIL: local player two stole PlayerInFocus");
        return ExitCode::from(14);
    }
    let Some(mut player_two) = world_players.player_mut(1) else {
        eprintln!("FAIL: local player two was not created");
        return ExitCode::from(15);
    };
    if player_two.ped.ped_type != ePedType::PEDTYPE_PLAYER2 {
        eprintln!("FAIL: local player two has wrong ped type");
        return ExitCode::from(16);
    }
    player_two.set_position(before);
    let p2_before = player_two.position();
    let mut p2_moved_frames = 0_u32;
    for _ in 0..60 {
        if player_two.apply_on_foot_pad_step(&force_pad, dt, true, false) {
            p2_moved_frames += 1;
        }
    }
    let p2_after = player_two.position();
    let p2_dist = ((p2_after.x - p2_before.x).powi(2) + (p2_after.y - p2_before.y).powi(2)).sqrt();
    if p2_moved_frames == 0 || p2_dist < 0.5 {
        eprintln!("FAIL: local player two control did not move p2=({p2_moved_frames},{p2_dist})");
        return ExitCode::from(17);
    }
    println!("player_control_harness: local P2 focus/type/control extension OK dist={p2_dist:.3}");

    println!("player_control_harness: PASS");
    println!(
        "  create_player_pos=({:.3},{:.3},{:.3})",
        spawn.x, spawn.y, spawn.z
    );
    println!("  motion_dist={dist:.3} moved_frames={moved_frames}");
    ExitCode::SUCCESS
}
