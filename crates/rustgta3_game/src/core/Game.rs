//! re3 reference: references/re-GTA/src/core/Game.cpp
//! re3 reference: references/re-GTA/src/core/Game.h
//!
//! Application startup, loading and cross-domain world lifecycle.
//! Domain plugins own their system registrations; `GtaProcessSet` defines frame order.

use std::{io, sync::OnceLock, time::Instant};

use bevy::{
    ecs::{schedule::IntoScheduleConfigs, system::SystemParam},
    prelude::*,
};
use rustgta3_assets::{
    assets::{
        AnimationAssets::CAnimationAssets,
        FrontendAssets::CFrontendAssets,
        WorldAssets::{CHandlingDataAsset, CSurfaceTableAsset, CWeaponDataAsset},
    },
    install::{
        FileMgr::{CFileMgr, CSeekWhence},
        OriginalGtaInstall::OriginalGtaInstall,
    },
    streaming::CdStream::CdStream,
};
use rustgta3_core::core::{
    Clock::{CClock, update_gta_clock},
    GameState::{
        CGameInitialiseStep, CGameState, ELevelName, GtaAppState, GtaBootRoute, GtaLevelBootMode,
    },
    General::CGeneral,
    Schedules::GtaProcessSet,
    Timer::CTimer,
};

#[cfg(test)]
use crate::peds::PlayerPed::CPlayerPed;

use crate::{
    animation::{AnimManager::CAnimManager, CutsceneMgr::CCutsceneMgr},
    audio::{
        AudioScriptObject::cAudioScriptObjectPool,
        DMAudio::{
            GtaAudioFrameIntents, GtaMissionAudioPlayback, GtaMusicPlayback, GtaSfxPlayback,
            cDMAudio, enter_audio_pause, exit_audio_pause, service_frontend_dmaudio, },
        PedTalk::PedTalkState,
        audio_enums::AUDIOTYPE_WEATHER,
    },
    collision::{Collision::CCollision, TempColModels::CTempColModels},
    control::{
        Bridge::CBridge,
        CarCtrl::CCarCtrl,
        Darkel::{CDarkel, draw_darkel_messages},
        GameLogic::CGameLogic,
        Garages::{CGarages, draw_garage_messages},
        OnscreenTimer::{COnscreenTimer, GtaOnscreenTimerPlugin},
        PathFind::PathDataPreparePhase,
        Phones::{CPhoneInfo, update_phone_info},
        Pickups::{CPacManPickups, CPickups},
        Record::{CRecordDataForChase, CRecordDataForGame},
        Remote::CRemote,
        Replay::{
            CReplay, ReplayPedRuntimeState, ReplayRuntimeWorldState, ReplayVehicleRuntimeState,
        },
        Restart::CRestart,
        RoadBlocks::{CRoadBlocks},
        Script::CTheScripts,
        ScriptDebug::{
            GtaMissionDebugSwitcher, },
        TrafficLights::CTrafficLights,
    },
    core::{
        Accident::CAccidentManager,
        Camera::{CCamera, CameraScriptMode},
        Clock::{apply_clock_day_rollovers, sync_clock_input_from_pads},
        ControllerConfig::CControllerConfigManager,
        EnhancedInput::{GtaEnhancedInputPlugin, collect_gameplay_input},
        EventList::CEventList,
        FileLoader::{CFileLoader, IncrementalLevelPlanLoad},
        Fire::CFireManager,
        Frontend::{CMenuManager, GtaFrontendPlugin},
        IniFile::CIniFile,
        LocalCoop::LocalCoopPlugin,
        Pad::CPads,
        PlayerInfo::CPlayerInfo,
        Pools::CPools,
        References::CReferences,
        Stats::CStats,
        Streaming::CStreaming,
        User::CUserDisplay,
        World::CWorld,
    },
    core::{
        FrontendFlow::{
            FrontendSettingsSyncState, apply_frame_limit, apply_frontend_preferences,
            enter_save_zone_frontend, on_enter_frontend, on_enter_pause_frontend,
            on_exit_frontend, reload_frontend_language, sync_frontend_custom_cursor,
            sync_frontend_settings, sync_frontend_window_settings, update_frontend,
        },
        Movie::{self, StartupMoviePlayer},
    },
    modelinfo::VehicleModelInfo::CVehicleColours,
    objects::{Object::CObjectGlobalState, ParticleObject::CParticleObjectRuntime},
    peds::{
        Gangs::CGangs,
        Ped::{CPedAnimationOffsets, CPedRuntime},
        PedFight::CFightMoveTable,
        PedRoutes::CPedRoutes,
        PedType::{CPedStatsRuntime, CPedTypeRuntime},
        PlayerPed::{CWorldPlayerState, StagedWorldPlayers, WorldPlayerAccess},
        Population::{CPopulation, DEFAULT_MAX_NUMBER_OF_PEDS},
        VehicleRelations::{PedEntity, VehicleRelationsPlugin},
    },
    renderer::{
        Antennas::{CAntennas, update_antennas},
        Clouds::CClouds,
        Coronas::CCoronas,
        Credits::CCredits,
        Draw::CDraw,
        Font::CFont,
        Glass::{CGlass, GtaGlassPlugin},
        Hud::CHud,
        Particle::CParticle,
        ParticleMgr::cParticleSystemMgr,
        Rubbish::{CRubbish, update_rubbish},
        Shadows::CShadows,
        Skidmarks::{CSkidmarks, update_skidmarks},
        SpecialFX::CSpecialFX,
        Timecycle::CTimeCycle,
        WaterCannon::{
            CWaterCannons, update_water_cannons_runtime,
        },
        WaterLevel::CWaterLevel,
        Weather::{CWeather, update_weather},
    },
    save::{
        GenericGameStorage::CGenericGameStorage,
        MemoryCard::CMemoryCard,
        PCSave::C_PcSave,
        SaveRuntime::{PcSaveRequest, process_pending_pc_save},
    },
    text::{
        Messages::{CMessages, GtaMessagesPlugin},
        Pager::{CPager, GtaPagerPlugin},
        Text::CText,
    },
    vehicles::{
        CarGen::CTheCarGenerators, Cranes::CCranes, HandlingMgr::cHandlingDataMgr,
        Heli::CHeliRuntime, Plane::CPlaneRuntime,
    },
    weapons::{
        Explosion::{CExplosionPool},
        ProjectileInfo::{CProjectileInfoPool},
        ShotInfo::{CShotInfoPool},
        WeaponEffects::CWeaponEffects,
        WeaponInfo::CWeaponInfoStore,
    },
};
// Focused lifecycle tests exercise canonical domain systems directly.
#[cfg(test)]
use crate::{
    animation::CutsceneMgr::restore_after_cutscene_clear,
    collision::{
        ColModel::{CColBox, CColPoint, CColSphere, CColVector},
        Collision::{
            PlayerDynamicGroundSupport, PlayerStaticGroundSupport, ScriptObjectCollisionSnapshot,
            object_collision_substeps, player_object_blocks_motion,
        },
    },
    control::{
        Garages::{
            CGarage, CStoredCar, EGarageState, EGarageType, NUM_GARAGE_STORED_CARS,
            garage_tidy_vehicle_slots, store_and_remove_hideout_cars,
            stored_car_from_script_vehicle,
        },
        Pickups::{add_vehicle_pacman_load, reset_vehicle_pacman_load},
        Script::{
            materialize_pickup_object, remove_materialized_pickup, script_vehicle_is_stopped,
            set_all_occupants_of_car_leave_car,
        },
        ScriptDynamicEntities::apply_script_char_objective,
        ScriptWorldEffects::{SCM_REQUEST_MODEL_FLAGS, request_scm_model},
    },
    core::{
        Camera::apply_garage_camera_override,
        Pools::{CPoolEntry, CPoolEntryKind, CPoolHandle},
        World::EEntityType,
    },
    entities::{
        Entity::eEntityStatus,
        Physical::{CPhysicalCollisionOptions, GRAVITY_WORLD_UNITS_PER_SECOND_SQ, GTA_PHYSICS_HZ},
    },
    math::Matrix::CMatrix,
    modelinfo::{
        ModelIndices::{MI_PICKUP_BONUS, MI_PLAYER},
        VehicleModelInfo::eVehicleType,
    },
    objects::Object::{CObject, CollisionSpecialResponseCase},
    peds::{
        Ped::{CPed, PedState, eMoveState, eObjective},
        PedType::ePedType,
        PlayerPed::{
            PlayerPhysicalSurfaceMotion, should_start_player_run_stop,
            with_player_native_collision_speed,
        },
        VehicleRelations::InVehicle,
    },
    vehicles::{
        Automobile::CAutomobile,
        Vehicle::{MISSION_VEHICLE, RANDOM_VEHICLE, eCarLock},
    },
};
#[cfg(test)]
use rustgta3_core::core::Coordinates::CVector;

pub(crate) use crate::peds::PlayerPed::{
    player_air_ground_below, player_dynamic_ground_below, player_dynamic_ground_support,
    player_dynamic_support_from_air_probe, player_jump_dynamic_sphere_blocked,
    player_static_ground_support, prepare_player_physical_for_control, process_player_weapon,
    process_player_weapon_switch, update_player_current_phys_surface,
    update_player_on_foot_control,
};
pub(crate) use crate::peds::PlayerPed::{
    apply_player_ped_air_resistance, apply_player_running_land,
    apply_player_standing_update_position, begin_player_collision_frame,
    finish_player_collision_frame, player_can_start_jump_from_surface,
    player_check_if_in_air_has_ground, player_check_if_in_air_has_static_foot_sphere,
    player_current_surface_motion, player_ground_support_extension,
    service_player_air_state_before_collision, set_player_jump_animation,
};
pub(crate) use crate::peds::Population::{
    AmbientPedModels, AmbientTrafficModels,
};
pub(crate) use crate::collision::Collision::{
    process_script_entity_collisions, resolve_ped_static_motion, resolve_player_dynamic_motion,
    resolve_player_ped_static_collision_result, script_object_collision_snapshots,
};
pub(crate) use crate::control::Garages::update_garages;
pub(crate) use crate::animation::CutsceneMgr::{sync_cutscene_pad_controls, update_cutscene_mgr};
pub(crate) use crate::control::Pickups::update_pacman_runtime;
// Public compatibility paths used by developer tools. Domain modules own
// the definitions and system registration.
pub use crate::{
    animation::CutsceneMgr::apply_cutscene_finish,
    peds::{
        PlayerPed::{
            advance_player_jump_launch, begin_player_jump, update_player_jump_animation,
            update_player_movement_animation,
        },
        VehicleRelations::{PedAutomobileProfile, automobile_door_flag},
    },
    vehicles::Automobile::{
        AutomobileRoadContacts, automobile_road_contacts, automobile_static_collision_step,
        store_automobile_wheel_contacts,
    },
};
pub(crate) use crate::control::Script::script_vehicle_base_can_be_deleted;
pub(crate) use crate::control::Script::{
    apply_script_dynamic_entity_effects, flush_immediate_script_streaming_effect, update_game_logic,
};
pub(crate) use crate::control::Script::{
    ScriptPickupIndex, ScriptRadarBlipIndex, ScriptTaxiLightState,
};
// Parity harness facade: external bins import these via `rustgta3_game::core::Game`.
pub use crate::control::Script::{
    ScriptEntityIndex, ScriptEntityWorld, ScriptGameplayBridge, ScriptVehicleEntity,
    apply_script_effects_to_world, apply_script_effects_to_world_with_streaming,
    apply_script_player_clothing_effects,
};
pub use crate::core::Camera::snap_follow_camera_resource;
pub(crate) use crate::control::Darkel::update_darkel;
pub(crate) use crate::vehicles::Heli::update_heli_runtime;
pub(crate) use crate::control::Bridge::update_bridge;
pub(crate) use crate::control::CarRails::{
    find_links_to_go_with_these_nodes, join_car_with_road_system,
    join_car_with_road_system_goto_coors,
};
pub(crate) use crate::core::Pad::{update_gameplay_mouse_capture, update_pads_from_bevy_input};
pub(crate) use crate::collision::Collision::integrate_physical_matrix;
pub(crate) use crate::control::Replay::service_replay_playback_runtime;
pub(crate) use crate::save::MemoryCard::process_pending_pc_save_load;
pub(crate) use crate::renderer::Timecycle::update_timecycle;
pub(crate) use crate::core::Zones::update_zones;
pub(crate) use crate::collision::Collision::update_collision_runtime;
pub(crate) use crate::core::EventList::update_event_list;
pub(crate) use crate::objects::ParticleObject::update_particle_objects;
pub(crate) use crate::core::Accident::update_accident_manager;
pub(crate) use crate::core::ZoneCull::update_cull_zone_runtime;
pub(crate) use crate::core::Camera::{focused_player_vehicle_handle, follow_script_player_camera};
pub use crate::core::World::{WORLD_SPAWN_RADIUS, count_world_entities_near};
pub(crate) use crate::animation::CutsceneMgr::cutscene_model_snapshot;
pub(crate) use crate::core::FileLoader::sync_file_loader_clock;
pub(crate) use crate::control::Script::{
    ScriptObjectEntity, ScriptObjectIdentity, ScriptObjectStore, ScriptPedEntity, remove_script_object,
    spawn_script_object,
};
#[cfg(test)]
pub(crate) use crate::control::Script::{
    script_entity_test_world, spawn_script_test_ped, spawn_script_test_vehicle,
    with_script_entity_test_world,
};
#[cfg(test)]
use crate::{
    control::Script::{apply_script_pacman_effects, process_active_scripts},
    objects::Object::process_script_vehicle_debris,
};
pub const MAX_CDCHANNELS: usize = 5;

/// Opt-in wall-clock timing for gameplay systems that can become hot when
/// population grows. This remains dormant unless the audit environment
/// variable is set, so the normal GTA timer and system order are untouched.
pub(crate) struct GameplayAuditTimer {
    label: &'static str,
    started: Option<Instant>,
}

impl GameplayAuditTimer {
    pub(crate) fn start(label: &'static str) -> Self {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        let enabled = *ENABLED.get_or_init(|| {
            cfg!(feature = "profiling")
                || std::env::var_os("RUSTGTA3_GAMEPLAY_AUDIT").is_some_and(|value| value != "0")
        });
        Self {
            label,
            started: enabled.then(Instant::now),
        }
    }
}

impl Drop for GameplayAuditTimer {
    fn drop(&mut self) {
        let Some(started) = self.started else {
            return;
        };
        let elapsed = started.elapsed();
        if elapsed.as_micros() >= 500 {
            #[cfg(feature = "profiling")]
            eprintln!(
                "GAMEPLAY_AUDIT system={} elapsed_ms={:.3}",
                self.label,
                elapsed.as_secs_f64() * 1_000.0
            );
            #[cfg(not(feature = "profiling"))]
            bevy::log::info!(
                target: "rustgta3_gameplay_audit",
                system = self.label,
                elapsed_ms = elapsed.as_secs_f64() * 1_000.0,
                "gameplay audit"
            );
        }
    }
}

/// Native window capture state for re3's gameplay mouse camera. The cursor is
/// locked only during active gameplay; Escape releases it and a click captures
/// it again, which maps cleanly to desktop and touch/pointer platforms.
#[derive(Debug, Clone, Copy, Resource)]
pub struct GameplayCursorCapture {
    pub captured: bool,
    pub just_captured: bool,
}

impl Default for GameplayCursorCapture {
    fn default() -> Self {
        Self {
            captured: false,
            just_captured: false,
        }
    }
}


impl Default for ScriptGameplayBridge {
    fn default() -> Self {
        Self {
            weapon_data_deferrals: 0,
            player_spawned: false,
            player_index: 0,
            player_position: rustgta3_core::core::Coordinates::CVector::ZERO,
            player_heading_degrees: 0.0,
            // re3 default: player visible until cutscene hides them
            player_visible: true,
            player_control_enabled: true,
            cutscene_name: String::new(),
            cutscene_requested: false,
            cutscene_started: false,
            widescreen: false,
            playing_intro: false,
            splash_screen_name: String::new(),
        }
    }
}


#[derive(Debug, Resource)]
pub struct CGameBeforeRw {
    pub file_mgr: CFileMgr,
    pub cd_stream: CdStream,
    pub version_name: String,
}

#[derive(Debug, Resource)]
pub struct CGameAfterRw {
    pub timer: CTimer,
}

#[derive(Debug, Clone, Resource)]
pub struct CGameStartupError {
    pub stage: CGameInitialiseOnceBeforeRwStep,
    pub message: String,
}

#[derive(Debug, Clone, Resource)]
pub struct CGameInitialiseError {
    pub stage: CGameInitialiseStep,
    pub message: String,
}

#[derive(Debug, Clone, Default, Resource)]
pub struct CGameInitialiseTrace {
    pub completed_steps: Vec<CGameInitialiseStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CGameInitialiseOnceBeforeRwStep {
    OriginalGtaInstallPath,
    FileMgrInitialise,
    CdStreamInit,
    ValidateVersion,
}

pub struct GtaGamePlugin;

/// The isolated developer arena owns keyboard/mouse/gamepad input while it is
/// open. This lives in the game crate so frontend/movie systems can share the
/// same schedule-level gate without depending on the arena crate.
#[derive(Debug, Clone, Copy, Resource, Default, PartialEq, Eq)]
pub struct GtaInputCapture {
    pub developer_arena: bool,
}

pub(crate) fn frontend_or_pause_active(
    state: Res<State<GtaAppState>>,
    capture: Option<Res<GtaInputCapture>>,
) -> bool {
    matches!(state.get(), GtaAppState::Frontend | GtaAppState::PauseMenu)
        && !capture.is_some_and(|capture| capture.developer_arena)
}

fn gta_movie_input_allowed(capture: Option<Res<GtaInputCapture>>) -> bool {
    !capture.is_some_and(|capture| capture.developer_arena)
}

#[derive(SystemParam)]
pub struct PcSaveLoadParams<'w, 's> {
    pub(crate) world_players: crate::peds::PlayerPed::CWorldPlayers<'w, 's>,
    pub(crate) general: ResMut<'w, CGeneral>,
    pub(crate) script_bridge: ResMut<'w, ScriptGameplayBridge>,
    pub(crate) script_entities: ScriptEntityWorld<'w, 's>,
    pub(crate) handling: Res<'w, cHandlingDataMgr>,
    pub(crate) taxi_lights: Res<'w, ScriptTaxiLightState>,
    pub(crate) car_ctrl: Option<ResMut<'w, CCarCtrl>>,
    pub(crate) memory_card: ResMut<'w, CMemoryCard>,
    pub(crate) storage: ResMut<'w, CGenericGameStorage>,
    pub(crate) pc_save: ResMut<'w, C_PcSave>,
    pub(crate) menu: Option<ResMut<'w, CMenuManager>>,
    pub(crate) game_state: ResMut<'w, CGameState>,
    pub(crate) file_loader: ResMut<'w, CFileLoader>,
    pub(crate) streaming: ResMut<'w, CStreaming>,
    pub(crate) before_rw: Option<ResMut<'w, CGameBeforeRw>>,
    pub(crate) restart: Option<ResMut<'w, CRestart>>,
    pub(crate) gangs: Option<ResMut<'w, CGangs>>,
    pub(crate) stats: Option<ResMut<'w, CStats>>,
    pub(crate) car_generators: Option<ResMut<'w, CTheCarGenerators>>,
    pub(crate) phone_info: Option<ResMut<'w, CPhoneInfo>>,
    pub(crate) ped_type: Option<ResMut<'w, CPedTypeRuntime>>,
    pub(crate) audio_script_objects: Option<ResMut<'w, cAudioScriptObjectPool>>,
    pub(crate) player_info: Option<ResMut<'w, CPlayerInfo>>,
    pub(crate) pickups: Option<ResMut<'w, CPickups>>,
    pub(crate) garages: Option<ResMut<'w, CGarages>>,
    pub(crate) particles: Option<ResMut<'w, CParticleObjectRuntime>>,
    pub(crate) cranes: Option<ResMut<'w, CCranes>>,
    pub(crate) pools: Option<ResMut<'w, CPools>>,
    pub(crate) replay_runtime: ResMut<'w, ReplayRuntimeWorldState>,
    pub(crate) scripts: Option<ResMut<'w, CTheScripts>>,
    pub(crate) clock: Option<ResMut<'w, CClock>>,
    pub(crate) timer: Option<ResMut<'w, CTimer>>,
    pub(crate) camera: Option<ResMut<'w, CCamera>>,
    pub(crate) pads: Option<ResMut<'w, CPads>>,
    pub(crate) weather: Option<ResMut<'w, CWeather>>,
    pub(crate) next_state: ResMut<'w, NextState<GtaAppState>>,
}

#[derive(SystemParam)]
struct GameLevelClockParams<'w> {
    clock: ResMut<'w, CClock>,
    timer: Res<'w, CTimer>,
}

#[derive(SystemParam)]
struct RestartRuntimeResources<'w> {
    player_info: Option<ResMut<'w, CPlayerInfo>>,
    game_logic: Option<ResMut<'w, CGameLogic>>,
    restart: Option<ResMut<'w, CRestart>>,
    pads: Option<ResMut<'w, CPads>>,
    remote: Option<ResMut<'w, CRemote>>,
    references: Option<ResMut<'w, CReferences>>,
    fire_manager: Option<ResMut<'w, CFireManager>>,
    explosions: Option<ResMut<'w, CExplosionPool>>,
    projectiles: Option<ResMut<'w, CProjectileInfoPool>>,
    shots: Option<ResMut<'w, CShotInfoPool>>,
    bullets: Option<ResMut<'w, crate::weapons::BulletInfo::CBulletInfoPool>>,
    particles: Option<ResMut<'w, CParticle>>,
    particle_objects: Option<ResMut<'w, CParticleObjectRuntime>>,
    replay_runtime: ResMut<'w, ReplayRuntimeWorldState>,
    dmaudio: Option<ResMut<'w, cDMAudio>>,
    weather: Option<ResMut<'w, CWeather>>,
    user_display: Option<ResMut<'w, CUserDisplay>>,
    messages: Option<ResMut<'w, CMessages>>,
    hud: Option<ResMut<'w, CHud>>,
    timecycle: Option<ResMut<'w, CTimeCycle>>,
}

impl Plugin for GtaGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::collision::VehicleSpatial::GtaCollisionPlugin)
            .init_resource::<CFileLoader>()
            .init_resource::<crate::vehicles::BoatControl::BoatControlDiagnostics>()
            .init_resource::<CStreaming>()
            .init_resource::<CGameInitialiseTrace>()
            .init_resource::<CGenericGameStorage>()
            .init_resource::<C_PcSave>()
            .init_resource::<CMemoryCard>()
            .init_resource::<PcSaveRequest>()
            .init_resource::<ReplayRuntimeWorldState>()
            .init_resource::<GtaAudioFrameIntents>()
            .init_resource::<GtaMissionAudioPlayback>()
            .init_resource::<GtaMusicPlayback>()
            .init_resource::<GtaSfxPlayback>()
            .init_resource::<PedTalkState>()
            .init_resource::<CExplosionPool>()
            .init_resource::<CProjectileInfoPool>()
            .init_resource::<crate::weapons::BulletInfo::CBulletInfoPool>()
            .init_resource::<CShotInfoPool>()
            .init_resource::<crate::core::FrontendFlow::FrontendMenuUi>()
            .init_resource::<crate::core::FrontendFlow::FrontendTxdTextures>()
            .init_resource::<FrontendSettingsSyncState>()
            .init_resource::<StartupMoviePlayer>()
            .init_resource::<crate::core::LoadingScreen::FrontendLoadingScreen>()
            .init_resource::<ScriptGameplayBridge>()
            .init_resource::<ScriptEntityIndex>()
            .init_resource::<ScriptTaxiLightState>()
            .init_resource::<ScriptPickupIndex>()
            .init_resource::<ScriptRadarBlipIndex>()
            .init_resource::<CObjectGlobalState>()
            .init_resource::<CWorldPlayerState>()
            .init_resource::<CRemote>()
            .init_resource::<GtaInputCapture>()
            .init_resource::<CPads>()
            .init_resource::<COnscreenTimer>()
            .init_resource::<GtaLevelBootMode>()
            .init_resource::<GameplayCursorCapture>()
            .init_resource::<GtaMissionDebugSwitcher>()
            .add_plugins(crate::core::Fire::GtaFirePlugin)
            .add_plugins((
                crate::control::ScriptPlugin::GtaScriptPlugin,
                crate::peds::PedSimulationPlugin::GtaPedSimulationPlugin,
                crate::vehicles::VehicleSimulationPlugin::GtaVehicleSimulationPlugin,
                crate::weapons::WeaponSimulationPlugin::GtaWeaponSimulationPlugin,
                crate::audio::AudioPlugin::GtaAudioPlugin,
                crate::objects::ObjectSimulationPlugin::GtaObjectSimulationPlugin,
                crate::core::Camera::GtaCameraPlugin,
                GtaEnhancedInputPlugin,
                LocalCoopPlugin,
                GtaFrontendPlugin,
                GtaMessagesPlugin,
                GtaPagerPlugin,
                GtaOnscreenTimerPlugin,
                VehicleRelationsPlugin,
                crate::core::Cheats::GtaCheatsPlugin,
            ))
            .add_plugins(crate::control::PickupRuntime::GtaPickupPlugin)
            .add_plugins(GtaGlassPlugin)
            .add_systems(
                Startup,
                (
                    initialise_once_before_rw_resource,
                    initialise_pc_save_state.after(initialise_once_before_rw_resource),
                    initialise_once_after_rw_data_resource.after(initialise_pc_save_state),
                    initialise_pre_level_runtime_resource
                        .after(initialise_once_after_rw_data_resource),
                ),
            )
            .add_systems(OnEnter(GtaAppState::LogoMpeg), Movie::setup_logo_movie)
            .add_systems(OnExit(GtaAppState::LogoMpeg), Movie::cleanup_movie)
            .add_systems(
                Update,
                Movie::update_logo_movie
                    .run_if(in_state(GtaAppState::LogoMpeg))
                    .run_if(gta_movie_input_allowed),
            )
            .add_systems(OnEnter(GtaAppState::IntroMpeg), Movie::setup_intro_movie)
            .add_systems(OnExit(GtaAppState::IntroMpeg), Movie::cleanup_movie)
            .add_systems(
                Update,
                Movie::update_intro_movie
                    .run_if(in_state(GtaAppState::IntroMpeg))
                    .run_if(gta_movie_input_allowed),
            )
            .add_systems(
                OnEnter(GtaAppState::LoadingFrontend),
                crate::core::LoadingScreen::enter_frontend_loading_screen,
            )
            .add_systems(
                Update,
                crate::core::LoadingScreen::present_frontend_loading_screen
                    .run_if(in_state(GtaAppState::LoadingFrontend)),
            )
            .add_systems(
                OnExit(GtaAppState::LoadingFrontend),
                crate::core::LoadingScreen::exit_frontend_loading_screen,
            )
            .add_systems(OnEnter(GtaAppState::Frontend), on_enter_frontend)
            .add_systems(
                Update,
                (
                    sync_frontend_settings,
                    sync_frontend_window_settings.after(sync_frontend_settings),
                    process_pending_pc_save
                        .before(update_frontend)
                        .run_if(in_state(GtaAppState::PauseMenu)),
                    update_frontend.after(sync_frontend_window_settings),
                    reload_frontend_language.after(update_frontend),
                    sync_frontend_custom_cursor.after(update_frontend),
                    apply_frontend_preferences
                        .after(update_frontend)
                        .after(sync_frontend_window_settings),
                )
                    .run_if(frontend_or_pause_active),
            )
            .add_systems(Update, (service_frontend_dmaudio.after(update_frontend),)
                    .run_if(frontend_or_pause_active))
            .add_systems(OnExit(GtaAppState::Frontend), on_exit_frontend)
            .add_systems(
                OnEnter(GtaAppState::PauseMenu),
                (enter_audio_pause, on_enter_pause_frontend).chain(),
            )
            .add_systems(
                OnExit(GtaAppState::PauseMenu),
                (exit_audio_pause, on_exit_frontend).chain(),
            )
            .add_systems(First, apply_frame_limit.before(bevy::time::TimeSystems))
            .add_systems(
                OnEnter(GtaAppState::LoadingGame),
                (
                    // re3 settings are global runtime state. Keep the direct
                    // New Game binary and the visible frontend path identical
                    // before any level/world work starts.
                    sync_frontend_settings,
                    sync_frontend_window_settings,
                    reset_game_level_data_for_loading,
                    apply_frontend_preferences,
                    clear_gameplay_entities_for_loading,
                    begin_game_level_loading,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    advance_game_level_loading,
                    finalise_game_level_loading.after(advance_game_level_loading),
                )
                    .run_if(in_state(GtaAppState::LoadingGame)),
            )

            .add_systems(
                Update,
                process_pending_pc_save_load.run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                enter_save_zone_frontend
                    .after(apply_script_dynamic_entity_effects)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                service_replay_playback_runtime
                    .in_set(GtaProcessSet::Replay)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_cutscene_mgr
                    .in_set(GtaProcessSet::Cutscene)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                sync_cutscene_pad_controls
                    .after(update_cutscene_mgr)
                    .in_set(GtaProcessSet::Cutscene)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    collect_gameplay_input,
                    update_pads_from_bevy_input.after(collect_gameplay_input),
                    sync_clock_input_from_pads.after(update_pads_from_bevy_input),
                    update_gameplay_mouse_capture.after(update_pads_from_bevy_input),
                    crate::core::Cheats::process_cheats.after(update_pads_from_bevy_input),
                )
                    .in_set(GtaProcessSet::Pad)
                    .run_if(in_state(GtaAppState::InGame)),
            )

            .add_systems(Update, (update_pacman_runtime.after(update_heli_runtime),)
                    .in_set(GtaProcessSet::Script)
                    .run_if(in_state(GtaAppState::InGame)))

            .add_systems(
                Update,
                update_darkel
                    .in_set(GtaProcessSet::Darkel)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    draw_darkel_messages,
                    draw_garage_messages.after(draw_darkel_messages),
                )
                    .in_set(GtaProcessSet::Render2d)
                    .run_if(in_state(GtaAppState::InGame)),
            )









            .add_systems(
                Update,
                (
                    apply_clock_day_rollovers.after(update_gta_clock),
                    sync_file_loader_clock.after(apply_clock_day_rollovers),
                )
                    .in_set(GtaProcessSet::Clock),
            )
            .add_systems(
                Update,
                update_cull_zone_runtime
                    // C++ calls CCullZones::Update after CTimeCycle and the
                    // camera has processed the current frame.  Running this
                    // in the early Zones set sampled the previous camera
                    // pose, so NORAIN/NOPOLICE/subway flags lagged one frame
                    // and timed map visibility could use the wrong zone.
                    .in_set(GtaProcessSet::CullZones)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_zones
                    .in_set(GtaProcessSet::Zones)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_weather
                    .in_set(GtaProcessSet::Weather)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_collision_runtime.in_set(GtaProcessSet::Collision),
            )
            .add_systems(Update, update_event_list.in_set(GtaProcessSet::Events))
            .add_systems(
                Update,
                update_particle_objects.in_set(GtaProcessSet::Particles),
            )
            .add_systems(
                Update,
                update_accident_manager.in_set(GtaProcessSet::Accidents),
            )
            .add_systems(
                Update,
                update_skidmarks
                    .in_set(GtaProcessSet::Skidmarks),
            )
            .add_systems(Update, update_antennas.in_set(GtaProcessSet::Antennas))
            .add_systems(
                Update,
                update_water_cannons_runtime
                    .in_set(GtaProcessSet::WaterCannons)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                update_garages
                    .in_set(GtaProcessSet::Garages)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(Update, update_rubbish.in_set(GtaProcessSet::Rubbish))
            .add_systems(Update, update_timecycle.in_set(GtaProcessSet::TimeCycle))
            .add_systems(Update, update_game_logic.in_set(GtaProcessSet::GameLogic))
            .add_systems(Update, update_bridge.in_set(GtaProcessSet::Bridge))
            .add_systems(Update, update_phone_info.in_set(GtaProcessSet::Phones))

            ;
    }
}








pub struct CGame;

impl CGame {
    pub fn initialise_once_before_rw(install: &OriginalGtaInstall) -> io::Result<CGameBeforeRw> {
        let mut file_mgr = install.file_mgr();
        let cd_stream = CdStream::init(MAX_CDCHANNELS);
        let version_name = validate_version(&mut file_mgr)?;

        Ok(CGameBeforeRw {
            file_mgr,
            cd_stream,
            version_name,
        })
    }

    pub fn initialise_once_after_rw() -> CGameAfterRw {
        CGameAfterRw {
            timer: CTimer::initialise(),
        }
    }

    pub fn initialise(game: &mut CGameState, dat_file: &str) {
        game.dat_file = dat_file.to_owned();
        game.curr_level = ELevelName::Industrial;
    }
}

pub fn validate_version(file_mgr: &mut CFileMgr) -> io::Result<String> {
    let fd = file_mgr.open_file("models\\coll\\peds.col", "rb")?;
    file_mgr.seek(fd, 100, CSeekWhence::Start)?;

    let mut decoded = Vec::with_capacity(128);
    for _ in 0..128 {
        let mut byte = [0];
        if file_mgr.read(fd, &mut byte)? == 0 {
            break;
        }

        let decoded_byte = byte[0].wrapping_sub(23);
        if decoded_byte == 0 {
            break;
        }

        decoded.push(decoded_byte);
        file_mgr.seek(fd, 99, CSeekWhence::Current)?;
    }
    file_mgr.close_file(fd)?;

    const SIGNATURE: &[u8] = b"grandtheftauto3";
    if decoded.len() >= SIGNATURE.len() && &decoded[..SIGNATURE.len()] == SIGNATURE {
        Ok(String::from_utf8_lossy(&decoded[SIGNATURE.len()..]).into_owned())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid GTA III install version signature",
        ))
    }
}

fn initialise_once_before_rw_resource(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GtaAppState>>,
    mut trace: ResMut<CGameInitialiseTrace>,
) {
    eprintln!("rustgta3: CGame startup InitialiseOnceBeforeRW begin");
    info!("CGame startup: InitialiseOnceBeforeRW begin");
    match initialise_once_before_rw_for_bevy() {
        Ok((install, before_rw)) => {
            eprintln!("rustgta3: CGame startup InitialiseOnceBeforeRW complete");
            info!("CGame startup: InitialiseOnceBeforeRW complete");
            trace.completed_steps.extend([
                CGameInitialiseStep::FileMgrInitialise,
                CGameInitialiseStep::CdStreamInit,
                CGameInitialiseStep::ValidateVersion,
            ]);
            commands.insert_resource(install);
            commands.insert_resource(before_rw);
        }
        Err(error) => {
            eprintln!(
                "rustgta3: CGame startup InitialiseOnceBeforeRW failed at {:?}: {}",
                error.stage, error.message
            );
            error!(
                "CGame startup: InitialiseOnceBeforeRW failed at {:?}: {}",
                error.stage, error.message
            );
            commands.insert_resource(error);
            next_state.set(GtaAppState::Error);
        }
    }
}

fn initialise_once_before_rw_for_bevy()
-> Result<(OriginalGtaInstall, CGameBeforeRw), CGameStartupError> {
    let install = OriginalGtaInstall::from_config_file().map_err(|error| CGameStartupError {
        stage: CGameInitialiseOnceBeforeRwStep::OriginalGtaInstallPath,
        message: error.to_string(),
    })?;
    info!(target: "gta_assets", root = %install.root().display(), "resolved original GTA III installation");

    let before_rw =
        CGame::initialise_once_before_rw(&install).map_err(|error| CGameStartupError {
            stage: CGameInitialiseOnceBeforeRwStep::ValidateVersion,
            message: error.to_string(),
        })?;

    Ok((install, before_rw))
}

fn initialise_pc_save_state(
    install: Option<Res<OriginalGtaInstall>>,
    mut storage: ResMut<CGenericGameStorage>,
    mut pc_save: ResMut<C_PcSave>,
    mut memory_card: ResMut<CMemoryCard>,
) {
    let Some(install) = install else {
        return;
    };
    let save_root = match rustgta3_assets::install::RuntimePaths::userfiles_root(install.root()) {
        Ok(root) => root,
        Err(error) => {
            error!("C_PcSave startup: save directory preparation failed: {error}");
            return;
        }
    };
    C_PcSave::SetSaveDirectory(&mut storage, &save_root);
    let _ = pc_save.PopulateSlotInfo(&mut storage, &save_root);
    memory_card.Init();
    C_PcSave::SyncMemoryCardSlots(&storage, &mut memory_card);
    memory_card.b_FoundRecentSavedGameWantToLoad = storage.b_FoundRecentSavedGameWantToLoad;
}

fn initialise_once_after_rw_data_resource(
    before_rw: Option<ResMut<CGameBeforeRw>>,
    mut file_loader: ResMut<CFileLoader>,
    mut trace: ResMut<CGameInitialiseTrace>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GtaAppState>>,
) {
    let Some(mut before_rw) = before_rw else {
        return;
    };
    let before_rw = &mut *before_rw;
    eprintln!("rustgta3: CGame startup InitialiseOnceAfterRW data begin");
    info!("CGame startup: InitialiseOnceAfterRW data begin");

    match CFrontendAssets::load_default_text(&mut before_rw.file_mgr) {
        Ok(frontend_assets) => {
            commands.insert_resource(CText::from_frontend_assets(&frontend_assets));
            commands.insert_resource(frontend_assets);
            trace.completed_steps.push(CGameInitialiseStep::TextLoad);
        }
        Err(error) => {
            error!("CGame startup: TextLoad failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::TextLoad,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    file_loader.temp_col_models = CTempColModels::initialise();
    trace
        .completed_steps
        .push(CGameInitialiseStep::TempColModelsInitialise);

    match CHandlingDataAsset::load(&mut before_rw.file_mgr) {
        Ok(handling) => {
            commands.insert_resource(handling);
            trace
                .completed_steps
                .push(CGameInitialiseStep::HandlingManagerInitialise);
        }
        Err(error) => {
            error!("CGame startup: HandlingManagerInitialise failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::HandlingManagerInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    match CSurfaceTableAsset::load(&mut before_rw.file_mgr) {
        Ok(surface_table) => {
            commands.insert_resource(surface_table);
            trace
                .completed_steps
                .push(CGameInitialiseStep::SurfaceTableInitialise);
        }
        Err(error) => {
            error!("CGame startup: SurfaceTableInitialise failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::SurfaceTableInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    match CPedStatsRuntime::initialise(&mut before_rw.file_mgr) {
        Ok(ped_stats) => {
            commands.insert_resource(ped_stats);
            trace
                .completed_steps
                .push(CGameInitialiseStep::PedStatsInitialise);
        }
        Err(error) => {
            error!("CGame startup: PedStatsInitialise failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::PedStatsInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    match CTimeCycle::initialise(&mut before_rw.file_mgr) {
        Ok(timecycle) => {
            commands.insert_resource(timecycle);
            trace
                .completed_steps
                .push(CGameInitialiseStep::TimeCycleInitialise);
        }
        Err(error) => {
            error!("CGame startup: TimeCycleInitialise failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::TimeCycleInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    // re3 `CParticle::ReloadConfig` calls
    // `mod_ParticleSystemManager.Initialise()` from DATA\PARTICLE.CFG before
    // creating the particle/object pools. The render plugin's zeroed resource
    // is only allocation shape; replacing it here is what supplies authored
    // radii, lifetimes, colours, flags and texture names at runtime.
    match cParticleSystemMgr::initialise(&mut before_rw.file_mgr) {
        Ok(particle_systems) => commands.insert_resource(particle_systems),
        Err(error) => {
            error!("CGame startup: ParticleInitialise failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::ParticleInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }
    eprintln!("rustgta3: CGame startup InitialiseOnceAfterRW data complete");
    info!("CGame startup: InitialiseOnceAfterRW data complete");
}

fn initialise_pre_level_runtime_resource(
    mut file_loader: ResMut<CFileLoader>,
    handling_asset: Option<Res<CHandlingDataAsset>>,
    install: Option<Res<OriginalGtaInstall>>,
    mut trace: ResMut<CGameInitialiseTrace>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GtaAppState>>,
    boot_route: Res<GtaBootRoute>,
) {
    eprintln!("rustgta3: CGame startup pre-level runtime begin");
    info!("CGame startup: pre-level runtime begin");
    commands.insert_resource(CPools::initialise());
    let Some(handling_asset) = handling_asset else {
        error!("CGame startup: CHandlingDataAsset is missing");
        commands.insert_resource(CGameInitialiseError {
            stage: CGameInitialiseStep::HandlingManagerInitialise,
            message: "CHandlingDataAsset is missing".to_owned(),
        });
        next_state.set(GtaAppState::Error);
        return;
    };
    let mut handling_manager = cHandlingDataMgr::new();
    if let Err(error) = handling_manager.initialise_from_cfg(&handling_asset.data) {
        error!("CGame startup: mod_HandlingManager initialise failed: {error}");
        commands.insert_resource(CGameInitialiseError {
            stage: CGameInitialiseStep::HandlingManagerInitialise,
            message: error,
        });
        next_state.set(GtaAppState::Error);
        return;
    }
    commands.insert_resource(handling_manager);
    commands.insert_resource(cAudioScriptObjectPool::initialise());
    let mut dmaudio = cDMAudio::default();
    if let Some(install) = install.as_deref()
        && let Ok(sdt) = install.read("AUDIO\\SFX.SDT")
    {
        dmaudio.LoadSampleMetadata(&sdt);
    }
    dmaudio.Initialise();
    let mut weather = CWeather::init();
    weather.sound_handle = dmaudio.CreateEntity(AUDIOTYPE_WEATHER, Some(1));
    if weather.sound_handle >= 0 {
        dmaudio.SetEntityStatus(weather.sound_handle, true);
    }
    commands.insert_resource(dmaudio);
    commands.insert_resource(CPlayerInfo::clear());
    commands.insert_resource(CGameLogic::init_at_start_of_game());
    commands.insert_resource(CReferences::init());
    let mut camera = CCamera::default();
    camera.Init();
    commands.insert_resource(camera);
    commands.insert_resource(weather);
    commands.insert_resource(CCollision::init());
    commands.insert_resource(CUserDisplay::init());
    commands.insert_resource(COnscreenTimer::default());
    commands.insert_resource(CMessages::init());
    commands.insert_resource(CHud::initialise());
    commands.insert_resource(CFont::initialise());
    commands.insert_resource(CPager::init());
    commands.insert_resource(CMenuManager::initialise());
    commands.insert_resource(CControllerConfigManager::default());
    commands.insert_resource(CRecordDataForGame::init_no_record_files());
    commands.insert_resource(CRestart::initialise());
    file_loader.world = CWorld::initialise();
    file_loader.path_find = crate::control::PathFind::CPathFind::init();
    file_loader.path_find.allocate_path_find_info_mem(4500);
    commands.insert_resource(CParticle::initialise());
    commands.insert_resource(CParticleObjectRuntime::initialise());
    commands.insert_resource(CCutsceneMgr::initialise());
    commands.insert_resource(CCarCtrl::init());
    commands.init_resource::<AmbientTrafficModels>();
    commands.init_resource::<AmbientPedModels>();

    trace.completed_steps.extend([
        CGameInitialiseStep::PoolsInitialise,
        CGameInitialiseStep::GameLogicInitAtStartOfGame,
        CGameInitialiseStep::ReferencesInit,
        CGameInitialiseStep::WeatherInit,
        CGameInitialiseStep::PathFindInit,
        CGameInitialiseStep::CullZonesInit,
        CGameInitialiseStep::CollisionInit,
        CGameInitialiseStep::ZonesInit,
        CGameInitialiseStep::UserDisplayInit,
        CGameInitialiseStep::MessagesInit,
        CGameInitialiseStep::MessagesClearAllDisplayed,
        CGameInitialiseStep::RecordDataForGameInit,
        CGameInitialiseStep::RestartInitialise,
        CGameInitialiseStep::WorldInitialise,
        CGameInitialiseStep::ParticleInitialise,
        CGameInitialiseStep::AnimManagerInitialise,
        CGameInitialiseStep::CutsceneMgrInitialise,
        CGameInitialiseStep::CarCtrlInit,
    ]);
    commands.insert_resource(CAnimManager::initialise());
    eprintln!("rustgta3: CGame startup pre-level runtime complete");
    info!("CGame startup: pre-level runtime complete");

    // re3: after the frontend-only data is ready, the game boots into the
    // Frontend (main menu) state rather than straight into gameplay. The heavy
    // level load happens later, when "START NEW GAME" is chosen (LoadingGame).
    next_state.set(match *boot_route {
        GtaBootRoute::Frontend => GtaAppState::LogoMpeg,
        GtaBootRoute::NewGame => GtaAppState::LoadingGame,
    });
}


#[cfg(test)]
mod garage_door_tests {
    use super::*;
    use crate::core::World::CWorldEntity;

    fn world_door(model_index: usize, position: [f32; 3]) -> CWorldEntity {
        CWorldEntity {
            entity_type: EEntityType::Dummy,
            model_index,
            name: "oddjgaragdoor".to_owned(),
            position,
            scale: [1.0; 3],
            rotation_axis: [0.0, 0.0, 1.0],
            rotation_w: 1.0,
            rotation_degrees: 0.0,
            matrix_basis_override: None,
            render_position_offset: [0.0; 3],
            level: 0,
            is_static: true,
            is_visible: true,
            is_big_building: false,
            is_subway: false,
            streaming_dont_delete: false,
            uses_collision: true,
            zone_culled: false,
            zone_culled2: false,
            building_pool_slot: None,
            treadable_pool_slot: None,
            map_object_slot: None,
        }
    }

    fn install_test_door(
        file_loader: &mut CFileLoader,
        model_index: usize,
        position: [f32; 3],
    ) -> usize {
        file_loader
            .model_info
            .add_simple_model(model_index)
            .base
            .set_model_name("oddjgaragdoor");
        let index = file_loader.world.entities.len();
        file_loader
            .world
            .entities
            .push(world_door(model_index, position));
        file_loader.world.add(index, &file_loader.model_info);
        index
    }

    #[test]
    fn door_search_keeps_nearest_candidate_when_both_are_on_same_side() {
        let mut file_loader = CFileLoader::new();
        let far = install_test_door(&mut file_loader, 5_000, [0.0, 10.0, 2.0]);
        let near = install_test_door(&mut file_loader, 5_001, [0.0, 5.0, 3.0]);
        let garage = CGarage::add_one(
            CVector::new(-1.0, -1.0, 0.0),
            CVector::new(1.0, 1.0, 5.0),
            EGarageType::Mission,
            0,
        );

        assert_eq!(
            crate::control::Garages::find_garage_door_entities(&garage, &file_loader),
            (Some(near), None)
        );
        assert_ne!(far, near);
    }

    #[test]
    fn rotated_door_basis_matches_build_rotated_door_matrix() {
        let mut entity = world_door(1, [0.0; 3]);
        crate::control::Garages::apply_rotated_garage_door_matrix(&mut entity, 0.5);
        let [right, forward, up] = entity.matrix_basis_override.unwrap();
        let root_half = std::f32::consts::FRAC_1_SQRT_2;

        for (actual, expected) in right.into_iter().zip([-root_half, 0.0, root_half]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
        assert_eq!(forward, [0.0, 1.0, 0.0]);
        for (actual, expected) in up.into_iter().zip([root_half, 0.0, root_half]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn stored_car_restore_roundtrips_cpp_vehicle_fields_and_basis() {
        let model_id = crate::modelinfo::ModelIndices::MI_LANDSTAL as i32;
        let mut file_loader = CFileLoader::new();
        let model = file_loader.model_info.add_vehicle_model(model_id as usize);
        model.vehicle_type = eVehicleType::VEHICLE_TYPE_CAR as i32;
        let stored = CStoredCar {
            model_index: model_id,
            position: CVector::new(11.0, 22.0, 3.5),
            angle: CVector::new(0.6, 0.8, 0.0),
            flags: CStoredCar::FLAG_BULLETPROOF
                | CStoredCar::FLAG_FIREPROOF
                | CStoredCar::FLAG_EXPLOSIONPROOF
                | CStoredCar::FLAG_COLLISIONPROOF
                | CStoredCar::FLAG_MELEEPROOF,
            primary_color: 12,
            secondary_color: 34,
            radio_station: 5,
            variation_a: 2,
            variation_b: 4,
            car_bomb_type: 3,
        };
        let mut world = script_entity_test_world();
        let mut pools = CPools::initialise();
        let mut general = CGeneral::with_seed(1);
        let handle = with_script_entity_test_world(&mut world, |entities| {
            entities
                .restore_stored_car(
                    &mut pools,
                    stored,
                    1_000,
                    file_loader.model_info.get_model_info(model_id as usize),
                    None,
                    &mut general,
                    Some(7),
                    false,
                )
                .expect("stored vehicle should restore")
        });
        with_script_entity_test_world(&mut world, |entities| {
            let restored = entities.vehicles.get(&entities.index, &handle).unwrap();
            let base = restored.base();
            assert_eq!(base.physical.entity.get_position(), stored.position);
            assert_eq!(base.physical.entity.get_forward(), stored.angle);
            assert_eq!(
                base.physical.entity.get_right(),
                CVector::new(stored.angle.y, -stored.angle.x, 0.0)
            );
            assert_eq!(base.physical.entity.get_up(), CVector::new(0.0, 0.0, 1.0));
            assert_eq!(base.appearance.current_colour1, 12);
            assert_eq!(base.appearance.current_colour2, 34);
            assert_eq!(base.audio.radio_station, 5);
            assert_eq!(base.appearance.extras, [2, 4]);
            assert!(!base.flags.freebies);
            assert!(base.flags.has_been_owned_by_player);
            assert_eq!(base.entry.door_lock, eCarLock::CARLOCK_UNLOCKED);
            assert!(base.physical.entity.flags.bullet_proof);
            assert!(base.physical.entity.flags.fire_proof);
            assert!(base.physical.entity.flags.explosion_proof);
            assert!(base.physical.entity.flags.collision_proof);
            assert!(base.physical.entity.flags.melee_proof);
            let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = restored else {
                panic!("landstal must restore as automobile");
            };
            assert_eq!(
                car.bomb_type,
                crate::vehicles::Automobile::eBombType::CARBOMB_REMOTE
            );
            assert_eq!(car.bomb_rigger, Some(7));
            let snapshot = stored_car_from_script_vehicle(restored);
            assert_eq!(snapshot.model_index, stored.model_index);
            assert_eq!(snapshot.position, stored.position);
            assert_eq!(snapshot.angle, stored.angle);
            assert_eq!(snapshot.flags, stored.flags);
            assert_eq!(snapshot.car_bomb_type, stored.car_bomb_type);
        });
    }

    #[test]
    fn hideout_store_removes_random_vehicle_but_preserves_mission_vehicle() {
        let model_id = crate::modelinfo::ModelIndices::MI_LANDSTAL as i32;
        let mut file_loader = CFileLoader::new();
        let model = file_loader.model_info.add_vehicle_model(model_id as usize);
        model.vehicle_type = eVehicleType::VEHICLE_TYPE_CAR as i32;
        model.clump.base.col_model = Some(crate::collision::ColModel::CColModel::new(
            CColSphere::new(1.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(
                CColVector::new(-1.0, -1.0, -1.0),
                CColVector::new(1.0, 1.0, 1.0),
                0,
                0,
            ),
            vec![CColSphere::new(1.0, CColVector::new(0.0, 0.0, 0.0), 0, 0)],
        ));
        let mut world = script_entity_test_world();
        let mut pools = CPools::initialise();
        let mut general = CGeneral::with_seed(2);
        let random_handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))
            .unwrap()
            .0;
        let mission_handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))
            .unwrap()
            .0;
        for (handle, created_by) in [
            (random_handle, RANDOM_VEHICLE),
            (mission_handle, MISSION_VEHICLE),
        ] {
            let mut car = CAutomobile::new(
                model_id,
                created_by,
                0,
                &mut general,
                None,
                Some((10, 20)),
                false,
            );
            car.vehicle.core.physical
                .entity
                .set_position(CVector::new(5.0, 5.0, 2.0));
            spawn_script_test_vehicle(&mut world, ScriptVehicleEntity::Automobile { handle, car });
        }
        let garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::HideoutOne,
            0,
        );
        let mut stored = [CStoredCar::init(); NUM_GARAGE_STORED_CARS];
        let mut players = StagedWorldPlayers::default();
        with_script_entity_test_world(&mut world, |entities| {
            store_and_remove_hideout_cars(
                &garage,
                &mut stored,
                entities,
                &mut pools,
                &mut players,
                &file_loader,
            );
        });
        assert!(stored[0].has_car());
        assert_eq!(stored[0].model_index, model_id);
        let (random_exists, mission_exists) =
            with_script_entity_test_world(&mut world, |entities| {
                (
                    entities.vehicles.contains_key(&entities.index, &random_handle),
                    entities.vehicles.contains_key(&entities.index, &mission_handle),
                )
            });
        assert!(!random_exists);
        assert!(mission_exists);
        assert!(
            pools
                .vehicle_pool
                .get_at(CPoolHandle(random_handle))
                .is_none()
        );
        assert!(
            pools
                .vehicle_pool
                .get_at(CPoolHandle(mission_handle))
                .is_some()
        );
    }

    #[test]
    fn garage_camera_override_is_fixed_outside_and_releases_to_follow_mode() {
        let mut garages = CGarages::init();
        garages.garages[0] = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Mission,
            0,
        );
        garages.garages[0].door1_world_index = Some(1);
        garages.garages[0].door1_x = 5.0;
        garages.garages[0].door1_y = 0.0;
        garages.cam_should_be_outside = true;
        let mut camera = CCamera::default();
        camera.garage_we_are_in = Some(0);
        let file_loader = CFileLoader::new();
        let target = CVector::new(5.0, 5.0, 1.0);

        assert!(apply_garage_camera_override(
            &mut camera,
            &garages,
            target,
            &file_loader,
        ));
        assert_eq!(camera.position.x, 5.0);
        assert_eq!(camera.position.y, -13.0);
        assert!((camera.position.z - 3.9).abs() < 1.0e-6);
        assert_eq!(camera.look_at, target);
        assert_eq!(camera.script_mode, CameraScriptMode::FollowPlayer);
        assert_eq!(
            camera.cams[camera.active_cam].mode,
            crate::core::Cam::eCamMode::MODE_FIXED
        );
        assert!(camera.player_is_in_garage);

        camera.garage_we_are_in = None;
        assert!(!apply_garage_camera_override(
            &mut camera,
            &garages,
            target,
            &file_loader,
        ));
        assert!(camera.just_came_out_of_garage);
        assert!(!camera.player_is_in_garage);
        assert!(!camera.garage_fixed_cam_position_set);
    }

    #[test]
    fn garage_tidy_matches_far_and_close_wreck_rules() {
        let model_id = crate::modelinfo::ModelIndices::MI_LANDSTAL as i32;
        let mut file_loader = CFileLoader::new();
        let model = file_loader.model_info.add_vehicle_model(model_id as usize);
        model.vehicle_type = eVehicleType::VEHICLE_TYPE_CAR as i32;
        model.clump.base.col_model = Some(crate::collision::ColModel::CColModel::new(
            CColSphere::new(2.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(
                CColVector::new(-1.0, -1.0, -1.0),
                CColVector::new(1.5, 1.0, 1.0),
                0,
                0,
            ),
            vec![
                CColSphere::new(0.5, CColVector::new(-0.25, 0.0, 0.0), 0, 0),
                CColSphere::new(0.25, CColVector::new(1.0, 0.0, 0.0), 0, 0),
            ],
        ));
        let mut world = script_entity_test_world();
        let mut pools = CPools::initialise();
        let mut general = CGeneral::with_seed(3);
        let mut slots = Vec::new();
        for (position, wrecked, inverted) in [
            (CVector::new(5.0, 5.0, 2.0), true, false),
            (CVector::new(6.0, 5.0, 2.0), false, true),
            (CVector::new(10.0, 5.0, 2.0), true, false),
        ] {
            let handle = pools
                .vehicle_pool
                .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))
                .unwrap()
                .0;
            let mut car =
                CAutomobile::new(model_id, RANDOM_VEHICLE, 0, &mut general, None, None, false);
            car.vehicle.core.physical.entity.set_position(position);
            if wrecked {
                car.vehicle.set_status(eEntityStatus::STATUS_WRECKED);
            }
            if inverted {
                car.vehicle.core.physical.entity.matrix.up = CVector::new(0.0, 0.0, 0.0);
            }
            slots.push(CPoolHandle(handle).index());
            spawn_script_test_vehicle(&mut world, ScriptVehicleEntity::Automobile { handle, car });
        }
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Mission,
            0,
        );
        let far = with_script_entity_test_world(&mut world, |entities| {
            garage_tidy_vehicle_slots(&garage, true, entities, &file_loader)
        });
        assert!(far.contains(&slots[0]));
        assert!(far.contains(&slots[1]));
        assert!(!far.contains(&slots[2]));

        garage.garage_state = EGarageState::Opened;
        let close = with_script_entity_test_world(&mut world, |entities| {
            garage_tidy_vehicle_slots(&garage, false, entities, &file_loader)
        });
        assert_eq!(close, vec![slots[2]]);

        garage.garage_state = EGarageState::FullyClosed;
        let closed = with_script_entity_test_world(&mut world, |entities| {
            garage_tidy_vehicle_slots(&garage, false, entities, &file_loader)
        });
        assert!(closed.contains(&slots[0]));
        assert!(!closed.contains(&slots[1]));
        assert!(closed.contains(&slots[2]));
    }
}



/// re3 CGame::ShutDownForRestart tears down level-owned world/streaming
/// state before CGame::InitialiseWhenRestarting rebuilds the level.
fn reset_game_level_data_for_loading(
    mut file_loader: ResMut<CFileLoader>,
    mut streaming: ResMut<CStreaming>,
    mut scripts: Option<ResMut<CTheScripts>>,
    mut script_entities: ScriptEntityWorld,
    mut camera: Option<ResMut<CCamera>>,
    mut cutscene: Option<ResMut<CCutsceneMgr>>,
    mut timer: Option<ResMut<CTimer>>,
    mut onscreen_timer: Option<ResMut<COnscreenTimer>>,
    mut pools: Option<ResMut<CPools>>,
    mut car_ctrl: Option<ResMut<CCarCtrl>>,
    mut traffic_models: Option<ResMut<AmbientTrafficModels>>,
    mut ped_models: Option<ResMut<AmbientPedModels>>,
    mut restart_runtime: RestartRuntimeResources,
) {
    if let Some(scripts) = scripts.as_deref_mut() {
        scripts.undo_entity_invisibility_settings(&mut file_loader.world, &mut script_entities);
    }
    file_loader.reset_for_restart();
    // CGame::Initialise allocates this before CFileLoader starts feeding path
    // records. CFileLoader::new() intentionally only creates the empty shape.
    file_loader.path_find.allocate_path_find_info_mem(4500);
    *streaming = CStreaming::default();

    // re3 ShutDownForRestart + ReInitGameObjectVariables rebuild these
    // process-global states for every New Game/restart.
    if let Some(camera) = camera.as_deref_mut() {
        *camera = CCamera::default();
        camera.Init();
    }
    if let Some(cutscene) = cutscene.as_deref_mut() {
        *cutscene = CCutsceneMgr::initialise();
    }
    if let Some(timer) = timer.as_deref_mut() {
        *timer = CTimer::initialise();
    }
    if let Some(onscreen_timer) = onscreen_timer.as_deref_mut() {
        onscreen_timer.Init();
    }
    if let Some(player_info) = restart_runtime.player_info.as_deref_mut() {
        *player_info = CPlayerInfo::clear();
    }
    if let Some(game_logic) = restart_runtime.game_logic.as_deref_mut() {
        *game_logic = CGameLogic::init_at_start_of_game();
    }
    if let Some(restart) = restart_runtime.restart.as_deref_mut() {
        *restart = CRestart::initialise();
    }
    if let Some(pads) = restart_runtime.pads.as_deref_mut() {
        let modes = [pads.pads[0].mode, pads.pads[1].mode];
        *pads = CPads::default();
        pads.pads[0].mode = modes[0];
        pads.pads[1].mode = modes[1];
    }
    if let Some(remote) = restart_runtime.remote.as_deref_mut() {
        *remote = CRemote::default();
    }
    if let Some(references) = restart_runtime.references.as_deref_mut() {
        *references = CReferences::init();
    }
    if let Some(fire_manager) = restart_runtime.fire_manager.as_deref_mut() {
        *fire_manager = CFireManager::default();
    }
    if let Some(explosions) = restart_runtime.explosions.as_deref_mut() {
        *explosions = CExplosionPool::Initialise();
    }
    if let Some(projectiles) = restart_runtime.projectiles.as_deref_mut() {
        *projectiles = CProjectileInfoPool::Initialise();
    }
    if let Some(shots) = restart_runtime.shots.as_deref_mut() {
        *shots = CShotInfoPool::Initialise();
    }
    if let Some(bullets) = restart_runtime.bullets.as_deref_mut() {
        *bullets = crate::weapons::BulletInfo::CBulletInfoPool::Initialise();
    }
    if let Some(particles) = restart_runtime.particles.as_deref_mut() {
        particles.reload_config_for_restart();
    }
    if let Some(particle_objects) = restart_runtime.particle_objects.as_deref_mut() {
        *particle_objects = CParticleObjectRuntime::initialise();
    }
    *restart_runtime.replay_runtime = ReplayRuntimeWorldState::default();
    if let Some(dmaudio) = restart_runtime.dmaudio.as_deref_mut() {
        dmaudio.DestroyAllGameCreatedEntities();
        if let Some(weather) = restart_runtime.weather.as_deref_mut() {
            let sound_handle = dmaudio.CreateEntity(AUDIOTYPE_WEATHER, Some(1));
            if sound_handle >= 0 {
                dmaudio.SetEntityStatus(sound_handle, true);
            }
            weather.reset_for_restart(sound_handle);
        }
    } else if let Some(weather) = restart_runtime.weather.as_deref_mut() {
        weather.reset_for_restart(-1);
    }
    if let Some(user_display) = restart_runtime.user_display.as_deref_mut() {
        *user_display = CUserDisplay::init();
    }
    if let Some(messages) = restart_runtime.messages.as_deref_mut() {
        *messages = CMessages::init();
    }
    if let Some(hud) = restart_runtime.hud.as_deref_mut() {
        hud.re_initialise();
    }
    if let Some(timecycle) = restart_runtime.timecycle.as_deref_mut() {
        timecycle.reset_for_restart();
    }

    if let Some(pools) = pools.as_deref_mut() {
        *pools = CPools::initialise();
    }
    if let Some(car_ctrl) = car_ctrl.as_deref_mut() {
        *car_ctrl = CCarCtrl::init();
    }
    if let Some(models) = traffic_models.as_deref_mut() {
        *models = AmbientTrafficModels::default();
    }
    if let Some(models) = ped_models.as_deref_mut() {
        *models = AmbientPedModels::default();
    }
}

/// Clear authoritative ECS gameplay entities before the freshly loaded SCM
/// starts creating handles again. Leaving the previous run's entities alive
/// produces duplicate handles/occupants and visibly stacks peds and vehicles.
fn clear_gameplay_entities_for_loading(
    entities: Query<
        Entity,
        Or<(
            With<crate::control::ScriptEntities::ScriptPedIdentity>,
            With<crate::control::ScriptEntities::ScriptVehicleIdentity>,
            With<ScriptObjectIdentity>,
            With<crate::peds::PlayerPed::PlayerPedState>,
            With<ReplayVehicleRuntimeState>,
            With<ReplayPedRuntimeState>,
        )>,
    >,
    mut script_index: ResMut<ScriptEntityIndex>,
    mut player_state: ResMut<CWorldPlayerState>,
    mut pickup_index: ResMut<ScriptPickupIndex>,
    mut radar_index: ResMut<ScriptRadarBlipIndex>,
    mut taxi_lights: ResMut<ScriptTaxiLightState>,
    mut commands: Commands,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    *script_index = ScriptEntityIndex::default();
    *player_state = CWorldPlayerState::default();
    *pickup_index = ScriptPickupIndex::default();
    *radar_index = ScriptRadarBlipIndex::default();
    *taxi_lights = ScriptTaxiLightState::default();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameLevelLoadPhase {
    LevelPlans,
    Paths,
    Finalise,
    Complete,
}

#[derive(Resource)]
struct CGameLevelLoadProgress {
    phase: GameLevelLoadPhase,
    level_plans: IncrementalLevelPlanLoad,
    path_phase: PathDataPreparePhase,
    developer_world_teleport: bool,
}

/// Start the level load without doing any heavy file conversion. re3's
/// `LoadLevel` pumps a loading screen between authored DAT entries; the old
/// Bevy port performed the whole operation in `OnEnter(LoadingGame)`, freezing
/// the desktop window for several seconds before it could present another
/// frame.
fn begin_game_level_loading(
    install: Option<Res<OriginalGtaInstall>>,
    mut before_rw: Option<ResMut<CGameBeforeRw>>,
    game_state: Res<CGameState>,
    mut file_loader: ResMut<CFileLoader>,
    mut level_boot_mode: ResMut<GtaLevelBootMode>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GtaAppState>>,
) {
    let developer_world_teleport = *level_boot_mode == GtaLevelBootMode::DeveloperWorldTeleport;
    // A failed load must not leave the next ordinary New Game in debug mode.
    *level_boot_mode = GtaLevelBootMode::NormalNewGame;
    let (Some(install), Some(before_rw)) = (install, before_rw.as_deref_mut()) else {
        return;
    };

    eprintln!("rustgta3: CGame startup game level plans begin");
    info!("CGame startup: game level plans begin");
    match file_loader.begin_default_and_dat_plans_incremental(
        &install,
        &mut before_rw.file_mgr,
        &mut before_rw.cd_stream,
        &game_state.dat_file,
    ) {
        Ok(level_plans) => commands.insert_resource(CGameLevelLoadProgress {
            phase: GameLevelLoadPhase::LevelPlans,
            level_plans,
            path_phase: PathDataPreparePhase::default(),
            developer_world_teleport,
        }),
        Err(error) => {
            error!("CGame startup: FileLoaderLoadGtaDat failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::FileLoaderLoadGtaDat,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
        }
    }
}

/// Advance one expensive loading unit per rendered frame. A single DAT entry
/// is the same granularity at which re3 calls `LoadingScreenLoadingFile`.
fn advance_game_level_loading(
    install: Option<Res<OriginalGtaInstall>>,
    mut before_rw: Option<ResMut<CGameBeforeRw>>,
    mut file_loader: ResMut<CFileLoader>,
    mut progress: Option<ResMut<CGameLevelLoadProgress>>,
    mut trace: ResMut<CGameInitialiseTrace>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GtaAppState>>,
) {
    let (Some(install), Some(before_rw), Some(progress)) =
        (install, before_rw.as_deref_mut(), progress.as_deref_mut())
    else {
        return;
    };

    match progress.phase {
        GameLevelLoadPhase::LevelPlans => {
            if let Some(filename) = progress.level_plans.current_file() {
                debug!("CGame incremental load: {filename}");
            }
            match file_loader.step_default_and_dat_plans_incremental(
                &install,
                &mut before_rw.file_mgr,
                &mut before_rw.cd_stream,
                &mut progress.level_plans,
            ) {
                Ok(true) => {
                    trace.completed_steps.extend([
                        CGameInitialiseStep::FileLoaderLoadDefaultDat,
                        CGameInitialiseStep::FileLoaderLoadGtaDat,
                    ]);
                    eprintln!("rustgta3: CGame startup DAT/IDE/IPL plans loaded");
                    info!("CGame startup: DAT/IDE/IPL plans loaded");
                    progress.phase = GameLevelLoadPhase::Paths;
                }
                Ok(false) => {}
                Err(error) => {
                    error!("CGame startup: FileLoaderLoadGtaDat failed: {error}");
                    commands.insert_resource(CGameInitialiseError {
                        stage: CGameInitialiseStep::FileLoaderLoadGtaDat,
                        message: error.to_string(),
                    });
                    progress.phase = GameLevelLoadPhase::Complete;
                    next_state.set(GtaAppState::Error);
                }
            }
        }
        GameLevelLoadPhase::Paths => {
            if file_loader
                .path_find
                .prepare_path_data_step(&mut progress.path_phase)
            {
                eprintln!("rustgta3: CGame startup path data prepared");
                info!("CGame startup: path data prepared");
                progress.phase = GameLevelLoadPhase::Finalise;
            }
        }
        GameLevelLoadPhase::Finalise | GameLevelLoadPhase::Complete => {}
    }
}

fn finalise_game_level_loading(
    install: Option<Res<OriginalGtaInstall>>,
    before_rw: Option<ResMut<CGameBeforeRw>>,
    memory_card: Option<Res<CMemoryCard>>,
    mut game_state: ResMut<CGameState>,
    mut file_loader: ResMut<CFileLoader>,
    mut cutscene: Option<ResMut<CCutsceneMgr>>,
    mut game_clock: GameLevelClockParams,
    mut general: ResMut<CGeneral>,
    mut streaming: ResMut<CStreaming>,
    mut camera: Option<ResMut<CCamera>>,
    mut weather: Option<ResMut<CWeather>>,
    mut trace: ResMut<CGameInitialiseTrace>,
    mut load_progress: Option<ResMut<CGameLevelLoadProgress>>,
    mut sfx_playback: Option<ResMut<GtaSfxPlayback>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GtaAppState>>,
) {
    let Some(load_progress) = load_progress.as_deref_mut() else {
        return;
    };
    if load_progress.phase != GameLevelLoadPhase::Finalise {
        return;
    }
    load_progress.phase = GameLevelLoadPhase::Complete;
    let developer_world_teleport = load_progress.developer_world_teleport;
    let (Some(install), Some(mut before_rw)) = (install, before_rw) else {
        return;
    };

    if let Some(sfx) = sfx_playback.as_deref_mut() {
        sfx.load_sfx_data(&install);
    }

    let before_rw = &mut *before_rw;
    let ini = CIniFile::LoadIniFile(&mut before_rw.file_mgr);
    let mut car_ctrl = CCarCtrl::init();
    car_ctrl.car_number_multiplier = ini.car_number_multiplier;
    car_ctrl.max_number_of_cars_in_use =
        (crate::control::CarCtrl::DEFAULT_MAX_NUMBER_OF_CARS as f32
            * ini.car_number_multiplier) as i32;
    commands.insert_resource(car_ctrl);
    commands.insert_resource(ini);
    trace
        .completed_steps
        .push(CGameInitialiseStep::ModelInfoInitialise);

    commands.insert_resource(CPickups::init());
    trace.completed_steps.push(CGameInitialiseStep::PickupsInit);

    // Built locally so boot SCM CREATE_CAR_GENERATOR effects can land before insert.
    let mut car_generators = crate::vehicles::CarGen::CTheCarGenerators::init();
    trace
        .completed_steps
        .push(CGameInitialiseStep::CarGeneratorsInit);

    match CVehicleColours::load(&mut before_rw.file_mgr, &mut file_loader.model_info) {
        Ok(vehicle_colours) => {
            commands.insert_resource(vehicle_colours);
            trace
                .completed_steps
                .push(CGameInitialiseStep::VehicleColoursLoad);
        }
        Err(error) => {
            error!("CGame startup: VehicleColoursLoad failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::VehicleColoursLoad,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    let (water_result, generated_water_cache) = match CWaterLevel::initialise(
        &mut before_rw.file_mgr,
    ) {
        Ok(water_level) => (Ok(water_level), false),
        Err(binary_error) if binary_error.kind() == std::io::ErrorKind::NotFound => {
            // Non-MASTER re3 falls back to DATA\\WATER.DAT, rasterizes its rectangles,
            // removes tiles obscured by real building/dummy collision, then derives the
            // coarse table from the resulting fine tiles.
            let result = CWaterLevel::initialise_from_water_dat(
                &mut before_rw.file_mgr,
                |start, end| {
                    file_loader
                        .world
                        .process_line_of_sight_building_bounds(
                            [start.x, start.y, start.z],
                            [end.x, end.y, end.z],
                            &file_loader.model_info,
                            None,
                        )
                        .is_some()
                },
            )
            .map_err(|text_error| {
                std::io::Error::new(
                    text_error.kind(),
                    format!(
                        "waterpro.dat failed ({binary_error}); WATER.DAT fallback failed ({text_error})"
                    ),
                )
            });
            (result, true)
        }
        Err(error) => (Err(error), false),
    };
    match water_result {
        Ok(water_level) => {
            // The development C++ path attempts to cache the generated tables,
            // but an unavailable write handle is explicitly non-fatal.
            if generated_water_cache {
                let _ = water_level.write_waterpro_cache(&mut before_rw.file_mgr);
            }
            commands.insert_resource(water_level);
            trace
                .completed_steps
                .push(CGameInitialiseStep::WaterInitialise);
        }
        Err(error) => {
            error!("CGame startup: WaterInitialise failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::WaterInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    commands.insert_resource(CDraw::initialise_for_game());
    trace
        .completed_steps
        .push(CGameInitialiseStep::DrawInitialise);

    // re3 `CGame::Initialise` allocates and pins the particle dictionary before
    // loading `MODELS/PARTICLE.TXD` (core/Game.cpp:432-435). The loose-TXD
    // loader below only loads dictionaries whose CTxdStore slot already
    // exists, so without this slot every particle/cloud/skid raster lookup is
    // permanently absent even though gameplay continues producing effects.
    let particle_txd_slot = file_loader.txd_store.add_txd_slot("particle");
    file_loader.txd_store.add_ref(particle_txd_slot);

    if let Err(error) = streaming.load_cd_directory(&before_rw.cd_stream, &mut *file_loader) {
        error!("CGame startup: StreamingInit failed: {error}");
        commands.insert_resource(CGameInitialiseError {
            stage: CGameInitialiseStep::StreamingInit,
            message: error.to_string(),
        });
        next_state.set(GtaAppState::Error);
        return;
    }
    // Loose MODELS/*.txd (generic/particle) + mark stream TXD slots Loaded so DFFs unblock.
    file_loader
        .ensure_loose_txds_loaded_with_streaming(&mut before_rw.file_mgr, Some(&mut *streaming));
    trace
        .completed_steps
        .push(CGameInitialiseStep::StreamingInit);
    eprintln!("rustgta3: CGame startup streaming directory loaded");
    info!("CGame startup: streaming directory loaded");

    file_loader.load_initial_vehicles(&mut streaming);
    trace
        .completed_steps
        .push(CGameInitialiseStep::StreamingLoadInitialVehicles);

    file_loader.load_initial_peds(&mut streaming);
    trace
        .completed_steps
        .push(CGameInitialiseStep::StreamingLoadInitialPeds);

    file_loader.load_initial_weapons(&mut streaming);

    file_loader.request_initial_big_buildings(&mut streaming);
    trace
        .completed_steps
        .push(CGameInitialiseStep::StreamingRequestBigBuildings);

    eprintln!("rustgta3: CGame startup initial requested model load begin");
    info!("CGame startup: initial requested model load begin");
    if let Err(error) =
        file_loader.load_all_requested_models(&mut streaming, &mut before_rw.cd_stream, false)
    {
        error!("CGame startup: StreamingLoadAllRequestedModels failed: {error}");
        commands.insert_resource(CGameInitialiseError {
            stage: CGameInitialiseStep::StreamingLoadAllRequestedModels,
            message: error.to_string(),
        });
        next_state.set(GtaAppState::Error);
        return;
    }
    trace
        .completed_steps
        .push(CGameInitialiseStep::StreamingLoadAllRequestedModels);
    eprintln!("rustgta3: CGame startup initial requested model load complete");
    info!("CGame startup: initial requested model load complete");

    match CAnimationAssets::load_anim_files(&mut before_rw.file_mgr) {
        Ok(animation_assets) => {
            let anim_manager = CAnimManager::load_anim_files(&animation_assets);
            let fight_moves = match CFightMoveTable::load(&mut before_rw.file_mgr, &anim_manager) {
                Ok(fight_moves) => fight_moves,
                Err(error) => {
                    error!("CGame startup: PedInitialise failed loading fistfite.dat: {error}");
                    commands.insert_resource(CGameInitialiseError {
                        stage: CGameInitialiseStep::PedInitialise,
                        message: error.to_string(),
                    });
                    next_state.set(GtaAppState::Error);
                    return;
                }
            };
            let Some(ped_animation_offsets) =
                CPedAnimationOffsets::from_anim_manager(&anim_manager)
            else {
                let message = "standard enter/exit vehicle animation hierarchy is missing";
                error!("CGame startup: PedInitialise failed: {message}");
                commands.insert_resource(CGameInitialiseError {
                    stage: CGameInitialiseStep::PedInitialise,
                    message: message.to_owned(),
                });
                next_state.set(GtaAppState::Error);
                return;
            };
            commands.insert_resource(animation_assets);
            commands.insert_resource(anim_manager);
            commands.insert_resource(fight_moves);
            commands.insert_resource(ped_animation_offsets);
            trace
                .completed_steps
                .push(CGameInitialiseStep::AnimManagerLoadAnimFiles);
        }
        Err(error) => {
            error!("CGame startup: AnimManagerLoadAnimFiles failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::AnimManagerLoadAnimFiles,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    match CPedTypeRuntime::initialise_from_game_data(&mut before_rw.file_mgr) {
        Ok(ped_types) => {
            commands.insert_resource(ped_types);
            let mut ped_runtime = CPedRuntime::initialise();
            ped_runtime.ped_type_initialised = true;
            ped_runtime.fight_data_loaded = true;
            ped_runtime.enter_exit_vehicle_anim_offsets_set = true;
            commands.insert_resource(ped_runtime);
            trace
                .completed_steps
                .push(CGameInitialiseStep::PedInitialise);
        }
        Err(error) => {
            error!("CGame startup: PedInitialise failed loading PED.DAT: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::PedInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    commands.insert_resource(CPedRoutes::initialise());
    commands.insert_resource(CAccidentManager::initialise());
    trace
        .completed_steps
        .push(CGameInitialiseStep::RouteNodeInitialise);

    commands.insert_resource(CEventList::initialise());
    trace
        .completed_steps
        .push(CGameInitialiseStep::EventListInitialise);

    file_loader.renderer.init();
    trace
        .completed_steps
        .push(CGameInitialiseStep::RendererInit);

    if let Err(error) =
        file_loader.load_cullzone_visibility_cache(&mut before_rw.file_mgr, &mut general)
    {
        error!("CGame startup: CullZonesVisibilityCache failed: {error}");
        commands.insert_resource(CGameInitialiseError {
            stage: CGameInitialiseStep::CullZonesVisibilityCache,
            message: error.to_string(),
        });
        next_state.set(GtaAppState::Error);
        return;
    }
    trace
        .completed_steps
        .push(CGameInitialiseStep::CullZonesVisibilityCache);

    file_loader.initialise_radar();
    trace
        .completed_steps
        .push(CGameInitialiseStep::RadarInitialise);

    match CWeaponDataAsset::load(&mut before_rw.file_mgr) {
        Ok(weapon_data) => {
            commands.insert_resource(CWeaponInfoStore::from_weapon_dat(&weapon_data.data));
            commands.insert_resource(weapon_data);
            trace
                .completed_steps
                .push(CGameInitialiseStep::WeaponInitialiseWeapons);
        }
        Err(error) => {
            error!("CGame startup: WeaponInitialiseWeapons failed: {error}");
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::WeaponInitialiseWeapons,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    let mut file_loader = &mut *file_loader;
    let traffic_light_model_index = file_loader
        .model_info
        .get_model_info_by_name("trafficlight1")
        .map(|(model_index, _)| model_index);
    let traffic_lights = CTrafficLights::scan_world_entities(
        &file_loader.world,
        &mut file_loader.path_find,
        traffic_light_model_index,
    );
    commands.insert_resource(traffic_lights);
    trace
        .completed_steps
        .push(CGameInitialiseStep::TrafficLightsScan);

    commands.insert_resource(CRoadBlocks::init_from_path_objects(
        file_loader
            .path_find
            .object_flags
            .iter()
            .take(file_loader.path_find.num_map_objects.max(0) as usize)
            .map(|flags| flags & crate::control::PathFind::USE_IN_ROADBLOCK != 0),
    ));
    trace
        .completed_steps
        .push(CGameInitialiseStep::RoadBlocksInit);

    match CPopulation::initialise(&mut before_rw.file_mgr, &file_loader.model_info) {
        Ok(mut population) => {
            population.max_number_of_peds_in_use =
                (DEFAULT_MAX_NUMBER_OF_PEDS as f32 * ini.ped_number_multiplier) as i32;
            commands.insert_resource(population);
            trace
                .completed_steps
                .push(CGameInitialiseStep::PopulationInitialise);
        }
        Err(error) => {
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::PopulationInitialise,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    }

    commands.insert_resource(CCoronas::init());
    trace.completed_steps.push(CGameInitialiseStep::CoronasInit);

    commands.insert_resource(CShadows::init());
    trace.completed_steps.push(CGameInitialiseStep::ShadowsInit);

    commands.insert_resource(CWeaponEffects::init());
    trace
        .completed_steps
        .push(CGameInitialiseStep::WeaponEffectsInit);

    commands.insert_resource(CSkidmarks::init());
    trace
        .completed_steps
        .push(CGameInitialiseStep::SkidmarksInit);

    commands.insert_resource(CAntennas::init());
    trace
        .completed_steps
        .push(CGameInitialiseStep::AntennasInit);

    commands.insert_resource(CGlass::init());
    trace.completed_steps.push(CGameInitialiseStep::GlassInit);

    commands.insert_resource(CPhoneInfo::initialise_from_world(&file_loader.world));
    trace
        .completed_steps
        .push(CGameInitialiseStep::PhoneInfoInitialise);

    let mut scripts = match CTheScripts::init(&mut before_rw.file_mgr) {
        Ok(scripts) => {
            trace.completed_steps.push(CGameInitialiseStep::ScriptsInit);
            scripts
        }
        Err(error) => {
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::ScriptsInit,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    };
    scripts.update_object_indices(&file_loader.model_info);

    // Keep local until after boot script effect drain so load-time opcodes can
    // apply SetGang* / stats totals (commands.insert_resource is not ResMut same frame).
    let mut gangs = CGangs::initialise();
    trace
        .completed_steps
        .push(CGameInitialiseStep::GangsInitialise);

    // re3 CGame::Initialise calls only CClock::Initialise(1000) here, which
    // starts at 12:00. MAIN.SCM owns all subsequent SET_TIME_OF_DAY changes.
    *game_clock.clock = CClock::initialise(1000, game_clock.timer.time_in_milliseconds());
    trace
        .completed_steps
        .push(CGameInitialiseStep::ClockInitialise);

    let planes = match CPlaneRuntime::initialise(&mut before_rw.file_mgr, |position, lower_z| {
        file_loader
            .world
            .process_vertical_line_static(
                [position.x, position.y, position.z],
                lower_z,
                &file_loader.model_info,
            )
            .map(|point| point.point.z)
    }) {
        Ok(planes) => planes,
        Err(error) => {
            commands.insert_resource(CGameInitialiseError {
                stage: CGameInitialiseStep::PlaneInit,
                message: error.to_string(),
            });
            next_state.set(GtaAppState::Error);
            return;
        }
    };
    commands.insert_resource(planes);
    trace.completed_steps.push(CGameInitialiseStep::PlaneInit);

    commands.insert_resource(CHeliRuntime::init_helis());
    trace.completed_steps.push(CGameInitialiseStep::HeliInit);

    commands.insert_resource(CCranes::init_from_world_without_spawning_hooks(
        &file_loader.world,
    ));
    trace.completed_steps.push(CGameInitialiseStep::CranesInit);

    commands.insert_resource(CDarkel::init());
    trace.completed_steps.push(CGameInitialiseStep::DarkelInit);

    let mut stats = CStats::init();
    trace.completed_steps.push(CGameInitialiseStep::StatsInit);

    commands.insert_resource(CPacManPickups::init());
    trace
        .completed_steps
        .push(CGameInitialiseStep::PacManPickupsInit);

    commands.insert_resource(CRubbish::init());
    trace.completed_steps.push(CGameInitialiseStep::RubbishInit);

    commands.insert_resource(CClouds::init());
    trace.completed_steps.push(CGameInitialiseStep::CloudsInit);

    commands.insert_resource(CSpecialFX::init());
    trace
        .completed_steps
        .push(CGameInitialiseStep::SpecialFXInit);

    commands.insert_resource(CWaterCannons::init());
    trace
        .completed_steps
        .push(CGameInitialiseStep::WaterCannonsInit);

    let mut bridge = CBridge::init_without_entity_lookup();
    if bridge.find_bridge_entities(&file_loader.world, &file_loader.model_info) {
        file_loader
            .path_find
            .set_links_bridge_lights(-330.0, -230.0, -700.0, -588.0, true);
    }
    commands.insert_resource(bridge);
    trace.completed_steps.push(CGameInitialiseStep::BridgeInit);

    commands.insert_resource(CGarages::init());
    trace.completed_steps.push(CGameInitialiseStep::GaragesInit);

    commands.insert_resource(CCredits::init());
    trace.completed_steps.push(CGameInitialiseStep::CreditsInit);

    commands.insert_resource(CRecordDataForChase::init());
    trace
        .completed_steps
        .push(CGameInitialiseStep::RecordDataForChaseInit);

    commands.insert_resource(CReplay::init());
    trace.completed_steps.push(CGameInitialiseStep::ReplayInit);

    file_loader
        .model_info
        .remove_col_models_from_other_levels(game_state.curr_level as i32);
    trace
        .completed_steps
        .push(CGameInitialiseStep::ModelInfoRemoveColModelsFromOtherLevels);

    let loading_save = memory_card
        .as_deref()
        .is_some_and(|card| card.m_bWantToLoad);
    if loading_save {
        game_state.playing_intro = false;
    } else if developer_world_teleport {
        // Insert -> Teleport is a developer world transition, not the
        // frontend "Start New Game" action. The selected player/vehicle is
        // promoted at the destination on the first InGame frame.
        game_state.playing_intro = false;
        info!("CTheScripts::StartTestScript skipped for developer world teleport");
    } else {
        // re3 CGame::Initialise (non-save path):
        //   CTheScripts::StartTestScript();
        //   CTheScripts::Process();
        // Intro cutscene is driven by MAIN.SCM opcodes, not a hardcoded load.
        // re3 Game.cpp Initialise ("Start script"):
        //   CTheScripts::StartTestScript();
        //   CTheScripts::Process();
        //   TheCamera.Process();
        // One Process only — further WAIT wake-ups happen in CGame::Process each frame.
        match scripts.start_test_script() {
            Ok(index) => {
                info!("CTheScripts::StartTestScript active_index={index}");
                // re3 CTimer after load is already running; use 0ms wake baseline like a
                // fresh initialise process slice (WAIT scripts sleep until later frames).
                let mut flush_immediate_streaming_effect = |scripts: &mut CTheScripts| {
                    flush_immediate_script_streaming_effect(
                        scripts,
                        &mut streaming,
                        &mut file_loader,
                    );
                };
                if let Err(error) = scripts
                    .process_active_scripts_with_entity_runtime_context_and_effect_flush(
                        0,
                        0.0,
                        1.0,
                        &mut general,
                        &mut game_clock.clock,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        false,
                        Some(&mut flush_immediate_streaming_effect),
                    )
                {
                    error!("CTheScripts::Process after StartTestScript failed: {error:?}");
                } else {
                    info!(
                        "CTheScripts::Process after StartTestScript: active={} commands={} effects={}",
                        scripts.active_script_count,
                        scripts.commands_executed,
                        scripts.pending_effects.len()
                    );
                }
                drop(flush_immediate_streaming_effect);
            }
            Err(error) => {
                error!("CTheScripts::StartTestScript failed: {error:?}");
            }
        }
        game_state.playing_intro = true;
    }

    // Drain boot effects into ScriptGameplayBridge + live world resources before
    // first InGame frame (re3 applies CREATE_PLAYER / camera / weather side effects
    // during Initialise Process; do not drop them by passing None).
    let mut bridge = ScriptGameplayBridge {
        playing_intro: game_state.playing_intro,
        player_visible: !game_state.playing_intro,
        player_control_enabled: !game_state.playing_intro,
        ..default()
    };
    let mut world_players = StagedWorldPlayers::default();
    apply_script_player_clothing_effects(&mut scripts, &mut world_players);
    apply_script_effects_to_world_with_streaming(
        &mut scripts,
        &mut bridge,
        Some(&mut game_state),
        Some(&mut stats),
        cutscene.as_deref_mut(),
        Some(&mut *streaming),
        camera.as_deref_mut(),
        weather.as_deref_mut(),
        Some(&mut gangs),
        None,
        None,
        Some(&mut world_players as &mut dyn WorldPlayerAccess),
        Some(&mut general),
        Some(&mut car_generators),
        Some(&mut *file_loader),
        Some(&mut before_rw.cd_stream),
        None,
        0,
        None,
    );
    if car_generators.num_of_car_generators > 0 {
        info!(
            "boot car generators: count={} active={}",
            car_generators.num_of_car_generators, car_generators.current_active_count
        );
    }
    commands.insert_resource(car_generators);
    if bridge.player_spawned {
        info!(
            "boot script bridge: player@({:.1},{:.1},{:.1}) heading={:.1} cutscene='{}'",
            bridge.player_position.x,
            bridge.player_position.y,
            bridge.player_position.z,
            bridge.player_heading_degrees,
            bridge.cutscene_name
        );
        // Keep TheCamera resource aligned with Bevy follow so construct_render_list
        // and probes use spawn vicinity, not Startup (0,650,850).
        if let Some(camera) = camera.as_mut() {
            if camera.script_mode == CameraScriptMode::FollowPlayer {
                snap_follow_camera_resource(
                    camera,
                    bridge.player_position,
                    bridge.player_heading_degrees,
                );
            }
        }
    } else {
        warn!(
            "boot script bridge: CREATE_PLAYER not reached yet (commands={}, effects left={})",
            scripts.commands_executed,
            scripts.pending_effects.len()
        );
    }

    // re3 world is filled from IPL during load — log measurable presence at spawn.
    let world_total = file_loader.world.entities.len();
    let world_near = count_world_entities_near(
        &file_loader.world,
        bridge.player_position,
        WORLD_SPAWN_RADIUS,
    );
    let streaming_requested = streaming.num_models_requested;
    info!(
        "boot world: entities_total={world_total} near_spawn_{WORLD_SPAWN_RADIUS:.0}m={world_near} streaming_requested={streaming_requested}"
    );
    if world_total == 0 {
        warn!("boot world: zero IPL entities after load — world presence missing");
    }

    if let Some(player) = world_players.player(bridge.player_index) {
        info!(
            "boot player ped: pool_handle={} active={} pos=({:.1},{:.1},{:.1}) heading={:.1}",
            player.pool_handle,
            player.active_in_world,
            player.position().x,
            player.position().y,
            player.position().z,
            player.heading_degrees()
        );
    }
    let mut player_state = CWorldPlayerState {
        player_in_focus: world_players.player_in_focus(),
        maximum_wanted_level: world_players.maximum_wanted_level(),
        maximum_chaos: world_players.maximum_chaos(),
        ..default()
    };
    for (index, player) in world_players.players.into_iter().enumerate() {
        let Some(player) = player else {
            continue;
        };
        let entity = commands.spawn((PedEntity, player.into_bundle())).id();
        player_state.entities[index] = Some(entity);
    }
    commands.insert_resource(gangs);
    commands.insert_resource(stats);
    commands.insert_resource(player_state);
    commands.insert_resource(bridge);
    commands.insert_resource(scripts);
    next_state.set(GtaAppState::InGame);
}



#[cfg(test)]
mod script_vehicle_stopped_tests {
    use super::script_vehicle_is_stopped;

    #[test]
    fn matches_re3_distance_travelled_threshold() {
        assert!(script_vehicle_is_stopped(0.03, 3.0));
        assert!(!script_vehicle_is_stopped(0.030_001, 3.0));
        assert!(script_vehicle_is_stopped(0.01, 1.0));
        assert!(!script_vehicle_is_stopped(0.010_001, 1.0));
    }
}


#[cfg(test)]
mod pacman_runtime_tests {
    use super::{add_vehicle_pacman_load, reset_vehicle_pacman_load};
    use crate::vehicles::Vehicle::CVehicle;
    use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};

    #[test]
    fn scramble_load_and_reset_preserve_cpp_mass_force_math() {
        let mut general = CGeneral::with_seed(1);
        let mut vehicle = CVehicle::new(0, 0, &mut general);
        vehicle.physical.mass = 1000.0;
        vehicle.physical.turn_mass = 2000.0;
        vehicle.physical.force_multiplier = 1.0;
        vehicle.physical.move_speed = CVector::new(4.0, -2.0, 1.0);

        add_vehicle_pacman_load(&mut vehicle);
        add_vehicle_pacman_load(&mut vehicle);
        assert_eq!(vehicle.pac_man_pickups_carried, 2);
        assert!((vehicle.physical.mass - 1500.0).abs() < 1.0e-5);
        assert!((vehicle.physical.turn_mass - 3000.0).abs() < 1.0e-5);
        assert!((vehicle.physical.force_multiplier - 1.5).abs() < 1.0e-5);
        assert!((vehicle.physical.move_speed.x - 1.69).abs() < 1.0e-5);

        reset_vehicle_pacman_load(&mut vehicle);
        assert_eq!(vehicle.pac_man_pickups_carried, 0);
        assert!((vehicle.physical.mass - 1000.0).abs() < 1.0e-5);
        assert!((vehicle.physical.turn_mass - 2000.0).abs() < 1.0e-5);
        assert_eq!(vehicle.physical.force_multiplier, 1.0);
    }
}


#[cfg(test)]
mod pickup_object_runtime_tests {
    use super::{
        MI_PICKUP_BONUS, ScriptEntityIndex, materialize_pickup_object,
        remove_materialized_pickup,
    };
    use crate::{
        control::Pickups::{CPickups, PICKUP_IN_SHOP},
        core::Pools::{CPoolHandle, CPools},
        objects::Object::MISSION_OBJECT,
    };
    use bevy::{
        ecs::world::CommandQueue,
        prelude::{Commands, World},
    };
    use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};

    #[test]
    fn pickup_object_pool_roundtrip_matches_give_us_a_pickup_object() {
        let mut general = CGeneral::with_seed(1);
        let mut pools = CPools::initialise();
        let mut pickups = CPickups::init();
        let mut world = World::new();
        let mut entity_index = ScriptEntityIndex::default();
        let file_loader = crate::core::FileLoader::CFileLoader::default();
        let position = CVector::new(12.0, -8.0, 3.5);
        let pickup_handle = pickups
            .generate_new_one(
                position,
                MI_PICKUP_BONUS as i32,
                i32::from(PICKUP_IN_SHOP),
                12,
                1_000,
            )
            .unwrap();

        let mut command_queue = CommandQueue::default();
        let object_handle = {
            let mut commands = Commands::new(&mut command_queue, &world);
            materialize_pickup_object(
                pickup_handle,
                &mut pickups,
                &mut commands,
                &mut entity_index,
                &mut pools,
                &mut general,
                &file_loader,
            )
            .unwrap()
        };
        command_queue.apply(&mut world);
        let pickup_slot = pickups.actual_pickup_index(pickup_handle).unwrap();
        let object_slot = CPoolHandle(object_handle).index();
        assert_eq!(
            pickups.pickups[pickup_slot].object_pool_index,
            Some(object_slot as i32)
        );
        assert!(
            pools
                .object_pool
                .get_at(CPoolHandle(object_handle))
                .is_some()
        );

        let object_entity = entity_index.object(object_handle).unwrap();
        let object = &world
            .get::<crate::objects::Object::CObject>(object_entity)
            .unwrap();
        assert_eq!(object.object_created_by, MISSION_OBJECT);
        assert_eq!(object.physical.entity.get_position(), position);
        assert!(
            (object.physical.entity.get_heading() + std::f32::consts::FRAC_PI_2).abs() < 1.0e-5
        );
        assert!(!object.physical.flags.affected_by_gravity);
        assert!(object.physical.entity.flags.explosion_proof);
        assert!(!object.physical.entity.flags.uses_collision);
        assert!(object.flags.is_pickup);
        assert!(object.flags.pickup_obj_with_message);
        assert!(!object.flags.out_of_stock);
        assert_eq!(object.bonus_value, 12);

        let mut command_queue = CommandQueue::default();
        assert!({
            let mut commands = Commands::new(&mut command_queue, &world);
            remove_materialized_pickup(
                pickup_handle,
                &mut pickups,
                &mut commands,
                &mut entity_index,
                &mut pools,
            )
        });
        command_queue.apply(&mut world);
        assert!(entity_index.object(object_handle).is_none());
        assert!(world.get::<crate::objects::Object::CObject>(object_entity).is_none());
        assert!(pools.object_pool.get_slot(object_slot).is_none());
        assert!(pickups.actual_pickup_index(pickup_handle).is_some());
        assert!(!pickups.pickups[pickup_slot].object_present);
        assert!(pickups.pickups[pickup_slot].object_pool_index.is_none());
    }
}

#[cfg(test)]
mod script_occupant_objective_tests {
    use super::*;
    use crate::peds::PedType::ePedType;

    #[test]
    fn player_char_handle_routes_set_char_leave_car_to_world_player_owner() {
        let mut general = CGeneral::with_seed(1);
        let mut players = StagedWorldPlayers::default();
        CPlayerPed::setup_player_ped(&mut players, 0, &mut general, None);
        let player_handle = players.player(0).unwrap().pool_handle;
        let vehicle_handle = 1793;
        let player = players.player_mut(0).unwrap();
        player.ped.flags.script_objective_completed = true;

        let mut world = script_entity_test_world();
        let vehicle_entity = spawn_script_test_vehicle(
            &mut world,
            ScriptVehicleEntity::Automobile {
                handle: vehicle_handle,
                car: CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false),
            },
        );
        players.set_vehicle_relation(0, InVehicle::driver(vehicle_entity));
        assert!(with_script_entity_test_world(&mut world, |entities| {
            apply_script_char_objective(
                entities,
                &mut players,
                player_handle,
                13,
                None,
                Some(vehicle_handle),
                None,
                0.0,
                None,
                &CPedRoutes::initialise(),
                1_000,
            )
        }));
        let player = players.player(0).unwrap();
        let vehicle_slot = CPoolHandle(vehicle_handle).index();
        assert_eq!(player.ped.objective, eObjective::OBJECTIVE_LEAVE_CAR);
        assert_eq!(player.ped.car_in_objective, Some(vehicle_slot));
        assert!(!player.ped.flags.script_objective_completed);
    }

    #[test]
    fn all_car_occupants_receive_leave_objective_in_driver_then_passenger_slots() {
        let mut general = CGeneral::with_seed(1);
        let mut players = StagedWorldPlayers::default();
        CPlayerPed::setup_player_ped(&mut players, 0, &mut general, None);
        let vehicle_handle = 1793;
        let vehicle_slot = CPoolHandle(vehicle_handle).index();
        let passenger_handle = 257;

        let player = players.player_mut(0).unwrap();
        player.ped.flags.script_objective_completed = true;

        let mut passenger = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        passenger.flags.script_objective_completed = true;

        let car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        // Occupants are attached below through `InVehicle`; no vehicle-side
        // integer driver/passenger slots participate in this test.

        let mut world = script_entity_test_world();
        let vehicle_entity = spawn_script_test_vehicle(
            &mut world,
            ScriptVehicleEntity::Automobile {
                handle: vehicle_handle,
                car,
            },
        );
        let passenger_entity = spawn_script_test_ped(
            &mut world,
            ScriptPedEntity {
                vitality: crate::peds::PedVitality::PedVitality::default(),
                locomotion: crate::peds::PedLocomotion::PedLocomotion::default(),
                animation: crate::peds::PedAnimation::PedAnimation::default(),
                handle: passenger_handle,
                ped: passenger,
                model_id: 0,
            },
        );
        world
            .entity_mut(passenger_entity)
            .insert(InVehicle::passenger(vehicle_entity, 0));
        players.set_vehicle_relation(0, InVehicle::driver(vehicle_entity));

        assert!(with_script_entity_test_world(&mut world, |entities| {
            set_all_occupants_of_car_leave_car(entities, &mut players, vehicle_handle, 1_000)
        }));
        let player = players.player(0).unwrap();
        let passenger_state = with_script_entity_test_world(&mut world, |entities| {
            let passenger = &entities.peds.get(&entities.index, &passenger_handle).unwrap().ped;
            (
                passenger.objective,
                passenger.car_in_objective,
                passenger.flags.script_objective_completed,
            )
        });
        assert_eq!(player.ped.objective, eObjective::OBJECTIVE_LEAVE_CAR);
        assert_eq!(player.ped.car_in_objective, Some(vehicle_slot));
        assert!(!player.ped.flags.script_objective_completed);
        assert_eq!(passenger_state.0, eObjective::OBJECTIVE_LEAVE_CAR);
        assert_eq!(passenger_state.1, Some(vehicle_slot));
        assert!(!passenger_state.2);
    }

    #[test]
    fn ped_side_vehicle_links_receive_leave_objective_when_vehicle_slots_lag_one_frame() {
        let mut general = CGeneral::with_seed(1);
        let mut players = StagedWorldPlayers::default();
        CPlayerPed::setup_player_ped(&mut players, 0, &mut general, None);
        let vehicle_handle = 1793;
        let vehicle_slot = CPoolHandle(vehicle_handle).index();
        let passenger_handle = 257;

        let player = players.player_mut(0).unwrap();
        player.ped.flags.script_objective_completed = true;

        let mut passenger = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        passenger.flags.script_objective_completed = true;

        let car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);

        let mut world = script_entity_test_world();
        let vehicle_entity = spawn_script_test_vehicle(
            &mut world,
            ScriptVehicleEntity::Automobile {
                handle: vehicle_handle,
                car,
            },
        );
        let passenger_entity = spawn_script_test_ped(
            &mut world,
            ScriptPedEntity {
                vitality: crate::peds::PedVitality::PedVitality::default(),
                locomotion: crate::peds::PedLocomotion::PedLocomotion::default(),
                animation: crate::peds::PedAnimation::PedAnimation::default(),
                handle: passenger_handle,
                ped: passenger,
                model_id: 0,
            },
        );
        world
            .entity_mut(passenger_entity)
            .insert(InVehicle::passenger(vehicle_entity, 0));
        players.set_vehicle_relation(0, InVehicle::driver(vehicle_entity));

        assert!(with_script_entity_test_world(&mut world, |entities| {
            set_all_occupants_of_car_leave_car(entities, &mut players, vehicle_handle, 1_000)
        }));
        let player = players.player(0).unwrap();
        let passenger_state = with_script_entity_test_world(&mut world, |entities| {
            let passenger = &entities.peds.get(&entities.index, &passenger_handle).unwrap().ped;
            (
                passenger.objective,
                passenger.car_in_objective,
                passenger.flags.script_objective_completed,
            )
        });
        assert_eq!(player.ped.objective, eObjective::OBJECTIVE_LEAVE_CAR);
        assert_eq!(player.ped.car_in_objective, Some(vehicle_slot));
        assert!(!player.ped.flags.script_objective_completed);
        assert_eq!(passenger_state.0, eObjective::OBJECTIVE_LEAVE_CAR);
        assert_eq!(passenger_state.1, Some(vehicle_slot));
        assert!(!passenger_state.2);
    }
}


#[cfg(test)]
mod scm_streaming_request_tests {
    use super::{SCM_REQUEST_MODEL_FLAGS, request_scm_model};
    use crate::core::Streaming::{CStreaming, StreamLoadState};

    #[test]
    fn request_model_uses_re3_script_owned_no_fade_dependency_flags() {
        let mut streaming = CStreaming::default();
        let model_id = 123;

        request_scm_model(&mut streaming, None, model_id as i32);

        assert_eq!(
            streaming.info_for_model[model_id].flags,
            SCM_REQUEST_MODEL_FLAGS
        );
        assert_eq!(
            streaming.info_for_model[model_id].load_state,
            StreamLoadState::InQueue
        );
    }
}


#[cfg(test)]
mod physical_collision_regression_tests {
    use super::*;

    fn object_snapshot(
        uproot_limit: f32,
        is_stuck: bool,
        infinite_mass: bool,
    ) -> ScriptObjectCollisionSnapshot {
        ScriptObjectCollisionSnapshot {
            entity: Entity::from_raw_u32(1).expect("test entity id"),
            matrix: CMatrix::unity(),
            model_index: 0,
            uses_collision: true,
            is_static: true,
            is_stuck,
            infinite_mass,
            has_been_damaged: false,
            uproot_limit,
        }
    }

    #[test]
    fn ped_motion_only_preblocks_cpp_immovable_object_cases() {
        assert!(!player_object_blocks_motion(&object_snapshot(
            0.0, false, false
        )));
        assert!(player_object_blocks_motion(&object_snapshot(
            1.0, false, false
        )));
        assert!(player_object_blocks_motion(&object_snapshot(
            0.0, true, false
        )));
        assert!(player_object_blocks_motion(&object_snapshot(
            0.0, false, true
        )));
    }

    #[test]
    fn object_collision_subdivision_matches_re3_response_thresholds() {
        let file_loader = CFileLoader::default();
        let mut general = CGeneral::default();
        let mut object = CObject::new(&mut general);

        object.special_collision_response_cases =
            CollisionSpecialResponseCase::COLLRESPONSE_UNKNOWN5 as u8;
        object.physical.move_speed = CVector::new(0.09, 0.0, 0.0);
        assert_eq!(object_collision_substeps(&object, &file_loader, 1.0), 1);
        object.physical.move_speed = CVector::new(0.10, 0.0, 0.0);
        assert_eq!(object_collision_substeps(&object, &file_loader, 1.0), 2);

        object.special_collision_response_cases =
            CollisionSpecialResponseCase::COLLRESPONSE_SMALLBOX as u8;
        object.physical.move_speed = CVector::new(0.149, 0.0, 0.0);
        assert_eq!(object_collision_substeps(&object, &file_loader, 1.0), 1);
        object.physical.move_speed = CVector::new(0.151, 0.0, 0.0);
        assert_eq!(object_collision_substeps(&object, &file_loader, 1.0), 2);

        object.special_collision_response_cases =
            CollisionSpecialResponseCase::COLLRESPONSE_NONE as u8;
        object.physical.move_speed = CVector::new(0.299, 0.0, 0.0);
        assert_eq!(object_collision_substeps(&object, &file_loader, 1.0), 1);
        object.physical.move_speed = CVector::new(0.301, 0.0, 0.0);
        assert_eq!(object_collision_substeps(&object, &file_loader, 1.0), 2);
    }

    #[test]
    fn seconds_based_player_physics_uses_re3_fifty_hz_boundary() {
        assert!((GTA_PHYSICS_HZ - 50.0).abs() < f32::EPSILON);
        assert!((GRAVITY_WORLD_UNITS_PER_SECOND_SQ - 20.0).abs() < f32::EPSILON);

        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.physical.move_speed = CVector::new(5.0, -10.0, 2.5);
        let marker = with_player_native_collision_speed(&mut ped, |ped| {
            assert!((ped.physical.move_speed.x - 0.1).abs() < 1.0e-6);
            assert!((ped.physical.move_speed.y + 0.2).abs() < 1.0e-6);
            assert!((ped.physical.move_speed.z - 0.05).abs() < 1.0e-6);
            ped.physical.move_speed = CVector::new(0.2, -0.1, 0.03);
            7
        });
        assert_eq!(marker, 7);
        assert!((ped.physical.move_speed.x - 10.0).abs() < 1.0e-6);
        assert!((ped.physical.move_speed.y + 5.0).abs() < 1.0e-6);
        assert!((ped.physical.move_speed.z - 1.5).abs() < 1.0e-6);
    }

    #[test]
    fn process_shift_does_not_publish_penetration_as_distance_travelled() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.physical.distance_travelled = 2.5;
        let old_position = ped.physical.entity.get_position();

        assert!(
            ped.physical
                .apply_process_shift(CVector::new(0.3, -0.4, 0.0))
        );
        assert_eq!(ped.physical.distance_travelled, 2.5);
        assert_ne!(ped.physical.entity.get_position(), old_position);
    }

    #[test]
    fn vehicle_ped_push_is_suppressed_only_for_the_actual_support_vehicle() {
        let mut general = CGeneral::default();
        let mut car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        car.vehicle.core.physical.mass = 1_000.0;
        car.vehicle.core.physical.turn_mass = 2_000.0;
        car.vehicle.core.physical.move_speed = CVector::ZERO;
        ped.physical.move_speed = CVector::new(1.0, 0.0, 0.0);
        let mut point = CColPoint::new();
        point.normal = CColVector::new(1.0, 0.0, 0.0);

        let response = car.vehicle.core.physical.apply_collision_dynamic(
            &mut ped.physical,
            &point,
            CPhysicalCollisionOptions {
                time_step: 1.0,
                ..Default::default()
            },
        );
        assert!(response.applied);
        assert!(car.vehicle.core.physical.move_speed.x > 0.0);

        let mut supported_car =
            CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        let mut supported_ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        supported_car.vehicle.core.physical.mass = 1_000.0;
        supported_car.vehicle.core.physical.turn_mass = 2_000.0;
        supported_car.vehicle.core.physical.move_speed = CVector::ZERO;
        supported_ped.physical.move_speed = CVector::new(1.0, 0.0, 0.0);
        let response = supported_car.vehicle.core.physical.apply_collision_dynamic(
            &mut supported_ped.physical,
            &point,
            CPhysicalCollisionOptions {
                a_is_current_physical_surface_for_b: true,
                time_step: 1.0,
                ..Default::default()
            },
        );
        assert!(response.applied);
        assert_eq!(supported_car.vehicle.core.physical.move_speed, CVector::ZERO);
    }

    #[test]
    fn player_collision_frame_only_publishes_collision_free_displacement() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.physical.distance_travelled = 9.0;
        ped.flags.hit_steep_slope = true;

        begin_player_collision_frame(&mut ped);
        assert_eq!(ped.physical.distance_travelled, 0.0);
        assert!(!ped.flags.hit_steep_slope);
        finish_player_collision_frame(&mut ped, CVector::ZERO, CVector::new(3.0, 4.0, 0.0), false);
        assert!((ped.physical.distance_travelled - 5.0).abs() < 1.0e-6);

        begin_player_collision_frame(&mut ped);
        finish_player_collision_frame(&mut ped, CVector::ZERO, CVector::new(3.0, 4.0, 0.0), true);
        assert_eq!(ped.physical.distance_travelled, 0.0);
    }

    #[test]
    fn player_physical_frame_start_consumes_friction_and_clears_collision_transients() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.physical.move_speed = CVector::new(1.0, 2.0, 3.0);
        ped.physical.move_friction = CVector::new(0.1, -0.02, 0.04);
        ped.physical.turn_friction = CVector::new(1.0, 2.0, 3.0);
        ped.physical.entity.flags.has_contacted = true;
        ped.physical.entity.flags.is_in_safe_position = true;
        ped.physical.entity.flags.was_postponed = true;
        ped.physical.entity.flags.has_hit_wall = true;
        ped.physical.entity.flags.has_collided = true;
        ped.physical.damage.piece_type = 3;
        ped.physical.damage.impulse = 12.0;
        ped.physical.damage.entity = Some(crate::entities::EntityReference::EntityReference::World(4));
        ped.physical.add_collision_record(crate::entities::EntityReference::EntityReference::World(9), 100);

        assert!(prepare_player_physical_for_control(&mut ped));
        assert!((ped.physical.move_speed.x - 6.0).abs() < 1.0e-6);
        assert!((ped.physical.move_speed.y - 1.0).abs() < 1.0e-6);
        assert!((ped.physical.move_speed.z - 5.0).abs() < 1.0e-6);
        assert_eq!(ped.physical.move_friction, CVector::ZERO);
        assert_eq!(ped.physical.turn_friction, CVector::ZERO);
        assert!(ped.physical.collision_records.is_empty());
        assert!(!ped.physical.entity.flags.has_contacted);
        assert!(!ped.physical.entity.flags.is_in_safe_position);
        assert!(!ped.physical.entity.flags.was_postponed);
        assert!(!ped.physical.entity.flags.has_hit_wall);
        assert!(!ped.physical.entity.flags.has_collided);
        assert_eq!(ped.physical.damage.piece_type, 0);
        assert_eq!(ped.physical.damage.impulse, 0.0);
        assert_eq!(ped.physical.damage.entity, None);
    }

    #[test]
    fn player_ground_support_extension_only_applies_to_previous_standing_state() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.flags.is_standing = false;
        begin_player_collision_frame(&mut ped);
        assert_eq!(player_ground_support_extension(&ped, 1.0 / 50.0), 0.0);

        ped.flags.is_standing = true;
        begin_player_collision_frame(&mut ped);
        assert!(ped.flags.was_standing);
        assert!(!ped.flags.is_standing);
        assert!((player_ground_support_extension(&ped, 1.0 / 50.0) - 0.15).abs() < 1.0e-6);
        assert!((player_ground_support_extension(&ped, 1.0 / 100.0) - 0.075).abs() < 1.0e-6);

        ped.flags.is_standing = false;
        assert!((player_ground_support_extension(&ped, 1.0 / 50.0) - 0.15).abs() < 1.0e-6);
    }

    #[test]
    fn airborne_dynamic_air_probe_reuses_only_the_native_feet_segment() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.flags.was_standing = false;
        let support = PlayerDynamicGroundSupport {
            z: 8.96,
            entity: Entity::from_raw_u32(1).expect("test entity id"),
            offset_from_surface: CVector::ZERO,
            surface: 0,
            normal: CVector::new(0.0, 0.0, 1.0),
        };
        assert!(player_dynamic_support_from_air_probe(10.0, &ped, Some(support)).is_some());

        let too_low = PlayerDynamicGroundSupport {
            z: 8.959,
            ..support
        };
        assert!(player_dynamic_support_from_air_probe(10.0, &ped, Some(too_low)).is_none());

        ped.flags.was_standing = true;
        assert!(player_dynamic_support_from_air_probe(10.0, &ped, Some(support)).is_none());
    }

    #[test]
    fn airborne_dynamic_subdivision_keeps_new_support_z_after_probe_restore() {
        let mut file_loader = CFileLoader::new();
        file_loader
            .model_info
            .add_ped_model(MI_PLAYER)
            .clump
            .base
            .col_model = Some(crate::collision::ColModel::CColModel::new(
            CColSphere::new(1.2, CColVector::new(0.0, 0.0, -0.5), 0, 0),
            CColBox::new(
                CColVector::new(-0.4, -0.4, -1.1),
                CColVector::new(0.4, 0.4, 0.1),
                0,
                0,
            ),
            vec![CColSphere::new(0.1, CColVector::new(0.0, 0.0, -0.7), 0, 0)],
        ));
        let object_model_id = 90_usize;
        file_loader
            .model_info
            .add_simple_model(object_model_id)
            .base
            .col_model = Some(crate::collision::ColModel::CColModel::new(
            CColSphere::new(0.5, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(
                CColVector::new(-0.5, -0.5, -0.5),
                CColVector::new(0.5, 0.5, 0.5),
                0,
                0,
            ),
            vec![CColSphere::new(0.5, CColVector::new(0.0, 0.0, 0.0), 0, 0)],
        ));
        let object = ScriptObjectCollisionSnapshot {
            entity: Entity::from_raw_u32(7).expect("test entity id"),
            matrix: CMatrix::unity(),
            model_index: object_model_id,
            uses_collision: true,
            is_static: false,
            is_stuck: false,
            infinite_mass: false,
            has_been_damaged: false,
            uproot_limit: 0.0,
        };
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.physical.entity.model_index = MI_PLAYER as i16;
        ped.flags.was_standing = false;
        let mut world = script_entity_test_world();
        let result = with_script_entity_test_world(&mut world, |entities| {
            resolve_player_dynamic_motion(
                CVector::new(0.0, 0.0, 2.0),
                CVector::new(0.0, 0.0, 1.4),
                &ped,
                entities,
                &[object],
                &file_loader,
                None,
            )
        });
        assert!(result.collision_probe.is_none());
        assert!(result.landing_support.is_some());
        assert!((result.position.z - 1.54).abs() < 1.0e-4);
    }

    #[test]
    fn airborne_static_subdivision_keeps_new_support_z_after_probe_restore() {
        let mut file_loader = CFileLoader::new();
        file_loader
            .model_info
            .add_ped_model(MI_PLAYER)
            .clump
            .base
            .col_model = Some(crate::collision::ColModel::CColModel::new(
            CColSphere::new(1.2, CColVector::new(0.0, 0.0, -0.5), 0, 0),
            CColBox::new(
                CColVector::new(-0.4, -0.4, -1.1),
                CColVector::new(0.4, 0.4, 0.1),
                0,
                0,
            ),
            vec![CColSphere::new(0.1, CColVector::new(0.0, 0.0, -0.7), 0, 0)],
        ));
        let ground_model_id = 91_usize;
        file_loader
            .model_info
            .add_simple_model(ground_model_id)
            .base
            .col_model = Some(crate::collision::ColModel::CColModel::new(
            CColSphere::new(0.5, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(
                CColVector::new(-0.5, -0.5, -0.5),
                CColVector::new(0.5, 0.5, 0.5),
                0,
                0,
            ),
            vec![CColSphere::new(0.5, CColVector::new(0.0, 0.0, 0.0), 0, 0)],
        ));
        let ground_index = file_loader.world.entities.len();
        file_loader
            .world
            .entities
            .push(crate::core::World::CWorldEntity {
                entity_type: crate::core::World::EEntityType::Building,
                model_index: ground_model_id,
                name: "landing_test".to_owned(),
                position: [0.0, 0.0, 0.0],
                scale: [1.0; 3],
                rotation_axis: [0.0, 0.0, 1.0],
                rotation_w: 1.0,
                rotation_degrees: 0.0,
                matrix_basis_override: None,
                render_position_offset: [0.0; 3],
                level: 0,
                is_static: true,
                is_visible: true,
                is_big_building: false,
                is_subway: false,
                streaming_dont_delete: false,
                uses_collision: true,
                zone_culled: false,
                zone_culled2: false,
                building_pool_slot: None,
                treadable_pool_slot: None,
                map_object_slot: None,
            });
        file_loader.world.add(ground_index, &file_loader.model_info);

        let surface_table = CSurfaceTableAsset {
            filename: String::new(),
            data: rustgta3_formats::formats::DataRead::SurfaceDat::parse(
                "Rubber 1\nHard 1 1\nRoad 1 1 1\nLoose 1 1 1 1\nWet 1 1 1 1 1"
            ).expect("complete unit-adhesion test table"),
        };
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.physical.entity.model_index = MI_PLAYER as i16;
        ped.physical
            .entity
            .set_position(CVector::new(0.0, 0.0, 2.0));
        ped.flags.was_standing = false;
        ped.physical.move_speed.z = -1.0;
        let result = resolve_player_ped_static_collision_result(
            CVector::new(0.0, 0.0, 2.0),
            CVector::new(0.0, 0.0, 1.4),
            &mut ped,
            &file_loader,
            &surface_table,
            1.0,
            0,
        );
        assert!(!result.movement_rejected);
        assert!((result.position.z - 1.54).abs() < 1.0e-4);
        assert_eq!(ped.physical.move_speed.z, 0.0);
        assert!(ped.flags.is_standing);
    }

    #[test]
    fn check_if_in_air_keeps_short_nonstanding_drop_out_of_air_state() {
        assert!(player_check_if_in_air_has_ground(10.0, Some(8.46)));
        assert!(!player_check_if_in_air_has_ground(10.0, Some(8.459)));
        assert!(!player_check_if_in_air_has_ground(10.0, None));
    }

    #[test]
    fn upward_nonstanding_player_uses_re3_air_resistance_threshold_and_factor() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.flags.is_standing = false;
        ped.physical.move_speed = CVector::new(10.0, -5.0, 0.25 * GTA_PHYSICS_HZ + 0.01);
        let before = ped.physical.move_speed;
        apply_player_ped_air_resistance(&mut ped, 1.0 / GTA_PHYSICS_HZ);
        assert!((ped.physical.move_speed.x - before.x * 0.95).abs() < 1.0e-6);
        assert!((ped.physical.move_speed.y - before.y * 0.95).abs() < 1.0e-6);
        assert!((ped.physical.move_speed.z - before.z * 0.95).abs() < 1.0e-6);

        ped.physical.move_speed = CVector::new(10.0, -5.0, 0.25 * GTA_PHYSICS_HZ);
        apply_player_ped_air_resistance(&mut ped, 1.0 / GTA_PHYSICS_HZ);
        assert_eq!(
            ped.physical.move_speed,
            CVector::new(10.0, -5.0, 0.25 * GTA_PHYSICS_HZ)
        );

        ped.flags.is_standing = true;
        ped.physical.move_speed = CVector::new(10.0, -5.0, 20.0);
        apply_player_ped_air_resistance(&mut ped, 1.0 / GTA_PHYSICS_HZ);
        assert_eq!(ped.physical.move_speed, CVector::new(10.0, -5.0, 20.0));
    }

    #[test]
    fn landing_contact_update_position_replaces_stale_airborne_xy_speed() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion::default();
        ped.flags.is_standing = true;
        ped.flags.is_landing = true;
        ped.physical.move_speed = CVector::new(6.0, -3.0, 0.0);

        let delta = apply_player_standing_update_position(
            &mut ped,
            &mut locomotion,
            CVector::ZERO,
            None,
            1.0 / GTA_PHYSICS_HZ,
        );

        assert!(ped.physical.move_speed.x.abs() < 1.0e-6);
        assert!(ped.physical.move_speed.y.abs() < 1.0e-6);
        assert!(delta.x.abs() < 1.0e-6);
        assert!(delta.y.abs() < 1.0e-6);
    }

    #[test]
    fn run_stop_only_starts_from_an_established_sprint_blend() {
        assert!(!should_start_player_run_stop(
            eMoveState::PEDMOVE_STILL,
            0.0,
            0.0,
            2.0,
        ));
        assert!(!should_start_player_run_stop(
            eMoveState::PEDMOVE_STILL,
            0.0,
            0.4,
            -2.0,
        ));
        assert!(should_start_player_run_stop(
            eMoveState::PEDMOVE_STILL,
            0.0,
            0.8,
            -2.0,
        ));
        assert!(should_start_player_run_stop(
            eMoveState::PEDMOVE_STILL,
            0.0,
            0.4,
            2.0,
        ));
    }

    #[test]
    fn jump_control_tracks_input_but_landing_zeros_scalar_anim_speed() {
        let mut general = CGeneral::default();
        let mut player = CPlayerPed::new(&mut general, None);
        player.active_in_world = true;
        player.ped.ped_state = PedState::PED_JUMP;
        player.ped.flags.is_in_the_air = true;

        let mut pad = crate::core::Pad::CPad::default();
        pad.new_state.left_stick_y = -128;
        let applies_ground_motion =
            player.apply_on_foot_pad_step_camera(&pad, 1.0 / GTA_PHYSICS_HZ, true, None, false);
        assert!(!applies_ground_motion);
        assert!(player.move_speed > 0.0);

        player.ped.flags.is_in_the_air = false;
        player.ped.flags.is_landing = true;
        let applies_ground_motion =
            player.apply_on_foot_pad_step_camera(&pad, 1.0 / GTA_PHYSICS_HZ, true, None, false);
        assert!(!applies_ground_motion);
        assert_eq!(player.move_speed, 0.0);
    }

    #[test]
    fn standing_player_inherits_surface_speed_through_native_update_position_clamp() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion::default();
        ped.flags.is_standing = true;
        locomotion.rotation_cur = 0.2;
        locomotion.rotation_dest = 0.3;
        ped.physical.move_speed = CVector::ZERO;

        let delta = apply_player_standing_update_position(
            &mut ped,
            &mut locomotion,
            CVector::ZERO,
            Some(PlayerPhysicalSurfaceMotion {
                speed_native: CVector::new(0.02, 0.0, 0.0),
                turn_speed_z_native: 0.1,
                is_vehicle: false,
                is_boat: false,
            }),
            1.0 / GTA_PHYSICS_HZ,
        );

        // GetSpeed.x=0.02 native => 1.0 world-unit/s, but ordinary physical
        // surfaces cap this frame's move-speed change at 0.01 native = 0.5/s.
        assert!((ped.physical.move_speed.x - 0.5).abs() < 1.0e-6);
        assert!((delta.x - 0.01).abs() < 1.0e-6);
        assert!((locomotion.rotation_cur - 0.3).abs() < 1.0e-6);
        assert!((locomotion.rotation_dest - 0.4).abs() < 1.0e-6);
    }

    #[test]
    fn steep_cliff_update_position_uses_native_reaction_and_downward_speed() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion::default();
        ped.flags.is_standing = true;
        ped.physical.surface_touched =
            crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8;
        ped.physical.damage.normal = CVector::new(1.0, 0.0, 0.0);
        ped.physical.move_speed = CVector::new(-5.0, 2.0, 8.0);

        let delta = apply_player_standing_update_position(
            &mut ped,
            &mut locomotion,
            CVector::ZERO,
            None,
            1.0 / GTA_PHYSICS_HZ,
        );

        // 0.02 native reaction = 1.0 world-unit/s at the controller boundary;
        // native also pins Z to -0.001 => -0.05 world-unit/s.
        assert!((ped.physical.move_speed.x - 1.0).abs() < 1.0e-6);
        assert!(ped.physical.move_speed.y.abs() < 1.0e-6);
        assert!((ped.physical.move_speed.z + 0.05).abs() < 1.0e-6);
        assert!((delta.x - 0.02).abs() < 1.0e-6);

        // Walking into the cliff cannot cancel the outward reaction into an
        // inward velocity; the native negative-dot projection removes it.
        let inward_delta = CVector::new(-0.04, 0.0, 0.0);
        let _ = apply_player_standing_update_position(
            &mut ped,
            &mut locomotion,
            inward_delta,
            None,
            1.0 / GTA_PHYSICS_HZ,
        );
        assert!(ped.physical.move_speed.x.abs() < 1.0e-6);
    }

    #[test]
    fn steep_cliff_jump_gate_matches_set_jump_forward_dot() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion::default();
        locomotion.rotation_cur = 0.0; // GTA forward = +Y.
        ped.physical.surface_touched =
            crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8;
        ped.physical.damage.normal = CVector::new(0.0, -1.0, 0.0);
        assert!(!player_can_start_jump_from_surface(&ped, &locomotion));

        ped.physical.damage.normal = CVector::new(0.0, 1.0, 0.0);
        assert!(player_can_start_jump_from_surface(&ped, &locomotion));

        ped.physical.surface_touched =
            crate::core::SurfaceTable::eSurfaceType::SURFACE_TARMAC as u8;
        ped.physical.damage.normal = CVector::new(0.0, -1.0, 0.0);
        assert!(player_can_start_jump_from_surface(&ped, &locomotion));
    }

    #[test]
    fn static_support_publishes_surface_and_steep_normal_without_dynamic_relation() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let normal = CVector::new(0.6, 0.0, 0.8);
        update_player_current_phys_surface(
            &mut ped,
            Some(PlayerStaticGroundSupport {
                z: 3.0,
                surface: crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8,
                normal,
            }),
            None,
            true,
        );

        assert_eq!(ped.current_phys_surface, None);
        assert_eq!(
            ped.physical.surface_touched,
            crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8
        );
        assert!(ped.flags.hit_steep_slope);
        assert_eq!(ped.physical.damage.normal, normal);
    }

    #[test]
    fn standing_player_on_boat_uses_full_surface_velocity_like_re3_player_branch() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion::default();
        ped.flags.is_standing = true;
        ped.physical.move_speed = CVector::ZERO;

        let delta = apply_player_standing_update_position(
            &mut ped,
            &mut locomotion,
            CVector::ZERO,
            Some(PlayerPhysicalSurfaceMotion {
                speed_native: CVector::new(0.02, -0.01, 0.0),
                turn_speed_z_native: 0.0,
                is_vehicle: true,
                is_boat: true,
            }),
            1.0 / GTA_PHYSICS_HZ,
        );

        assert!((ped.physical.move_speed.x - 1.0).abs() < 1.0e-6);
        assert!((ped.physical.move_speed.y + 0.5).abs() < 1.0e-6);
        assert!((delta.x - 0.02).abs() < 1.0e-6);
        assert!((delta.y + 0.01).abs() < 1.0e-6);
    }

    #[test]
    fn nonstanding_player_does_not_consume_standing_surface_motion() {
        let mut general = CGeneral::default();
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut general);
        let mut locomotion = crate::peds::PedLocomotion::PedLocomotion::default();
        ped.flags.is_standing = false;
        ped.physical.move_speed = CVector::new(3.0, 4.0, 5.0);

        let delta = apply_player_standing_update_position(
            &mut ped,
            &mut locomotion,
            CVector::new(1.0, 1.0, 0.0),
            Some(PlayerPhysicalSurfaceMotion {
                speed_native: CVector::new(1.0, 1.0, 0.0),
                turn_speed_z_native: 1.0,
                is_vehicle: false,
                is_boat: false,
            }),
            1.0 / GTA_PHYSICS_HZ,
        );

        assert_eq!(delta, CVector::ZERO);
        assert_eq!(ped.physical.move_speed, CVector::new(3.0, 4.0, 5.0));
    }
}

#[cfg(test)]
mod system_param_conflict_tests {
    use super::*;
    use bevy::ecs::system::{IntoSystem, System};

    #[test]
    fn script_ecs_system_params_initialize_without_resource_conflicts() {
        let mut world = World::new();
        macro_rules! init_system {
            ($system:path) => {{
                let mut system = IntoSystem::into_system($system);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    system.initialize(&mut world);
                }));
                assert!(
                    result.is_ok(),
                    "SystemParam conflict while initializing {}",
                    stringify!($system)
                );
            }};
        }

        init_system!(process_active_scripts);
        init_system!(update_pacman_runtime);
        init_system!(apply_script_pacman_effects);
        init_system!(process_script_vehicle_debris);
        init_system!(apply_script_dynamic_entity_effects);
        init_system!(process_script_entity_collisions);
        init_system!(crate::renderer::HudBevy::flush_hud);
    }
}

#[cfg(test)]
mod cutscene_control_handoff_tests {
    use super::*;
    use crate::core::Pad::PLAYERCONTROL_CUTSCENE;

    #[test]
    fn malformed_spline_finishes_instead_of_locking_flyby_forever() {
        let mut cutscene = CCutsceneMgr::initialise();
        cutscene.load_cutscene_data("BROKEN", false);
        cutscene.running = true;
        cutscene.cutscene_load_status = crate::animation::CutsceneMgr::CutsceneLoadStatus::Loading0;

        let mut camera = CCamera::default();
        camera.take_control_with_spline();

        let mut app = App::new();
        app.insert_resource(cutscene)
            .insert_resource(camera)
            .insert_resource(ScriptGameplayBridge {
                player_spawned: true,
                player_visible: false,
                player_control_enabled: false,
                ..default()
            })
            .insert_resource(CPads::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(CTimer::default())
            .add_systems(Update, update_cutscene_mgr);

        app.update();

        let camera = app.world().resource::<CCamera>();
        assert_eq!(camera.script_mode, CameraScriptMode::FlyBy);
        assert!(camera.cutscene_finished);
        assert_eq!(camera.position_along_spline, 1.0);
        let bridge = app.world().resource::<ScriptGameplayBridge>();
        assert!(bridge.player_visible);
    }

    #[test]
    fn cutscene_clear_rebuilds_follow_camera_from_player_in_the_same_handoff() {
        let mut cutscene = CCutsceneMgr::initialise();
        cutscene.load_cutscene_data("INTRO", false);
        cutscene.running = true;
        let mut camera = CCamera::default();
        camera.take_control_with_spline();
        camera.position = CVector::new(1_000.0, -2_000.0, 500.0);
        camera.look_at = CVector::new(900.0, -2_100.0, 450.0);
        camera.finish_cutscene();
        let mut bridge = ScriptGameplayBridge {
            player_spawned: true,
            player_visible: false,
            player_control_enabled: false,
            playing_intro: false,
            ..default()
        };
        let mut game_state = CGameState {
            playing_intro: false,
            ..default()
        };

        restore_after_cutscene_clear(
            &mut bridge,
            Some(&mut game_state),
            &mut cutscene,
            Some(&mut camera),
        );

        let player = CVector::new(120.0, -840.0, 14.0);
        camera.process_follow_ped_with_mouse(player, 0.0, |_, _| None);

        assert!(!cutscene.loaded);
        assert!(!cutscene.running);
        assert!(bridge.player_visible);
        assert!(bridge.player_control_enabled);
        assert_eq!(camera.script_mode, CameraScriptMode::FollowPlayer);
        assert!(!camera.camera_just_restored);
        assert_eq!(
            camera.look_at,
            CVector::new(player.x, player.y, player.z + 0.8)
        );
        assert!(camera.position.magnitude() < 2_100.0);
        assert!((camera.position.x - 1_000.0).abs() > 100.0);
    }

    #[test]
    fn cutscene_owner_tracks_manager_lifetime_not_playing_intro() {
        let mut app = App::new();
        app.insert_resource(CCutsceneMgr::initialise())
            .insert_resource(ScriptGameplayBridge {
                playing_intro: true,
                ..default()
            })
            .insert_resource(CPads::default())
            .add_systems(Update, sync_cutscene_pad_controls);

        // re3 CGame::playingIntro does not own PLAYERCONTROL_CUTSCENE. The
        // owner is acquired by LoadCutsceneData and released by
        // DeleteCutsceneData, including the gap between back-to-back intro
        // cutscenes.
        app.update();
        assert!(
            !app.world()
                .resource::<CPads>()
                .get_pad(0)
                .is_player_controls_disabled_by(PLAYERCONTROL_CUTSCENE)
        );

        app.world_mut()
            .resource_mut::<CCutsceneMgr>()
            .start_cutscene_processing();
        app.update();
        assert!(
            app.world()
                .resource::<CPads>()
                .get_pad(0)
                .is_player_controls_disabled_by(PLAYERCONTROL_CUTSCENE)
        );

        app.world_mut()
            .resource_mut::<CCutsceneMgr>()
            .load_cutscene_data("BET", false);
        app.update();
        assert!(
            app.world()
                .resource::<CPads>()
                .get_pad(0)
                .is_player_controls_disabled_by(PLAYERCONTROL_CUTSCENE)
        );

        app.world_mut()
            .resource_mut::<CCutsceneMgr>()
            .delete_cutscene_data();
        app.update();
        let pad = app.world().resource::<CPads>().get_pad(0);
        assert!(!pad.is_player_controls_disabled_by(PLAYERCONTROL_CUTSCENE));
        assert!(!pad.disable_player_controls);
    }
}
