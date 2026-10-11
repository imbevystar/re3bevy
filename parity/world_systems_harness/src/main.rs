//! Non-production: exercise World FindGroundZ, CarGen range Process + parked insert,
//! Wanted thresholds, zone name lookup — away from SCM hot path.
//!
//! cargo run -p re3bevy_world_systems_harness

use bevy::ecs::system::SystemState;
use bevy::ecs::world::World;
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_assets::streaming::CdStream::CdStream;
use re3bevy_core::core::Coordinates::CVector;
use re3bevy_core::core::General::CGeneral;
use re3bevy_game::core::FileLoader::CFileLoader;
use re3bevy_game::core::Game::{ScriptEntityIndex, ScriptEntityWorld};
use re3bevy_game::core::Pools::CPools;
use re3bevy_game::core::Wanted::CWanted;
use re3bevy_game::core::Wanted::CWanted as PlayerWanted;
use re3bevy_game::vehicles::CarGen::{CTheCarGenerators, cargen_in_range};
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
        let mut entities = self
            .state
            .get_mut(&mut self.world)
            .expect("ScriptEntityWorld parameters must validate");
        f(&mut entities)
    }
}

fn main() -> ExitCode {
    let install = match OriginalGtaInstall::from_config_path("config/original_gta3_path.txt") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("FAIL: install: {e}");
            return ExitCode::from(2);
        }
    };
    let mut file_mgr = install.file_mgr();
    let mut cd_stream = CdStream::init(5);
    let mut file_loader = CFileLoader::default();
    if let Err(e) = file_loader.load_default_and_dat_plans(
        &install,
        &mut file_mgr,
        &mut cd_stream,
        "DATA\\GTA3.DAT",
    ) {
        eprintln!("FAIL: DAT: {e}");
        return ExitCode::from(3);
    }

    // --- FindGroundZ ---
    let spawn = CVector::new(811.875, -939.938, 35.75);
    let ground =
        file_loader
            .world
            .find_ground_z_for_coord(spawn.x, spawn.y, &file_loader.model_info);
    println!("ground_z={ground:.3}");
    if !(ground > -50.0 && ground < 500.0) {
        eprintln!("FAIL: ground_z out of world range");
        return ExitCode::from(4);
    }

    // --- Zone name ---
    let zone = file_loader
        .world
        .zones
        .zone_name_for_point([spawn.x, spawn.y, spawn.z]);
    println!("zone_name={zone}");
    if zone.is_empty() {
        eprintln!("FAIL: empty zone name");
        return ExitCode::from(5);
    }

    // --- Wanted thresholds (re3) ---
    let mut wanted = CWanted::default();
    wanted.SetWantedLevel(3);
    println!(
        "wanted level=3 => chaos={} stars={}",
        wanted.m_nChaos, wanted.m_nWantedLevel
    );
    if wanted.m_nChaos != 420 || wanted.m_nWantedLevel != 3 {
        eprintln!("FAIL: SetWantedLevel(3) chaos/stars mismatch");
        return ExitCode::from(6);
    }
    wanted.register_chaos(500);
    println!(
        "after +500 chaos={} stars={}",
        wanted.m_nChaos, wanted.m_nWantedLevel
    );
    if wanted.m_nWantedLevel < 3 {
        eprintln!("FAIL: chaos add should not drop stars below 3");
        return ExitCode::from(7);
    }

    // --- CarGen range ---
    let near = cargen_in_range(
        CVector::new(spawn.x + 110.0, spawn.y, spawn.z),
        spawn,
        CVector::ZERO,
        1.0,
        false,
        false,
    );
    let far = cargen_in_range(
        CVector::new(spawn.x + 200.0, spawn.y, spawn.z),
        spawn,
        CVector::ZERO,
        1.0,
        false,
        false,
    );
    let close = cargen_in_range(
        CVector::new(spawn.x + 5.0, spawn.y, spawn.z),
        spawn,
        CVector::ZERO,
        1.0,
        false,
        false,
    );
    println!("cargen near(110)={near} far(200)={far} close(5)={close}");
    if !near || far || close {
        eprintln!("FAIL: cargen range bands (expect near=true far=false close=false)");
        return ExitCode::from(8);
    }

    let mut gens = CTheCarGenerators::init();
    let gen_pos = CVector::new(spawn.x + 110.0, spawn.y, spawn.z);
    let idx = gens.create_car_generator(
        gen_pos, 90.0, 90, // model id placeholder (Infernus-ish range in GTA3)
        -1, -1, 0, 0, 0, 0, 0,
    );
    gens.switch_on_generator(idx, 1000);
    // re3 Process strides by 4 using ProcessCounter; generator 0 is hit on the 4th call.
    let mut intents = Vec::new();
    for _ in 0..4 {
        intents.extend(gens.process(2000, spawn, CVector::ZERO, false, 1.0));
    }
    println!(
        "cargen process active={} intents={}",
        gens.current_active_count,
        intents.len()
    );
    if intents.is_empty() {
        eprintln!("FAIL: expected spawn intent for switched-on generator in range");
        return ExitCode::from(9);
    }

    // --- Parked vehicle insert (re3 DoInternalProcessing automobile path) ---
    let mut pools = CPools::initialise();
    let mut ecs = HarnessEcs::new();
    let mut general = CGeneral::default();
    let intent = &intents[0];
    let ground_z = file_loader.world.find_ground_z_for_3d_coord(
        intent.position.x,
        intent.position.y,
        intent.position.z,
        &file_loader.model_info,
    );
    let model_info = file_loader
        .model_info
        .get_model_info(intent.model_index.max(0) as usize);
    let handle = ecs.with(|entities| {
        entities.insert_parked_car_from_generator(
            &mut pools,
            intent.model_index,
            intent.position,
            intent.angle_degrees,
            2000,
            model_info,
            None,
            &mut general,
            ground_z,
            false,
        )
    });
    let vehicle_count = ecs.with(|entities| entities.vehicle_count());
    println!("parked insert handle={handle:?} vehicles={vehicle_count} ground_z={ground_z:?}");
    let Some(handle) = handle else {
        eprintln!("FAIL: insert_parked_car_from_generator returned None");
        return ExitCode::from(10);
    };
    gens.note_spawn_attempt_finished(intent.generator_index, 2000, Some(handle));
    if gens.car_generator_array[intent.generator_index].vehicle_handle != handle {
        eprintln!("FAIL: note_spawn_attempt_finished did not stick handle on generator");
        return ExitCode::from(11);
    }
    let Some((placed_z, engine_on)) =
        ecs.with(|entities| entities.vehicle_position_and_engine(handle))
    else {
        eprintln!("FAIL: parked entity not automobile in ScriptEntityWorld");
        return ExitCode::from(12);
    };
    if engine_on {
        eprintln!("FAIL: parked car engine should be off");
        return ExitCode::from(13);
    }
    println!("parked car z={placed_z:.3} engine_on={engine_on}");

    // --- Player wanted stars (HUD source) ---
    let mut player_wanted = PlayerWanted::initialise();
    player_wanted.set_wanted_level(3);
    println!(
        "player wanted stars={} chaos={}",
        player_wanted.get_wanted_level(),
        player_wanted.m_nChaos
    );
    if player_wanted.get_wanted_level() != 3 || player_wanted.m_nChaos != 420 {
        eprintln!("FAIL: PlayerPed CWanted stars/chaos mismatch");
        return ExitCode::from(14);
    }

    // --- SWITCH_CAR_GENERATOR limited uses (re3 count ≤ 100) ---
    let mut gens2 = CTheCarGenerators::init();
    let idx2 =
        gens2.create_car_generator(CVector::new(0.0, 0.0, 0.0), 0.0, 90, 1, 2, 0, 50, 50, 0, 0);
    gens2.switch_car_generator(idx2, 3, 1000);
    if gens2.car_generator_array[idx2 as usize].uses_remaining != 3 {
        eprintln!(
            "FAIL: switch_car_generator uses expected 3 got {}",
            gens2.car_generator_array[idx2 as usize].uses_remaining
        );
        return ExitCode::from(15);
    }
    gens2.switch_car_generator(idx2, 0, 1000);
    if gens2.car_generator_array[idx2 as usize].uses_remaining != 0 {
        eprintln!("FAIL: switch off should clear uses");
        return ExitCode::from(16);
    }
    println!("switch_car_generator limited_uses+off OK");

    println!("world_systems_harness: PASS");
    ExitCode::SUCCESS
}
