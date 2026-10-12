//! In-process developer arena for testing GTA III runtime systems.
//!
//! `Insert` suspends the current GTA frame without changing `GtaAppState`,
//! replaces its cameras/UI with an isolated grid viewport, and restores the
//! exact camera/UI/timer/cursor state on the next `Insert`.

#![allow(clippy::type_complexity)]

mod catalog;
mod controls;
mod scene;
mod ui;
mod vehicle_access;
mod vehicle_construction;
mod player_access;
mod ped_audio;
mod vehicle_spatial;
use vehicle_access::{ArenaAutomobileRuntime, ArenaAutomobileSpawn, ArenaAutomobileView,
    ArenaAutomobileMut, ArenaVehicleSettings};

use bevy::{
    camera::visibility::RenderLayers,
    ecs::hierarchy::ChildOf,
    ecs::schedule::IntoScheduleConfigs,
    ecs::system::SystemParam,
    prelude::*,
    render::storage::ShaderBuffer,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use re3bevy_assets::{
    assets::{
        AnimationAssets::CAnimationAssets,
        WorldAssets::{CHandlingDataAsset, CSurfaceTableAsset},
    },
    install::OriginalGtaInstall::OriginalGtaInstall,
};
use re3bevy_core::core::General::CGeneral;
use re3bevy_core::core::{Schedules::GtaFrameSet, Timer::CTimer};
use re3bevy_game::{
    animation::{
        AnimBlendAssociation::CAnimBlendAssociation, AnimManager::CAnimManager,
        PedVehicleAnimation::PedVehicleAnimationPhase,
    },
    core::{
        FileLoader::CFileLoader,
        FrontendFlow::FrontendTxdTextures,
        Game::{CGame, CGameBeforeRw, GtaInputCapture},
        LocalCoop::LocalCoopSession,
        Pad::CPad,
        Streaming::{CStreaming, STREAMFLAGS_DEPENDENCY, STREAMFLAGS_DONT_REMOVE},
    },
    modelinfo::ModelInfo::ModelInfoSlot,
    modelinfo::VehicleModelInfo::CVehicleColours,
    peds::{
        Ped::{CPedAnimationOffsets, PedState},
        PedFight::CFightMoveTable,
        PlayerPed::CPlayerPed,
        PlayerPed::CWorldPlayers,
    },
    renderer::BevyRender::{
        GtaWorldTextureCache, VehicleColourRemap, sync_model_texture_cache,
        sync_world_texture_cache,
    },
    renderer::{
        RendererMaterials::{GtaGlobalLighting, GtaUnlitMaterial},
        Timecycle::{CTimeCycle, TimeCycleUpdateContext},
        Weather::CWeather,
    },
};

pub const ARENA_RENDER_LAYER: usize = 29;
pub const ARENA_PANEL_WIDTH: f32 = 320.0;
pub(crate) const ARENA_LIGHTING_BUFFER: Handle<ShaderBuffer> =
    bevy::asset::uuid_handle!("f26c3199-0414-47dd-bf85-a76ae7749676");
/// re3 `FEET_OFFSET`: CPed's matrix origin sits this far above contacted ground.
pub(crate) const ARENA_PED_BASE_OFFSET: f32 = 1.04;

use re3bevy_game::renderer::{
    BevyRender::GtaParticleBillboard, Particle::CParticle, ParticleMgr::cParticleSystemMgr,
    Skidmarks::CSkidmarks,
};

/// Original GTA resources owned exclusively by the isolated developer arena.
///
/// The frontend movies run before the gameplay DAT/streaming/animation
/// resources are resident. Sharing those partially initialised resources made
/// Insert produce no Claude/Kuruma until the main game happened to finish
/// booting. re3's original files are loaded into this separate runtime so the
/// arena works at every application state without mutating or advancing the
/// suspended GTA world.
pub(crate) struct ArenaOriginalData {
    pub file_loader: CFileLoader,
    pub streaming: CStreaming,
    pub before_rw: CGameBeforeRw,
    pub handling: CHandlingDataAsset,
    pub surface_table: CSurfaceTableAsset,
    pub anim_manager: CAnimManager,
    pub ped_animation_offsets: CPedAnimationOffsets,
    /// Loaded from the original `DATA\\fistfite.dat`, exactly as
    /// `CPed::LoadFightData` does during `PedInitialise`.
    pub fight_moves: CFightMoveTable,
    pub vehicle_colours: CVehicleColours,
    pub timecycle: CTimeCycle,
    pub particle_systems: cParticleSystemMgr,
}

#[derive(Resource, Default)]
pub(crate) struct ArenaOriginalRuntime {
    pub data: Option<ArenaOriginalData>,
    attempted: bool,
    load_error: Option<String>,
}

impl ArenaOriginalRuntime {
    pub fn ensure_loaded(&mut self) -> Result<&mut ArenaOriginalData, String> {
        if self.data.is_some() {
            return Ok(self.data.as_mut().expect("arena data checked above"));
        }
        if self.attempted {
            return Err(self
                .load_error
                .clone()
                .unwrap_or_else(|| "original GTA arena runtime unavailable".to_owned()));
        }
        self.attempted = true;
        match load_arena_original_data() {
            Ok(data) => {
                self.data = Some(data);
                Ok(self.data.as_mut().expect("arena data inserted above"))
            }
            Err(error) => {
                self.load_error = Some(error.clone());
                Err(error)
            }
        }
    }
}

fn load_arena_original_data() -> Result<ArenaOriginalData, String> {
    let install = OriginalGtaInstall::from_config_file()
        .map_err(|error| format!("GTA III install resolution failed: {error}"))?;
    let mut before_rw = CGame::initialise_once_before_rw(&install)
        .map_err(|error| format!("GTA archive initialisation failed: {error}"))?;
    let handling = CHandlingDataAsset::load(&mut before_rw.file_mgr)
        .map_err(|error| format!("HANDLING.CFG load failed: {error}"))?;
    let surface_table = CSurfaceTableAsset::load(&mut before_rw.file_mgr)
        .map_err(|error| format!("SURFACE.DAT load failed: {error}"))?;
    let animation_assets = CAnimationAssets::load_anim_files(&mut before_rw.file_mgr)
        .map_err(|error| format!("PED.IFP load failed: {error}"))?;
    let anim_manager = CAnimManager::load_anim_files(&animation_assets);
    let ped_animation_offsets = CPedAnimationOffsets::from_anim_manager(&anim_manager)
        .ok_or_else(|| "PED.IFP vehicle-entry offsets are unavailable".to_owned())?;
    let fight_moves = CFightMoveTable::load(&mut before_rw.file_mgr, &anim_manager)
        .map_err(|error| format!("DATA\\fistfite.dat load failed: {error}"))?;
    let mut timecycle = CTimeCycle::initialise(&mut before_rw.file_mgr)
        .map_err(|error| format!("TIMECYC.DAT load failed: {error}"))?;
    let _ = timecycle.update(
        &CWeather::default(),
        TimeCycleUpdateContext {
            hour: 12,
            minute: 0,
            camera_forward_z: 0.0,
            camera_motion_blur_light_scene: false,
            no_rain_from_cullzone: false,
            cutscene_running: false,
        },
    );

    let mut file_loader = CFileLoader::new();
    file_loader
        .load_default_and_dat_plans(
            &install,
            &mut before_rw.file_mgr,
            &mut before_rw.cd_stream,
            "DATA\\GTA3.DAT",
        )
        .map_err(|error| format!("DEFAULT.DAT/GTA3.DAT load failed: {error}"))?;
    let vehicle_colours =
        CVehicleColours::load(&mut before_rw.file_mgr, &mut file_loader.model_info)
            .map_err(|error| format!("CARCOLS.DAT load failed: {error}"))?;
    let mut streaming = CStreaming::default();
    streaming
        .load_cd_directory(&before_rw.cd_stream, &mut file_loader)
        .map_err(|error| format!("GTA3.DIR load failed: {error}"))?;
    let particle_systems = cParticleSystemMgr::initialise(&mut before_rw.file_mgr)
        .map_err(|error| format!("DATA\\PARTICLE.CFG load failed: {error}"))?;

    Ok(ArenaOriginalData {
        file_loader,
        streaming,
        before_rw,
        handling,
        surface_table,
        anim_manager,
        ped_animation_offsets,
        fight_moves,
        vehicle_colours,
        timecycle,
        particle_systems,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArenaAnimation {
    #[default]
    Idle,
    Walk,
    Run,
    Sprint,
    Jump,
    EnterVehicle,
    Fight,
}

impl ArenaAnimation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Walk => "Walk",
            Self::Run => "Run",
            Self::Sprint => "Sprint",
            Self::Jump => "Jump",
            Self::EnterVehicle => "Enter Vehicle",
            Self::Fight => "Fight",
        }
    }
}

#[derive(Debug, Resource)]
pub struct DeveloperArena {
    pub active: bool,
    pub selected_character: usize,
    pub selected_vehicle: usize,
    pub selected_prop: usize,
    pub teleport_dropdown_open: bool,
    pub selected_teleport: usize,
    pub custom_model_index: usize,
    pub animation: ArenaAnimation,
    pub animation_speed: f32,
    pub runtime_animation_selection: Option<ArenaAnimation>,
    pub runtime_animation_name: Option<String>,
    pub runtime_animation_time: f32,
    pub runtime_animation_duration: f32,
    pub runtime_animation_mesh_bound: bool,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_distance: f32,
    pub zoom_mode: bool,
    pub controlled_vehicle: Option<Entity>,
    pub(crate) vehicle_transition: Option<ArenaVehicleTransition>,
    pub request_enter_exit: bool,
    pub player_entity: Option<Entity>,
    pub test_pad: CPad,
    pub spawned_characters: u32,
    pub spawned_vehicles: u32,
    pub spawned_props: u32,
    pub status: String,
    previous_code_pause: bool,
    previous_cursor_visible: bool,
    previous_cursor_grab: CursorGrabMode,
}

impl Default for DeveloperArena {
    fn default() -> Self {
        Self {
            active: false,
            selected_character: 0,
            selected_vehicle: 0,
            selected_prop: 0,
            teleport_dropdown_open: false,
            selected_teleport: 0,
            custom_model_index: 0,
            animation: ArenaAnimation::Idle,
            animation_speed: 1.0,
            runtime_animation_selection: None,
            runtime_animation_name: None,
            runtime_animation_time: 0.0,
            runtime_animation_duration: 0.0,
            runtime_animation_mesh_bound: false,
            camera_yaw: 0.0,
            camera_pitch: 0.32,
            camera_distance: 8.0,
            zoom_mode: false,
            controlled_vehicle: None,
            vehicle_transition: None,
            request_enter_exit: false,
            player_entity: None,
            test_pad: CPad::default(),
            spawned_characters: 0,
            spawned_vehicles: 0,
            spawned_props: 0,
            status: "Insert: open developer arena".to_owned(),
            previous_code_pause: false,
            previous_cursor_visible: true,
            previous_cursor_grab: CursorGrabMode::None,
        }
    }
}

/// Persistent `m_pVehicleAnim` ownership for an Arena player. Production uses
/// a CPools handle; the isolated Arena retains the ECS vehicle entity while
/// traversing the same phase and association chain.
#[derive(Debug, Clone)]
pub(crate) struct ArenaVehicleTransition {
    pub vehicle: Entity,
    pub door: usize,
    pub association: CAnimBlendAssociation,
    pub phase: PedVehicleAnimationPhase,
    /// re3 `PED_SEEK_CAR`: locomotion owns the ped until the 0.5-unit
    /// arrival radius is reached. The align association starts only after
    /// `SetEnterCar_AllClear` reserves the door and disables ped collision.
    pub seeking: bool,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaOwned;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaSceneRoot;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaCamera {
    pub local_player: u8,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaGrid;

/// Root for the streamed original-GTA district displayed inside the isolated
/// developer arena. It never belongs to the production gameplay world.
#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaWorldMap;

/// One streamed IPL instance owned by the isolated Arena world renderer.
#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaWorldMapEntity {
    pub entity_index: usize,
    /// The concrete `CSimpleModelInfo` atomic selected for the nearest Arena
    /// camera.  re3 changes this binding as the camera crosses IDE draw-distance
    /// bands; keeping it here lets the isolated renderer rebuild only then.
    pub lod_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaPlayer {
    /// Bevy X/Z velocity retained from re3's `m_vecMoveSpeed.x/y` while the
    /// launch/glide association owns locomotion.
    pub horizontal_velocity: Vec2,
    pub vertical_velocity: f32,
    pub grounded: bool,
    pub previous_position: Vec3,
}

#[derive(Debug, Clone, Component)]
pub(crate) struct ArenaCoopPlayerTwo {
    pub pad: CPad,
    pub horizontal_velocity: Vec2,
    pub vertical_velocity: f32,
    pub grounded: bool,
    pub previous_position: Vec3,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_distance: f32,
    pub controlled_vehicle: Option<Entity>,
    pub vehicle_transition: Option<ArenaVehicleTransition>,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaVehicle {
    pub model_index: usize,
    pub speed: f32,
    pub steering: f32,
    pub occupied: bool,
    pub colour_remap: VehicleColourRemap,
}


#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaProp {
    pub model_index: usize,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaSpawned;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaCharacterVisual {
    pub local_player: u8,
    pub model_index: usize,
    pub loaded_dff: bool,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaCharacterModelPart {
    pub local_player: u8,
    pub part_index: usize,
    pub rigid_frame_index: Option<i32>,
}

/// Immutable original drawable order owned by this resident visual instance.
/// Entity generations and ChildOf are checked against live ECS parts at pose time.
#[derive(Debug, Component)]
pub(crate) struct ArenaCharacterPoseManifest {
    pub model_index: usize,
    pub parts: Vec<Entity>,
    pub rigid_frame_indices: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Component)]
pub(crate) struct ArenaVehicleModelPart {
    pub vehicle: Entity,
    pub dynamic: bool,
    pub component_baseline: Option<Transform>,
    pub source_part_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaModelPart;

#[derive(Debug, Clone, Copy, Component)]
struct ArenaDisabledCamera {
    was_active: bool,
}

#[derive(Debug, Clone, Copy, Component)]
struct ArenaHiddenUi {
    previous_visibility: Visibility,
    previous_display: Display,
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaStatusText;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaMouseCursor;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaTeleportOption;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaTeleportList;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ArenaTeleportLabel;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub(crate) enum ArenaAction {
    Reset,
    ClearSpawned,
    ToggleLocalCoop,
    PreviousCharacter,
    NextCharacter,
    SpawnCharacter,
    Animation(ArenaAnimation),
    AnimationSlower,
    AnimationFaster,
    PreviousVehicle,
    NextVehicle,
    SpawnVehicle,
    EnterExitVehicle,
    PreviousProp,
    NextProp,
    SpawnProp,
    SpawnTestCourse,
    CustomModelMinus10,
    CustomModelMinus1,
    CustomModelPlus1,
    CustomModelPlus10,
    SpawnCustomModel,
    ToggleTeleportDropdown,
    SelectTeleport(usize),
    ExecuteTeleport,
    TeleportForward,
}

pub struct DeveloperArenaPlugin;

/// The isolated arena shares one native random stream across all its actors.
/// Its app lifetime is independent of asset residency and scene retirement.
#[derive(Default, Resource)]
pub(crate) struct ArenaRandomState(pub CGeneral);

impl Plugin for DeveloperArenaPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<ShaderBuffer>>()
            .insert(
                ARENA_LIGHTING_BUFFER.id(),
                ShaderBuffer::from(vec![GtaGlobalLighting::default()]),
            )
            .expect("constant developer arena lighting buffer id must be valid");
        app.init_resource::<DeveloperArena>()
            .init_resource::<vehicle_spatial::ArenaVehicleSpatial>()
            .init_resource::<ArenaRandomState>()
            .init_resource::<vehicle_construction::ArenaVehicleSpawnDiagnostics>()
            .init_resource::<re3bevy_game::audio::AudioLogic::VehicleOneShotAudioState>()
            .init_resource::<re3bevy_game::audio::PlayerEngineAudio::PlayerEngineAudioState>()
            .init_resource::<ArenaOriginalRuntime>()
            .init_resource::<re3bevy_game::renderer::Particle::ParticleCreationPolicy>()
            .init_resource::<CParticle>()
            .init_resource::<cParticleSystemMgr>()
            .init_resource::<CSkidmarks>()
            .init_resource::<re3bevy_game::renderer::BevyRender::GtaImmediateMeshCache>()
            .configure_sets(
                Update,
                GtaFrameSet::GameProcess.run_if(developer_arena_inactive),
            )
            .add_systems(
                PreUpdate,
                toggle_developer_arena.after(bevy::input::InputSystems),
            )
            .add_systems(
                Update,
                (
                    (
                        ui::handle_arena_buttons,
                        sync_arena_local_coop,
                        ui::sync_teleport_dropdown,
                        ui::scroll_teleport_dropdown,
                        ui::sync_arena_custom_cursor,
                        isolate_non_arena_ui,
                        controls::control_arena,
                        controls::control_arena_player_two,
                        controls::step_uncontrolled_arena_automobiles,
                        controls::resolve_arena_vehicle_collisions,
                        controls::resolve_arena_static_collisions,
                    )
                        .chain(),
                    (
                        controls::animate_arena_player_two,
                        scene::update_arena_vehicle_visuals,
                        scene::update_arena_vehicle_effects.in_set(scene::ArenaParticleStage::Effects),
                        scene::allocate_arena_vehicle_audio_identities,
                        scene::update_arena_vehicle_audio,
                        ped_audio::process_arena_ped_audio,
                        (controls::advance_arena_primary_animation,
                            controls::present_arena_primary_pose).chain(),
                        scene::update_grid_origin,
                        scene::update_arena_camera,
                        scene::update_arena_world_streaming,
                        re3bevy_game::renderer::BevyRender::consume_particle_intents.in_set(scene::ArenaParticleStage::Consume),
                        re3bevy_game::renderer::BevyRender::update_particle_billboards,
                        re3bevy_game::renderer::ParticleSprite::build_particle_sprite_frame,
                        scene::update_viewport,
                        ui::update_status_text,
                    )
                        .chain(),
                )
                    .chain()
                    .after(scene::ArenaParticleStage::Inputs)
                    .run_if(developer_arena_active),
            )
            .add_systems(Update, vehicle_spatial::sync_arena_vehicle_spatial
                .after(controls::step_uncontrolled_arena_automobiles)
                .before(controls::resolve_arena_vehicle_collisions))
            .add_systems(Update, scene::sync_arena_particle_creation_inputs
                .in_set(scene::ArenaParticleStage::Inputs)
                .after(re3bevy_game::renderer::Particle::ParticleCreationSet::Inputs)
                .run_if(developer_arena_active))
            .add_systems(PostUpdate, (scene::retire_arena_vehicle_audio, ped_audio::retire_arena_ped_audio));
    }
}

fn sync_arena_local_coop(
    mut commands: Commands,
    arena: Res<DeveloperArena>,
    mut session: ResMut<LocalCoopSession>,
    scene_root: Query<Entity, With<ArenaSceneRoot>>,
    player_two: Query<(Entity, &ArenaCoopPlayerTwo)>,
    mut vehicles: Query<&mut ArenaVehicle>,
    runtime: ArenaRuntimeAssets,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    primary: player_access::ArenaPrimaryPlayerReader,
) {
    if !session.enabled {
        for (entity, player) in &player_two {
            if let Some(vehicle) = player.controlled_vehicle
                && let Ok(mut vehicle) = vehicles.get_mut(vehicle)
            {
                vehicle.occupied = false;
            }
            commands.entity(entity).despawn();
        }
        session.player_two_spawned = false;
        return;
    }
    if !player_two.is_empty() {
        session.player_two_spawned = true;
        return;
    }
    let Ok(root) = scene_root.single() else {
        return;
    };
    let Some(source) = primary.get(arena.player_entity) else {
        return;
    };
    let mut player = CPlayerPed { ped: source.ped.clone(), vitality: source.vitality.clone(),
        locomotion: source.locomotion.clone(), animation: source.animation.clone(), state: source.state.clone() };
    let mut position = player.position();
    position.x += 1.5;
    player.set_position(position);
    player.ped.ped_state = PedState::PED_IDLE;
    player.ped.physical.audio_entity_id = -1;
    player.ped.queued_footstep = None;
    let entity = commands
        .spawn((
            player.into_bundle(),
            ArenaCoopPlayerTwo {
                pad: CPad::default(),
                horizontal_velocity: Vec2::ZERO,
                vertical_velocity: 0.0,
                grounded: true,
                previous_position: position.to_bevy(),
                camera_yaw: arena.camera_yaw,
                camera_pitch: arena.camera_pitch,
                camera_distance: arena.camera_distance,
                controlled_vehicle: None,
                vehicle_transition: None,
            },
            Transform::from_translation(position.to_bevy()),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new("Arena Local Co-op Player 2"),
        ))
        .id();
    commands.entity(root).add_child(entity);
    let model = catalog::CHARACTERS[arena.selected_character].model_index;
    let file_loader = runtime.original.data.as_ref().map(|data| &data.file_loader);
    scene::spawn_character_visual(
        &mut commands,
        entity,
        1,
        model,
        file_loader,
        &runtime.texture_cache,
        &mut meshes,
        &mut materials,
        &RenderLayers::layer(ARENA_RENDER_LAYER),
    );
    session.player_two_spawned = true;
}

fn arena_global_lighting(timecycle: &CTimeCycle) -> GtaGlobalLighting {
    let current = &timecycle.current;
    // rw_unlit performs GTA III's D3D8 fog blend in encoded channel space,
    // then converts the final fragment for Bevy's linear view target.
    let fog = Vec3::new(
        current.fog_colour[0].clamp(0, 255) as f32 / 255.0,
        current.fog_colour[1].clamp(0, 255) as f32 / 255.0,
        current.fog_colour[2].clamp(0, 255) as f32 / 255.0,
    );
    let sun = timecycle.vector_to_sun[timecycle.current_stored_value.rem_euclid(16) as usize]
        .to_bevy()
        .normalize_or_zero();
    let sun = if sun.length_squared() > 1.0e-6 {
        sun
    } else {
        Vec3::Z
    };
    GtaGlobalLighting {
        fog_colour_and_start: fog.extend(current.fog_start),
        ambient: Vec3::from_array(current.ambient).extend(0.0),
        actor_ambient: (Vec3::from_array(current.ambient) * 1.3).min(Vec3::ONE).extend(0.0),
        directional: Vec3::from_array(current.directional).extend(0.0),
        light_direction_and_far_clip: sun.extend(current.far_clip),
        postfx_blur_colour: Vec4::new(
            (current.blur[0] / 255.0).clamp(0.0, 1.0),
            (current.blur[1] / 255.0).clamp(0.0, 1.0),
            (current.blur[2] / 255.0).clamp(0.0, 1.0),
            (current.blur[3] / 255.0).clamp(0.0, 1.0),
        ),
        ..Default::default()
    }
}

pub fn developer_arena_active(arena: Res<DeveloperArena>) -> bool {
    arena.active
}

pub fn developer_arena_inactive(arena: Res<DeveloperArena>) -> bool {
    !arena.active
}

fn model_has_renderable_geometry(file_loader: &CFileLoader, model_index: usize) -> bool {
    file_loader
        .model_info
        .get_model_info(model_index)
        .is_some_and(|model| {
            model
                .base()
                .clump_summary
                .as_ref()
                .is_some_and(|summary| !summary.geometries.is_empty())
                || file_loader
                    .model_info
                    .simple_model_info(model_index)
                    .is_some_and(|simple| !simple.atomic_bindings.is_empty())
        })
}

fn model_texture_dictionary_is_resident(file_loader: &CFileLoader, model_index: usize) -> bool {
    file_loader
        .model_info
        .get_model_info(model_index)
        .is_some_and(|model| {
            let txd_slot = model.base().txd_slot;
            txd_slot < 0 || file_loader.txd_store.is_txd_loaded(txd_slot as usize)
        })
}

fn arena_model_dependencies(file_loader: &CFileLoader, model_index: usize) -> Vec<usize> {
    let mut dependencies = vec![model_index];
    if let Some(ModelInfoSlot::Vehicle(vehicle)) =
        file_loader.model_info.get_model_info(model_index)
        && let Ok(wheel_model) = usize::try_from(vehicle.wheel_id)
        && wheel_model != model_index
    {
        dependencies.push(wheel_model);
    }
    dependencies
}

/// Arena equivalent of re3's `RequestModel` + `LoadAllRequestedModels`.
///
/// The arena pauses the normal GTA frame, so relying on the world streaming
/// update can never make a newly selected model resident. Load the selected
/// original GTA model and its TXD synchronously before creating its Bevy mesh.
pub(crate) fn ensure_arena_model_resident(
    model_index: usize,
    file_loader: Option<&mut CFileLoader>,
    streaming: Option<&mut CStreaming>,
    before_rw: Option<&mut CGameBeforeRw>,
) -> Result<(), String> {
    let file_loader =
        file_loader.ok_or_else(|| "GTA file loader is not initialised yet".to_owned())?;
    let dependencies = arena_model_dependencies(file_loader, model_index);
    if dependencies.iter().copied().all(|dependency| {
        model_has_renderable_geometry(file_loader, dependency)
            && model_texture_dictionary_is_resident(file_loader, dependency)
    }) {
        return Ok(());
    }
    let streaming =
        streaming.ok_or_else(|| "GTA streaming runtime is not initialised yet".to_owned())?;
    let before_rw =
        before_rw.ok_or_else(|| "GTA CD streaming archive is not initialised yet".to_owned())?;

    let flags = STREAMFLAGS_DONT_REMOVE | STREAMFLAGS_DEPENDENCY;
    for dependency in dependencies.iter().copied() {
        let txd_slot = file_loader
            .model_info
            .get_model_info(dependency)
            .map(|model| model.base().txd_slot)
            .filter(|slot| *slot >= 0)
            .map(|slot| slot as usize);
        if let Some(txd_slot) = txd_slot
            && !file_loader.txd_store.is_txd_loaded(txd_slot)
        {
            streaming.request_txd(txd_slot, flags);
        }
        if dependency == re3bevy_game::modelinfo::ModelIndices::MI_PLAYER
            && streaming.extra_objects_dir.find_item("player").is_some()
        {
            streaming.request_special_model(dependency, "player", flags, file_loader);
        } else {
            streaming.request_model_with_adapter(dependency, flags, file_loader);
        }
    }
    file_loader
        .load_all_requested_models(streaming, &mut before_rw.cd_stream, false)
        .map_err(|error| format!("GTA model #{model_index} streaming failed: {error}"))?;

    dependencies
        .iter()
        .copied()
        .all(|dependency| {
            model_has_renderable_geometry(file_loader, dependency)
                && model_texture_dictionary_is_resident(file_loader, dependency)
        })
        .then_some(())
        .ok_or_else(|| {
            let name = file_loader
                .model_info
                .get_model_info(model_index)
                .map(|model| model.base().name.as_str())
                .unwrap_or("<unknown>");
            format!("GTA model #{model_index} ({name}) loaded without renderable DFF/TXD data")
        })
}

#[derive(SystemParam)]
pub(crate) struct ArenaRuntimeAssets<'w> {
    pub original: ResMut<'w, ArenaOriginalRuntime>,
    pub texture_cache: ResMut<'w, GtaWorldTextureCache>,
    pub images: ResMut<'w, Assets<Image>>,
    pub lighting_buffers: Option<ResMut<'w, Assets<ShaderBuffer>>>,
}

#[derive(SystemParam)]
struct ArenaToggleInput<'w> {
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    mouse_buttons: ResMut<'w, ButtonInput<MouseButton>>,
    capture: ResMut<'w, GtaInputCapture>,
}

#[derive(SystemParam)]
struct ArenaViewSuspension<'w, 's> {
    cameras: Query<
        'w,
        's,
        (Entity, &'static mut Camera),
        (Without<ArenaCamera>, Without<ArenaDisabledCamera>),
    >,
    disabled_cameras: Query<'w, 's, (Entity, &'static mut Camera, &'static ArenaDisabledCamera)>,
    ui_roots: Query<
        'w,
        's,
        (Entity, &'static mut Node, &'static mut Visibility),
        (With<Node>, Without<ArenaOwned>, Without<ArenaHiddenUi>),
    >,
    hidden_ui: Query<
        'w,
        's,
        (
            Entity,
            &'static mut Node,
            &'static mut Visibility,
            &'static ArenaHiddenUi,
        ),
    >,
}

#[derive(SystemParam)]
struct ArenaAssetResources<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    gta_materials: ResMut<'w, Assets<GtaUnlitMaterial>>,
}

fn toggle_developer_arena(
    mut commands: Commands,
    mut input: ArenaToggleInput,
    mut arena: ResMut<DeveloperArena>,
    mut timer: Option<ResMut<CTimer>>,
    mut particles: Option<ResMut<CParticle>>,
    mut particle_mgr: Option<ResMut<cParticleSystemMgr>>,
    world_players: Option<CWorldPlayers<'_, '_>>,
    frontend_textures: Option<Res<FrontendTxdTextures>>,
    mut runtime: ArenaRuntimeAssets,
    mut suspension: ArenaViewSuspension,
    arena_entities: Query<Entity, With<ArenaOwned>>,
    particles_query: Query<Entity, With<GtaParticleBillboard>>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    assets: ArenaAssetResources,
    mut random: ResMut<ArenaRandomState>,
) {
    let insert_pressed = input.keys.just_pressed(KeyCode::Insert);
    if !insert_pressed {
        return;
    }
    if insert_pressed {
        input.keys.clear_just_pressed(KeyCode::Insert);
    }
    let arena_cursor_image = frontend_textures
        .as_ref()
        .and_then(|textures| textures.image_handle("mouse"));

    if arena.active {
        input.keys.reset_all();
        input.mouse_buttons.reset_all();
        input.capture.developer_arena = false;
        if let Some(particles) = particles.as_mut() {
            let _ = particles.drain_frame_intents();
        }
        for entity in &arena_entities {
            commands.entity(entity).despawn();
        }
        for entity in &particles_query {
            commands.entity(entity).despawn();
        }
        for (entity, mut camera, snapshot) in &mut suspension.disabled_cameras {
            camera.is_active = snapshot.was_active;
            commands.entity(entity).remove::<ArenaDisabledCamera>();
        }
        for (entity, mut node, mut visibility, snapshot) in &mut suspension.hidden_ui {
            node.display = snapshot.previous_display;
            *visibility = snapshot.previous_visibility;
            commands.entity(entity).remove::<ArenaHiddenUi>();
        }
        if let Some(timer) = timer.as_mut() {
            timer.set_code_pause(arena.previous_code_pause);
        }
        if let Ok(mut cursor) = cursor.single_mut() {
            cursor.visible = arena.previous_cursor_visible;
            cursor.grab_mode = arena.previous_cursor_grab;
        }
        arena.active = false;
        arena.controlled_vehicle = None;
        arena.vehicle_transition = None;
        arena.player_entity = None;
        arena.runtime_animation_selection = None;
        arena.runtime_animation_name = None;
        arena.runtime_animation_time = 0.0;
        arena.runtime_animation_duration = 0.0;
        arena.runtime_animation_mesh_bound = false;
        arena.status = "Developer arena closed; GTA state resumed".to_owned();
        info!("Developer arena closed; GTA state resumed");
        return;
    }

    // Opening Insert is consumed here; no frontend system may see it or any
    // other key edge from the same frame while the arena owns input.
    input.keys.reset_all();
    input.mouse_buttons.reset_all();
    input.capture.developer_arena = true;
    // Particle intents and billboards belong to the world that produced them.
    // The gameplay world is suspended while the arena is active, so carrying
    // its already-spawned smoke into the isolated camera makes exhaust appear
    // detached from arena vehicles. Start the arena particle view clean.
    if let Some(particles) = particles.as_mut() {
        let _ = particles.drain_frame_intents();
    }
    for entity in &particles_query {
        commands.entity(entity).despawn();
    }

    if let Some(timer) = timer.as_mut() {
        arena.previous_code_pause = timer.get_is_code_paused();
        timer.set_code_pause(true);
    }
    if let Ok(mut cursor) = cursor.single_mut() {
        arena.previous_cursor_visible = cursor.visible;
        arena.previous_cursor_grab = cursor.grab_mode;
        // The arena renders the original GTA III frontend cursor as a Bevy UI
        // node. Never reveal the native Windows pointer while it owns input.
        cursor.visible = arena_cursor_image.is_none();
        cursor.grab_mode = CursorGrabMode::None;
    }
    for (entity, mut camera) in &mut suspension.cameras {
        let was_active = camera.is_active;
        camera.is_active = false;
        commands
            .entity(entity)
            .insert(ArenaDisabledCamera { was_active });
    }
    for (entity, mut node, mut visibility) in &mut suspension.ui_roots {
        let previous_visibility = *visibility;
        let previous_display = node.display;
        node.display = Display::None;
        *visibility = Visibility::Hidden;
        commands.entity(entity).insert(ArenaHiddenUi {
            previous_visibility,
            previous_display,
        });
    }
    arena.active = true;
    info!("Developer arena opened; GTA state suspended");
    arena.controlled_vehicle = None;
    arena.vehicle_transition = None;
    let mut test_player = world_players
        .as_ref()
        .and_then(|players| players.focused())
        .map(|player| CPlayerPed {
            ped: player.ped.clone(),
            vitality: player.vitality.clone(),
            locomotion: player.locomotion.clone(),
            animation: player.animation.clone(),
            state: player.state.clone(),
        })
        .unwrap_or_else(|| {
            CPlayerPed::new(&mut random.0, None)
        });
    test_player.active_in_world = true;
    test_player.ped.physical.audio_entity_id = -1;
    test_player.ped.queued_footstep = None;
    test_player.ped.ped_state = PedState::PED_IDLE;
    test_player.ped.last_ped_state = PedState::PED_NONE;
    test_player.ped.move_state = re3bevy_game::peds::Ped::eMoveState::PEDMOVE_STILL;
    test_player.move_speed = 0.0;
    test_player.locomotion.anim_move_delta = re3bevy_core::core::Coordinates::CVector2D::new(0.0, 0.0);
    test_player.locomotion.moved = re3bevy_core::core::Coordinates::CVector2D::new(0.0, 0.0);
    test_player.ped.physical.move_speed = re3bevy_core::core::Coordinates::CVector::ZERO;
    test_player.ped.physical.turn_speed = re3bevy_core::core::Coordinates::CVector::ZERO;
    test_player.ped.physical.move_friction = re3bevy_core::core::Coordinates::CVector::ZERO;
    test_player.ped.physical.turn_friction = re3bevy_core::core::Coordinates::CVector::ZERO;
    test_player.ped.physical.entity.flags.has_hit_wall = false;
    test_player.ped.physical.entity.flags.has_collided = false;
    test_player.ped.physical.entity.flags.has_contacted = false;
    test_player.ped.physical.entity.flags.uses_collision = true;
    test_player.ped.flags.is_standing = true;
    test_player.ped.flags.was_standing = true;
    test_player.ped.flags.is_in_the_air = false;
    test_player.ped.flags.is_landing = false;
    test_player.ped.flags.hit_steep_slope = false;
    test_player.jump_association = None;
    test_player.jump_animation_time = 0.0;
    test_player.mouse_leg_turn_angle = None;
    test_player.set_position(re3bevy_core::core::Coordinates::CVector::new(
        0.0,
        0.0,
        ARENA_PED_BASE_OFFSET,
    ));
    test_player.set_heading_degrees(0.0);
    arena.test_pad = CPad::default();
    arena.runtime_animation_selection = None;
    arena.runtime_animation_name = None;
    arena.runtime_animation_time = 0.0;
    arena.runtime_animation_duration = 0.0;
    arena.runtime_animation_mesh_bound = false;
    let initial_model = catalog::CHARACTERS[arena.selected_character];
    let arena_load = runtime.original.ensure_loaded().and_then(|data| {
        ensure_arena_model_resident(
            initial_model.model_index,
            Some(&mut data.file_loader),
            Some(&mut data.streaming),
            Some(&mut data.before_rw),
        )
    });
    let arena_load = arena_load.and_then(|()| {
        let data = runtime
            .original
            .data
            .as_ref()
            .ok_or_else(|| "original GTA arena runtime disappeared".to_owned())?;
        sync_model_texture_cache(
            initial_model.model_index,
            &data.file_loader,
            &mut runtime.texture_cache,
            &mut runtime.images,
        )
        .then_some(())
        .ok_or_else(|| {
            format!(
                "Arena model #{} TXD raster upload did not complete",
                initial_model.model_index
            )
        })
    });
    arena.status = match arena_load {
        Ok(()) => format!(
            "Arena ready — original GTA {} DFF/TXD/PED.IFP resident",
            initial_model.label
        ),
        Err(error) => format!("Arena model load failed: {error}"),
    };
    if let Some(data) = runtime.original.data.as_ref() {
        sync_world_texture_cache(
            &data.file_loader,
            &mut runtime.texture_cache,
            &mut runtime.images,
        );
        if let Some(mgr) = particle_mgr.as_mut() {
            if mgr.particles.is_empty() || mgr.particles.iter().all(|p| p.name.is_empty()) {
                **mgr = data.particle_systems.clone();
            }
        }
        if let Some(lighting_buffers) = runtime.lighting_buffers.as_mut()
            && let Some(mut buffer) = lighting_buffers.get_mut(ARENA_LIGHTING_BUFFER.id())
        {
            buffer.clear();
            buffer.extend_from_slice(&[arena_global_lighting(&data.timecycle)]);
        }
    }
    let file_loader = runtime.original.data.as_ref().map(|data| &data.file_loader);
    scene::spawn_arena(
        &mut commands,
        &mut arena,
        test_player,
        file_loader,
        &runtime.texture_cache,
        assets.meshes,
        assets.materials,
        assets.gta_materials,
        RenderLayers::layer(ARENA_RENDER_LAYER),
    );
    ui::spawn_arena_ui(&mut commands, arena_cursor_image);
}

fn isolate_non_arena_ui(
    mut commands: Commands,
    mut ui: Query<
        (Entity, &mut Node, &mut Visibility),
        (With<Node>, Without<ArenaOwned>, Without<ArenaHiddenUi>),
    >,
    parent_query: Query<&ChildOf>,
    arena_owned_query: Query<(), With<ArenaOwned>>,
) {
    for (entity, mut node, mut visibility) in &mut ui {
        if is_arena_ui_descendant(entity, &parent_query, &arena_owned_query) {
            continue;
        }
        let previous_visibility = *visibility;
        let previous_display = node.display;
        node.display = Display::None;
        *visibility = Visibility::Hidden;
        commands.entity(entity).insert(ArenaHiddenUi {
            previous_visibility,
            previous_display,
        });
    }
}

fn is_arena_ui_descendant(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    arena_owned_query: &Query<(), With<ArenaOwned>>,
) -> bool {
    let mut current = entity;
    // A malformed hierarchy must never turn this diagnostic filter into an
    // infinite loop. Bevy's normal hierarchy depth is far below this bound.
    for _ in 0..256 {
        let Ok(parent) = parent_query.get(current) else {
            return false;
        };
        current = parent.parent();
        if arena_owned_query.get(current).is_ok() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use re3bevy_core::core::Coordinates::CVector;

    #[test]
    fn arena_close_and_reopen_preserve_the_continuing_random_stream() {
        let mut app = toggle_test_app();
        app.world_mut().resource_mut::<ArenaRandomState>().0.mysrand(777);
        press_insert(&mut app);
        let opened_seed = app.world().resource::<ArenaRandomState>().0.seed();
        press_insert(&mut app);
        assert_eq!(app.world().resource::<ArenaRandomState>().0.seed(), opened_seed);
        let mut expected = app.world().resource::<ArenaRandomState>().0.clone();
        let _new_player = CPlayerPed::new(&mut expected, None);
        press_insert(&mut app);
        assert_eq!(app.world().resource::<ArenaRandomState>().0.seed(), expected.seed(),
            "reopen consumes the next native player construction, never a reset stream");
    }

    #[test]
    fn primary_arena_actor_bundle_retires_and_reopens_with_fresh_native_state() {
        let mut app = toggle_test_app();
        press_insert(&mut app);
        let first = app.world().resource::<DeveloperArena>().player_entity.unwrap();
        assert!(app.world().get::<re3bevy_game::peds::Ped::CPed>(first).is_some());
        assert!(app.world().get::<re3bevy_game::peds::PedAnimation::PedAnimation>(first).is_some());
        assert!(app.world().get::<re3bevy_game::peds::PedLocomotion::PedLocomotion>(first).is_some());
        app.world_mut().get_mut::<re3bevy_game::peds::PedVitality::PedVitality>(first).unwrap().health = 72.0;
        press_insert(&mut app);
        assert!(app.world().resource::<DeveloperArena>().player_entity.is_none());
        assert!(app.world().get_entity(first).is_err());
        press_insert(&mut app);
        let second = app.world().resource::<DeveloperArena>().player_entity.unwrap();
        assert_ne!(first, second);
        assert_eq!(app.world().get::<re3bevy_game::peds::PedVitality::PedVitality>(second).unwrap().health, 100.0);
        assert_eq!(*app.world().get::<re3bevy_game::peds::PedObstacle::PedObstacleAvoidance>(second).unwrap(), Default::default());
    }

    fn toggle_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<DeveloperArena>()
            .init_resource::<ArenaRandomState>()
            .insert_resource(ArenaOriginalRuntime {
                attempted: true,
                load_error: Some("unit test intentionally has no proprietary runtime".to_owned()),
                ..default()
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<GtaInputCapture>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<GtaUnlitMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<GtaWorldTextureCache>()
            .init_resource::<CTimer>()
            .init_resource::<re3bevy_game::renderer::BevyRender::GtaImmediateMeshCache>()
            .add_systems(PreUpdate, toggle_developer_arena)
            .add_systems(Update, isolate_non_arena_ui.run_if(developer_arena_active));
        app
    }

    fn press_insert(app: &mut App) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Insert);
        keys.press(KeyCode::Insert);
        drop(keys);
        app.update();
    }

    #[test]
    fn animation_labels_are_stable_for_the_debug_ui() {
        assert_eq!(ArenaAnimation::Idle.label(), "Idle");
        assert_eq!(ArenaAnimation::EnterVehicle.label(), "Enter Vehicle");
    }

    #[test]
    fn developer_arena_defaults_to_a_non_destructive_closed_state() {
        let arena = DeveloperArena::default();
        assert!(!arena.active);
        assert!(arena.controlled_vehicle.is_none());
        assert_eq!(arena.camera_distance, 8.0);
    }

    #[test]
    fn insert_round_trip_restores_timer_cameras_ui_and_cursor_exactly() {
        let mut app = toggle_test_app();
        let active_camera = app
            .world_mut()
            .spawn(Camera {
                is_active: true,
                ..default()
            })
            .id();
        let inactive_camera = app
            .world_mut()
            .spawn(Camera {
                is_active: false,
                ..default()
            })
            .id();
        let ui_root = app
            .world_mut()
            .spawn((Node::default(), Visibility::Inherited))
            .id();
        let ui_child = app
            .world_mut()
            .spawn((Node::default(), Visibility::Visible))
            .id();
        app.world_mut().entity_mut(ui_root).add_child(ui_child);
        let window = app
            .world_mut()
            .spawn((
                PrimaryWindow,
                CursorOptions {
                    visible: false,
                    grab_mode: CursorGrabMode::Locked,
                    ..default()
                },
            ))
            .id();

        press_insert(&mut app);

        assert!(app.world().resource::<DeveloperArena>().active);
        assert!(app.world().resource::<CTimer>().get_is_code_paused());
        assert!(!app.world().get::<Camera>(active_camera).unwrap().is_active);
        assert!(
            !app.world()
                .get::<Camera>(inactive_camera)
                .unwrap()
                .is_active
        );
        assert_eq!(
            *app.world().get::<Visibility>(ui_root).unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            *app.world().get::<Visibility>(ui_child).unwrap(),
            Visibility::Hidden
        );
        let cursor = app.world().get::<CursorOptions>(window).unwrap();
        assert!(cursor.visible);
        assert_eq!(cursor.grab_mode, CursorGrabMode::None);
        let arena_camera_uses_raw_rw_colour = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<
                &bevy::render::view::Tonemapping,
                With<ArenaCamera>,
            >();
            query.iter(world).all(|tonemapping| {
                *tonemapping == bevy::render::view::Tonemapping::None
            })
        };
        assert!(arena_camera_uses_raw_rw_colour);

        press_insert(&mut app);

        assert!(!app.world().resource::<DeveloperArena>().active);
        assert!(!app.world().resource::<CTimer>().get_is_code_paused());
        assert!(app.world().get::<Camera>(active_camera).unwrap().is_active);
        assert!(
            !app.world()
                .get::<Camera>(inactive_camera)
                .unwrap()
                .is_active
        );
        assert_eq!(
            *app.world().get::<Visibility>(ui_root).unwrap(),
            Visibility::Inherited
        );
        assert_eq!(
            *app.world().get::<Visibility>(ui_child).unwrap(),
            Visibility::Visible
        );
        let cursor = app.world().get::<CursorOptions>(window).unwrap();
        assert!(!cursor.visible);
        assert_eq!(cursor.grab_mode, CursorGrabMode::Locked);
        let arena_owned_count = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<ArenaOwned>>();
            query.iter(world).count()
        };
        assert_eq!(
            arena_owned_count, 0,
            "closing the arena must remove every arena-owned entity"
        );
    }

    #[test]
    fn arena_capture_consumes_open_close_edges_without_leaking_space_to_frontend() {
        let mut app = toggle_test_app();
        press_insert(&mut app);
        assert!(app.world().resource::<DeveloperArena>().active);
        assert!(app.world().resource::<GtaInputCapture>().developer_arena);

        {
            let world = app.world_mut();
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Space);
            keys.press(KeyCode::Insert);
        }
        app.update();

        assert!(!app.world().resource::<DeveloperArena>().active);
        assert!(!app.world().resource::<GtaInputCapture>().developer_arena);
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::Space)
        );
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Space)
        );
    }

    #[test]
    fn frontend_nodes_created_after_insert_are_hidden_and_restored() {
        let mut app = toggle_test_app();
        press_insert(&mut app);

        let arena_root = app
            .world_mut()
            .spawn((ArenaOwned, Node::default(), Visibility::Visible))
            .id();
        let arena_child = app
            .world_mut()
            .spawn((Node::default(), Visibility::Visible))
            .id();
        app.world_mut()
            .entity_mut(arena_root)
            .add_child(arena_child);

        let late_selection_bar = app
            .world_mut()
            .spawn((Node::default(), Visibility::Visible))
            .id();
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(late_selection_bar).unwrap(),
            Visibility::Hidden,
            "frontend selection bars spawned after Insert must not leak into the arena"
        );
        assert!(
            app.world()
                .get::<ArenaHiddenUi>(late_selection_bar)
                .is_some(),
            "late frontend UI must retain its restoration snapshot"
        );
        assert_eq!(
            *app.world().get::<Visibility>(arena_child).unwrap(),
            Visibility::Visible,
            "arena UI descendants must remain visible even without ArenaOwned on every child"
        );

        press_insert(&mut app);
        assert_eq!(
            *app.world().get::<Visibility>(late_selection_bar).unwrap(),
            Visibility::Visible
        );
    }

    #[test]
    fn original_frontend_mouse_texture_replaces_native_cursor_in_arena() {
        let mut app = toggle_test_app();
        let mut frontend = FrontendTxdTextures::default();
        frontend
            .images
            .insert("mouse".to_owned(), Handle::<Image>::default());
        app.insert_resource(frontend);
        let window = app
            .world_mut()
            .spawn((PrimaryWindow, CursorOptions::default()))
            .id();

        press_insert(&mut app);

        assert!(!app.world().get::<CursorOptions>(window).unwrap().visible);
        let cursor_count = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<ArenaMouseCursor>>();
            query.iter(world).count()
        };
        assert_eq!(cursor_count, 1);
    }

    /// Local proprietary-asset smoke test. Kept ignored for portable CI, but
    /// exercised against `config/original_gta3_path.txt` while developing the
    /// arena so proxy geometry can never masquerade as a successful spawn.
    #[test]
    #[ignore = "requires the configured original GTA III installation"]
    fn original_install_streams_real_player_and_kuruma_drawables() {
        use re3bevy_game::{
            animation::{
                AnimAssocDefinitions::ASSOCGRP_PLAYER,
                AnimationId::{ANIM_STD_IDLE, ANIM_STD_RUNFAST},
            },
            core::Game::{advance_player_jump_launch, begin_player_jump},
            modelinfo::ModelIndices::{MI_KURUMA, MI_LANDSTAL, MI_PLAYER},
            peds::PlayerPed::CPlayerPed,
            renderer::BevyRender::{
                VehicleColourRemap, arena_texture_bind_for_drawable,
                build_animated_ped_material_drawables, build_model_material_drawables,
                build_rigid_ped_material_drawables, rigid_ped_part_transforms_from_clump,
                sync_model_texture_cache,
            },
        };

        let mut data = load_arena_original_data()
            .expect("isolated arena must bootstrap DFF/TXD/PED.IFP from original GTA III");
        let mut texture_cache = GtaWorldTextureCache::default();
        let mut images = Assets::<Image>::default();

        for (model_index, expected_name) in [
            (MI_PLAYER, "player"),
            (MI_KURUMA, "kuruma"),
            (MI_LANDSTAL, "landstal"),
        ] {
            ensure_arena_model_resident(
                model_index,
                Some(&mut data.file_loader),
                Some(&mut data.streaming),
                Some(&mut data.before_rw),
            )
            .unwrap_or_else(|error| panic!("{expected_name} failed to become resident: {error}"));

            let drawables = build_model_material_drawables(
                model_index,
                0.0,
                false,
                &data.file_loader,
                VehicleColourRemap::default(),
            );
            assert!(
                !drawables.is_empty(),
                "{expected_name} must produce original DFF drawables"
            );
            assert!(
                drawables
                    .iter()
                    .map(|drawable| drawable.vertex_count)
                    .sum::<usize>()
                    > 0,
                "{expected_name} DFF drawables must contain vertices"
            );
            assert!(
                model_texture_dictionary_is_resident(&data.file_loader, model_index),
                "{expected_name} TXD must be resident with its DFF"
            );
            if matches!(model_index, MI_KURUMA | MI_LANDSTAL) {
                assert!(
                    drawables
                        .iter()
                        .any(|drawable| drawable.dynamic_vehicle_component),
                    "Kuruma must expose real wheel/component drawables for CAutomobile::Render"
                );
            }
            assert!(
                sync_model_texture_cache(
                    model_index,
                    &data.file_loader,
                    &mut texture_cache,
                    &mut images,
                ),
                "{expected_name} TXD must be completely decoded before arena spawn"
            );
            let resolved_textures = drawables
                .iter()
                .filter(|drawable| drawable.texture_name.is_some())
                .filter(|drawable| {
                    arena_texture_bind_for_drawable(drawable, &data.file_loader, &texture_cache)
                        .is_some()
                })
                .count();
            assert!(
                resolved_textures > 0,
                "{expected_name} must bind decoded textures from its original TXD"
            );
            let unresolved = drawables
                .iter()
                .filter_map(|drawable| {
                    let name = drawable.texture_name.as_deref()?.trim();
                    (!name.is_empty() && !name.starts_with('@')).then_some((name, drawable))
                })
                .filter(|(_, drawable)| {
                    arena_texture_bind_for_drawable(drawable, &data.file_loader, &texture_cache)
                        .is_none()
                })
                .map(|(name, _)| name.to_owned())
                .collect::<Vec<_>>();
            assert!(
                unresolved.is_empty(),
                "{expected_name} has unresolved original TXD material textures: {unresolved:?}"
            );
            if model_index == MI_PLAYER {
                let slot = data
                    .file_loader
                    .model_info
                    .get_model_info(model_index)
                    .expect("player model info")
                    .base()
                    .txd_slot as usize;
                let summary = data
                    .file_loader
                    .txd_store
                    .slot(slot)
                    .and_then(|txd| txd.texture_summary.as_ref())
                    .expect("player TXD summary");
                for texture in &summary.textures {
                    if let Some(handle) = texture_cache
                        .images
                        .get(&(slot, texture.name.to_ascii_lowercase()))
                        && let Some(image) = images.get(handle)
                        && let Some(bytes) = image.data.as_ref()
                    {
                        let mut sums = [0_u64; 3];
                        for pixel in bytes.chunks_exact(4) {
                            sums[0] += u64::from(pixel[0]);
                            sums[1] += u64::from(pixel[1]);
                            sums[2] += u64::from(pixel[2]);
                        }
                        assert!(
                            sums[0] > sums[2],
                            "player PALETTEENTRY must remain RGBA; swapping R/B turns Claude's skin blue: {sums:?}"
                        );
                    }
                }
            }
        }

        let mut test_random = CGeneral::default();
        let mut player = CPlayerPed::new(&mut test_random, None);
        let mut idle = data
            .anim_manager
            .create_anim_association(ASSOCGRP_PLAYER, ANIM_STD_IDLE as usize)
            .expect("PED.IFP must provide the Claude idle association");
        idle.set_blend(1.0, 0.0);
        idle.start(0.0);
        idle.update_time(0.1, 1.0);
        player.animation.movement_anim_clump.associations.push(idle);
        let animated = build_animated_ped_material_drawables(
            MI_PLAYER,
            &data.file_loader,
            &player.animation.movement_anim_clump,
        );
        assert!(
            !animated.is_empty(),
            "real player.dff must bind to the real PED.IFP idle pose"
        );
        let (rigid_drawables, rigid_frames) =
            build_rigid_ped_material_drawables(MI_PLAYER, &data.file_loader)
                .expect("GTA III PC player.dff must use non-skinned RwFrame animation");
        let rigid_transforms = rigid_ped_part_transforms_from_clump(
            MI_PLAYER,
            &data.file_loader,
            &player.animation.movement_anim_clump,
            &rigid_frames,
        )
        .expect("Claude PED.IFP pose must resolve every non-skinned limb frame");
        assert_eq!(rigid_drawables.len(), rigid_transforms.len());

        let mut sprint = data
            .anim_manager
            .create_anim_association(ASSOCGRP_PLAYER, ANIM_STD_RUNFAST as usize)
            .expect("PED.IFP must provide the Claude sprint association");
        sprint.set_blend(1.0, 0.0);
        sprint.start(0.0);
        player.animation.movement_anim_clump.associations.clear();
        player.animation.movement_anim_clump.associations.push(sprint);
        player.ped.physical.move_speed = CVector::ZERO;
        begin_player_jump(&mut player.as_mut(), &data.anim_manager);
        let launch_velocity = (0..300)
            .find_map(|_| advance_player_jump_launch(&mut player.as_mut(), &data.anim_manager, 1.0 / 60.0))
            .expect("PED.IFP jump launch must reach FinishLaunchCB");
        assert!(
            (launch_velocity.y - 6.6).abs() < 1.0e-4,
            "full sprint blend must produce re3's 0.22 units/step forward launch"
        );
        assert!(
            (launch_velocity.z - (8.5 / 70.0 * 30.0)).abs() < 1.0e-4,
            "player jump must apply re3's 8.5/mass vertical force"
        );
    }

    #[test]
    #[ignore = "requires the configured original GTA III installation"]
    fn insert_before_gameplay_boot_creates_a_real_animated_player_runtime() {
        let mut app = App::new();
        app.init_resource::<DeveloperArena>()
            .init_resource::<ArenaRandomState>()
            .init_resource::<ArenaOriginalRuntime>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<GtaInputCapture>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<GtaUnlitMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<GtaWorldTextureCache>()
            .init_resource::<CTimer>()
            .init_resource::<re3bevy_game::renderer::BevyRender::GtaImmediateMeshCache>()
            .add_systems(PreUpdate, toggle_developer_arena);

        press_insert(&mut app);

        let arena = app.world().resource::<DeveloperArena>();
        assert!(arena.active);
        assert!(
            arena.player_entity.is_some(),
            "frontend Insert must construct a real ported CPlayerPed"
        );
        let original = app.world().resource::<ArenaOriginalRuntime>();
        let data = original
            .data
            .as_ref()
            .expect("frontend Insert must bootstrap isolated original GTA data");
        assert!(
            data.anim_manager
                .create_anim_association(
                    re3bevy_game::animation::AnimAssocDefinitions::ASSOCGRP_PLAYER,
                    re3bevy_game::animation::AnimationId::ANIM_STD_IDLE as usize,
                )
                .is_some(),
            "frontend Insert must have the real PED.IFP idle association"
        );
        let loaded_visual = {
            let world = app.world_mut();
            let mut visuals = world.query::<&ArenaCharacterVisual>();
            visuals.iter(world).any(|visual| {
                visual.loaded_dff
                    && visual.model_index == re3bevy_game::modelinfo::ModelIndices::MI_PLAYER
            })
        };
        assert!(
            loaded_visual,
            "frontend Insert must spawn the real player DFF, never a proxy"
        );
    }
}
