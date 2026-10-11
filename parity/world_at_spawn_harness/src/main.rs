//! Non-production harness: load real DAT/IDE/IPL via production FileLoader path
//! and measure world entity presence around CREATE_PLAYER spawn (or a given point).
//!
//! Does not invent entities. Counts only IPL-derived world content.
//!
//! Usage (from repo root):
//!   cargo run -p re3bevy_world_at_spawn_harness
//!   cargo run -p re3bevy_world_at_spawn_harness -- 811.875 -939.938 35.75

use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_assets::streaming::CdStream::CdStream;
use re3bevy_core::core::Clock::CClock;
use re3bevy_core::core::Coordinates::CVector;
use re3bevy_core::core::General::CGeneral;
use re3bevy_game::control::Script::CTheScripts;
use re3bevy_game::core::Camera::CCamera;
use re3bevy_game::core::FileLoader::CFileLoader;
use re3bevy_game::core::Game::{
    ScriptGameplayBridge, WORLD_SPAWN_RADIUS, apply_script_effects_to_world,
    count_world_entities_near,
};
use re3bevy_game::core::Stats::CStats;
use re3bevy_game::core::Streaming::CStreaming;
use re3bevy_game::peds::Gangs::CGangs;
use re3bevy_game::peds::PlayerPed::StagedWorldPlayers;
use re3bevy_game::renderer::Weather::CWeather;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let config = args
        .next()
        .filter(|a| a.contains("path") || a.ends_with(".txt") || a.contains("config"))
        .unwrap_or_else(|| "config/original_gta3_path.txt".to_string());

    // Optional explicit spawn; if omitted, run StartTestScript once to get CREATE_PLAYER.
    let explicit: Option<(f32, f32, f32)> = {
        let rest: Vec<String> = env::args().skip(1).collect();
        let nums: Vec<f32> = rest.iter().filter_map(|s| s.parse::<f32>().ok()).collect();
        if nums.len() >= 3 {
            Some((nums[0], nums[1], nums[2]))
        } else {
            None
        }
    };

    println!("world_at_spawn_harness: config={config}");
    let install = match OriginalGtaInstall::from_config_path(&config) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("FAIL: install: {e}");
            return ExitCode::from(2);
        }
    };

    let mut file_mgr = install.file_mgr();
    let mut cd_stream = CdStream::init(5);
    let mut file_loader = CFileLoader::default();
    let mut streaming = CStreaming::default();

    println!("world_at_spawn_harness: loading DEFAULT.DAT + GTA3.DAT plans (IPL)...");
    if let Err(e) = file_loader.load_default_and_dat_plans(
        &install,
        &mut file_mgr,
        &mut cd_stream,
        "DATA\\GTA3.DAT",
    ) {
        eprintln!("FAIL: load_default_and_dat_plans: {e}");
        return ExitCode::from(3);
    }

    let total = file_loader.world.entities.len();
    println!("world_at_spawn_harness: entities_total={total}");
    if total == 0 {
        eprintln!("FAIL: zero IPL entities after real DAT/IPL load");
        return ExitCode::from(4);
    }

    let spawn = if let Some((x, y, z)) = explicit {
        CVector::new(x, y, z)
    } else {
        // Real MAIN.SCM path: StartTestScript + one Process + drain CreatePlayer.
        let mut scripts = match CTheScripts::init(&mut file_mgr) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("FAIL: CTheScripts::init: {e}");
                return ExitCode::from(5);
            }
        };
        let mut general = CGeneral::default();
        let mut clock = CClock::initialise(1000, 0);
        let mut bridge = ScriptGameplayBridge::default();
        let mut camera = CCamera::default();
        camera.Init();
        let mut weather = CWeather::init();
        let mut gangs = CGangs::initialise();
        let mut stats = CStats::init();
        let mut world_players = StagedWorldPlayers::default();

        if scripts.start_test_script().is_err() {
            eprintln!("FAIL: StartTestScript");
            return ExitCode::from(6);
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
            eprintln!("FAIL: Process: {e:?}");
            return ExitCode::from(7);
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
            eprintln!("FAIL: CREATE_PLAYER not reached on load Process");
            return ExitCode::from(8);
        }
        println!(
            "world_at_spawn_harness: CREATE_PLAYER pos=({:.3},{:.3},{:.3})",
            bridge.player_position.x, bridge.player_position.y, bridge.player_position.z
        );
        bridge.player_position
    };

    let near = count_world_entities_near(&file_loader.world, spawn, WORLD_SPAWN_RADIUS);
    println!(
        "world_at_spawn_harness: near_spawn radius={WORLD_SPAWN_RADIUS} center=({:.3},{:.3},{:.3}) count={near}",
        spawn.x, spawn.y, spawn.z
    );

    // Sector occupancy around spawn (real sector lists from IPL placement).
    let sx = re3bevy_game::core::World::CWorld::get_sector_index_x(spawn.x);
    let sy = re3bevy_game::core::World::CWorld::get_sector_index_y(spawn.y);
    let sector_idx = sy * re3bevy_game::core::World::NUMSECTORS_X + sx;
    let sector = file_loader.world.sectors.get(sector_idx);
    let sector_buildings = sector.map(|s| s.buildings.len()).unwrap_or(0);
    let sector_dummies = sector.map(|s| s.dummies.len()).unwrap_or(0);
    let sector_objects = sector.map(|s| s.objects.len()).unwrap_or(0);
    println!(
        "world_at_spawn_harness: sector=({sx},{sy}) buildings={sector_buildings} dummies={sector_dummies} objects={sector_objects}"
    );

    if near == 0 && sector_buildings + sector_dummies + sector_objects == 0 {
        eprintln!("FAIL: no world content at spawn vicinity (empty void)");
        return ExitCode::from(9);
    }

    // re3 FindGroundZForCoord around CREATE_PLAYER (col models on IPL buildings).
    let ground_z =
        file_loader
            .world
            .find_ground_z_for_coord(spawn.x, spawn.y, &file_loader.model_info);
    println!(
        "world_at_spawn_harness: FindGroundZForCoord({:.3},{:.3}) => {ground_z:.3} (spawn_z={:.3})",
        spawn.x, spawn.y, spawn.z
    );

    // Continuous stream pass around spawn (request list + radar + CD slices).
    match file_loader.update_streaming_around_position(
        &mut streaming,
        &mut cd_stream,
        [spawn.x, spawn.y, spawn.z],
    ) {
        Ok(n) => {
            println!("world_at_spawn_harness: update_streaming_around_position loaded_slices={n}")
        }
        Err(e) => {
            eprintln!("FAIL: update_streaming_around_position: {e}");
            return ExitCode::from(10);
        }
    }

    println!("world_at_spawn_harness: PASS");
    println!("  entities_total={total}");
    println!("  near_spawn={near}");
    println!(
        "  sector_lists={}",
        sector_buildings + sector_dummies + sector_objects
    );
    println!("  ground_z={ground_z:.3}");
    ExitCode::SUCCESS
}
