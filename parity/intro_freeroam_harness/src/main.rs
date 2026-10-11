//! Non-production: MAIN.SCM intro cutscene advance + CLEAR restore → free-roam-ready.
//!
//! Uses production apply_script_effects_to_world, cutscene update, finish/clear helpers.
//!
//! Usage: cargo run -p re3bevy_intro_freeroam_harness

use bevy::ecs::system::SystemState;
use bevy::ecs::world::World;
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_assets::streaming::CdStream::CdStream;
use re3bevy_core::core::Clock::CClock;
use re3bevy_core::core::GameState::CGameState;
use re3bevy_core::core::General::CGeneral;
use re3bevy_game::animation::CutsceneMgr::CCutsceneMgr;
use re3bevy_game::control::Script::{
    CTheScripts, ScriptEntityMutation, ScriptEntityMutationResult, ScriptEntityRuntimeAccess,
    ScriptEntitySnapshot, ScriptPlayerSnapshot,
};
use re3bevy_game::core::Camera::{CCamera, CameraScriptMode};
use re3bevy_game::core::FileLoader::CFileLoader;
use re3bevy_game::core::Game::{
    ScriptEntityIndex, ScriptEntityWorld, ScriptGameplayBridge, apply_cutscene_finish,
    apply_script_effects_to_world_with_streaming, apply_script_player_clothing_effects,
};
use re3bevy_game::core::Pad::CPads;
use re3bevy_game::core::Pools::CPools;
use re3bevy_game::core::Stats::CStats;
use re3bevy_game::core::Streaming::CStreaming;
use re3bevy_game::peds::Gangs::CGangs;
use re3bevy_game::peds::PlayerPed::{StagedWorldPlayers, WorldPlayerAccess};
use re3bevy_game::renderer::Weather::CWeather;
use std::env;
use std::io::Cursor;
use std::process::ExitCode;

struct HarnessEcs {
    world: World,
    state: SystemState<ScriptEntityWorld<'static, 'static>>,
}

impl HarnessEcs {
    fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(ScriptEntityIndex::default());
        let state = SystemState::new(&mut world);
        Self { world, state }
    }

    fn with<R>(&mut self, f: impl FnOnce(&mut ScriptEntityWorld<'_, '_>) -> R) -> R {
        let result = {
            let mut entities = self
                .state
                .get_mut(&mut self.world)
                .expect("ScriptEntityWorld parameters must validate");
            f(&mut entities)
        };
        // `ScriptEntityWorld` uses deferred Commands exactly like the live Bevy
        // schedule.  A parity harness must flush them between calls or a char
        // created in one SCM slice exists only in the transient same-pass
        // overlay and disappears from queries on the next frame.
        self.state.apply(&mut self.world);
        result
    }
}

struct HarnessScriptEntityRuntime<'a> {
    ecs: &'a mut HarnessEcs,
    pools: &'a mut CPools,
    file_loader: &'a CFileLoader,
    player_entity: Option<(i32, ScriptEntitySnapshot)>,
}

impl ScriptEntityRuntimeAccess for HarnessScriptEntityRuntime<'_> {
    fn supports_mutation(&self, request: &ScriptEntityMutation) -> bool {
        matches!(
            request,
            ScriptEntityMutation::CreateChar(_) | ScriptEntityMutation::CreateCar(_)
        )
    }

    fn mutate(
        &mut self,
        request: ScriptEntityMutation,
        general: Option<&mut CGeneral>,
    ) -> Option<ScriptEntityMutationResult> {
        match request {
            ScriptEntityMutation::CreateChar(request) => self.ecs.with(|entities| {
                entities
                    .insert_script_char(self.pools, request, general?, None, self.file_loader)
                    .map(ScriptEntityMutationResult::Created)
            }),
            ScriptEntityMutation::CreateCar(request) => {
                let model_info = self
                    .file_loader
                    .model_info
                    .get_model_info(request.model_id.max(0) as usize);
                self.ecs.with(|entities| {
                    entities
                        .insert_script_car(
                            self.pools,
                            request,
                            model_info,
                            None,
                            general?,
                            self.file_loader,
                            false,
                        )
                        .map(ScriptEntityMutationResult::Created)
                })
            }
            ScriptEntityMutation::ObjectMotion(_)
            | ScriptEntityMutation::CreateObject(_)
            | ScriptEntityMutation::SetAllTaxiLights(_) => None,
        }
    }

    fn char_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        if let Some((player_handle, snapshot)) = self.player_entity {
            if handle == player_handle {
                return Some(snapshot);
            }
        }
        self.ecs.with(|entities| entities.char_snapshot(handle))
    }

    fn vehicle_snapshot(&mut self, handle: i32) -> Option<ScriptEntitySnapshot> {
        self.ecs.with(|entities| entities.vehicle_snapshot(handle))
    }

    fn object_snapshot(&mut self, _handle: i32) -> Option<ScriptEntitySnapshot> {
        None
    }
}

fn validate_intro_mission_audio(install: &OriginalGtaInstall) -> Result<(), String> {
    for name in ["lib_a1", "lib_a2", "lib_a"] {
        let bytes = install
            .read(format!("AUDIO\\{name}.WAV"))
            .map_err(|error| format!("{name}: {error}"))?;
        let sample_count = rodio::Decoder::try_from(Cursor::new(bytes))
            .map_err(|error| format!("{name}: {error}"))?
            .count();
        if sample_count == 0 {
            return Err(format!("{name}: decoder returned no samples"));
        }
        println!("intro_freeroam_harness: decoded {name} samples={sample_count}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let config = env::args()
        .nth(1)
        .unwrap_or_else(|| "config/original_gta3_path.txt".to_string());
    println!("intro_freeroam_harness: config={config}");

    let install = match OriginalGtaInstall::from_config_path(&config) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("FAIL: install: {e}");
            return ExitCode::from(2);
        }
    };
    if let Err(error) = validate_intro_mission_audio(&install) {
        eprintln!("FAIL: original intro mission audio decode: {error}");
        return ExitCode::from(3);
    }
    let mut file_mgr = install.file_mgr();
    let mut cd_stream = CdStream::init(5);
    let mut file_loader = CFileLoader::default();
    if let Err(error) = file_loader.load_default_and_dat_plans(
        &install,
        &mut file_mgr,
        &mut cd_stream,
        "DATA\\GTA3.DAT",
    ) {
        eprintln!("FAIL: production DAT/IDE/IPL load: {error}");
        return ExitCode::from(3);
    }
    let mut streaming = CStreaming::default();
    if let Err(error) = streaming.load_cd_directory(&cd_stream, &mut file_loader) {
        eprintln!("FAIL: production streaming directory load: {error}");
        return ExitCode::from(3);
    }
    file_loader.ensure_loose_txds_loaded_with_streaming(&mut file_mgr, Some(&mut streaming));
    let mut scripts = match CTheScripts::init(&mut file_mgr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FAIL: MAIN.SCM: {e}");
            return ExitCode::from(3);
        }
    };

    let mut general = CGeneral::default();
    let mut clock = CClock::initialise(1000, 0);
    let mut bridge = ScriptGameplayBridge::default();
    let mut camera = {
        let mut c = CCamera::default();
        c.Init();
        c
    };
    let mut weather = CWeather::init();
    let mut gangs = CGangs::initialise();
    let mut stats = CStats::init();
    let mut cutscene = CCutsceneMgr::initialise();
    let mut world_players = StagedWorldPlayers::default();
    let mut game_state = CGameState::default();
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
    apply_script_effects_to_world_with_streaming(
        &mut scripts,
        &mut bridge,
        Some(&mut game_state),
        Some(&mut stats),
        Some(&mut cutscene),
        Some(&mut streaming),
        Some(&mut camera),
        Some(&mut weather),
        Some(&mut gangs),
        None,
        None,
        Some(&mut world_players),
        Some(&mut general),
        None,
        Some(&mut file_loader),
        Some(&mut cd_stream),
        None,
        0,
        None,
    );
    // re3 `CREATE_PLAYER` immediately performs
    // RequestSpecialModel(MI_PLAYER, "player") + LoadAllRequestedModels.
    // The Bevy app's `ensure_player_special_model_loaded` system owns that
    // synchronous drain; reproduce the same production step in this headless
    // loop instead of fabricating HAS_MODEL_LOADED state.
    streaming.request_special_model(
        re3bevy_game::modelinfo::ModelIndices::MI_PLAYER,
        "player",
        re3bevy_game::core::Streaming::STREAMFLAGS_DONT_REMOVE
            | re3bevy_game::core::Streaming::STREAMFLAGS_DEPENDENCY,
        &mut file_loader,
    );
    if let Err(error) = file_loader.load_all_requested_models(&mut streaming, &mut cd_stream, false)
    {
        eprintln!("FAIL: CREATE_PLAYER synchronous model load: {error}");
        return ExitCode::from(6);
    }

    if !bridge.player_spawned {
        eprintln!("FAIL: no CREATE_PLAYER");
        return ExitCode::from(6);
    }
    let ped = world_players.player(bridge.player_index);
    if ped.is_none() {
        eprintln!("FAIL: no live ped");
        return ExitCode::from(7);
    }
    let spawn = ped.unwrap().position();
    println!(
        "intro_freeroam_harness: CREATE_PLAYER ped@({:.3},{:.3},{:.3})",
        spawn.x, spawn.y, spawn.z
    );

    // Multi-frame InGame-shaped loop: Process + apply + cutscene Update + flyby.
    let mut time_ms = 0u32;
    const STEP_MS: u32 = 16;
    const MAX_FRAMES: u32 = 12_000;
    let mut t0_ms: Option<i32> = None;
    let mut saw_started = false;
    let mut saw_time_increase = false;
    let mut saw_finished = false;
    let mut saw_clear = false;
    let mut saw_jb_started = false;
    let mut saw_jb_clear = false;
    let mut saw_visible_free_roam = false;
    let mut load_scene_residency_mismatches = 0usize;
    let mut ecs = HarnessEcs::new();
    let mut pools = CPools::initialise();

    for frame in 1..=MAX_FRAMES {
        time_ms = time_ms.saturating_add(STEP_MS);

        // Cutscene Update (production update_cutscene_mgr core).
        let was_running = cutscene.running;
        let offset_before = cutscene.cutscene_offset;
        let _skip = cutscene.update(STEP_MS as f32 * 0.001, false);
        if !was_running && cutscene.running {
            camera.set_cam_cutscene_offset(re3bevy_core::core::Coordinates::CVector::new(
                offset_before.x,
                offset_before.y,
                offset_before.z,
            ));
            camera.take_control_with_spline();
            camera.wide_screen_on = true;
            saw_started = true;
            saw_jb_started |= cutscene.cutscene_name.eq_ignore_ascii_case("jb");
            println!(
                "intro_freeroam_harness: cutscene running name='{}' frame={frame}",
                cutscene.cutscene_name
            );
        }
        if cutscene.running && camera.script_mode == CameraScriptMode::FlyBy {
            if let Err(e) = camera.update_cutscene_spline(STEP_MS as f32) {
                // Splines may be empty if asset missing — still exercise finish API later.
                if frame < 5 {
                    println!("intro_freeroam_harness: flyby note: {e}");
                }
            }
            if camera.cutscene_finished || (camera.position_along_spline - 1.0).abs() < 1e-4 {
                apply_cutscene_finish(&mut cutscene, &mut camera, &mut bridge);
                saw_finished = true;
            }
        }

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
        scripts.cutscene_time_ms = cutscene.get_cutscene_time_in_milliseconds();
        scripts.cutscene_finished = camera.cutscene_finished
            || (camera.position_along_spline - 1.0).abs() < 1e-4
            || cutscene.has_cutscene_finished(camera.position_along_spline);

        if cutscene.running {
            match t0_ms {
                None => t0_ms = Some(scripts.cutscene_time_ms),
                Some(t0) if scripts.cutscene_time_ms > t0 => saw_time_increase = true,
                _ => {}
            }
        }

        let pad0 = pads.pads[0];
        let player_entity = bridge.player_spawned.then_some((
            handle,
            ScriptEntitySnapshot {
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
            },
        ));
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
        let mut ground_probe = |_: f32, _: f32| pos.z;
        let mut camera_probe = |_: _, _: _| true;
        let mut pad_probe = |pad: i32, button: i32| {
            let p = if pad <= 0 { &pad0 } else { &pads.pads[1] };
            p.get_pad_state_button(button)
        };
        let mut entity_runtime = HarnessScriptEntityRuntime {
            ecs: &mut ecs,
            pools: &mut pools,
            file_loader: &file_loader,
            player_entity,
        };
        let mut flush_immediate_camera_effect = |scripts: &mut CTheScripts| {
            let effect = match scripts.pending_effects.last() {
                Some(re3bevy_game::control::Script::ScriptCommandEffect::DoFade { .. })
                | Some(re3bevy_game::control::Script::ScriptCommandEffect::SetFadingColour {
                    ..
                }) => scripts.pending_effects.pop(),
                _ => None,
            };
            let Some(effect) = effect else {
                return;
            };
            match effect {
                re3bevy_game::control::Script::ScriptCommandEffect::DoFade {
                    time_ms,
                    direction,
                } => camera.fade(time_ms as f32 / 1_000.0, direction),
                re3bevy_game::control::Script::ScriptCommandEffect::SetFadingColour { r, g, b } => {
                    camera.set_fade_colour(r, g, b)
                }
                _ => unreachable!(),
            }
        };

        // Production `process_active_scripts` mirrors CStreaming residency into
        // SCM-visible HAS_MODEL_LOADED / HAS_SPECIAL_CHARACTER_LOADED state.
        for (model_id, loaded) in scripts.model_loaded.iter_mut().enumerate() {
            *loaded = streaming.has_model_loaded(model_id);
        }
        for (slot, loaded) in scripts.special_char_loaded.iter_mut().enumerate() {
            *loaded = streaming
                .has_model_loaded(re3bevy_game::modelinfo::ModelIndices::MI_SPECIAL01 + slot);
        }

        if let Err(e) = scripts.process_active_scripts_with_entity_runtime_context_and_effect_flush(
            time_ms,
            STEP_MS as f32,
            1.0,
            &mut general,
            &mut clock,
            Some(&mut camera_probe),
            Some(&mut ground_probe),
            None,
            None,
            Some(&mut entity_runtime),
            Some(&mut player_probe),
            Some(&mut pad_probe),
            None,
            false,
            Some(&mut flush_immediate_camera_effect),
        ) {
            let msg = format!("{e:?}");
            if msg.contains("MissingPlayerProbe") || msg.contains("MissingPadProbe") {
                eprintln!("FAIL: probe hard-fail frame={frame}: {e:?}");
                return ExitCode::from(8);
            }
            // Continue past soft VM issues only if not probe — still report.
            eprintln!("WARN: Process frame={frame}: {e:?}");
        }
        camera.update_fade(STEP_MS as f32 * 0.001);
        scripts.camera_fading = camera.get_fading();

        let had_clear = scripts.pending_effects.iter().any(|e| {
            matches!(
                e,
                re3bevy_game::control::Script::ScriptCommandEffect::ClearCutscene
            )
        });
        let cleared_cutscene_name = had_clear.then(|| cutscene.cutscene_name.clone());
        let loaded_before_load_scene = scripts
            .pending_effects
            .iter()
            .any(|effect| {
                matches!(
                    effect,
                    re3bevy_game::control::Script::ScriptCommandEffect::LoadScene { .. }
                )
            })
            .then(|| {
                streaming
                    .info_for_model
                    .iter()
                    .enumerate()
                    .filter(|(model_id, _)| {
                        streaming.has_model_loaded(*model_id)
                            && file_loader
                                .model_info
                                .get_model_info(*model_id)
                                .is_some_and(|model| model.base().rw_object_loaded)
                    })
                    .map(|(model_id, _)| model_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if (2300..=2360).contains(&frame) || frame == 100 || frame == 2000 {
            let mission = scripts
                .active_scripts
                .iter()
                .copied()
                .find(|script_index| scripts.scripts[*script_index].is_mission_script);
            if let Some(index) = mission {
                let s = &scripts.scripts[index];
                let ip = s.ip as usize;
                let start = ip.saturating_sub(16);
                let end = (ip + 32).min(scripts.script_space.len());
                let bytes = scripts.script_space[start..end]
                    .iter()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!(
                    "DEBUG INTRO frame={frame} ip={ip} wake={} cond={} bytes={bytes}",
                    s.wake_time, s.cond_result
                );
            }
        }
        if frame <= 6 || frame % 1000 == 0 {
            let mission = scripts
                .active_scripts
                .iter()
                .copied()
                .find(|script_index| scripts.scripts[*script_index].is_mission_script);
            let mission_info = mission.map(|index| {
                let s = &scripts.scripts[index];
                let name: String = s
                    .script_name
                    .iter()
                    .take_while(|byte| **byte != 0)
                    .map(|byte| *byte as char)
                    .collect();
                format!("name={name} ip={}", s.ip)
            });
            if frame <= 6 || frame % 1000 == 0 {
                if let Some(index) = mission {
                    let ip = scripts.scripts[index].ip as usize;
                    let start = ip.saturating_sub(32);
                    let end = (ip + 48).min(scripts.script_space.len());
                    let bytes = scripts.script_space[start..end]
                        .iter()
                        .map(|byte| format!("{byte:02X}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!(
                        "DEBUG mission_window={start}..{end} wake={} cond={} bytes={bytes}",
                        scripts.scripts[index].wake_time, scripts.scripts[index].cond_result,
                    );
                }
            }
            let cut_effects = scripts
                .pending_effects
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        re3bevy_game::control::Script::ScriptCommandEffect::LoadCutscene { .. }
                            | re3bevy_game::control::Script::ScriptCommandEffect::StartCutscene
                    )
                })
                .count();
            let mut effect_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for e in &scripts.pending_effects {
                let name = format!("{:?}", e);
                let tag = name
                    .split('(')
                    .next()
                    .unwrap_or(&name)
                    .split('{')
                    .next()
                    .unwrap_or(&name)
                    .trim()
                    .to_string();
                *effect_counts.entry(tag).or_default() += 1;
            }
            println!(
                "DEBUG frame={frame} mission={mission_info:?} pending={} effects={:?} cut_effects={cut_effects} loaded={} running={}",
                scripts.pending_effects.len(),
                effect_counts,
                cutscene.loaded,
                cutscene.running,
            );
        }
        apply_script_effects_to_world_with_streaming(
            &mut scripts,
            &mut bridge,
            Some(&mut game_state),
            Some(&mut stats),
            Some(&mut cutscene),
            Some(&mut streaming),
            Some(&mut camera),
            Some(&mut weather),
            Some(&mut gangs),
            None,
            None,
            Some(&mut world_players),
            Some(&mut general),
            None,
            Some(&mut file_loader),
            Some(&mut cd_stream),
            None,
            time_ms,
            None,
        );
        load_scene_residency_mismatches += loaded_before_load_scene
            .into_iter()
            .filter(|model_id| {
                streaming.has_model_loaded(*model_id)
                    && file_loader
                        .model_info
                        .get_model_info(*model_id)
                        .is_none_or(|model| !model.base().rw_object_loaded)
            })
            .count();
        apply_script_player_clothing_effects(&mut scripts, &mut world_players);
        if let Err(error) = file_loader.update_streaming_around_position(
            &mut streaming,
            &mut cd_stream,
            [pos.x, pos.y, pos.z],
        ) {
            eprintln!("FAIL: production streaming frame={frame}: {error}");
            return ExitCode::from(9);
        }
        // CdStream is an actual worker thread. Give it wall-clock scheduling
        if had_clear {
            saw_clear = true;
            saw_jb_clear |= cleared_cutscene_name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case("jb"));
            println!("intro_freeroam_harness: CLEAR_CUTSCENE applied frame={frame} total_clears");
        }
        if saw_jb_clear && !bridge.playing_intro && !camera.fading && camera.fade_value <= 0.5 {
            saw_visible_free_roam = true;
            println!("intro_freeroam_harness: visible free roam frame={frame}");
            if let Some(player) = world_players.player(bridge.player_index) {
                let player_pos = player.position();
                let car_nodes = file_loader.path_find.num_car_path_nodes.max(0) as usize;
                let disabled = file_loader.path_find.path_nodes[..car_nodes]
                    .iter()
                    .filter(|node| node.disabled)
                    .count();
                let between_levels = file_loader.path_find.path_nodes[..car_nodes]
                    .iter()
                    .filter(|node| node.between_levels)
                    .count();
                let target_node = file_loader.path_find.find_node_closest_to_coors(
                    player_pos,
                    re3bevy_game::control::PathFind::PATH_CAR,
                    999_999.88,
                    false,
                    false,
                );
                let mut generated = 0usize;
                let mut close = 0usize;
                for _ in 0..1000 {
                    if let Some(spawn) = file_loader.path_find.new_generate_car_creation_coors(
                        player_pos.x,
                        player_pos.y,
                        1.0,
                        0.0,
                        40.0,
                        0.707,
                        false,
                        false,
                        &mut general,
                    ) {
                        generated += 1;
                        if file_loader.path_find.test_coors_closeness(
                            player_pos,
                            re3bevy_game::control::PathFind::PATH_CAR,
                            spawn.position,
                        ) {
                            close += 1;
                        }
                    }
                }
                println!(
                    "TRAFFIC_PATH_LIVE player={:?} target_node={} disabled={} between_levels={} generated={} close={}",
                    player_pos, target_node, disabled, between_levels, generated, close
                );
            }
            break;
        }

        if frame % 200 == 0 || cutscene.running != was_running {
            println!(
                "intro_freeroam_harness: frame={frame} running={} name='{}' time_ms={} finished={} intro={} control={} cam={:?}",
                cutscene.running,
                cutscene.cutscene_name,
                scripts.cutscene_time_ms,
                scripts.cutscene_finished,
                bridge.playing_intro,
                bridge.player_control_enabled,
                camera.script_mode
            );
        }
    }

    // A parity harness must prove the real MAIN.SCM path; it must never replace
    // a stalled mission with a direct/fake cutscene bootstrap.
    if !saw_started {
        eprintln!("FAIL: real MAIN.SCM never started the intro cutscene in budget");
        return ExitCode::from(9);
    } else if !saw_clear {
        eprintln!("FAIL: real MAIN.SCM did not emit CLEAR_CUTSCENE in budget");
        return ExitCode::from(10);
    } else if !saw_jb_started || !saw_jb_clear || !saw_visible_free_roam || cutscene.running {
        eprintln!(
            "FAIL: real MAIN.SCM did not complete JB and enter visible free-roam scene (started={saw_jb_started} clear={saw_jb_clear} visible={saw_visible_free_roam} running={})",
            cutscene.running
        );
        return ExitCode::from(10);
    }

    let ped_after = world_players
        .player(bridge.player_index)
        .map(|p| p.position());
    let ped_ok = ped_after.is_some_and(|p| p.x.abs() > 0.01 || p.y.abs() > 0.01);

    println!("intro_freeroam_harness: summary");
    println!("  saw_started={saw_started}");
    println!("  saw_time_increase={saw_time_increase}");
    println!("  saw_finished={saw_finished}");
    println!("  saw_clear={saw_clear}");
    println!("  saw_jb_started={saw_jb_started}");
    println!("  saw_jb_clear={saw_jb_clear}");
    println!("  saw_visible_free_roam={saw_visible_free_roam}");
    println!(
        "  playing_intro={} control={} visible={}",
        bridge.playing_intro, bridge.player_control_enabled, bridge.player_visible
    );
    println!(
        "  game_state.playing_intro={} cam_mode={:?}",
        game_state.playing_intro, camera.script_mode
    );
    println!("  ped_still_non_origin={ped_ok}");
    println!("  load_scene_residency_mismatches={load_scene_residency_mismatches}");
    println!(
        "  fade_value={} fading={}",
        camera.fade_value, camera.fading
    );

    if !saw_started {
        eprintln!("FAIL: cutscene never started (SCM or direct)");
        return ExitCode::from(9);
    }
    if !saw_time_increase && !saw_finished {
        eprintln!("FAIL: cutscene time did not advance and finish not observed");
        return ExitCode::from(10);
    }
    if bridge.playing_intro || game_state.playing_intro {
        eprintln!("FAIL: free-roam restore incomplete (intro flags)");
        return ExitCode::from(11);
    }
    if !ped_ok {
        eprintln!("FAIL: ped lost or at origin after cutscene path");
        return ExitCode::from(12);
    }
    if load_scene_residency_mismatches != 0 {
        eprintln!(
            "FAIL: LOAD_SCENE cleared {load_scene_residency_mismatches} shared model assets while streaming still reported them loaded"
        );
        return ExitCode::from(13);
    }
    if camera.script_mode != CameraScriptMode::FollowPlayer {
        // restore() should leave FollowPlayer
        eprintln!(
            "WARN: camera mode {:?} after clear (expected FollowPlayer)",
            camera.script_mode
        );
    }

    println!("intro_freeroam_harness: PASS");
    ExitCode::SUCCESS
}
