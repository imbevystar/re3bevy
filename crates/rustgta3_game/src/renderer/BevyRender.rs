//! re3 reference: references/re-GTA/src/core/main.cpp
//! re3 reference: references/re-GTA/src/renderer/Renderer.cpp
//!
//! Bevy-side render adapter for the RenderWare frame setup boundary.

#[path = "ShadowSubmission.rs"]
mod shadow_submission;
use shadow_submission::flush_shadow_meshes;

#[path = "WorldModelLifecycle.rs"]
mod world_model_lifecycle;
pub use world_model_lifecycle::WorldModelReplacementDiagnostics;

#[path = "WaterRender.rs"]
mod water_render;
use water_render::{
    GtaWaterAnimationState, emit_water_birds, spawn_water_mesh_once, sync_water_surface_animation,
    sync_water_timecycle_material,
};
pub use water_render::{GtaWaterMesh, GtaWaterRenderStats, GtaWaterWaveMesh};

#[path = "ModelDrawables.rs"]
mod model_drawables;
pub use model_drawables::{
    MeshBindReport, ModelMaterialDrawable, VehicleColourRemap, VehicleDamageRenderState,
    VehicleDoorRenderState, VehicleResidentMeshState, VehicleWheelRenderState,
};
use model_drawables::{
    append_geometry_drawables, apply_player_mouse_diagonal_leg_pose, build_cutscene_drawables,
    build_rigid_cutscene_drawables, cutscene_object_render_transform, model_uses_cpu_skinning,
    ped_bone_poses_from_association, ped_frame_transform, ped_ifp_bone_poses,
    ped_ifp_bone_poses_for_one_shot, player_anim_clump_bone_poses,
    player_anim_clump_bone_poses_with_overlay, rigid_cutscene_part_transforms,
};
pub use model_drawables::{
    build_animated_ped_material_drawables, build_animated_ped_material_drawables_from_association,
    build_model_material_drawables, build_rigid_ped_material_drawables,
    build_vehicle_component_transforms, diagnose_model_mesh_bind,
    diagnose_model_mesh_bind_with_colours, rigid_ped_part_transforms_from_association,
    rigid_ped_part_transforms_from_clump,
};
pub(crate) use model_drawables::{ped_frame_world_point_from_clump, ped_frame_world_point_static, ped_weapon_hit_model};
#[path = "FrontendSkinRender.rs"]
mod frontend_skin;
pub use frontend_skin::GtaFrontendSkinRenderDiagnostics;
#[path = "CarLightRender.rs"]
mod car_light_render;
pub use car_light_render::{GtaCarLightMaterial, GtaCarLightProjection};
#[path = "MarkerRender.rs"]
mod marker_render;
pub use marker_render::{GtaMarkerMaterial, GtaMarkerDraw};
#[path = "PickupRender.rs"]
mod pickup_render;
pub use runtime_submission::GtaRuntimeSubmissionDiagnostics;
pub use crate::vehicles::VehicleConstruction::GtaSuspensionDiagnostics as GtaSuspensionRenderDiagnostics;

/// Cross-domain boundaries retain original render-preparation ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bevy::prelude::SystemSet)]
pub enum NativeRenderPreparationSet { WorldRequests, Audit }
#[path = "RenderDiagnostics.rs"]
mod diagnostics;
#[cfg(test)]
use diagnostics::{build_f9_probe_report, f9_yaw_pitch_deg, march_f9_ray};

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::OnceLock,
    time::Instant,
};

use bevy::{
    asset::RenderAssetUsages,
    camera::{CameraProjection, visibility::NoFrustumCulling},
    ecs::system::SystemParam,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::storage::ShaderBuffer,
    render::view::{DebandDither, Msaa, NoIndirectDrawing, Tonemapping},
    ui::widget::NodeImageMode,
    window::PrimaryWindow,
};
use rustgta3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use rustgta3_core::core::{
    Clock::CClock,
    Coordinates::CVector,
    GameState::{ELevelName, GtaAppState},
    General::CGeneral,
    Schedules::GtaProcessSet,
    Timer::CTimer,
};
use rustgta3_formats::formats::{
    ClumpRead::read_clump,
    FileLoader::GtaTwoDfx,
};

use crate::{
    animation::{
        AnimManager::CAnimManager, CutsceneMgr::CCutsceneMgr,
        PedVehicleAnimation::CPedVehicleAnimation,
    },
    collision::Collision::CCollision,
    control::{
        Bridge::CBridge,
        Pickups::{CPacManPickups, PACMAN_NONE},
        Replay::{
            ReplayPedPoolOverlay, ReplayPedRuntimeState, ReplayVehiclePoolOverlay,
            ReplayVehicleRuntimeState,
        },
        TrafficLights::{
            CAR_LIGHTS_GREEN, CAR_LIGHTS_YELLOW, CTrafficLights, PED_LIGHTS_DONT_WALK,
            PED_LIGHTS_WALK,
        },
    },
    core::{
        Camera::{CCamera, CameraFrustum, CameraScriptMode},
        FileLoader::CFileLoader,
        Frontend::CMenuManager,
        Game::{
            ScriptEntityWorld, ScriptGameplayBridge, ScriptTaxiLightState,
        },
        Pools::{
            CPools, CompatibleCompressedMatrix, CompatibleObjectSave, CompatiblePedSave,
            CompatibleVehicleMatrix, CompatibleVehicleSave,
        },
        Streaming::{CStreaming, ModelInfoType},
        User::CUserDisplay,
        World::{
            CWorld, CWorldEntity, NUMSECTORS_X, NUMSECTORS_Y, SECTOR_SIZE_X, SECTOR_SIZE_Y,
            WORLD_MIN_X, WORLD_MIN_Y, ZONE_DEFAULT, ZONE_NAVIG,
        },
    },
    entities::Entity::eEntityStatus,
    extras::{
        postfx::{CPostFX, PostFxIntentKind},
        screendroplets::ScreenDroplets,
    },
    modelinfo::ModelIndices::{
        MI_BORGNINE, MI_CABBIE, MI_DODO, MI_FBICAR,
        MI_PLAYER, MI_RCBANDIT, MI_RHINO, MI_TAXI,
    },
    modelinfo::ModelInfo::{CModelInfo, ModelInfoSlot},
    peds::{
        Ped::CPed,
        PlayerPed::{CWorldPlayerState, PlayerPedState},
        PlayerRead::CWorldPlayersRead,
        VehicleRelations::{InVehicle, PedEntity},
    },
    text::Text::CText,
    vehicles::{
        Automobile::HANDLING_HALOGEN_LIGHTS,
        DamageManager::{eLightStatus, eLights},
        HandlingMgr::cHandlingDataMgr,
    },
    weapons::Explosion::{CExplosionPool, eExplosionType},
};

use super::{
    Antennas::CAntennas,
    Clouds::CClouds,
    Coronas::CCoronas,
    Fluff::{CMovingThings, FluffRenderIntent},
    Glass::{CGlass, GlassBlend},
    Hud::CHud,
    Lines::{CLines, LineIntent},
    MBlur::CMBlur,
    Particle::{CParticle, ParticleColour},
    ParticleMgr::{
        CYCLE_ANIM, DRAWTOP2D, RAND_VERT_V, ZCHECK_BUMP, ZCHECK_BUMP_FIRST, ZCHECK_FIRST,
        ZCHECK_STEP, cParticleSystemMgr, particle_raster_names,
    },
    ParticleType::{
        PARTICLE_DEBRIS2, PARTICLE_ENGINE_SMOKE2, PARTICLE_EXPLOSION_LARGE,
        PARTICLE_EXPLOSION_LFAST, PARTICLE_EXPLOSION_MEDIUM, PARTICLE_EXPLOSION_MFAST,
        PARTICLE_WATER_HYDRANT,
    },
    PointLights::{CPointLights, LIGHT_POINT},
    RenderBuffer::RenderBuffer,
    Renderer::{
        CRenderer, FADE_DISTANCE, LOD_DISTANCE, RendererEntitySnapshot, RendererModelSnapshot,
        RendererModelType, RendererPassPlan, STREAM_DISTANCE, entity_cull_zone_visible,
        zone_cull_disabled,
    },
    RendererMaterials::{
        GTA_GLOBAL_LIGHTING_BUFFER, GtaGlobalLighting, GtaSkyMaterial, GtaUnlitMaterial,
        GtaWaterMaterial, RwMaterialParams, RwTextureAlphaMode, SkyMaterialParams,
    },
    Rubbish::CRubbish,
    Shadows::{CShadows, ScriptShadow, StaticShadowIntent},
    Skidmarks::CSkidmarks,
    SpecialFX::{
        BRIGHTLIGHT_TRAFFIC_GREEN, BRIGHTLIGHT_TRAFFIC_RED, BRIGHTLIGHT_TRAFFIC_YELLOW,
        BrightLightRenderIntent, CSpecialFX, MARKERTYPE_ARROW,
        ShinyTextRenderIntent,
    },
    Sprite::{CSprite, SpriteBatch},
    Timecycle::CTimeCycle,
    WaterCannon::CWaterCannons,
    WaterLevel::{
        CWaterLevel, SMALL_SECTOR_SIZE, WATER_Z_OFFSET,
    },
    Weather::CWeather,
};

const RADAR_3D_MARKER_PREFIX: u32 = 0x5240_0000;
const RADAR_3D_MARKER_MASK: u32 = 0xFFF0_0000;

pub const ARENA_RENDER_LAYER: usize = 29;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaSceneCamera {
    /// Local viewport owner. Player zero is the original re3 RenderWare camera.
    pub local_player: u8,
}

/// re3 `CDraw::FadeValue` fullscreen pass. Kept as one persistent UI node so
/// SCM fades do not allocate/despawn render entities every frame.
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaScreenFade;

#[derive(Component)]
pub struct GtaWideScreenBorder {
    bottom: bool,
}

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaDirectionalLight;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaPointLight;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaStreetFxMesh;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaGlassMesh;

#[derive(Debug, Clone, Copy, Component, PartialEq)]
pub struct GtaPointLightIntent {
    pub radius: f32,
    pub colour: [f32; 3],
    pub cast_extra_shadows: bool,
}

#[derive(Debug, Clone, Copy, Component, Default)]
struct GtaPersistentPointLight;

#[derive(Debug, Clone, Default, Resource)]
struct GtaPointLightPool {
    entities: Vec<Entity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct GtaRenderFrameState {
    pub sky_top: [u8; 3],
    pub sky_bottom: [u8; 3],
    pub far_clip: f32,
    pub fog_start: f32,
}

/// Timecycle/RW lighting inputs shared by the world and runtime mesh passes.
/// re3 uses the same ambient and directional colours for all atomics, with
/// peds/cars/objects receiving the 1.3 ambient boost in `Lights.cpp`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GtaRenderLighting {
    fog_colour: [f32; 3],
    fog_start: f32,
    far_clip: f32,
    ambient: [f32; 3],
    directional: [f32; 3],
    sun_direction: [f32; 3],
    ambient_multiplier: f32,
    directional_enabled: bool,
}

impl Default for GtaRenderLighting {
    fn default() -> Self {
        Self {
            fog_colour: [0.5, 0.5, 0.55],
            fog_start: 100.0,
            far_clip: 2000.0,
            ambient: [0.25, 0.25, 0.2],
            directional: [1.0, 0.85, 0.45],
            sun_direction: [0.0, 0.0, 1.0],
            ambient_multiplier: 1.0,
            directional_enabled: true,
        }
    }
}

impl GtaRenderLighting {
    fn for_peds_cars_and_objects(mut self) -> Self {
        self.ambient_multiplier = 1.3;
        self.directional_enabled = true;
        self
    }

    fn without_directional(mut self) -> Self {
        self.directional_enabled = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Resource)]
pub struct GtaRenderPassPlanState {
    pub last_plan: Option<RendererPassPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Resource)]
pub struct GtaWorldRenderStats {
    pub planned_entities: usize,
    pub spawned_meshes: usize,
    pub spawned_pool_meshes: usize,
    pub spawned_replay_meshes: usize,
    pub textured_meshes: usize,
    pub skipped_missing_model: usize,
    pub skipped_missing_geometry: usize,
    pub skipped_missing_texture: usize,
}

#[path = "MaterialResolution.rs"]
mod material_resolution;
pub(crate) use material_resolution::sync_txd_slot_texture_cache;
pub use material_resolution::{
    GtaWorldTextureCache, arena_texture_bind_for_drawable, sync_model_texture_cache,
    sync_world_texture_cache,
};
use material_resolution::{
    ensure_colour_texture_table, ensure_environment_maps, resolve_drawable_material_bind,
    resolve_txd_texture_asset, texture_cache_lookup, texture_handle_for_model_material,
};

pub struct GtaBevyRenderPlugin;

#[path = "RuntimeMeshSubmission.rs"]
mod runtime_submission;
use runtime_submission::{GtaRuntimeMeshRefresh, RuntimeActorsRead};
pub use runtime_submission::vehicle_component_relative_transform;


#[derive(SystemParam)]
struct WorldMeshRenderResources<'w> {
    texture_cache: ResMut<'w, GtaWorldTextureCache>,
    model_assets: ResMut<'w, GtaWorldModelAssetCache>,
    mesh_registry: ResMut<'w, GtaWorldMeshRegistry>,
    collision: Option<Res<'w, CCollision>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<GtaUnlitMaterial>>,
    images: ResMut<'w, Assets<Image>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum GtaRuntimeModelRefSource {
    World,
    Ped,
    Vehicle,
    Object,
}

type GtaRuntimeModelRefKey = (GtaRuntimeModelRefSource, usize);

/// Bevy-side ownership mirror of `CEntity::m_rwObject` model refs for live
/// world/runtime entities. The logical entity survives render-list changes, so
/// its source DFF/TXD must not become a streaming eviction target while a Bevy
/// instance still mirrors the C++ `m_rwObject`.
#[derive(Debug, Default, Resource)]
struct GtaRuntimeEntityModelRefs {
    owned: std::collections::HashMap<GtaRuntimeModelRefKey, usize>,
}

impl Plugin for GtaBevyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::WorldSubmission::WorldSubmissionPlugin);
        app.init_resource::<CRenderer>()
            .init_resource::<RenderBuffer>()
            .init_resource::<CLines>()
            .init_resource::<CSprite>()
            .init_resource::<CPointLights>()
            .init_resource::<CMBlur>()
            .init_resource::<CMovingThings>()
            .init_resource::<cParticleSystemMgr>()
            .init_resource::<CPostFX>()
            .init_resource::<ScreenDroplets>()
            .init_resource::<GtaRenderFrameState>()
            .init_resource::<GtaRenderPassPlanState>()
            .init_resource::<GtaWorldRenderStats>()
            .init_resource::<crate::renderer::HudBevy::GtaHudRasterCache>()
            .init_resource::<crate::renderer::HudBevy::GtaScriptUiTextureCache>()
            .init_resource::<GtaWorldMeshRegistry>()
            .init_resource::<GtaWorldModelAssetCache>()
            .init_resource::<GtaImmediateMeshCache>()
            .init_resource::<shadow_submission::ShadowSubmissionDiagnostics>()
            .init_resource::<GtaMarkerAssetCache>()
            .init_resource::<GtaPointLightPool>()
            .init_resource::<GtaRuntimeEntityModelRefs>()
            .init_resource::<WorldModelReplacementDiagnostics>()
            .init_resource::<GtaParticleSpawnSequence>()
            .init_resource::<GtaWaterAnimationState>()
            .init_resource::<GtaWaterRenderStats>()
            .init_resource::<super::VehicleLightingAccess::VehicleLightingDiagnostics>()
            .add_plugins(super::Particle::ParticleCreationInputPlugin)
            .init_resource::<super::Shadows::ShadowProjectionDiagnostics>()
            .add_plugins(MaterialPlugin::<GtaCarLightMaterial>::default())
            .add_plugins(MaterialPlugin::<GtaMarkerMaterial>::default())
            .add_systems(OnEnter(GtaAppState::LoadingGame), marker_render::reset_marker_draws)
            .add_systems(OnEnter(GtaAppState::LoadingGame), reset_level_render_state)
            .add_systems(OnEnter(GtaAppState::LoadingFrontend), marker_render::reset_marker_draws)
            .add_systems(OnEnter(GtaAppState::Error), marker_render::reset_marker_draws)
            .add_systems(OnEnter(GtaAppState::LoadingGame), car_light_render::reset_car_light_meshes)
            .add_systems(OnEnter(GtaAppState::LoadingFrontend), car_light_render::reset_car_light_meshes)
            .add_systems(OnEnter(GtaAppState::Error), car_light_render::reset_car_light_meshes)
            .add_systems(
                OnEnter(GtaAppState::LoadingGame),
                water_render::reset_water_render,
            )
            .add_systems(
                OnEnter(GtaAppState::LoadingFrontend),
                water_render::reset_water_render,
            )
            .add_systems(
                OnEnter(GtaAppState::Error),
                water_render::reset_water_render,
            )
            .add_plugins((
                crate::renderer::ParticleSprite::GtaParticleSpritePlugin,
                crate::renderer::PostFxRender::GtaPostFxRenderPlugin,
                crate::renderer::ScreenDropletRender::GtaScreenDropletRenderPlugin,
                crate::renderer::NeoRender::GtaNeoRenderPlugin,
                crate::renderer::HudBevy::radar::RadarPlugin,
                frontend_skin::FrontendSkinRenderPlugin,
                diagnostics::RenderDiagnosticsPlugin,
                runtime_submission::RuntimeMeshSubmissionPlugin,
                material_resolution::MaterialResolutionPlugin,
            ))
            .add_systems(
                Startup,
                (
                    setup_renderware_camera,
                    setup_screen_fade,
                    setup_widescreen_borders,
                ),
            )
            .add_systems(
                Update,
                (consume_particle_intents, update_particle_billboards)
                    .chain()
                    .after(crate::objects::ParticleObject::update_particle_objects)
                    .in_set(GtaProcessSet::Particles)
                    .run_if(in_state(GtaAppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    (
                        world_model_lifecycle::consume_world_streaming_deletions,
                        world_model_lifecycle::consume_world_model_replacements,
                    )
                        .chain()
                        .after(sync_runtime_entity_model_refs)
                        .after(GtaProcessSet::Audio)
                        .before(GtaProcessSet::RenderPrep),
                    sync_screen_fade.in_set(GtaProcessSet::RenderPrep),
                    sync_widescreen_borders.in_set(GtaProcessSet::RenderPrep),
                    sync_runtime_entity_model_refs
                        .in_set(GtaProcessSet::Streaming)
                        .before(update_world_streaming_around_camera),
                    update_world_streaming_around_camera
                        .in_set(GtaProcessSet::Streaming)
                        .run_if(in_state(GtaAppState::InGame)),
                    update_user_display_on_hud
                        .in_set(GtaProcessSet::UserDisplay)
                        .run_if(in_state(GtaAppState::InGame)),
                    process_car_generators_near_player
                        .in_set(GtaProcessSet::CarGenerators)
                        .run_if(in_state(GtaAppState::InGame)),
                    (
                        sync_timecycle_lights,
                        sync_sky_horizon_mesh,
                        spawn_water_mesh_once,
                        sync_water_timecycle_material,
                        sync_water_surface_animation,
                        emit_water_birds,
                        crate::renderer::HudBevy::flush_hud,
                    )
                        .chain()
                        .in_set(GtaProcessSet::RenderPrep)
                        .run_if(in_state(GtaAppState::InGame)),
                    // re3 CGame::Process ends with render prep after camera update.
                    // Must run *after* follow_script_player_camera (Script set) so the
                    // Bevy camera eye matches construct_render_list / streaming.
                    (
                        crate::control::ScriptDynamicEntities::draw_script_spheres,
                        emit_radar_3d_markers,
                        prepare_runtime_contact_shadows,
                        (
                            clear_immediate_meshes,
                            clear_point_lights,
                            update_coronas_lifecycle,
                            update_explosion_and_vehicle_lights,
                            pickup_render::prepare_pickup_effects,
                            update_atmosphere_and_fx,
                            // The light list is complete only after the
                            // world 2dfx/corona and vehicle producers run.
                            (sync_renderware_frame_state, flush_world_render_pass).chain(),
                            request_radar_sections_after_world_render.in_set(NativeRenderPreparationSet::WorldRequests),
                        )
                            .chain(),
                        (
                            log_render_audit.in_set(NativeRenderPreparationSet::Audit),
                            crate::renderer::Antennas::register_automobile_antennas,
                            consume_particle_intents.in_set(super::Particle::NativeParticleRenderSet::Consume),
                            // The developer arena uses code-pause and its own
                            // render cadence. Production updates in Particles,
                            // before Fires/World/PreRender emit fresh sprites.
                            update_particle_billboards.run_if(|timer: Option<Res<CTimer>>| timer.is_some_and(|timer| timer.get_is_code_paused())),
                            crate::renderer::ParticleSprite::build_particle_sprite_frame,
                            crate::renderer::ParticleSprite::append_corona_sprite_frame,
                            crate::renderer::ParticleSprite::append_cloud_sprite_frame,
                            flush_fluff_intents,
                            flush_pacman_billboards,
                            flush_cloud_billboards,
                        )
                            .chain(),
                        (
                            car_light_render::flush_car_light_meshes,
                            flush_shadow_meshes,
                            flush_special_fx_meshes,
                            marker_render::flush_markers,
                            flush_render_buffer_batches,
                            prepare_legacy_street_fx_render,
                            flush_legacy_street_fx_meshes,
                            report_fx_runtime_diagnostics,
                            flush_glass_meshes,
                            flush_line_intents,
                            flush_sprite_batches,
                            flush_mblur_intents,
                            flush_postfx_intents,
                            flush_screen_droplets,
                            flush_point_light_fog,
                            flush_point_lights,
                        )
                            .chain(),
                    )
                        .chain()
                        .in_set(GtaProcessSet::RenderPrep)
                        .run_if(in_state(GtaAppState::InGame)),
                ),
            );

        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.add_systems(
                bevy::render::Render,
                super::CameraViewport::clamp_extracted_camera_viewports
                    .in_set(bevy::render::RenderSystems::PrepareViews)
                    .after(bevy::render::view::window::prepare_windows),
            );
        }
    }
}

/// CGame::ShutDownForRestart destroys the old level's RwObjects, model
/// dictionaries and per-frame effects. A new CFileLoader starts its TXD and
/// model generations from their initial values, so the render-side caches
/// cannot use those generation numbers to distinguish the previous game.
/// LOAD_SCENE within a running game still uses the recyclable slot path.
fn reset_level_render_state(
    mut commands: Commands,
    mut registry: ResMut<GtaWorldMeshRegistry>,
    mut model_assets: ResMut<GtaWorldModelAssetCache>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut refs: ResMut<GtaRuntimeEntityModelRefs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut coronas: Option<ResMut<CCoronas>>,
    mut point_lights: ResMut<CPointLights>,
) {
    for slot in registry.slots.values().chain(registry.retired_slots.values()) {
        despawn_world_mesh_slot(&mut commands, &mut materials, slot);
    }
    for entry in model_assets.entries.values() {
        for material in &entry.materials {
            materials.remove(material.id());
        }
    }
    for geometry in model_assets.geometry.values() {
        for mesh in &geometry.meshes {
            meshes.remove(mesh.id());
        }
    }
    // All handles resolved from the previous level's dictionaries must be
    // decoded again. Their numeric slot/generation can repeat after restart.
    for handle in texture_cache.images.values()
        .chain(texture_cache.solid_colours.values())
        .chain(texture_cache.player_skins.values())
    {
        images.remove(handle.id());
    }
    // Model-ref reconciliation observes this counter across game loads. Do
    // not let the fresh registry reuse the previous run's slots epoch.
    let next_slots_epoch = registry.slots_epoch.wrapping_add(1);
    *registry = GtaWorldMeshRegistry::default();
    registry.slots_epoch = next_slots_epoch;
    *model_assets = GtaWorldModelAssetCache::default();
    *texture_cache = GtaWorldTextureCache::default();
    refs.owned.clear();
    // The first boot creates CCoronas only when level loading finalises.
    // Subsequent New Game entries can already have one from the old level.
    if let Some(coronas) = coronas.as_deref_mut() {
        *coronas = CCoronas::init();
    }
    point_lights.init_per_frame();
}

fn load_marker_parts(
    install: &OriginalGtaInstall,
    meshes: &mut Assets<Mesh>,
    gta_path: &str,
    label: &str,
) -> Result<Vec<GtaMarkerMeshPart>, String> {
    let bytes = install
        .read(gta_path)
        .map_err(|error| format!("failed to read {label}: {error}"))?;
    let summary = read_clump(&bytes)
        .ok_or_else(|| format!("failed to decode {label} as a RenderWare clump"))?;
    // MarkerAtomicCB stores each visited atomic into the same pointer, so the
    // final atomic in the clump is the one cloned by re3.
    let atomic = summary
        .atomics
        .last()
        .ok_or_else(|| format!("{label} contains no atomic"))?;
    let geometry_index = usize::try_from(atomic.geometry_index)
        .map_err(|_| format!("{label} atomic has a negative geometry index"))?;
    let geometry = summary
        .geometries
        .get(geometry_index)
        .ok_or_else(|| format!("{label} atomic references a missing geometry"))?;
    let mut drawables = Vec::new();
    // AddMarker replaces the original atomic frame with a new identity frame.
    append_geometry_drawables(
        0,
        geometry,
        VehicleColourRemap::default(),
        None,
        &mut drawables,
    );
    if drawables.is_empty() {
        return Err(format!("{label} produced no drawable material geometry"));
    }
    Ok(drawables
        .into_iter()
        .map(|drawable| GtaMarkerMeshPart {
            mesh: meshes.add(drawable.mesh),
            texture_name: drawable.texture_name,
            surface_ambient: drawable.surface_ambient,
            surface_diffuse: drawable.surface_diffuse,
            has_prelit: drawable.has_prelit,
        })
        .collect())
}

fn load_marker_cylinder_parts(
    install: &OriginalGtaInstall,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<GtaMarkerMeshPart>, String> {
    load_marker_parts(
        install,
        meshes,
        "models\\generic\\zonecylb.dff",
        "models/generic/zonecylb.dff",
    )
}

fn load_marker_arrow_parts(
    install: &OriginalGtaInstall,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<GtaMarkerMeshPart>, String> {
    load_marker_parts(
        install,
        meshes,
        "models\\generic\\arrow.dff",
        "models/generic/arrow.dff",
    )
}

fn sync_runtime_entity_model_refs(
    mut file_loader: Option<ResMut<CFileLoader>>,
    mut streaming: Option<ResMut<CStreaming>>,
    script_entities: Option<RuntimeActorsRead>,
    world_meshes: Res<GtaWorldMeshRegistry>,
    mut refs: ResMut<GtaRuntimeEntityModelRefs>,
    mut dynamic_desired: Local<std::collections::HashMap<GtaRuntimeModelRefKey, usize>>,
    mut to_add: Local<Vec<(GtaRuntimeModelRefKey, usize)>>,
    mut to_remove: Local<Vec<(GtaRuntimeModelRefKey, usize)>>,
    mut last_slots_epoch: Local<u64>,
    mut epoch_initialized: Local<bool>,
) {
    let (Some(file_loader), Some(streaming)) = (file_loader.as_mut(), streaming.as_mut()) else {
        return;
    };

    to_add.clear();
    to_remove.clear();

    let world_epoch_changed = !*epoch_initialized || world_meshes.slots_epoch != *last_slots_epoch;
    if world_epoch_changed {
        *epoch_initialized = true;
        *last_slots_epoch = world_meshes.slots_epoch;

        for (&entity_index, slot) in &world_meshes.slots {
            let key = (GtaRuntimeModelRefSource::World, entity_index);
            if refs.owned.get(&key).copied() != Some(slot.model_index)
                && runtime_model_geometry_is_resident(file_loader, slot.model_index)
            {
                to_add.push((key, slot.model_index));
            }
        }

        for (&key, &model_index) in refs.owned.iter() {
            if key.0 == GtaRuntimeModelRefSource::World
                && world_meshes
                    .slots
                    .get(&key.1)
                    .is_none_or(|slot| slot.model_index != model_index)
            {
                to_remove.push((key, model_index));
            }
        }
    }

    dynamic_desired.clear();
    if let Some(script_entities) = script_entities.as_ref() {
        for (handle, entity) in script_entities.peds() {
            let model_index = entity.ped.physical.entity.model_index;
            if model_index >= 0 {
                dynamic_desired.insert(
                    (GtaRuntimeModelRefSource::Ped, handle as u32 as usize),
                    model_index as usize,
                );
            }
        }
        for (handle, entity) in script_entities.vehicles() {
            let model_index = entity.base().physical.entity.model_index;
            if model_index >= 0 {
                dynamic_desired.insert(
                    (GtaRuntimeModelRefSource::Vehicle, handle as u32 as usize),
                    model_index as usize,
                );
            }
        }
        for entity in script_entities.objects() {
            let model_index = entity.object.physical.entity.model_index;
            if model_index >= 0 {
                dynamic_desired.insert(
                    (
                        GtaRuntimeModelRefSource::Object,
                        entity.handle as u32 as usize,
                    ),
                    model_index as usize,
                );
            }
        }
    }

    for (&key, &model_index) in dynamic_desired.iter() {
        if refs.owned.get(&key).copied() == Some(model_index) {
            continue;
        }
        if runtime_model_geometry_is_resident(file_loader, model_index) {
            to_add.push((key, model_index));
        } else if !matches!(key.0, GtaRuntimeModelRefSource::Object)
            || file_loader.model_info.get_model_info(model_index)
                .is_some_and(|model| model.base().default_col_model.is_none())
        {
            // A live actor still requires its clump even before its first
            // render. Pin it on residency; do not leave construction/effects
            // waiting forever for a model absent from camera-sector requests.
            streaming.request_model_with_adapter(
                model_index, crate::core::Streaming::STREAMFLAGS_DEPENDENCY, &mut **file_loader,
            );
        }
    }

    for (&key, &model_index) in refs.owned.iter() {
        if key.0 != GtaRuntimeModelRefSource::World
            && dynamic_desired.get(&key).copied() != Some(model_index)
        {
            to_remove.push((key, model_index));
        }
    }

    // Acquire replacements before releasing old refs. This preserves the re3
    // TXD lifetime when a pool slot is recycled for another live entity using
    // the same model in the same frame.
    for (key, model_index) in to_add.iter().copied() {
        if file_loader.add_model_ref(model_index) {
            refs.owned.insert(key, model_index);
        }
    }
    for (key, model_index) in to_remove.iter().copied() {
        if refs.owned.get(&key).copied() == Some(model_index) {
            refs.owned.remove(&key);
        }
        streaming.remove_model_ref_with_adapter(model_index, &mut **file_loader);
    }
}

/// re3 streaming around camera/player — FileLoader path, not SCM/Game lifecycle.
fn update_world_streaming_around_camera(
    mut file_loader: Option<ResMut<CFileLoader>>,
    mut streaming: Option<ResMut<crate::core::Streaming::CStreaming>>,
    before_rw: Option<ResMut<crate::core::Game::CGameBeforeRw>>,
    mut refresh: ResMut<GtaRuntimeMeshRefresh>,
    // The streaming pass is in Update, before Bevy's PostUpdate transform
    // propagation.  Consume the camera Transform written by the GTA Camera
    // schedule so the request ring follows the same frame as CStreaming.
    camera_query: Query<(&Camera, &Transform, &GtaSceneCamera)>,
    cutscene: Option<Res<CCutsceneMgr>>,
    mut timer: Option<ResMut<CTimer>>,
    players: CWorldPlayersRead,
    script_objects: Query<
        &crate::objects::Object::CObject,
        With<crate::control::ScriptEntities::ScriptObjectIdentity>,
    >,
    mut pending_cutscene_models: Local<Vec<usize>>,
    mut cut_models: Local<Vec<usize>>,
    mut pending_script_models: Local<Vec<usize>>,
    mut script_models_to_request: Local<Vec<usize>>,
    mut camera_positions: Local<Vec<(u8, [f32; 3])>>,
    mut streaming_positions: Local<Vec<[f32; 3]>>,
) {
    let (Some(file_loader), Some(streaming), Some(mut before_rw)) =
        (file_loader.as_mut(), streaming.as_mut(), before_rw)
    else {
        return;
    };

    // Gameplay SCM LOAD_SCENE / LOAD_ALL_MODELS_NOW are cooperative versions
    // of re3's blocking CStreaming calls. Keep game clock/script/world
    // code-paused, but continue servicing one non-blocking CD slice per
    // rendered frame. Barrier requests bypass ordinary one-frame cleanup.
    if file_loader.script_streaming_barrier_in_progress() {
        if let Some(timer) = timer.as_deref_mut() {
            if !timer.get_is_code_paused() {
                timer.set_code_pause(true);
                file_loader.set_script_streaming_owns_code_pause(true);
            }
        }
        match file_loader.pump_script_streaming_barrier(streaming, &mut before_rw.cd_stream) {
            Ok(complete) => {
                if complete {
                    refresh.force_rebuild = true;
                    if file_loader.script_streaming_owns_code_pause() {
                        if let Some(timer) = timer.as_deref_mut() {
                            timer.set_code_pause(false);
                        }
                        file_loader.set_script_streaming_owns_code_pause(false);
                    }
                }
            }
            Err(error) => {
                bevy::log::warn!("cooperative SCM streaming barrier: {error}");
                // Never leave the whole game code-paused on an I/O failure.
                file_loader.abort_script_streaming_barrier();
                if file_loader.script_streaming_owns_code_pause() {
                    if let Some(timer) = timer.as_deref_mut() {
                        timer.set_code_pause(false);
                    }
                    file_loader.set_script_streaming_owns_code_pause(false);
                }
            }
        }
        return;
    }

    // Normal CStreaming::Update is called every frame, but returns immediately
    // after its bookkeeping when CTimer is paused.
    if timer.as_ref().is_some_and(|timer| timer.get_is_paused()) {
        return;
    }
    pending_cutscene_models.clear();
    cut_models.clear();
    pending_script_models.clear();
    script_models_to_request.clear();
    camera_positions.clear();
    streaming_positions.clear();

    // re3 cutscene objects: RequestModel for each loaded cutscene object slot.
    if let Some(cutscene) = cutscene.as_ref() {
        if cutscene.loaded || cutscene.running {
            cut_models.extend(
                cutscene
                    .cutscene_objects
                    .iter()
                    .filter_map(|o| (o.model_id >= 0).then_some(o.model_id as usize)),
            );
            if !cut_models.is_empty() {
                pending_cutscene_models.extend(cut_models.iter().copied().filter(|model_id| {
                    !runtime_model_geometry_is_resident(&file_loader, *model_id)
                }));
                file_loader.request_models_for_runtime(
                    streaming,
                    cut_models.iter().copied(),
                    crate::core::Streaming::STREAMFLAGS_PRIORITY
                        | crate::core::Streaming::STREAMFLAGS_DONT_REMOVE,
                );
            }
        }
    }

    camera_positions.extend(
        camera_query
            .iter()
            .filter(|(camera, _, _)| camera.is_active)
            .map(|(_, transform, marker)| {
                let position = CVector::from_bevy(transform.translation);
                (marker.local_player, [position.x, position.y, position.z])
            }),
    );
    camera_positions.sort_unstable_by_key(|(local_player, _)| *local_player);
    if camera_positions.is_empty() {
        if !cut_models.is_empty() {
            match file_loader.pump_streaming_channels(streaming, &mut before_rw.cd_stream) {
                Ok(_) => {
                    refresh.force_rebuild |= pending_cutscene_models.iter().any(|model_id| {
                        runtime_model_geometry_is_resident(&file_loader, *model_id)
                    });
                }
                Err(error) => bevy::log::warn!("cutscene streaming: {error}"),
            }
        }
        return;
    }

    streaming_positions.extend(camera_positions.iter().map(|(_, position)| *position));

    // re3 `CStreaming::Update` keys the subway request off the player train's
    // height; the focused player rides it, so its underground flag matches.
    let player_coors = players
        .focused()
        .map(|player| player.position())
        .map(|position| [position.x, position.y, position.z]);
    if let Some(player_coor) = player_coors {
        let covered = streaming_positions.iter().any(|pos| {
            let dx = pos[0] - player_coor[0];
            let dy = pos[1] - player_coor[1];
            dx * dx + dy * dy < 40.0 * 40.0
        });
        if !covered {
            streaming_positions.push(player_coor);
        }
    }

    // re3 CStreaming::ProcessEntitiesInSectorList(sect->m_lists[ENTITYLIST_OBJECTS]):
    // dynamic/script objects in the world sectors are scanned every frame and their
    // model requested when within streaming range. In rustgta3, script objects live as
    // Bevy CObject entities. Request models for all active script objects
    // within LOD distance (+ STREAM_DISTANCE) of any camera.
    for script_object in &script_objects {
        let model_index = script_object.physical.entity.model_index;
        if model_index < 0 {
            continue;
        }
        let model_id = model_index as usize;
        let pos = script_object.physical.entity.matrix.position;
        let lod_dist = file_loader
            .model_info
            .simple_model_info(model_id)
            .map(|s| {
                s.largest_lod_distance(file_loader.renderer.lod_distance_multiplier(), false) + 30.0
            })
            .unwrap_or(crate::core::FileLoader::STREAM_DIST);
        let in_range = streaming_positions.iter().any(|&cam_pos| {
            let dx = cam_pos[0] - pos.x;
            let dy = cam_pos[1] - pos.y;
            dx * dx + dy * dy <= lod_dist * lod_dist
        });
        if in_range {
            script_models_to_request.push(model_id);
        }
    }

    if !script_models_to_request.is_empty() {
        script_models_to_request.sort_unstable();
        script_models_to_request.dedup();
        pending_script_models.extend(
            script_models_to_request
                .iter()
                .copied()
                .filter(|model_id| !runtime_model_geometry_is_resident(&file_loader, *model_id)),
        );
        file_loader.request_models_for_runtime(
            streaming,
            script_models_to_request.iter().copied(),
            crate::core::Streaming::STREAMFLAGS_PRIORITY
                | crate::core::Streaming::STREAMFLAGS_DONT_REMOVE,
        );
    }

    match file_loader.update_streaming_around_positions(
        streaming,
        &mut before_rw.cd_stream,
        streaming_positions.as_slice(),
        player_coors,
    ) {
        Ok(_) => {
            refresh.force_rebuild |= pending_cutscene_models
                .iter()
                .chain(pending_script_models.iter())
                .any(|model_id| runtime_model_geometry_is_resident(&file_loader, *model_id));
        }
        Err(error) => bevy::log::warn!("update_streaming_around_positions: {error}"),
    }
}

/// re3 `CRadar::DrawRadarMap` requests its 3x3 radar TXD window during the
/// render phase, after `CStreaming::Update` has already serviced the CD
/// channels for the frame. Keep that ordering so radar dependencies cannot
/// displace road/world requests from the current streaming service step.
fn request_radar_sections_after_world_render(
    mut file_loader: Option<ResMut<CFileLoader>>,
    mut streaming: Option<ResMut<crate::core::Streaming::CStreaming>>,
    hud: Option<Res<crate::renderer::Hud::CHud>>,
    gta_camera: Option<Res<CCamera>>,
    menu: Option<Res<CMenuManager>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    camera_query: Query<(&Camera, &Transform, &GtaSceneCamera)>,
) {
    let Some(hud) = hud.as_ref() else {
        return;
    };
    if !hud.wants_to_draw_hud
        || gta_camera
            .as_ref()
            .is_some_and(|camera| camera.wide_screen_on)
        || menu.as_ref().is_some_and(|menu| menu.menu_active)
        || cutscene
            .as_ref()
            .is_some_and(|cutscene| cutscene.loaded || cutscene.running)
    {
        return;
    }
    let (Some(file_loader), Some(streaming)) = (file_loader.as_mut(), streaming.as_mut()) else {
        return;
    };

    let mut camera_positions = camera_query
        .iter()
        .filter(|(camera, _, _)| camera.is_active)
        .map(|(_, transform, marker)| {
            let position = CVector::from_bevy(transform.translation);
            (marker.local_player, [position.x, position.y, position.z])
        })
        .collect::<Vec<_>>();
    camera_positions.sort_unstable_by_key(|(local_player, _)| *local_player);
    let positions = camera_positions
        .into_iter()
        .map(|(_, position)| position)
        .collect::<Vec<_>>();
    if positions.is_empty() {
        return;
    }

    file_loader.stream_radar_sections_for_positions(streaming, &positions);
}

fn runtime_model_geometry_is_resident(file_loader: &CFileLoader, model_id: usize) -> bool {
    let Some(model) = file_loader.model_info.get_model_info(model_id) else {
        return false;
    };
    model
        .base()
        .clump_summary
        .as_ref()
        .is_some_and(|summary| !summary.atomics.is_empty() || !summary.geometries.is_empty())
        || file_loader
            .model_info
            .simple_model_info(model_id)
            .is_some_and(|simple| !simple.atomic_bindings.is_empty())
}

/// re3 `ScanSectorList_Priority` raises `m_loadingPriority` only when a
/// priority STREAMME model is genuinely not loaded
/// (`m_loadState != STREAMSTATE_LOADED`). Distance-band STREAMMEs for
/// already-resident models must not latch the sector-walk throttle.
fn priority_stream_needs_loading(
    priority_stream_requests: &[usize],
    streaming: Option<&crate::core::Streaming::CStreaming>,
) -> bool {
    let Some(streaming) = streaming else {
        return !priority_stream_requests.is_empty();
    };
    priority_stream_requests
        .iter()
        .any(|model_index| !streaming.has_model_loaded(*model_index))
}

fn request_ordinary_renderer_stream_models(
    file_loader: &CFileLoader,
    streaming: &mut CStreaming,
    model_ids: impl IntoIterator<Item = usize>,
    loading_priority: bool,
) {
    for model_id in model_ids {
        // re3 ScanSectorList stops adding ordinary VIS_STREAMME requests once
        // the priority centre scan has filled the queue to ten models. Without
        // this gate a full Bevy render plan can enqueue hundreds of models that
        // CStreaming::Update then discards before the CD channels reach them.
        if loading_priority && streaming.num_models_requested >= 10 {
            break;
        }
        streaming.request_model_with_adapter(model_id, 0, file_loader);
    }
}

#[cfg(test)]
mod renderer_stream_request_tests {
    use super::*;

    #[test]
    fn priority_scan_caps_ordinary_renderer_requests_at_ten_models() {
        let file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();

        request_ordinary_renderer_stream_models(&file_loader, &mut streaming, 100..130, true);

        assert_eq!(streaming.num_models_requested, 10);
        assert_eq!(streaming.requested_list().count(), 10);
    }

    #[test]
    fn ordinary_renderer_scan_is_unbounded_without_loading_priority() {
        let file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();

        request_ordinary_renderer_stream_models(&file_loader, &mut streaming, 100..130, false);

        assert_eq!(streaming.num_models_requested, 30);
        assert_eq!(streaming.requested_list().count(), 30);
    }

    #[test]
    fn loading_priority_ignores_already_loaded_priority_models() {
        use crate::core::Streaming::StreamLoadState;

        let mut streaming = CStreaming::default();
        let loaded = 10usize;
        let pending = 11usize;
        streaming.info_for_model[loaded].load_state = StreamLoadState::Loaded;

        assert!(!priority_stream_needs_loading(&[loaded], Some(&streaming)));
        assert!(priority_stream_needs_loading(
            &[loaded, pending],
            Some(&streaming)
        ));
        assert!(priority_stream_needs_loading(&[pending], Some(&streaming)));
        assert!(!priority_stream_needs_loading(&[], Some(&streaming)));
        // Without streaming state (menu/boot) any priority request throttles.
        assert!(priority_stream_needs_loading(&[loaded], None));
    }
}

#[cfg(test)]
mod f9_probe_tests {
    use super::*;

    #[test]
    fn march_orders_hits_by_distance_and_rejects_perp() {
        let origin = [0.0, 0.0, 0.0];
        let dir = [0.0, 1.0, 0.0];
        let points = vec![[0.0, 10.0, 0.0], [50.0, 5.0, 0.0], [0.0, 4.0, 0.0]];
        let hits = march_f9_ray(origin, dir, &points, 100.0, 4.0);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, 2);
        assert_eq!(hits[1].0, 0);
    }

    #[test]
    fn yaw_pitch_matches_gta_heading_convention() {
        let (yaw, pitch) = f9_yaw_pitch_deg([0.0, 1.0, 0.0]);
        assert!(yaw.abs() < 0.01);
        assert!(pitch.abs() < 0.01);
        let (_, down) = f9_yaw_pitch_deg([0.0, 0.0, -1.0]);
        assert!((down + 90.0).abs() < 0.1);
    }

    #[test]
    fn report_without_world_still_names_probe() {
        let report = build_f9_probe_report(
            None,
            None,
            Some(CVector::new(1.0, 2.0, 3.0)),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(report.contains("RUSTGTA3_F9_PROBE"));
        assert!(report.contains("world=none"));
        assert!(report.contains("player=1.00,2.00,3.00"));
    }
}

#[cfg(test)]
mod render_prep_schedule_conflict_tests {
    use super::*;
    use bevy::ecs::{system::IntoSystem, world::World};

    #[test]
    fn new_game_invalidates_level_texture_generations_and_light_state() {
        let mut app = App::new();
        app.init_resource::<GtaWorldMeshRegistry>()
            .init_resource::<GtaWorldModelAssetCache>()
            .init_resource::<GtaWorldTextureCache>()
            .init_resource::<GtaRuntimeEntityModelRefs>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<GtaUnlitMaterial>>()
            .init_resource::<Assets<Image>>()
            .insert_resource(CCoronas::init())
            .init_resource::<CPointLights>()
            .add_systems(Update, reset_level_render_state);
        let old_image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
        {
            let mut cache = app.world_mut().resource_mut::<GtaWorldTextureCache>();
            cache.images.insert((2, "coronastar".to_owned()), old_image.clone());
            cache.synced_txd_generations.insert(2, 1);
            cache.synced_store_revision = Some(1);
        }
        app.world_mut().resource_mut::<GtaWorldMeshRegistry>().slots_epoch = 42;
        app.world_mut().resource_mut::<GtaRuntimeEntityModelRefs>().owned.insert(
            (GtaRuntimeModelRefSource::World, 11), 1432,
        );
        app.world_mut().resource_mut::<CCoronas>().register_corona(
            44, 255, 200, 50, 255, CVector::ZERO, 2.0, 80.0,
            super::super::Coronas::TYPE_NORMAL, 0, 0, 0, 0, 0.0,
        );
        app.world_mut().resource_mut::<CPointLights>().add_light(
            LIGHT_POINT, CVector::ZERO, CVector::ZERO, 4.0,
            1.0, 1.0, 0.5, 0, false, CVector::ZERO,
        );

        app.update();
        let registry = app.world().resource::<GtaWorldMeshRegistry>();
        assert_eq!(registry.slots_epoch, 43);
        assert!(registry.slots.is_empty());
        let cache = app.world().resource::<GtaWorldTextureCache>();
        assert!(cache.images.is_empty());
        assert!(cache.synced_txd_generations.is_empty());
        assert!(app.world().resource::<Assets<Image>>().get(&old_image).is_none());
        assert!(app.world().resource::<GtaRuntimeEntityModelRefs>().owned.is_empty());
        assert!(app.world().resource::<CCoronas>().active_coronas().next().is_none());
        assert!(app.world().resource::<CPointLights>().lights.is_empty());
        // First boot reaches LoadingGame before finalise_game_level_loading
        // inserts the corona pool. Reset must also work without that resource.
        app.world_mut().remove_resource::<CCoronas>();
        app.update();
        assert_eq!(app.world().resource::<GtaWorldMeshRegistry>().slots_epoch, 44);
    }

    /// B0001 regression: same-frame camera readers (`&Transform`) must stay
    /// disjoint from the `&mut Transform` slot writers in the same system.
    /// System init builds the combined component-access graph and panics on
    /// conflict without needing any assets or game state.
    #[test]
    fn camera_readers_initialize_without_query_conflicts() {
        let mut world = World::new();
        IntoSystem::into_system(sync_sky_horizon_mesh).initialize(&mut world);
        IntoSystem::into_system(update_coronas_lifecycle).initialize(&mut world);
        IntoSystem::into_system(update_atmosphere_and_fx).initialize(&mut world);
        IntoSystem::into_system(crate::renderer::ParticleSprite::append_corona_sprite_frame).initialize(&mut world);
        IntoSystem::into_system(flush_pacman_billboards).initialize(&mut world);
        IntoSystem::into_system(flush_sprite_batches).initialize(&mut world);
        IntoSystem::into_system(flush_screen_droplets).initialize(&mut world);
        IntoSystem::into_system(flush_point_light_fog).initialize(&mut world);
        IntoSystem::into_system(update_world_streaming_around_camera).initialize(&mut world);
        IntoSystem::into_system(update_particle_billboards).initialize(&mut world);
        IntoSystem::into_system(crate::renderer::ParticleSprite::build_particle_sprite_frame)
            .initialize(&mut world);
        IntoSystem::into_system(crate::renderer::ParticleSprite::append_cloud_sprite_frame)
            .initialize(&mut world);
    }

    #[test]
    fn update_world_streaming_around_camera_requests_script_object_models() {
        let mut world = World::new();
        let file_loader = CFileLoader::default();
        let mut streaming = crate::core::Streaming::CStreaming::default();
        streaming.image_offsets[0] = 0;
        let game_before_rw = crate::core::Game::CGameBeforeRw {
            file_mgr: rustgta3_assets::install::FileMgr::CFileMgr::initialise("."),
            cd_stream: rustgta3_assets::streaming::CdStream::CdStream::init(1),
            version_name: "test".into(),
        };
        let refresh = GtaRuntimeMeshRefresh::default();

        world.spawn((
            Camera {
                is_active: true,
                ..default()
            },
            Transform::from_translation(CVector::new(800.0, -940.0, 36.0).to_bevy()),
            GtaSceneCamera {
                local_player: 0,
                ..default()
            },
        ));

        let mut general = CGeneral::default();
        let mut object = crate::objects::Object::CObject::new_with_model(724, false, &mut general);
        object.physical.entity.matrix.position = CVector::new(787.81, -939.19, 38.94);
        world.spawn(crate::control::Script::ScriptObjectEntity { handle: 1, object }.into_bundle());

        world.insert_resource(file_loader);
        world.insert_resource(streaming);
        world.insert_resource(game_before_rw);
        world.insert_resource(refresh);

        let mut schedule = bevy::ecs::schedule::Schedule::default();
        schedule.add_systems(update_world_streaming_around_camera);
        schedule.run(&mut world);

        let streaming = world.resource::<crate::core::Streaming::CStreaming>();
        assert!(streaming.has_model_loaded(724));
    }

    #[test]
    fn cooperative_scene_load_services_streaming_while_code_paused_and_releases_own_pause() {
        let mut world = World::new();
        let mut file_loader = CFileLoader::default();
        let mut streaming = crate::core::Streaming::CStreaming::default();
        file_loader.begin_load_scene_streaming(&mut streaming, [0.0, 0.0, 0.0]);
        // This unit fixture has no GTA3.IMG/image-offset table. Remove the
        // synthetic scene requests after exercising begin; the assertion below
        // targets the paused service/resume control path, not CD decoding.
        streaming.flush_request_list();
        file_loader.set_script_streaming_owns_code_pause(true);

        let mut timer = CTimer::default();
        timer.set_code_pause(true);
        world.insert_resource(file_loader);
        world.insert_resource(streaming);
        world.insert_resource(crate::core::Game::CGameBeforeRw {
            file_mgr: rustgta3_assets::install::FileMgr::CFileMgr::initialise("."),
            cd_stream: rustgta3_assets::streaming::CdStream::CdStream::init(1),
            version_name: "test".into(),
        });
        world.insert_resource(GtaRuntimeMeshRefresh::default());
        world.insert_resource(timer);

        let mut schedule = bevy::ecs::schedule::Schedule::default();
        schedule.add_systems(update_world_streaming_around_camera);
        for _ in 0..64 {
            schedule.run(&mut world);
            if !world.resource::<CFileLoader>().scene_load_in_progress() {
                break;
            }
        }

        assert!(
            !world.resource::<CFileLoader>().scene_load_in_progress(),
            "scene barrier must drain even though normal CStreaming::Update is paused"
        );
        assert!(
            !world.resource::<CTimer>().get_is_code_paused(),
            "scene loader must release the code pause it owns"
        );
    }
}

/// re3 \`CUserDisplay::Process\`: update place/vehicle names from the focused
/// player's live ECS state before CHud advances its fade timers.
fn update_user_display_on_hud(
    file_loader: Option<Res<CFileLoader>>,
    text: Option<Res<CText>>,
    mut user_display: Option<ResMut<CUserDisplay>>,
    world_players: CWorldPlayersRead,
    vehicle_index: Res<crate::control::ScriptEntities::ScriptEntityIndex>,
    vehicles: crate::control::ScriptEntities::VehicleReadStore,
    mut hud: Option<ResMut<crate::renderer::Hud::CHud>>,
) {
    let (Some(file_loader), Some(text), Some(display), Some(hud)) = (
        file_loader.as_ref(),
        text.as_ref(),
        user_display.as_mut(),
        hud.as_mut(),
    ) else {
        return;
    };
    let Some(player) = world_players.focused() else {
        hud.zone_name = None;
        hud.vehicle_name = None;
        display.process_current_vehicle(None);
        return;
    };

    let pos = player.position();
    let zone_position = [pos.x, pos.y, pos.z];
    let navig_zone = file_loader
        .world
        .zones
        .find_zone_index_for_point_type(zone_position, ZONE_NAVIG);
    let default_zone = file_loader
        .world
        .zones
        .find_zone_index_for_point_type(zone_position, ZONE_DEFAULT);
    if let Some(zone_index) = display.process_place_name(navig_zone, default_zone) {
        if let Some(zone) = file_loader.world.zones.zones.get(zone_index) {
            hud.set_zone_name(text.get(&zone.name));
        } else {
            hud.zone_name = None;
        }
    } else {
        hud.zone_name = None;
    }

    let focused = world_players.player_in_focus() as i32;
    let vehicle_entity = world_players
        .relation(focused)
        .map(|relation| relation.vehicle);
    display.process_current_vehicle(vehicle_entity);
    let vehicle_game_name = vehicle_entity
        .and_then(|entity| {
            let vehicle = vehicles.get(entity).ok()?;
            (vehicle_index.vehicle(vehicle.handle()) == Some(entity)).then_some(vehicle)
        })
        .and_then(|vehicle| {
            let model_index = vehicle.base().physical.entity.model_index.max(0) as usize;
            match file_loader.model_info.get_model_info(model_index) {
                Some(ModelInfoSlot::Vehicle(model)) => Some(model.game_name.as_str()),
                _ => None,
            }
        });
    if let Some(game_name) = vehicle_game_name {
        hud.set_vehicle_name(text.get(game_name));
    } else {
        hud.vehicle_name = None;
    }
}

/// re3 max parked cars from car generators (`CCarCtrl::NumParkedCars >= 10`).
const MAX_PARKED_CARS_FROM_GENERATORS: i32 = 10;

/// re3 `CCarGenerator::CheckForBlockage`: `FindObjectsKindaColliding` is
/// called with vehicles and peds enabled. The generator uses the maximum
/// magnitude of its authored COL box as the query radius (`m_fSize`, cached
/// on the generator like re3 `Setup`; never silently disabled).
fn car_generator_is_blocked(
    intent: &crate::vehicles::CarGen::CarGeneratorSpawnIntent,
    generator_radius: f32,
    script_entities: &ScriptEntityWorld,
    world_players: Option<&CWorldPlayersRead>,
    file_loader: &CFileLoader,
) -> bool {
    let overlaps = |position: CVector, radius: f32| {
        let dx = position.x - intent.position.x;
        let dy = position.y - intent.position.y;
        dx * dx + dy * dy < (generator_radius + radius).powi(2)
    };

    if script_entities.vehicles.values().any(|vehicle| {
        let base = vehicle.base();
        let radius = file_loader
            .model_info
            .col_model(base.physical.entity.model_index.max(0) as usize)
            .map_or(0.0, |col| col.bounding_sphere.radius);
        overlaps(base.physical.entity.get_position(), radius)
    }) {
        return true;
    }
    if script_entities.peds.values().any(|ped| {
        let radius = file_loader
            .model_info
            .col_model(ped.model_id.max(0) as usize)
            .map_or(0.5, |col| col.bounding_sphere.radius);
        overlaps(ped.ped.physical.entity.get_position(), radius)
    }) {
        return true;
    }
    world_players.is_some_and(|players| {
        players.iter().any(|player| {
            let model_id = player.ped.physical.entity.model_index.max(0) as usize;
            let radius = file_loader
                .model_info
                .col_model(model_id)
                .map_or(0.5, |col| col.bounding_sphere.radius);
            overlaps(player.ped.physical.entity.get_position(), radius)
        })
    })
}

/// re3 `CTheCarGenerators::Process` + `CCarGenerator::DoInternalProcessing` parked path.
/// Emits range intents then inserts PARKED_VEHICLE automobiles into ScriptEntityWorld.
fn process_car_generators_near_player(
    mut car_gens: Option<ResMut<crate::vehicles::CarGen::CTheCarGenerators>>,
    world_players: CWorldPlayersRead,
    bridge: Option<Res<ScriptGameplayBridge>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    camera: Option<Res<CCamera>>,
    timer: Option<Res<CTimer>>,
    mut script_entities: Option<ScriptEntityWorld>,
    mut pools: Option<ResMut<CPools>>,
    mut file_loader: Option<ResMut<CFileLoader>>,
    mut streaming: Option<ResMut<CStreaming>>,
    mut general: Option<ResMut<CGeneral>>,
    handling_manager: Option<Res<cHandlingDataMgr>>,
    mut car_ctrl: Option<ResMut<crate::control::CarCtrl::CCarCtrl>>,
    taxi_lights: Option<Res<ScriptTaxiLightState>>,
) {
    let Some(car_gens) = car_gens.as_mut() else {
        return;
    };

    // re3 CCarGenerator::Process bookkeeping (CarGen.cpp:143-156): pool slot
    // gone -> clear handle; generated car taken by the player -> timer += 60s,
    // handle cleared, generator blocks until the player leaves the far clip.
    if let Some(script_entities) = script_entities.as_mut() {
        let n = car_gens.num_of_car_generators.max(0) as usize;
        for i in 0..n {
            let handle = car_gens.car_generator_array[i].vehicle_handle;
            if handle < 0 {
                continue;
            }
            let Some(mut vehicle) = script_entities
                .vehicles
                .get_mut(&script_entities.index, &handle)
            else {
                car_gens.note_vehicle_gone(i);
                continue;
            };
            if vehicle.base().physical.entity.status
                == crate::entities::Entity::eEntityStatus::STATUS_PLAYER
            {
                vehicle.base_mut().flags.extended_range = false;
                car_gens.note_vehicle_taken_by_player(i);
            }
        }
    }

    let pos = world_players.focused().map(|p| p.position()).or_else(|| {
        bridge
            .as_ref()
            .and_then(|b| b.player_spawned.then_some(b.player_position))
    });
    let Some(pos) = pos else {
        return;
    };
    // re3 FindPlayerSpeed: on foot the ped move speed, in a vehicle the
    // vehicle's; generators only spawn while the player is not driving at them.
    let player_speed = world_players
        .focused()
        .map(|p| p.ped.physical.move_speed)
        .unwrap_or(CVector::ZERO);
    let cutscene_processing = cutscene
        .as_ref()
        .is_some_and(|c| c.is_cutscene_processing() || c.is_running());
    let time_ms = timer
        .as_ref()
        .map(|t| t.time_in_milliseconds())
        .unwrap_or(0);
    let generation_dist_multiplier = camera
        .as_ref()
        .map_or(1.0, |camera| camera.generation_dist_multiplier);
    let intents = car_gens.process(
        time_ms,
        pos,
        player_speed,
        cutscene_processing,
        generation_dist_multiplier,
    );
    if intents.is_empty() {
        return;
    }
    let (Some(script_entities), Some(pools), Some(file_loader), Some(general), Some(streaming)) = (
        script_entities.as_mut(),
        pools.as_mut(),
        file_loader.as_deref_mut(),
        general.as_mut(),
        streaming.as_deref_mut(),
    ) else {
        bevy::log::debug!(
            "CTheCarGenerators::Process intents={} but pools/world not ready",
            intents.len()
        );
        return;
    };
    let handling = handling_manager.as_deref();
    let mut spawned = 0usize;
    for intent in intents {
        // re3 DoInternalProcessing: RequestModel + HasModelLoaded wait runs
        // before any spawn bookkeeping; an unloaded model retries next stride
        // without consuming a use or touching the timer.
        let Ok(model_index) = usize::try_from(intent.model_index) else { continue; };
        streaming.request_model_with_adapter(model_index, crate::core::Streaming::STREAMFLAGS_DEPENDENCY, file_loader);
        if !streaming.has_model_loaded(model_index) {
            continue;
        }
        let Some(generator_col) = file_loader
            .model_info
            .col_model(intent.model_index.max(0) as usize)
        else {
            continue;
        };
        let model_info = file_loader
            .model_info
            .get_model_info(intent.model_index.max(0) as usize);
        let Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(_)) = model_info else {
            continue;
        };
        // re3 Setup caches m_fSize from the COL box; mirror it once the COL is
        // resident so CheckForBlockage always has a real radius.
        if car_gens.car_generator_array[intent.generator_index].size <= 0.0 {
            car_gens.set_col_bounds(
                intent.generator_index,
                CVector::new(
                    generator_col.bounding_box.min.x,
                    generator_col.bounding_box.min.y,
                    generator_col.bounding_box.min.z,
                ),
                CVector::new(
                    generator_col.bounding_box.max.x,
                    generator_col.bounding_box.max.y,
                    generator_col.bounding_box.max.z,
                ),
            );
        }
        let generator_radius = car_gens.car_generator_array[intent.generator_index].size;
        if car_generator_is_blocked(
            &intent,
            generator_radius,
            script_entities,
            Some(&world_players),
            file_loader,
        ) {
            car_gens.note_spawn_blocked(intent.generator_index);
            continue;
        }
        let current_parked = car_ctrl.as_ref().map(|c| c.num_parked_cars).unwrap_or(0);
        if current_parked >= MAX_PARKED_CARS_FROM_GENERATORS {
            break;
        }
        // re3 DoInternalProcessing ground Z:
        // z > -100 → FindGroundZFor3DCoord; else ProcessVerticalLine / FindGroundZForCoord.
        let ground_z = if intent.position.z > -100.0 {
            file_loader.world.find_ground_z_for_3d_coord(
                intent.position.x,
                intent.position.y,
                intent.position.z,
                &file_loader.model_info,
            )
        } else {
            Some(file_loader.world.find_ground_z_for_coord(
                intent.position.x,
                intent.position.y,
                &file_loader.model_info,
            ))
        };
        let Some(ground_z) = ground_z else {
            bevy::log::debug!(
                "CarGen skip: no ground z x={:.1} y={:.1}",
                intent.position.x,
                intent.position.y
            );
            car_gens.note_spawn_attempt_finished(intent.generator_index, time_ms, None);
            continue;
        };
        let Some(handle) = script_entities.insert_parked_car_from_generator(
            pools,
            intent.model_index,
            intent.position,
            intent.angle_degrees,
            time_ms,
            model_info,
            handling,
            general,
            Some(ground_z),
            taxi_lights.as_ref().is_some_and(|state| state.all_enabled),
        ) else {
            continue;
        };
        // re3 DoInternalProcessing post-spawn: fixed colours, alarm, door lock chance.
        if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) = script_entities
            .vehicles
            .get_mut(&script_entities.index, &handle)
        {
            let col = file_loader
                .model_info
                .col_model(intent.model_index.max(0) as usize);
            let (bbox_min, bbox_max) = col
                .map(|c| {
                    (
                        CVector::new(
                            c.bounding_box.min.x,
                            c.bounding_box.min.y,
                            c.bounding_box.min.z,
                        ),
                        CVector::new(
                            c.bounding_box.max.x,
                            c.bounding_box.max.y,
                            c.bounding_box.max.z,
                        ),
                    )
                })
                .unwrap_or((CVector::new(-1.0, -2.0, -0.8), CVector::new(1.0, 2.0, 0.8)));
            car.place_on_road_properly(bbox_min, bbox_max, |probe, low_z| {
                file_loader
                    .world
                    .process_vertical_line_buildings_only_with_entity(
                        [probe.x, probe.y, probe.z],
                        low_z,
                        &file_loader.model_info,
                    )
                    .map(|(point, entity)| (point.point.z, entity))
            });
            // re3: `if (m_nColor1 != -1 && m_nColor2)` — color2 truthy, not != -1.
            if intent.color1 != -1 && intent.color2 != 0 {
                car.vehicle.appearance.current_colour1 = intent.color1 as u8;
                car.vehicle.appearance.current_colour2 = intent.color2 as u8;
            }
            if general.get_random_number_in_range_i32(0, 100) < intent.alarm as i32 {
                car.vehicle.audio.alarm_state = -1;
            }
            if general.get_random_number_in_range_i32(0, 100) < intent.doorlock as i32 {
                car.vehicle.entry.door_lock = crate::vehicles::Vehicle::eCarLock::CARLOCK_LOCKED;
            }
            car.vehicle.drive.engine_on = false;
        }
        car_gens.note_spawn_attempt_finished(intent.generator_index, time_ms, Some(handle));
        if let Some(car_ctrl) = car_ctrl.as_mut() {
            car_ctrl.num_parked_cars = car_ctrl.num_parked_cars.saturating_add(1);
        }
        spawned += 1;
    }
    if spawned > 0 {
        bevy::log::debug!(
            "CTheCarGenerators::Process parked spawn count={} vehicles={}",
            spawned,
            script_entities.vehicles.len()
        );
    }
}

fn setup_renderware_camera(mut commands: Commands) {
    // GTA III world units are large (±2k); default Bevy far=1000 clips most map.
    // Follow camera tightens this; keep a sane default from frame 0 so mesh
    // extraction / frustum culling can actually hit world geometry.
    commands.spawn((
        Camera3d::default(),
        NoIndirectDrawing,
        // The GTA world uses the unlit `GtaUnlitMaterial`; only the small
        // particle/fluff billboards are Bevy-lit. A single light cluster keeps
        // those lit while removing the per-frame GPU clustering compute pass
        // (`cluster_on_gpu`) and its buffer preparation, which are pure
        // overhead for this light budget.
        bevy::light::cluster::ClusterConfig::Single,
        Camera {
            // Match clear from timecycle once loaded; avoid pure black void.
            clear_color: ClearColorConfig::Default,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            // 70° is GTA/re3's horizontal FOV. 1280×720 default window needs
            // the corresponding vertical projection until the camera adapter
            // updates it from the live aspect ratio.
            fov: 42.995_f32.to_radians(),
            near: 0.3,
            far: 3000.0,
            ..default()
        }),
        Tonemapping::None,
        DebandDither::Disabled,
        // GTA III's RenderWare D3D8 path does not use 4× MSAA. Avoid shading
        // the already-heavy streamed world four times in the development build.
        Msaa::Off,
        Transform::from_xyz(0.0, 650.0, 850.0).looking_at(Vec3::ZERO, Vec3::Y),
        GlobalTransform::default(),
        GtaSceneCamera { local_player: 0 },
    ));
    // UI must target a full-window camera. Otherwise Bevy's implicit camera
    // selection would attach HUD/frontend nodes to one half-screen 3D camera.
    commands.spawn((
        Camera2d,
        // Full-window UI uses direct draws. WorldSubmission selects the
        // supported grouped submission mode only for 3D cameras.
        NoIndirectDrawing,
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
        Name::new("re3 Full-window UI Camera"),
    ));
}

fn setup_screen_fade(mut commands: Commands) {
    commands.spawn((
        GtaScreenFade,
        bevy::picking::Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ImageNode {
            image_mode: NodeImageMode::Stretch,
            color: Color::NONE,
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(20_000),
    ));
}

fn setup_widescreen_borders(mut commands: Commands) {
    // re3 CCamera::ProcessWideScreenOn fixes ScreenReductionPercentage at 30.
    // DrawBordersForWideScreen then subtracts SCREEN_SCALE_Y(8) from the top
    // edge and from the bottom edge's inner coordinate.  In the 448-high HUD
    // virtual raster that is 13.214285% at the top and 16.785715% at bottom.
    const TOP_HEIGHT_PERCENT: f32 = 15.0 - (8.0 / 448.0 * 100.0);
    const BOTTOM_HEIGHT_PERCENT: f32 = 15.0 + (8.0 / 448.0 * 100.0);

    for (bottom, height) in [(false, TOP_HEIGHT_PERCENT), (true, BOTTOM_HEIGHT_PERCENT)] {
        let mut node = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(height),
            display: Display::None,
            ..default()
        };
        if bottom {
            node.bottom = Val::Px(0.0);
        } else {
            node.top = Val::Px(0.0);
        }
        commands.spawn((
            GtaWideScreenBorder { bottom },
            bevy::picking::Pickable::IGNORE,
            node,
            BackgroundColor(Color::BLACK),
            GlobalZIndex(19_000),
        ));
    }
}

fn sync_widescreen_borders(
    camera: Option<Res<CCamera>>,
    bridge: Option<Res<ScriptGameplayBridge>>,
    menu: Option<Res<CMenuManager>>,
    mut borders: Query<(&GtaWideScreenBorder, &mut Node)>,
) {
    let widescreen = camera.as_ref().is_some_and(|camera| camera.wide_screen_on)
        || bridge.as_ref().is_some_and(|bridge| bridge.widescreen);
    let visible = widescreen && !menu.as_ref().is_some_and(|menu| menu.menu_active);
    for (border, mut node) in &mut borders {
        let _ = border.bottom;
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn sync_screen_fade(
    camera: Option<Res<CCamera>>,
    bridge: Option<Res<ScriptGameplayBridge>>,
    file_loader: Option<Res<CFileLoader>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut images: ResMut<Assets<Image>>,
    mut fade_query: Query<(&mut Node, &mut BackgroundColor, &mut ImageNode), With<GtaScreenFade>>,
) {
    let Some((mut node, mut background, mut image_node)) = fade_query.iter_mut().next() else {
        return;
    };
    let Some(camera) = camera else {
        node.display = Display::None;
        return;
    };
    let alpha = camera.fade_value.round().clamp(0.0, 255.0) as u8;
    node.display = if alpha == 0 {
        Display::None
    } else {
        Display::Flex
    };
    if alpha == 0 {
        image_node.color = Color::NONE;
        *background = BackgroundColor(Color::NONE);
        return;
    }

    // re3 `DoFade`: black is a sentinel for the currently loaded splash
    // (`m_FadeTargetIsSplashScreen`), not an ordinary black overlay. Decode
    // that dedicated TXD through the same cache used by world materials and
    // alpha-composite it using CDraw::FadeValue.
    if camera.fade_target_is_splash_screen
        && let (Some(bridge), Some(file_loader)) = (bridge.as_deref(), file_loader.as_deref())
        && !bridge.splash_screen_name.is_empty()
        && let Some(slot) = file_loader.txd_store.find_txd_slot("splash")
    {
        let _ = sync_txd_slot_texture_cache(slot, file_loader, &mut texture_cache, &mut images);
        let key = (slot, bridge.splash_screen_name.to_ascii_lowercase());
        if let Some(image) = texture_cache.images.get(&key) {
            image_node.image = image.clone();
            image_node.color = Color::srgba_u8(255, 255, 255, alpha);
            *background = BackgroundColor(Color::NONE);
            return;
        }
    }

    image_node.color = Color::NONE;
    *background = BackgroundColor(Color::srgba_u8(
        camera.fade_colour[0],
        camera.fade_colour[1],
        camera.fade_colour[2],
        alpha,
    ));
}

/// Last values written by [`sync_timecycle_lights`]. All outputs are a pure
/// function of the timecycle state plus the brightness preference, so
/// skipping identical writes is behaviour-identical while keeping Bevy change
/// detection (and the GPU uniform upload) quiet on static time-of-day.
#[derive(Default)]
struct TimecycleLightCache {
    ambient_color: Option<Color>,
    ambient_brightness: Option<f32>,
    dir_color: Option<Color>,
    dir_illuminance: Option<f32>,
    dir_rotation: Option<Quat>,
}

fn sync_timecycle_lights(
    timecycle: Option<Res<CTimeCycle>>,
    menu: Option<Res<CMenuManager>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut commands: Commands,
    mut directional_query: Query<
        (Entity, &mut DirectionalLight, &mut Transform),
        With<GtaDirectionalLight>,
    >,
    mut cache: Local<TimecycleLightCache>,
) {
    let Some(timecycle) = timecycle else {
        return;
    };
    let current = timecycle.current;
    let brightness = menu.as_ref().map(|m| m.prefs_brightness).unwrap_or(256);
    let (ambient_multiplier, directional_multiplier) = if brightness > 256 {
        let excess = brightness as f32 / 256.0 - 1.0;
        (3.0 * excess * 0.6 + 1.0, 2.0 * excess * 0.6 + 1.0)
    } else {
        (1.0, 1.0)
    };
    let ar = (current.ambient[0] * ambient_multiplier).clamp(0.0, 1.0);
    let ag = (current.ambient[1] * ambient_multiplier).clamp(0.0, 1.0);
    let ab = (current.ambient[2] * ambient_multiplier).clamp(0.0, 1.0);
    let ambient_color = Color::srgb(ar, ag, ab);
    let sun_slot = timecycle.current_stored_value.rem_euclid(16) as usize;
    let direction = timecycle.vector_to_sun[sun_slot]
        .to_bevy()
        .normalize_or_zero();
    let direction = if direction.length_squared() < 1e-6 {
        Vec3::new(0.3, 0.8, 0.2).normalize()
    } else {
        direction
    };
    let rotation = Transform::default()
        .looking_to(-direction, Vec3::Y)
        .rotation;
    let dir_r = (current.directional[0] * directional_multiplier).clamp(0.0, 1.0);
    let dir_g = (current.directional[1] * directional_multiplier).clamp(0.0, 1.0);
    let dir_b = (current.directional[2] * directional_multiplier).clamp(0.0, 1.0);
    let dir_color = Color::srgb(dir_r, dir_g, dir_b);
    const DIR_ILLUMINANCE: f32 = 25_000.0;
    const AMBIENT_BRIGHTNESS: f32 = 140.0;
    let cache_hit = cache.ambient_color == Some(ambient_color)
        && cache.ambient_brightness == Some(AMBIENT_BRIGHTNESS)
        && cache.dir_color == Some(dir_color)
        && cache.dir_illuminance == Some(DIR_ILLUMINANCE)
        && cache.dir_rotation == Some(rotation);
    if directional_query.is_empty() {
        commands.spawn((
            GtaDirectionalLight,
            DirectionalLight {
                color: dir_color,
                illuminance: DIR_ILLUMINANCE,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform {
                rotation,
                ..default()
            },
            GlobalTransform::default(),
        ));
    } else if cache_hit {
        return;
    } else if let Some((_, mut light, mut transform)) = directional_query.iter_mut().next() {
        light.color = dir_color;
        light.illuminance = DIR_ILLUMINANCE;
        transform.rotation = rotation;
    }
    ambient.color = ambient_color;
    ambient.brightness = AMBIENT_BRIGHTNESS;
    cache.ambient_color = Some(ambient_color);
    cache.ambient_brightness = Some(AMBIENT_BRIGHTNESS);
    cache.dir_color = Some(dir_color);
    cache.dir_illuminance = Some(DIR_ILLUMINANCE);
    cache.dir_rotation = Some(rotation);
}

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaSkyHorizonMesh;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaImmediateMesh;

#[derive(Debug, Clone, Copy, Component)]
pub struct GtaWorldMesh {
    pub entity_index: usize,
    pub pass: GtaWorldRenderPass,
}

/// Persistent world submeshes keyed by IPL entity index (no full despawn each frame).
#[derive(Debug, Clone, Default, Resource)]
pub struct GtaWorldMeshRegistry {
    /// entity_index → spawned Bevy entities (one per material submesh)
    pub slots: BTreeMap<usize, GtaWorldMeshSlot>,
    /// Hidden Bevy mirrors detached by re3 `DeleteAllRwObjects`.
    ///
    /// A RenderWare RwObject is cheap to destroy/recreate; a Bevy mesh entity
    /// also carries ECS extraction and GPU-facing asset state. Keeping that
    /// mirror recyclable prevents LOAD_SCENE from turning one logical RW
    /// reset into a mass despawn/spawn storm and a visible world flash.
    retired_slots: BTreeMap<usize, GtaWorldMeshSlot>,
    pub slot_presence: Vec<u64>,
    /// Last consumed `CStreaming::DeleteAllRwObjects` generation.
    rw_object_epoch: u64,
    pub slots_epoch: u64,
}

impl GtaWorldMeshRegistry {
    #[inline(always)]
    pub fn has_slot(&self, entity_index: usize) -> bool {
        let word = entity_index / 64;
        let bit = 1u64 << (entity_index % 64);
        self.slot_presence.get(word).is_some_and(|w| (w & bit) != 0)
    }

    #[inline(always)]
    fn has_current_slot(&self, entity_index: usize, model_index: usize) -> bool {
        self.slots
            .get(&entity_index)
            .is_some_and(|slot| slot.model_index == model_index)
    }

    #[inline]
    pub fn set_slot_presence(&mut self, entity_index: usize, present: bool) {
        let word = entity_index / 64;
        let bit = 1u64 << (entity_index % 64);
        if present {
            if word >= self.slot_presence.len() {
                self.slot_presence.resize(word + 1, 0);
            }
            self.slot_presence[word] |= bit;
        } else if word < self.slot_presence.len() {
            self.slot_presence[word] &= !bit;
        }
    }

    #[inline]
    pub fn rebuild_slot_presence(&mut self) {
        self.slot_presence.fill(0);
        for &index in self.slots.keys() {
            let word = index / 64;
            let bit = 1u64 << (index % 64);
            if word >= self.slot_presence.len() {
                self.slot_presence.resize(word + 1, 0);
            }
            self.slot_presence[word] |= bit;
        }
    }

    #[inline]
    pub fn clear_slot_presence(&mut self) {
        self.slot_presence.fill(0);
    }
}

#[derive(Debug, Clone)]
pub struct GtaWorldMeshSlot {
    pub bevy_entities: Vec<Entity>,
    /// Entity-local material clones used while re3's `bDistanceFade` is
    /// active. The normal cache remains shared by every world instance.
    fade_materials: Vec<Handle<GtaUnlitMaterial>>,
    asset_key: GtaWorldModelAssetKey,
    pub model_index: usize,
    pub pass: GtaWorldRenderPass,
    pub submesh_count: usize,
    pub is_visible: bool,
    pub textured_count: usize,
    pub missing_texture_count: usize,
    pub txd_store_revision: u64,
    pub lod_multiplier: f32,
    pub lod_min_dist: f32,
    pub lod_max_dist: f32,
    /// Last CEntity transform mirrored into the persistent Bevy drawables.
    /// Garage doors and other script-driven map objects can move while still
    /// being physically "static" in re3, so static-ness cannot be used as a
    /// render-transform cache key.
    pub render_transform: Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GtaWorldModelAssetKey {
    model_index: usize,
    lod_index: usize,
    txd_generation: u64,
    txd_revision: u64,
}

#[derive(Debug, Clone)]
struct GtaWorldModelAssets {
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<GtaUnlitMaterial>>,
    textured_count: usize,
    missing_texture_count: usize,
}

/// TXD-generation-independent geometry: C++ keeps the entity RwObject
/// (geometry) alive while TXD dictionaries cycle underneath it and only the
/// texture bindings refresh. Keying shared Mesh assets by TXD generation
/// turned every TXD evict+reload into a full geometry decode, GPU re-upload
/// and entity respawn storm for all dependent models.
#[derive(Debug, Clone, Default)]
struct GtaWorldModelGeometry {
    meshes: Vec<Handle<Mesh>>,
    /// Material recipes 1:1 with `meshes`. The `mesh` field is an empty
    /// placeholder: shape info lives in `vertex_count`/`index_count`, and
    /// only the remaining fields feed material resolution.
    recipes: Vec<ModelMaterialDrawable>,
}

#[derive(Debug, Clone, Default, Resource)]
struct GtaWorldModelAssetCache {
    entries: BTreeMap<GtaWorldModelAssetKey, GtaWorldModelAssets>,
    geometry: BTreeMap<(usize, usize), GtaWorldModelGeometry>,
    /// Recently-evicted GTA models keep only their already-uploaded Bevy Mesh
    /// assets for a few prune passes. This does not hold a GTA streaming ref or
    /// keep the source DFF resident; it only prevents sector-boundary
    /// unload/reload oscillation from repeatedly freeing and reallocating the
    /// same GPU geometry.
    geometry_idle_prunes: BTreeMap<(usize, usize), u8>,
    prune_tick: u8,
}

/// Asset-cache pruning runs once every 30 render passes. Four idle passes are
/// long enough to absorb ordinary sector-boundary oscillation while remaining
/// bounded as the player travels through the map.
const WORLD_GEOMETRY_CACHE_GRACE_PRUNES: u8 = 4;

fn world_geometry_cache_can_grace(model_index: usize) -> bool {
    use crate::modelinfo::ModelIndices::{MI_SPECIAL01, MI_SPECIAL04};

    // Special-model slots are intentionally reusable by SCM. Keeping geometry
    // from a previous special-model name could bind the wrong DFF after the
    // slot is repurposed, so preserve native immediate invalidation there.
    !(MI_SPECIAL01..=MI_SPECIAL04).contains(&model_index)
}

/// Geometry handles for `(model_index, lod_index)` are alive when every
/// stored Mesh asset still exists, regardless of TXD generation.
fn world_model_geometry_ready(
    model_assets: &GtaWorldModelAssetCache,
    meshes: &Assets<Mesh>,
    model_index: usize,
    lod_index: usize,
) -> bool {
    model_assets
        .geometry
        .get(&(model_index, lod_index))
        .is_some_and(|geometry| {
            geometry.meshes.len() == geometry.recipes.len()
                && !geometry.meshes.is_empty()
                && geometry
                    .meshes
                    .iter()
                    .all(|handle| meshes.get(handle).is_some())
        })
}

fn world_model_assets_ready(
    model_assets: &GtaWorldModelAssetCache,
    meshes: &Assets<Mesh>,
    materials: &Assets<GtaUnlitMaterial>,
    asset_key: GtaWorldModelAssetKey,
) -> bool {
    model_assets.entries.get(&asset_key).is_some_and(|assets| {
        assets.meshes.len() == assets.materials.len()
            && !assets.meshes.is_empty()
            && assets
                .meshes
                .iter()
                .all(|handle| meshes.get(handle).is_some())
            && assets
                .materials
                .iter()
                .all(|handle| materials.get(handle).is_some())
    })
}

#[derive(Debug, Clone, Copy, Component)]
pub struct GtaRuntimePoolMesh {
    pub source: GtaRuntimePoolMeshSource,
    pub slot: usize,
    pub model_index: usize,
    pub render_damaged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GtaRuntimePoolMeshSource {
    SavePed,
    SaveVehicle,
    SaveObject,
    ReplayPed,
    ReplayVehicle,
    /// Live `CWorldPlayersRead` player ped after CREATE_PLAYER / SetupPlayerPed.
    WorldPlayer,
    /// Weapon atomic attached to the player's animated right-hand frame.
    WorldPlayerWeapon,
    /// Live CProjectile world object backed by CProjectileInfo's fixed slot.
    Projectile,
    /// Live SCM-created mission ped.
    ScriptPed,
    /// Weapon atomic attached to a script/NPC ped's animated right hand.
    ScriptPedWeapon,
    /// Stationary SCM-created mission vehicle; geometry/materials are reusable.
    ScriptVehicle,
    /// Moving/steering SCM vehicle whose baked wheel frames update at 30 Hz.
    ScriptVehicleDynamic,
    /// Live SCM-created mission object.
    ScriptObject,
    /// Cutscene object/head slot from `CCutsceneMgr` (re3 cutscene world objects).
    CutsceneObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GtaWorldRenderPass {
    Roads,
    EverythingBarRoads,
    FadingIn,
}

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaSpriteMesh;

#[derive(Debug, Clone, Copy, Component, PartialEq)]
pub struct GtaSpriteBatchIntent {
    pub z_test_disabled: bool,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct GtaPostFxOverlay {
    pub kind: PostFxIntentKind,
    pub blur_type: i32,
    pub blur_alpha: u32,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct GtaScreenDropletMesh {
    pub drop_index: usize,
    pub magnification: f32,
}

#[derive(Debug, Clone, Copy, Component)]

pub struct GtaFluffMesh {
    pub intent: FluffRenderIntent,
}

/// Monotonic spawn counter giving particles re3's linked-list insertion order
/// for rendering (`CParticle::Render` walks each system list head-to-tail).
#[derive(Debug, Default, Resource)]
pub struct GtaParticleSpawnSequence(pub u64);

#[derive(Debug, Clone, Component)]
pub struct GtaParticleBillboard {
    pub particle_type: i32,
    pub tracking_id: Option<u64>,
    pub spawn_sequence: u64,
    /// Last frame's device-pixel screen position for re3 SCREEN_TRAIL
    /// (`CParticle::m_vecScreenPosition`, zero = unset).
    pub screen_position: Vec2,
    pub velocity: Vec3,
    pub spawn_time_ms: u32,
    pub death_time_ms: u32,
    pub size: f32,
    pub expansion_rate: f32,
    pub gravitational_acceleration: f32,
    pub friction_deceleration: i32,
    pub rotation_degrees: f32,
    pub rotation_step: f32,
    pub colour: [u8; 4],
    pub fade_alpha_initial_intensity: u8,
    pub fade_alpha_time: u16,
    pub fade_alpha_amount: i16,
    pub alpha: u8,
    pub fade_alpha_timer: u16,
    pub fade_to_black_initial_intensity: u8,
    pub fade_to_black_time: u16,
    pub fade_to_black_amount: i16,
    pub intensity: u8,
    pub fade_to_black_timer: u16,
    pub current_z_rotation: u16,
    pub z_rotation_change_time: u16,
    pub z_rotation_change_amount: i16,
    pub z_rotation_timer: u16,
    pub current_z_radius: f32,
    pub z_radius_change_time: u16,
    pub z_radius_change_amount: f32,
    pub z_radius_timer: u16,
    pub particle_movement_offset: Vec3,
    pub ground_z: Option<f32>,
    pub flags: u32,
    pub animation_frames: Vec<Handle<Image>>,
    pub current_frame: usize,
    pub start_frame: usize,
    pub final_frame: usize,
    pub animation_speed: u16,
    pub animation_timer: u16,
    pub base_colour: [u8; 4],
    pub fade_destination_colour: [u8; 4],
    pub colour_fade_time: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Component)]
pub struct GtaParticleScreenSpace;

/// Shared geometry for short-lived billboard effects.  re3 submits these
/// through its immediate RenderWare path without rebuilding a vertex buffer
/// for every corona/cloud/pickup, so retain the Bevy mesh handle as well.
#[derive(Debug, Clone, Default, Resource)]
pub struct GtaImmediateMeshCache {
    pub unit_billboard: Option<Handle<Mesh>>,
    pub cloud_entities: Vec<Entity>,
    pub pacman_entities: Vec<Entity>,
    pub render_buffer_entities: Vec<Entity>,
    pub special_fx_entities: Vec<Entity>,
    pub shadow_entities: Vec<Entity>,
    pub fluff_entities: Vec<Entity>,
    pub line_entities: Vec<Entity>,
    pub sprite_entities: Vec<Entity>,
    pub droplet_entities: Vec<Entity>,
    pub point_light_fog_entities: Vec<Entity>,
    pub street_fx_entities: Vec<Entity>,
    pub glass_entities: Vec<Entity>,
    /// Last drained line intents. The line mesh is a pure function of these,
    /// so an unchanged list leaves identical slot state and the mesh rebuild
    /// is skipped. Capacity is retained via `clone_from`.
    last_line_intents: Vec<LineIntent>,
    /// Last drained sprite batches, same content-equality rule as above.
    last_sprite_batches: Vec<SpriteBatch>,
    /// View inputs of the last built sprite meshes (see [`SpriteViewKey`]).
    last_sprite_view: Option<SpriteViewKey>,
    /// Active sprite slot count of the last build; reused for the hide tail
    /// on content-identical frames.
    last_sprite_active_count: usize,
}

/// Exact inputs of [`build_sprite_mesh`] beyond the batch itself, verified
/// against `Camera::viewport_to_world`: the ray comes from
/// `clip_from_view` (projection + viewport size), the camera transform affine
/// and the NDC rect, while screen points are additionally clamped by the
/// window size. Equal batches plus an equal key therefore build byte-identical
/// meshes and the whole slot update can be skipped.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SpriteViewKey {
    cam_translation: [u32; 3],
    cam_rotation: [u32; 4],
    cam_scale: [u32; 3],
    clip_from_view: [u32; 16],
    viewport_rect: Option<([u32; 2], [u32; 2])>,
    window_size: [u32; 2],
}

fn sprite_view_key(
    camera: &Camera,
    camera_transform: &Transform,
    window: &Window,
) -> SpriteViewKey {
    let bits = |v: Vec3| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()];
    let translation = camera_transform.translation;
    let rotation = camera_transform.rotation;
    SpriteViewKey {
        cam_translation: bits(translation),
        cam_rotation: [
            rotation.x.to_bits(),
            rotation.y.to_bits(),
            rotation.z.to_bits(),
            rotation.w.to_bits(),
        ],
        cam_scale: bits(camera_transform.scale),
        clip_from_view: camera
            .computed
            .clip_from_view
            .to_cols_array()
            .map(f32::to_bits),
        viewport_rect: camera.logical_viewport_rect().map(|rect| {
            (
                [rect.min.x.to_bits(), rect.min.y.to_bits()],
                [rect.max.x.to_bits(), rect.max.y.to_bits()],
            )
        }),
        window_size: [window.width().to_bits(), window.height().to_bits()],
    }
}

#[derive(Debug, Default, Resource)]
struct GtaMarkerAssetCache {
    cylinder_parts: Vec<GtaMarkerMeshPart>,
    arrow_parts: Vec<GtaMarkerMeshPart>,
}

#[derive(Debug)]
struct GtaMarkerMeshPart {
    mesh: Handle<Mesh>,
    texture_name: Option<String>,
    surface_ambient: f32,
    surface_diffuse: f32,
    has_prelit: bool,
}

fn unit_billboard_mesh_handle(
    cache: &mut GtaImmediateMeshCache,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    if let Some(handle) = cache.unit_billboard.as_ref()
        && meshes.get(handle).is_some()
    {
        return handle.clone();
    }
    let handle = meshes.add(build_unit_billboard_mesh());
    cache.unit_billboard = Some(handle.clone());
    handle
}

/// Persistent slots used by effects that are rebuilt from a per-frame intent
/// list.  The slot is hidden when its source intent disappears; it is never
/// handed to the generic immediate cleanup path.
#[derive(Debug, Clone, Copy, Component, Default)]
struct GtaPersistentImmediate;

type PersistentBillboardQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Mesh3d,
        &'static MeshMaterial3d<StandardMaterial>,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    With<GtaPersistentImmediate>,
>;

fn immediate_material_matches(left: &StandardMaterial, right: &StandardMaterial) -> bool {
    left.base_color == right.base_color
        && left.base_color_texture.as_ref().map(Handle::id)
            == right.base_color_texture.as_ref().map(Handle::id)
        && left.alpha_mode == right.alpha_mode
        && left.unlit == right.unlit
        && left.double_sided == right.double_sided
        && left.cull_mode == right.cull_mode
}

fn update_persistent_billboard_slot(
    entity: Option<Entity>,
    transform: Transform,
    material: StandardMaterial,
    mesh: &Handle<Mesh>,
    query: &mut PersistentBillboardQuery<'_, '_>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
) -> Entity {
    if let Some(entity) = entity
        && let Ok((_, material_handle, mut existing_transform, mut visibility)) =
            query.get_mut(entity)
    {
        if *existing_transform != transform {
            *existing_transform = transform;
        }
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
        if materials.get(&material_handle.0).is_some_and(|resident|
            !immediate_material_matches(resident, &material))
            && let Some(mut existing_material) = materials.get_mut(&material_handle.0)
        {
            *existing_material = material;
        }
        return entity;
    }

    commands
        .spawn((
            GtaImmediateMesh,
            GtaPersistentImmediate,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(materials.add(material)),
            transform,
            GlobalTransform::default(),
            Visibility::Visible,
        ))
        .id()
}

fn update_persistent_mesh_slot(
    entity: Option<Entity>,
    mesh: Mesh,
    material: StandardMaterial,
    transform: Transform,
    query: &mut PersistentBillboardQuery<'_, '_>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
) -> Entity {
    if let Some(entity) = entity
        && let Ok((mesh_handle, material_handle, mut existing_transform, mut visibility)) =
            query.get_mut(entity)
    {
        // The immediate slot is persistent, but its batch shape can change on
        // every frame. Skip identical bytes before mutable asset access;
        // changed vertices still require Bevy GPU extraction/allocation even
        // with a stable AssetId. A changed shape gets a replacement handle.
        if !reuse_persistent_mesh_shape(meshes, &mesh_handle.0, &mesh) {
            let replacement = meshes.add(mesh);
            meshes.remove(mesh_handle.0.id());
            commands.entity(entity).insert(Mesh3d(replacement));
        }
        if materials.get(&material_handle.0).is_some_and(|resident|
            !immediate_material_matches(resident, &material))
            && let Some(mut existing_material) = materials.get_mut(&material_handle.0)
        {
            *existing_material = material;
        }
        if *existing_transform != transform {
            *existing_transform = transform;
        }
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
        return entity;
    }

    commands
        .spawn((
            GtaImmediateMesh,
            GtaPersistentImmediate,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            transform,
            GlobalTransform::default(),
            Visibility::Visible,
        ))
        .id()
}

/// Rewrite `replacement` into the resident Mesh asset when the shape matches.
/// Returns false (caller must add/remove) when topology, vertex count or
/// index count differ. Attribute sets are stable per call site: every
/// attribute the new mesh carries overwrites the resident one.
fn reuse_persistent_mesh_shape(
    meshes: &mut Assets<Mesh>,
    handle: &Handle<Mesh>,
    replacement: &Mesh,
) -> bool {
    fn index_len(mesh: &Mesh) -> Option<usize> {
        mesh.indices().map(|indices| match indices {
            bevy::render::mesh::Indices::U16(values) => values.len(),
            bevy::render::mesh::Indices::U32(values) => values.len(),
        })
    }
    let Some(resident) = meshes.get(handle) else {
        return false;
    };
    if resident.primitive_topology() != replacement.primitive_topology()
        || resident.count_vertices() != replacement.count_vertices()
        || index_len(&resident) != index_len(replacement)
    {
        return false;
    }
    let indices_changed = resident.indices() != replacement.indices();
    let attributes_changed = replacement
        .attributes()
        .any(|(id, values)| resident.attribute(id.clone()) != Some(values));
    if !indices_changed && !attributes_changed {
        // Dereferencing AssetMut for a write marks the asset as changed.
        // Identical intent bytes need no write or GPU extraction/allocation.
        return true;
    }
    let Some(mut resident) = meshes.get_mut(handle) else {
        return false;
    };
    if indices_changed {
        match replacement.indices() {
            Some(indices) => resident.insert_indices(indices.clone()),
            None => {
                resident.remove_indices();
            }
        }
    }
    for (id, values) in replacement.attributes() {
        if resident.attribute(id.clone()) != Some(values) {
            resident.insert_attribute(id.clone(), values.clone());
        }
    }
    true
}

fn hide_persistent_billboard_slots(
    entities: &[Entity],
    first_hidden: usize,
    query: &mut PersistentBillboardQuery<'_, '_>,
) {
    for entity in entities.iter().skip(first_hidden).copied() {
        if let Ok((_, _, _, mut visibility)) = query.get_mut(entity)
            && *visibility != Visibility::Hidden
        {
            *visibility = Visibility::Hidden;
        }
    }
}

fn clear_immediate_meshes(
    mut commands: Commands,
    existing: Query<
        (Entity, &Mesh3d, Option<&MeshMaterial3d<StandardMaterial>>),
        (
            With<GtaImmediateMesh>,
            Without<GtaRuntimePoolMesh>,
            Without<GtaParticleBillboard>,
            Without<GtaPersistentImmediate>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cache: Res<GtaImmediateMeshCache>,
) {
    // Only ephemeral overlays (particles, coronas, sprites, lines). World meshes
    // use GtaWorldMeshRegistry and are NOT tagged GtaImmediateMesh.
    for (entity, mesh, material) in &existing {
        if cache
            .unit_billboard
            .as_ref()
            .is_none_or(|shared| shared.id() != mesh.0.id())
        {
            meshes.remove(mesh.0.id());
        }
        if let Some(material) = material {
            materials.remove(material.0.id());
        }
        commands.entity(entity).despawn();
    }
}

fn clear_point_lights(
    mut commands: Commands,
    existing: Query<Entity, (With<GtaPointLight>, Without<GtaPersistentPointLight>)>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
}

fn update_coronas_lifecycle(
    coronas: Option<ResMut<CCoronas>>,
    timer: Option<Res<CTimer>>,
) {
    if let Some(mut coronas) = coronas {
        coronas.update_with_time_step(timer.as_deref().map_or(1.0, CTimer::time_step));
    }
}

fn sync_renderware_frame_state(
    timecycle: Option<Res<CTimeCycle>>,
    camera: Option<Res<CCamera>>,
    postfx: Option<Res<CPostFX>>,
    point_lights: Option<Res<CPointLights>>,
    coronas: Option<Res<CCoronas>>,
    weather: Option<Res<CWeather>>,
    menu: Option<Res<CMenuManager>>,
    mut clear_color: ResMut<ClearColor>,
    mut frame_state: ResMut<GtaRenderFrameState>,
    mut lighting_buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(timecycle) = timecycle else {
        return;
    };
    let current = timecycle.current;
    frame_state.sky_top = rgb_i32_to_u8(current.sky_top);
    frame_state.sky_bottom = rgb_i32_to_u8(current.sky_bottom);
    frame_state.far_clip = current.far_clip;
    frame_state.fog_start = current.fog_start;
    let sun = timecycle.vector_to_sun[timecycle.current_stored_value.rem_euclid(16) as usize]
        .to_bevy()
        .normalize_or_zero();
    let sun = if sun.length_squared() > 1.0e-6 {
        sun
    } else {
        Vec3::Z
    };
    let postfx_colour = postfx
        .as_deref()
        .zip(camera.as_deref())
        .and_then(|(postfx, camera)| postfx.effective_colour_filter(camera))
        .map(Vec4::from_array)
        .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 0.0));
    // re3 Lights.cpp::SetLightsWithTimeOfDayColour: local coronas can dim
    // ambient/directional globally, lightning overrides ambient, and the
    // display brightness preference uses distinct ambient/direct factors.
    let lights_mult = coronas.as_deref().map_or(1.0, |c| c.lights_mult);
    let mut ambient = Vec3::from_array(current.ambient) * lights_mult;
    let mut directional = Vec3::from_array(current.directional) * lights_mult;
    if weather.as_deref().is_some_and(|w| w.lightning_flash) {
        ambient = Vec3::ONE;
    }
    // Lights.cpp derives the actor colour before applying the distinct
    // display-brightness factors. Multiplying the world colour by 1.3 in
    // the shader loses both this clamp and the actor/world distinction.
    let mut actor_ambient = (ambient * 1.3).min(Vec3::ONE);
    let brightness = menu.as_deref().map_or(256, |m| m.prefs_brightness);
    if brightness > 256 {
        let excess = brightness as f32 / 256.0 - 1.0;
        ambient = (ambient * (1.0 + 1.8 * excess)).min(Vec3::ONE);
        actor_ambient = (actor_ambient * (1.0 + 1.2 * excess)).min(Vec3::ONE);
        directional = (directional * (1.0 + 1.2 * excess)).min(Vec3::ONE);
    }
    let mut local_light_positions = [Vec4::ZERO; 32];
    let mut local_light_colours = [Vec4::ZERO; 32];
    let mut local_light_directions = [Vec4::ZERO; 32];
    let mut local_light_count = 0;
    if let Some(lights) = point_lights.as_deref() {
        for (index, light) in lights.lights.iter().take(32).enumerate() {
            local_light_positions[index] = light.coors.to_bevy().extend(light.radius);
            local_light_colours[index] = Vec4::new(
                light.red, light.green, light.blue, f32::from(light.light_type),
            );
            local_light_directions[index] = light.dir.to_bevy().extend(0.0);
            local_light_count += 1;
        }
    }
    if let Some(mut buffer) = lighting_buffers.get_mut(GTA_GLOBAL_LIGHTING_BUFFER.id()) {
        // RenderWare installs these lights once per frame. Keep fog in its
        // encoded D3D8 channel space; rw_unlit converts only the final RGB.
        buffer.clear();
        buffer.extend_from_slice(&[GtaGlobalLighting {
            fog_colour_and_start: Vec3::new(
                current.fog_colour[0].clamp(0, 255) as f32 / 255.0,
                current.fog_colour[1].clamp(0, 255) as f32 / 255.0,
                current.fog_colour[2].clamp(0, 255) as f32 / 255.0,
            )
            .extend(current.fog_start),
            ambient: ambient.extend(0.0),
            actor_ambient: actor_ambient.extend(0.0),
            directional: directional.extend(0.0),
            light_direction_and_far_clip: sun.extend(current.far_clip),
            postfx_blur_colour: postfx_colour,
            local_light_positions,
            local_light_colours,
            local_light_directions,
            local_light_count: Vec4::new(local_light_count as f32, 0.0, 0.0, 0.0),
        }]);
    }
    *clear_color = ClearColor(Color::srgb_u8(
        frame_state.sky_top[0],
        frame_state.sky_top[1],
        frame_state.sky_top[2],
    ));
}

fn sync_sky_horizon_mesh(
    mut commands: Commands,
    timecycle: Option<Res<CTimeCycle>>,
    gta_camera: Option<Res<CCamera>>,
    postfx: Option<Res<CPostFX>>,
    // re3 draws the sky background with the live view matrix the same frame
    // (Clouds::RenderBackground / CSprite::CalcHorizonCoors). Read Transform,
    // written by the Camera set earlier this frame; GlobalTransform is only
    // propagated in PostUpdate and would pin the sky to last frame's camera.
    // Without<> keeps this disjoint from the &mut Transform slot writer below.
    camera_query: Query<
        (&GtaSceneCamera, &Camera, &Transform, &Projection),
        (With<Camera3d>, Without<GtaSkyHorizonMesh>),
    >,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut existing: Query<(&MeshMaterial3d<GtaSkyMaterial>, &mut Transform), With<GtaSkyHorizonMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaSkyMaterial>>,
) {
    let Some(timecycle) = timecycle else {
        return;
    };
    let Some((_, camera, camera_transform, Projection::Perspective(perspective))) = camera_query
        .iter()
        .find(|(scene_camera, _, _, _)| scene_camera.local_player == 0)
        .or_else(|| camera_query.iter().next())
    else {
        return;
    };

    // re3 `CSprite::CalcHorizonCoors`: project a point 3000 units along the
    // horizontal camera heading after forcing GTA Z (Bevy Y) to zero.
    let camera_forward = *camera_transform.forward();
    let horizontal_forward = Vec3::new(camera_forward.x, 0.0, camera_forward.z).normalize_or_zero();
    let mut horizon_world = camera_transform.translation + horizontal_forward * 3000.0;
    horizon_world.y = 0.0;
    let clip_from_world = perspective.get_clip_from_view() * camera_transform.to_matrix().inverse();
    let clip = clip_from_world * horizon_world.extend(1.0);
    let horizon = if clip.w.abs() > f32::EPSILON {
        let ndc_y = clip.y / clip.w;
        // Keep off-screen values: RenderBackground branches on a horizon above
        // or below the raster rather than clamping it to the viewport edge.
        1.0 - (ndc_y * 0.5 + 0.5)
    } else {
        // The reference divides by the transformed Z here. Do not fabricate a
        // horizon if this adapter cannot project the same point reliably.
        return;
    };

    let mut params = SkyMaterialParams::from_timecycle(&timecycle.current);
    params.horizon = horizon;
    let viewport_height = camera
        .viewport
        .as_ref()
        .map(|viewport| viewport.physical_size.y as f32)
        .or_else(|| window_query.iter().next().map(Window::height))
        .unwrap_or(448.0)
        .max(1.0);
    let right_gta = CVector::from_bevy(*camera_transform.right());
    let up_gta = CVector::from_bevy(*camera_transform.up());
    let forward_gta = CVector::from_bevy(camera_forward);
    let camera_z = CVector::from_bevy(camera_transform.translation).z.max(0.0);
    let roll_extent = if up_gta.z < 0.0 {
        1.0
    } else {
        right_gta.z.abs()
    };
    let lod_multiplier = gta_camera
        .as_deref()
        .map_or(1.0, |camera| camera.lod_dist_multiplier.max(0.0));
    params.horizon_end = (horizon
        + 4.0 / viewport_height
        + (48.0 / 400.0 + camera_z / 300.0 + roll_extent) * lod_multiplier)
        .min(1.0);
    params.dark_background = forward_gta.z < -0.9;
    params.postfx_blur_colour = postfx
        .as_deref()
        .zip(gta_camera.as_deref())
        .and_then(|(postfx, camera)| postfx.effective_colour_filter(camera))
        .unwrap_or([1.0, 1.0, 1.0, 0.0]);
    let material = GtaSkyMaterial::from(params);

    // The previous adapter used one fixed world-space quad at Z=1200. Camera
    // translation/rotation exposed its edges and even its individual
    // triangles. Keep a camera-facing quad just inside the far plane so it is
    // always a full-raster background, which is what RenderBackground is.
    let distance = (perspective.far * 0.95).max(perspective.near + 1.0);
    let half_height = distance * (perspective.fov * 0.5).tan() * 1.01;
    let half_width = half_height * perspective.aspect_ratio * 1.01;
    let camera_rotation = camera_transform.rotation;
    let sky_transform = Transform {
        translation: camera_transform.translation + camera_forward * distance,
        rotation: camera_rotation,
        scale: Vec3::new(half_width, half_height, 1.0),
    };

    if let Some((handle, mut transform)) = existing.iter_mut().next() {
        if let Some(mut existing_material) = materials.get_mut(&handle.0) {
            *existing_material = material;
        }
        *transform = sky_transform;
        return;
    }

    commands.spawn((
        GtaSkyHorizonMesh,
        Mesh3d(meshes.add(build_sky_horizon_mesh())),
        MeshMaterial3d(materials.add(material)),
        sky_transform,
        GlobalTransform::default(),
        NoFrustumCulling,
    ));
}

pub fn consume_particle_intents(
    particles: Option<ResMut<CParticle>>,
    particle_mgr: Res<cParticleSystemMgr>,
    timer: Option<Res<CTimer>>,
    time: Option<Res<Time>>,
    mut general: Option<ResMut<CGeneral>>,
    file_loader: Option<Res<CFileLoader>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut images: ResMut<Assets<Image>>,
    mut spawn_sequence: ResMut<GtaParticleSpawnSequence>,
    // Only the world camera may define a billboard orientation.  The HUD
    // Camera2d is also active during the arena flow, but has an identity
    // rotation and would pin particles to world axes (the classic "paper"
    // symptom).
    camera_query: Query<
        (&Camera, &GlobalTransform),
        (With<Camera3d>, Without<GtaParticleBillboard>),
    >,
    mut commands: Commands,
    existing: Query<(Entity, &GtaParticleBillboard)>,
    // Per-frame memo of resolved particle raster lists. `texture_cache` is
    // only synced above, so resolving once per particle type per frame is
    // identical to resolving per spawn without the repeated lowercase +
    // full-cache linear scans on this hot path.
    mut frame_raster_memo: Local<HashMap<i32, Vec<Handle<Image>>>>,
) {
    let Some(mut particles) = particles else {
        return;
    };
    // CParticle::Initialise loads MODELS/PARTICLE.TXD before any AddParticle
    // can be rendered.  Keep that ordering in the ECS path; otherwise wheel
    // smoke intents are created with an empty raster list and are silently
    // culled by the WESL sprite pass.
    if let (Some(fl), Some(slot)) = (
        file_loader.as_deref(),
        file_loader
            .as_ref()
            .and_then(|loader| loader.txd_store.find_txd_slot("particle")),
    ) {
        let _ = sync_txd_slot_texture_cache(slot, fl, &mut texture_cache, &mut images);
    }
    frame_raster_memo.clear();
    // The particle dictionary slot cannot change while this system runs
    // (`file_loader` is shared here); resolve it once instead of hashing the
    // TXD name for every spawned particle.
    let particle_slot = file_loader
        .as_ref()
        .and_then(|loader| loader.txd_store.find_txd_slot("particle"));
    let now = if timer.as_ref().is_some_and(|t| t.get_is_code_paused()) {
        time.as_ref()
            .map(|t| (t.elapsed_secs() * 1000.0) as u32)
            .unwrap_or(0)
    } else {
        timer
            .as_ref()
            .map(|t| t.time_in_milliseconds())
            .unwrap_or(0)
    };
    let paused = timer
        .as_ref()
        .map(|timer| timer.get_is_user_paused())
        .unwrap_or(false);
    let (add_intents, remove_intents) = particles.drain_frame_intents();
    let mut camera_positions = camera_query
        .iter()
        .filter(|(camera, _)| camera.is_active)
        .map(|(_, camera)| CVector::from_bevy(camera.translation()))
        .collect::<Vec<_>>();
    if camera_positions.is_empty() {
        camera_positions = camera_query
            .iter()
            .map(|(_, camera)| CVector::from_bevy(camera.translation()))
            .collect::<Vec<_>>();
    }

    for particle_type in remove_intents {
        for (entity, billboard) in &existing {
            if billboard.particle_type == particle_type {
                particles.retire_live_particle(entity);
                if let Some(tracking_id) = billboard.tracking_id {
                    particles.remove_tracked_particle(tracking_id);
                }
                commands.entity(entity).despawn();
            }
        }
    }

    if paused {
        return;
    }

    for intent in add_intents {
        if particles.live_particle_count() >= super::Particle::MAX_PARTICLES_ON_SCREEN {
            if let Some(id) = intent.tracking_id { particles.remove_tracked_particle(id); }
            continue;
        }
        let psystem = particle_mgr.particle(intent.particle_type);
        // TODO(parity): CParticle::AddParticle performs this check at request
        // admission against TheCamera, before slot/ID reservation. Render-camera
        // selection here is a remaining interface divergence, including multiple
        // or inactive cameras; it does not validate the tracking return value.
        if !intent.range_admitted && psystem.is_some_and(|entry| {
            entry.create_range != 0.0
                && !camera_positions.iter().any(|camera| {
                    let delta = CVector::new(
                        intent.position.x - camera.x,
                        intent.position.y - camera.y,
                        intent.position.z - camera.z,
                    );
                    delta.magnitude_sqr() <= entry.create_range
                })
        }) {
            if let Some(tracking_id) = intent.tracking_id {
                particles.remove_tracked_particle(tracking_id);
            }
            continue;
        }
        let flags = psystem.map(|entry| entry.flags).unwrap_or(0);
        let mut position = intent.position;
        let mut velocity = intent.direction;
        let mut base_size = psystem
            .map(|entry| entry.default_initial_radius)
            .filter(|size| *size != 0.0)
            .unwrap_or(0.2);
        let mut expansion_rate = psystem.map(|entry| entry.expansion_rate).unwrap_or(0.0);
        let mut lifespan = if intent.lifespan != 0 {
            intent.lifespan.max(1)
        } else {
            psystem.map(|entry| entry.life_span.max(1)).unwrap_or(1000)
        };
        let mut colour =
            particle_colour(intent.colour, psystem.map(|entry| entry.render_colouring));
        let mut rotation_step = if intent.rotation_speed != 0 {
            intent.rotation_speed
        } else {
            psystem
                .map(|entry| i32::from(entry.rotation_speed))
                .unwrap_or(0)
        };
        let mut animation_timer = 0_u16;

        if let Some(entry) = psystem {
            let rand_table = particles.rand_table;
            let mut next_rand = || {
                let index = general
                    .as_deref_mut()
                    .map(|general| usize::from(general.get_random_number()) % rand_table.len())
                    .unwrap_or(0);
                rand_table[index]
            };
            if entry.position_random_error != 0.0 {
                position.x += entry.position_random_error * next_rand();
                position.y += entry.position_random_error * next_rand();
                if flags & RAND_VERT_V != 0 {
                    position.z += entry.position_random_error * next_rand();
                }
            }
            if entry.velocity_random_error != 0.0 {
                velocity.x += entry.velocity_random_error * next_rand();
                velocity.y += entry.velocity_random_error * next_rand();
                if flags & RAND_VERT_V != 0 {
                    velocity.z += entry.velocity_random_error * next_rand();
                }
            }
            if entry.expansion_rate_error != 0.0 {
                expansion_rate +=
                    entry.expansion_rate_error * next_rand() + entry.expansion_rate_error;
            }
            drop(next_rand);

            if entry.rotation_rate_error != 0 {
                rotation_step += general.as_deref_mut().map_or(0, |general| {
                    general.get_random_number_in_range_i32(
                        -entry.rotation_rate_error,
                        entry.rotation_rate_error,
                    )
                });
            }
            if general
                .as_deref_mut()
                .is_some_and(|general| general.get_random_number() & 1 != 0)
            {
                rotation_step = -rotation_step;
            }
            if intent.lifespan == 0 && entry.life_span_error_shape != 0 {
                let rand = {
                    let index = general
                        .as_deref_mut()
                        .map(|general| {
                            usize::from(general.get_random_number()) % particles.rand_table.len()
                        })
                        .unwrap_or(0);
                    particles.rand_table[index]
                };
                let shape = entry.life_span_error_shape as f32;
                let delta = if rand > 0.0 {
                    entry.life_span as f32 * rand * shape
                } else {
                    entry.life_span as f32 * rand / shape
                };
                lifespan = lifespan.saturating_add(delta as i32).max(1);
            }
            if intent.colour.alpha == 0 && entry.initial_color_variation != 0 {
                let variation = general.as_deref_mut().map_or(0, |general| {
                    general.get_random_number_in_range_i32(
                        -i32::from(entry.initial_color_variation),
                        i32::from(entry.initial_color_variation),
                    )
                });
                for channel in &mut colour[..3] {
                    let delta = i32::from(*channel) * variation / 100;
                    *channel = (i32::from(*channel) + delta).clamp(0, 255) as u8;
                }
            }
            match intent.particle_type {
                PARTICLE_EXPLOSION_MEDIUM => {
                    if general
                        .as_deref_mut()
                        .is_some_and(|general| general.get_random_number() & 1 != 0)
                    {
                        colour[0] = colour[0].saturating_sub(30);
                        colour[1] = colour[1].saturating_sub(30);
                        colour[2] = colour[2].saturating_sub(30);
                    }
                    animation_timer = general
                        .as_deref_mut()
                        .map_or(0, |general| general.get_random_number() & 7);
                    base_size = general.as_deref_mut().map_or(0.3, |general| {
                        general.get_random_number_in_range_f32(0.3, 0.8)
                    });
                    position.z -= general.as_deref_mut().map_or(0.0, |general| {
                        general.get_random_number_in_range_f32(-0.1, 0.1)
                    });
                }
                PARTICLE_EXPLOSION_LARGE => {
                    if general
                        .as_deref_mut()
                        .is_some_and(|general| general.get_random_number() & 1 != 0)
                    {
                        colour[0] = colour[0].saturating_sub(30);
                        colour[1] = colour[1].saturating_sub(30);
                        colour[2] = colour[2].saturating_sub(30);
                    }
                    animation_timer = general
                        .as_deref_mut()
                        .map_or(0, |general| general.get_random_number() & 7);
                    base_size = general.as_deref_mut().map_or(0.8, |general| {
                        general.get_random_number_in_range_f32(0.8, 1.4)
                    });
                    position.z -= general.as_deref_mut().map_or(0.0, |general| {
                        general.get_random_number_in_range_f32(-0.3, 0.3)
                    });
                }
                PARTICLE_WATER_HYDRANT => {
                    let rand = {
                        let index = general
                            .as_deref_mut()
                            .map(|general| {
                                usize::from(general.get_random_number())
                                    % particles.rand_table.len()
                            })
                            .unwrap_or(0);
                        particles.rand_table[index]
                    };
                    position.z += 20.0 * entry.position_random_error * rand;
                }
                _ => {}
            }
        }
        if intent.size != 0.0 {
            base_size = intent.size;
        }
        let lifespan = lifespan.max(1) as u32;
        let ground_z = if flags & ZCHECK_FIRST != 0 {
            let hit = file_loader.as_ref().and_then(|loader| {
                loader.world.process_vertical_line_static(
                    [position.x, position.y, position.z + 0.5],
                    -100.0,
                    &loader.model_info,
                )
            });
            let Some(hit) = hit else {
                continue;
            };
            if hit.point.z >= position.z {
                continue;
            }
            Some(hit.point.z)
        } else if flags & ZCHECK_BUMP != 0 && flags & ZCHECK_BUMP_FIRST != 0 {
            let Some(ground) = file_loader.as_ref().and_then(|loader| {
                loader.world.find_ground_z_for_3d_coord(
                    position.x,
                    position.y,
                    position.z,
                    &loader.model_info,
                )
            }) else {
                continue;
            };
            Some(ground)
        } else {
            None
        };
        let alpha = psystem
            .map(|entry| entry.fade_alpha_initial_intensity)
            .filter(|alpha| *alpha != 0)
            .unwrap_or(colour[3]);
        let intensity = psystem
            .map(|entry| entry.fade_to_black_initial_intensity)
            .filter(|intensity| *intensity != 0)
            .unwrap_or(255);
        // re3 particle raster arrays. Animated systems switch the bound raster
        // using `m_nCurrentFrame`; pinning every system to frame zero hides
        // most of a gun flash/explosion sequence.
        let animation_frames = if let Some(cached) = frame_raster_memo.get(&intent.particle_type) {
            cached.clone()
        } else {
            let resolved: Vec<Handle<Image>> = particle_raster_names(intent.particle_type)
                .iter()
                .filter_map(|name| {
                    let name_lower = name.to_ascii_lowercase();
                    if let Some(slot) = particle_slot {
                        if let Some(handle) = texture_cache.images.get(&(slot, name_lower.clone()))
                        {
                            return Some(handle.clone());
                        }
                    }
                    warn_once!("Particle raster '{}' missing from particle TXD", name_lower);
                    None
                })
                .collect::<Vec<_>>();
            frame_raster_memo.insert(intent.particle_type, resolved.clone());
            resolved
        };
        // re3 `CParticle::Render` leaves the current RW raster untouched when
        // a system has no raster entry (notably GUNSMOKE/SPLASH).  The sprite
        // pass preserves that state across the ordered particle list; it is
        // consumed by `build_particle_sprite_frame` below.  Do not invent a
        // white texture, and do not drop the particle object—the previous
        // raster is part of the original renderer's observable behaviour.
        let start_frame = psystem
            .map(|entry| usize::from(entry.start_animation_frame))
            .unwrap_or(0);
        let final_frame = psystem
            .map(|entry| usize::from(entry.final_animation_frame))
            .unwrap_or(0)
            .min(animation_frames.len().saturating_sub(1));
        let current_frame = if intent.current_frame != 0 {
            intent.current_frame.max(0) as usize
        } else {
            start_frame
        }
        .min(animation_frames.len().saturating_sub(1));
        let sequence = spawn_sequence.0;
        spawn_sequence.0 = spawn_sequence.0.wrapping_add(1);
        // `DRAWTOP2D` positions/velocities are already in device pixels in
        // re3 (`Weather.cpp::AddRain`).  Do not pass them through the GTA
        // world-axis conversion (x,z,-y), otherwise every drop is pinned to
        // the top edge and its vertical motion is lost.  World particles keep
        // the ordinary CVector -> Bevy conversion.
        let (render_position, render_velocity) = if flags & DRAWTOP2D != 0 {
            (
                Vec3::new(position.x, position.y, 0.0),
                Vec3::new(velocity.x, velocity.y, 0.0),
            )
        } else {
            (position.to_bevy(), velocity.to_bevy())
        };
        if let Some(tracking_id) = intent.tracking_id {
            particles.set_tracked_particle_position(tracking_id, position);
        }
        let particle_entity = commands.spawn((
            GtaParticleBillboard {
                particle_type: intent.particle_type,
                tracking_id: intent.tracking_id,
                spawn_sequence: sequence,
                screen_position: Vec2::ZERO,
                velocity: render_velocity,
                spawn_time_ms: now,
                death_time_ms: now.wrapping_add(lifespan),
                size: base_size,
                expansion_rate,
                gravitational_acceleration: psystem
                    .map(|entry| entry.gravitational_acceleration)
                    .unwrap_or(0.0),
                friction_deceleration: psystem
                    .map(|entry| entry.friction_decceleration)
                    .unwrap_or(0),
                rotation_degrees: normalised_degrees(intent.rotation as f32),
                rotation_step: rotation_step as f32,
                colour,
                fade_alpha_initial_intensity: psystem
                    .map(|entry| entry.fade_alpha_initial_intensity)
                    .unwrap_or(0),
                fade_alpha_time: psystem.map(|entry| entry.fade_alpha_time).unwrap_or(0),
                fade_alpha_amount: psystem.map(|entry| entry.fade_alpha_amount).unwrap_or(0),
                alpha,
                fade_alpha_timer: 0,
                fade_to_black_initial_intensity: psystem
                    .map(|entry| entry.fade_to_black_initial_intensity)
                    .unwrap_or(0),
                fade_to_black_time: psystem.map(|entry| entry.fade_to_black_time).unwrap_or(0),
                fade_to_black_amount: psystem.map(|entry| entry.fade_to_black_amount).unwrap_or(0),
                intensity,
                fade_to_black_timer: 0,
                current_z_rotation: psystem
                    .map(|entry| entry.z_rotation_initial_angle)
                    .unwrap_or(0),
                z_rotation_change_time: psystem
                    .map(|entry| entry.z_rotation_change_time)
                    .unwrap_or(0),
                z_rotation_change_amount: psystem
                    .map(|entry| entry.z_rotation_angle_change_amount)
                    .unwrap_or(0),
                z_rotation_timer: 0,
                current_z_radius: psystem.map(|entry| entry.initial_z_radius).unwrap_or(0.0),
                z_radius_change_time: psystem.map(|entry| entry.z_radius_change_time).unwrap_or(0),
                z_radius_change_amount: psystem
                    .map(|entry| entry.z_radius_change_amount)
                    .unwrap_or(0.0),
                z_radius_timer: 0,
                particle_movement_offset: Vec3::ZERO,
                ground_z,
                flags,
                animation_frames,
                current_frame,
                start_frame,
                final_frame,
                animation_speed: psystem.map(|entry| entry.animation_speed).unwrap_or(0),
                animation_timer,
                base_colour: colour,
                fade_destination_colour: psystem
                    .map(|entry| {
                        [
                            entry.fade_destination_color.red,
                            entry.fade_destination_color.green,
                            entry.fade_destination_color.blue,
                            entry.fade_destination_color.alpha,
                        ]
                    })
                    .unwrap_or(colour),
                colour_fade_time: if intent.colour.alpha == 0 {
                    psystem.map(|entry| entry.color_fade_time).unwrap_or(0)
                } else {
                    0
                },
            },
            Transform::from_translation(render_position),
            GlobalTransform::default(),
        )).id();
        particles.register_live_particle(particle_entity, intent.tracking_id);
    }
}

pub fn update_particle_billboards(
    timer: Option<Res<CTimer>>,
    time: Option<Res<Time>>,
    file_loader: Option<Res<CFileLoader>>,
    mut commands: Commands,
    mut particle_runtime: ResMut<CParticle>,
    mut particles: Query<(Entity, &mut Transform, &mut GtaParticleBillboard)>,
) {
    let (now, time_step) = if timer.as_ref().is_some_and(|t| t.get_is_code_paused()) {
        let now = time
            .as_ref()
            .map(|t| (t.elapsed_secs() * 1000.0) as u32)
            .unwrap_or(0);
        let dt = time
            .as_ref()
            .map(|t| t.delta_secs().min(0.05))
            .unwrap_or(0.02);
        (now, dt * 50.0)
    } else {
        let now = timer
            .as_ref()
            .map(|t| t.time_in_milliseconds())
            .unwrap_or(0);
        let step = timer.as_ref().map(|t| t.time_step()).unwrap_or(1.0);
        (now, step)
    };

    for (entity, mut transform, mut particle) in &mut particles {
        if now.wrapping_sub(particle.death_time_ms) < u32::MAX / 2 {
            if let Some(tracking_id) = particle.tracking_id {
                particle_runtime.remove_tracked_particle(tracking_id);
            }
            particle_runtime.retire_live_particle(entity);
            commands.entity(entity).despawn();
            continue;
        }
        // re3 particles store velocity in GTA units per normalised 50 Hz
        // timestep. Expansion and rotation are authored as per-update deltas
        // and are deliberately not multiplied by seconds.
        transform.translation += particle.velocity * time_step;
        particle.size = (particle.size + particle.expansion_rate).max(0.001);
        particle.rotation_degrees =
            normalised_degrees(particle.rotation_degrees + particle.rotation_step);
        if let Some(friction) = match particle.friction_deceleration {
            50 => Some(0.50_f32),
            80 => Some(0.80_f32),
            90 => Some(0.90_f32),
            95 => Some(0.95_f32),
            96 => Some(0.96_f32),
            99 => Some(0.99_f32),
            _ => None,
        } {
            particle.velocity *= friction.powf(time_step);
        }
        if particle.gravitational_acceleration > 0.0
            && particle.velocity.y > -50.0 * particle.gravitational_acceleration
        {
            // GTA Z is Bevy Y.
            particle.velocity.y -= particle.gravitational_acceleration * time_step;
        } else if particle.gravitational_acceleration < 0.0
            && particle.velocity.y < -5.0 * particle.gravitational_acceleration
        {
            particle.velocity.y -= particle.gravitational_acceleration * time_step;
        }
        // re3 `CParticle::Update` ZCHECK_STEP branch (Particle.cpp): when the
        // frame's downward step would cross the world surface, clamp to it
        // instead of tunneling below or hovering on a stale spawn ground_z.
        // Only DEBRIS2 bounces (damped, inverted vertical velocity); other
        // systems keep integrating and re-clamp every tick, like C++.
        if particle.flags & ZCHECK_STEP != 0
            && particle.gravitational_acceleration > 0.0
            && particle.velocity.y < 0.0
        {
            if let Some(loader) = file_loader.as_ref() {
                // GTA Z is Bevy Y; GTA Y is -Bevy Z.
                let gta_pos = [
                    transform.translation.x,
                    -transform.translation.z,
                    transform.translation.y,
                ];
                let step_z = gta_pos[2] + particle.velocity.y * time_step;
                if step_z < gta_pos[2] {
                    if let Some(hit) = loader.world.process_vertical_line_static(
                        gta_pos,
                        step_z,
                        &loader.model_info,
                    ) {
                        if hit.point.z >= step_z {
                            transform.translation.y = hit.point.z;
                            particle.ground_z = Some(hit.point.z);
                            if particle.particle_type == PARTICLE_DEBRIS2 {
                                particle.velocity.x *= 0.8;
                                particle.velocity.z *= 0.8;
                                particle.velocity.y *= -0.4;
                                if particle.velocity.y < 0.005 {
                                    particle.velocity.y = 0.0;
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(ground_z) = particle.ground_z
            && transform.translation.y < ground_z
            && particle.flags & (ZCHECK_FIRST | ZCHECK_BUMP) != 0
        {
            if let Some(tracking_id) = particle.tracking_id {
                particle_runtime.remove_tracked_particle(tracking_id);
            }
            particle_runtime.retire_live_particle(entity);
            commands.entity(entity).despawn();
            continue;
        }
        if particle.fade_to_black_amount != 0 {
            if particle.fade_to_black_timer >= particle.fade_to_black_time {
                particle.fade_to_black_timer = 0;
                particle.intensity = (i32::from(particle.intensity)
                    - i32::from(particle.fade_to_black_amount))
                .clamp(0, 255) as u8;
            } else {
                particle.fade_to_black_timer = particle.fade_to_black_timer.saturating_add(1);
            }
        }
        if particle.fade_alpha_amount != 0 {
            if particle.fade_alpha_timer >= particle.fade_alpha_time {
                particle.fade_alpha_timer = 0;
                particle.alpha = (i32::from(particle.alpha) - i32::from(particle.fade_alpha_amount))
                    .clamp(0, 255) as u8;
                if particle.alpha == 0 {
                    if let Some(tracking_id) = particle.tracking_id {
                        particle_runtime.remove_tracked_particle(tracking_id);
                    }
                    particle_runtime.retire_live_particle(entity);
            commands.entity(entity).despawn();
                    continue;
                }
            } else {
                particle.fade_alpha_timer = particle.fade_alpha_timer.saturating_add(1);
            }
        }
        if particle.z_rotation_change_amount != 0 {
            if particle.z_rotation_timer >= particle.z_rotation_change_time {
                particle.z_rotation_timer = 0;
                particle.current_z_rotation = i32::from(particle.current_z_rotation)
                    .wrapping_add(i32::from(particle.z_rotation_change_amount))
                    as u16;
            } else {
                particle.z_rotation_timer = particle.z_rotation_timer.saturating_add(1);
            }
        }
        if particle.z_radius_change_amount != 0.0 {
            if particle.z_radius_timer >= particle.z_radius_change_time {
                particle.z_radius_timer = 0;
                particle.current_z_radius += particle.z_radius_change_amount;
            } else {
                particle.z_radius_timer = particle.z_radius_timer.saturating_add(1);
            }
        }
        if particle.current_z_radius != 0.0 {
            let index = usize::from(particle.current_z_rotation) % (1024 - 1);
            let angle = index as f32 * std::f32::consts::TAU / 1024.0;
            let sin = angle.sin();
            let cos = angle.cos();
            let offset = CVector::new(
                (cos - sin) * particle.current_z_radius,
                (sin + cos) * particle.current_z_radius,
                0.0,
            )
            .to_bevy();
            transform.translation -= particle.particle_movement_offset;
            transform.translation += offset;
            particle.particle_movement_offset = offset;
        }
        // Orientation, animation-frame texture binding and colour/intensity
        // modulation are applied by the screen-space sprite pass
        // (`ParticleSprite::build_particle_sprite_frame`), matching
        // CParticle::Render's per-draw state.
        if particle.animation_speed != 0 && !particle.animation_frames.is_empty() {
            if particle.animation_timer > particle.animation_speed {
                particle.animation_timer = 0;
                particle.current_frame = particle.current_frame.saturating_add(1);
                if particle.current_frame > particle.final_frame {
                    if particle.flags & CYCLE_ANIM != 0 {
                        particle.current_frame = particle.start_frame;
                    } else {
                        particle.current_frame = particle.final_frame;
                    }
                }
            } else {
                particle.animation_timer = particle.animation_timer.saturating_add(1);
            }
        }
        if particle.colour_fade_time != 0 {
            let age_ms = now.wrapping_sub(particle.spawn_time_ms);
            let t = (age_ms as f32 / particle.colour_fade_time as f32).clamp(0.0, 1.0);
            for channel in 0..3 {
                particle.colour[channel] = (f32::from(particle.base_colour[channel])
                    + f32::from(
                        particle.fade_destination_colour[channel] as i16
                            - particle.base_colour[channel] as i16,
                    ) * t)
                    .clamp(0.0, 255.0) as u8;
            }
        }
        if let Some(tracking_id) = particle.tracking_id {
            particle_runtime.set_tracked_particle_position(
                tracking_id,
                CVector::from_bevy(transform.translation),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_explosion_and_vehicle_lights(
    timer: Option<Res<CTimer>>,
    clock: Option<Res<CClock>>,
    weather: Option<Res<super::Weather::CWeather>>,
    camera: Option<Res<CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    handling_manager: Option<Res<cHandlingDataMgr>>,
    mut general: Option<ResMut<CGeneral>>,
    mut explosions: Option<ResMut<CExplosionPool>>,
    mut particles: Option<ResMut<CParticle>>,
    particle_systems: Option<Res<cParticleSystemMgr>>,
    mut coronas: Option<ResMut<CCoronas>>,
    mut point_lights: Option<ResMut<CPointLights>>,
    mut shadows: Option<ResMut<CShadows>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    mut light_actors: super::VehicleLightingAccess::VehicleLightingAccess,
    mut special_fx: Option<ResMut<CSpecialFX>>,
) {
    let now_ms = timer
        .as_ref()
        .map_or(0, |timer| timer.time_in_milliseconds());
    let frame_counter = timer.as_ref().map_or(0, |timer| timer.frame_counter());
    let camera_pos = camera
        .as_ref()
        .map_or(CVector::ZERO, |camera| camera.position);
    let lod_dist = camera
        .as_ref()
        .map_or(1.0, |camera| camera.lod_dist_multiplier.max(0.01));
    let mut particle_emitter = particles.as_deref_mut().map(|particles| {
        particles.emitter_with_context(
            particle_systems.as_deref(), camera.as_ref().map(|camera| camera.position))
    });

    // CExplosion::Update + the visual side of AddExplosion. The gameplay pool
    // already owns blast damage/audio; this restores the original particle,
    // corona and point-light production that was previously absent.
    if let Some(explosions) = explosions.as_deref_mut() {
        let gray = ParticleColour {
            red: 160,
            green: 160,
            blue: 160,
            alpha: 255,
        };
        for (slot_index, slot) in explosions.explosions.iter_mut().enumerate() {
            let Some(explosion) = slot.as_mut() else {
                continue;
            };

            if !explosion.visual_started {
                explosion.visual_started = true;
                let camera_distance = vec3_distance(explosion.position, camera_pos);
                if let Some(particles) = particle_emitter.as_mut() {
                    match explosion.explosion_type {
                        eExplosionType::EXPLOSION_GRENADE | eExplosionType::EXPLOSION_ROCKET => {
                            if camera_distance < 40.0 {
                                particles.add_particle_coloured(
                                    PARTICLE_EXPLOSION_LFAST,
                                    explosion.position,
                                    CVector::ZERO,
                                    None,
                                    5.5,
                                    gray,
                                    0,
                                    0,
                                    0,
                                    0,
                                );
                            }
                        }
                        eExplosionType::EXPLOSION_CAR | eExplosionType::EXPLOSION_CAR_QUICK => {
                            if let Some(general) = general.as_deref_mut() {
                                let count = (general.get_random_number() & 1) + 2;
                                for _ in 0..count {
                                    particles.add_particle_coloured(
                                        PARTICLE_EXPLOSION_MEDIUM,
                                        explosion.position,
                                        CVector::ZERO,
                                        None,
                                        3.5,
                                        ParticleColour::default(),
                                        0,
                                        0,
                                        0,
                                        0,
                                    );
                                    particles.add_particle_coloured(
                                        PARTICLE_EXPLOSION_LFAST,
                                        explosion.position,
                                        CVector::ZERO,
                                        None,
                                        5.5,
                                        gray,
                                        0,
                                        0,
                                        0,
                                        0,
                                    );
                                }
                                particles.add_jet_explosion(explosion.position, 1.4, 0.0, general);
                            }
                        }
                        eExplosionType::EXPLOSION_HELI => {
                            if let Some(general) = general.as_deref_mut() {
                                for _ in 0..10 {
                                    let random_pos = |general: &mut CGeneral| {
                                        CVector::new(
                                            explosion.position.x
                                                + (i32::from(general.get_random_number() as u8)
                                                    - 128)
                                                    as f32
                                                    / 20.0,
                                            explosion.position.y
                                                + (i32::from(general.get_random_number() as u8)
                                                    - 128)
                                                    as f32
                                                    / 20.0,
                                            explosion.position.z
                                                + (i32::from(general.get_random_number() as u8)
                                                    - 128)
                                                    as f32
                                                    / 20.0,
                                        )
                                    };
                                    let p = random_pos(general);
                                    particles.add_particle_coloured(
                                        PARTICLE_EXPLOSION_MFAST,
                                        p,
                                        CVector::ZERO,
                                        None,
                                        2.5,
                                        gray,
                                        0,
                                        0,
                                        0,
                                        0,
                                    );
                                    let p = random_pos(general);
                                    particles.add_particle_coloured(
                                        PARTICLE_EXPLOSION_LFAST,
                                        p,
                                        CVector::ZERO,
                                        None,
                                        5.0,
                                        gray,
                                        0,
                                        0,
                                        0,
                                        0,
                                    );
                                    let p = random_pos(general);
                                    particles.add_jet_explosion(p, 1.4, 3.0, general);
                                }
                            }
                        }
                        eExplosionType::EXPLOSION_BARREL => {
                            if let Some(general) = general.as_deref_mut() {
                                for _ in 0..6 {
                                    let x =
                                        (i32::from(general.get_random_number() as u8) - 128) as f32;
                                    let y =
                                        (i32::from(general.get_random_number() as u8) - 128) as f32;
                                    let z =
                                        (i32::from(general.get_random_number() as u8) - 128) as f32;
                                    particles.add_particle_coloured(
                                        PARTICLE_EXPLOSION_MEDIUM,
                                        CVector::new(
                                            explosion.position.x + x / 50.0,
                                            explosion.position.y + y / 50.0,
                                            explosion.position.z + z / 25.0,
                                        ),
                                        CVector::ZERO,
                                        None,
                                        0.0,
                                        ParticleColour::default(),
                                        0,
                                        0,
                                        0,
                                        0,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(shadows) = shadows.as_deref_mut() {
                    shadows.static_shadows.push(StaticShadowIntent {
                        key: 0xE100_0000u32.wrapping_add(slot_index as u32),
                        texture: 2, // gpShadowHeliTex in AddExplosion
                        position: CVector::new(
                            explosion.position.x,
                            explosion.position.y,
                            explosion.position.z + 5.0,
                        ),
                        front: [8.0, 0.0],
                        side: [0.0, -8.0],
                        intensity: 200,
                        color: [0, 0, 0],
                    });
                }
            }

            // CExplosionPool::Update owns gameplay lifetime/radius/iteration.
            // Render prep only consumes the already-advanced explosion state.
            if explosion.particles_expire_time != 0 {
                continue;
            }

            let time_left = explosion.stop_time - now_ms as f32;
            let id = 0xE000_0000u32.wrapping_add((slot_index as u32) * 4);
            match explosion.explosion_type {
                eExplosionType::EXPLOSION_GRENADE
                | eExplosionType::EXPLOSION_ROCKET
                | eExplosionType::EXPLOSION_HELI
                | eExplosionType::EXPLOSION_MINE
                | eExplosionType::EXPLOSION_BARREL => {
                    if let Some(point_lights) = point_lights.as_deref_mut()
                        && frame_counter & 1 != 0
                    {
                        point_lights.add_light(
                            LIGHT_POINT,
                            explosion.position,
                            CVector::ZERO,
                            20.0,
                            1.0,
                            1.0,
                            0.5,
                            super::PointLights::FOG_NONE,
                            true,
                            camera_pos,
                        );
                    }
                    if let Some(coronas) = coronas.as_deref_mut() {
                        let (r, g, b) = if frame_counter & 1 != 0 {
                            (255, 255, 200)
                        } else {
                            (128, 128, 100)
                        };
                        coronas.register_corona(
                            id,
                            r,
                            g,
                            b,
                            255,
                            explosion.position,
                            8.0,
                            120.0,
                            super::Coronas::TYPE_NORMAL,
                            super::Coronas::FLARE_NONE,
                            super::Coronas::REFLECTION_ON,
                            super::Coronas::LOSCHECK_OFF,
                            super::Coronas::STREAK_OFF,
                            0.0,
                        );
                        coronas.register_corona(
                            id + 1,
                            30,
                            30,
                            25,
                            255,
                            explosion.position,
                            explosion.radius,
                            120.0,
                            super::Coronas::TYPE_STAR,
                            super::Coronas::FLARE_NONE,
                            super::Coronas::REFLECTION_OFF,
                            super::Coronas::LOSCHECK_OFF,
                            super::Coronas::STREAK_OFF,
                            0.0,
                        );
                    }
                }
                eExplosionType::EXPLOSION_CAR | eExplosionType::EXPLOSION_CAR_QUICK => {
                    if time_left >= 3500.0 {
                        if let Some(particles) = particle_emitter.as_mut()
                            && let Some(general) = general.as_deref_mut()
                            && now_ms as f32 > explosion.start_time
                        {
                            explosion.start_time = now_ms
                                .wrapping_add(125 + u32::from(general.get_random_number() & 0x7F))
                                as f32;
                            let count = (general.get_random_number() & 1) + 1;
                            for _ in 0..count {
                                particles.add_particle_coloured(
                                    PARTICLE_EXPLOSION_MEDIUM,
                                    explosion.position,
                                    CVector::ZERO,
                                    None,
                                    3.5,
                                    gray,
                                    0,
                                    0,
                                    0,
                                    0,
                                );
                                particles.add_particle_coloured(
                                    PARTICLE_EXPLOSION_LARGE,
                                    explosion.position,
                                    CVector::ZERO,
                                    None,
                                    5.5,
                                    gray,
                                    0,
                                    0,
                                    0,
                                    0,
                                );
                            }
                        }
                        if let Some(point_lights) = point_lights.as_deref_mut()
                            && frame_counter & 1 != 0
                        {
                            point_lights.add_light(
                                LIGHT_POINT,
                                explosion.position,
                                CVector::ZERO,
                                15.0,
                                1.0,
                                0.0,
                                0.0,
                                super::PointLights::FOG_NONE,
                                true,
                                camera_pos,
                            );
                        }
                        if let Some(coronas) = coronas.as_deref_mut() {
                            let (r, g, b, size) = if frame_counter & 1 != 0 {
                                (200, 100, 0, 6.0)
                            } else {
                                (128, 0, 0, 8.0)
                            };
                            coronas.register_corona(
                                id,
                                r,
                                g,
                                b,
                                255,
                                explosion.position,
                                size,
                                80.0,
                                super::Coronas::TYPE_NORMAL,
                                super::Coronas::FLARE_NONE,
                                super::Coronas::REFLECTION_ON,
                                super::Coronas::LOSCHECK_OFF,
                                super::Coronas::STREAK_OFF,
                                0.0,
                            );
                            coronas.register_corona(
                                id + 1,
                                30,
                                15,
                                0,
                                255,
                                explosion.position,
                                explosion.radius,
                                80.0,
                                super::Coronas::TYPE_STAR,
                                super::Coronas::FLARE_NONE,
                                super::Coronas::REFLECTION_OFF,
                                super::Coronas::LOSCHECK_OFF,
                                super::Coronas::STREAK_OFF,
                                0.0,
                            );
                        }
                    } else if explosion.iteration & 1 != 0
                        && let Some(particles) = particle_emitter.as_mut()
                        && let Some(general) = general.as_deref_mut()
                    {
                        particles.add_particle_coloured(
                            PARTICLE_ENGINE_SMOKE2,
                            CVector::new(
                                explosion.position.x,
                                explosion.position.y,
                                explosion.position.z + 1.0,
                            ),
                            CVector::new(0.0, 0.0, 0.11),
                            None,
                            general.get_random_number_in_range_f32(0.5, 2.0),
                            gray,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                }
                eExplosionType::EXPLOSION_TANK_GRENADE | eExplosionType::EXPLOSION_HELI_BOMB => {
                    if explosion.iteration < 5
                        && let Some(particles) = particle_emitter.as_mut()
                        && let Some(general) = general.as_deref_mut()
                    {
                        let ff = f32::from(explosion.iteration) * 0.65;
                        for _ in 0..(10.0 * ff) as i32 {
                            let x = i32::from(general.get_random_number() as u8) - 128;
                            let y = i32::from(general.get_random_number() as u8) - 128;
                            let z = i32::from(general.get_random_number() as u8) % 128 + 1;
                            let mut dir = CVector::new(x as f32, y as f32, z as f32);
                            let length = dir.magnitude().max(f32::EPSILON);
                            dir.x *= ff / (5.0 * length);
                            dir.y *= ff / (5.0 * length);
                            dir.z *= ff / (5.0 * length);
                            particles.add_particle_coloured(
                                PARTICLE_EXPLOSION_LARGE,
                                CVector::new(
                                    explosion.position.x + dir.x,
                                    explosion.position.y + dir.y,
                                    explosion.position.z + dir.z + 0.5,
                                ),
                                CVector::ZERO,
                                None,
                                0.0,
                                gray,
                                general.get_random_number_in_range_f32(-3.0, 3.0) as i32,
                                general.get_random_number_in_range_f32(-180.0, 180.0) as i32,
                                0,
                                0,
                            );
                        }
                    }
                }
                eExplosionType::EXPLOSION_MOLOTOV => {}
            }
        }
    }

    let (
        Some(file_loader), Some(index), Some(clock), Some(_), Some(_),
        Some(handling_manager), Some(weather),
    ) = (
        file_loader.as_deref(), light_actors.index.as_deref(), clock.as_deref(),
        timer.as_deref(), camera.as_deref(), handling_manager.as_deref(), weather.as_deref(),
    )
    else {
        if !light_actors.cars.is_empty() {
            light_actors.diagnostics.missing_dependencies += 1;
            bevy::log::warn_once!(target: "rustgta3::vehicle_lighting",
                "required vehicle-light model/index/clock/timer/camera/handling/weather dependency is absent");
        }
        return;
    };
    let hour = clock.hours();
    let minute = clock.minutes();
    let foggyness = weather.foggyness;
    let wet_roads = weather.wet_roads;
    let shadow_view = super::Shadows::CarLightShadowView {
        position: camera_pos,
        front: camera.as_ref().and_then(|camera| camera.cams.get(camera.active_cam))
            .map(|cam| Vec2::new(cam.front.x, cam.front.y).normalize_or_zero().to_array())
            .unwrap_or([0.0; 2]),
        special_camera: cutscene.as_ref().is_some_and(|cutscene| cutscene.running)
            || camera.as_ref().and_then(|camera| camera.cams.get(camera.active_cam)).is_some_and(|cam|
                matches!(cam.mode, crate::core::Cam::eCamMode::MODE_TOPDOWN | crate::core::Cam::eCamMode::MODE_TOP_DOWN_PED)),
    };

    for (entity, identity, vehicle, drive, audio, mut appearance, car) in light_actors.cars.iter_mut() {
        let handle = identity.handle;
        if index.vehicle(handle) != Some(entity) {
            light_actors.diagnostics.retired_entities += 1;
            continue;
        }
        if identity.kind != crate::control::ScriptEntities::ScriptVehicleKind::Automobile {
            light_actors.diagnostics.invalid_actor_kind += 1;
            bevy::log::warn_once!(target: "rustgta3::vehicle_lighting",
                handle, "automobile light state has an incompatible actor identity");
            continue;
        }
        light_actors.diagnostics.processed += 1;
        let model_index = vehicle.physical.entity.model_index.max(0) as usize;
        let matrix = vehicle.physical.entity.matrix;
        let position = matrix.position;
        let right = matrix.right;
        let forward = matrix.forward;
        let up = matrix.up;
        let corona_base = 0xC000_0000u32.wrapping_add((handle as u32).wrapping_mul(32));

        // Emergency/taxi roof lamps are handled before the generic head/tail lamps.
        if audio.siren_or_alarm {
            if let Some(layout) = super::VehicleLightingAccess::emergency_light_layout(model_index) {
                let world1 = matrix.transform_point(layout.right);
                let world2 = matrix.transform_point(layout.left);
                let color1 = layout.primary_colour;
                let color2 = layout.secondary_colour;
                for i in 0..4u32 {
                    let t = i as f32 / 3.0;
                    let lamp_pos = CVector::new(
                        world2.x + (world1.x - world2.x) * t,
                        world2.y + (world1.y - world2.y) * t,
                        world2.z + (world1.z - world2.z) * t,
                    );
                    let phase = ((now_ms.wrapping_add(i << 6)) >> 8) & 3;
                    if let Some(coronas) = coronas.as_deref_mut() {
                        match phase {
                            0 => coronas.register_corona(
                                corona_base + 21 + i,
                                color1[0],
                                color1[1],
                                color1[2],
                                255,
                                lamp_pos,
                                0.4,
                                50.0,
                                super::Coronas::TYPE_STAR,
                                if i == 1 {
                                    super::Coronas::FLARE_HEADLIGHTS
                                } else {
                                    super::Coronas::FLARE_NONE
                                },
                                super::Coronas::REFLECTION_OFF,
                                super::Coronas::LOSCHECK_OFF,
                                super::Coronas::STREAK_OFF,
                                0.0,
                            ),
                            2 => coronas.register_corona(
                                corona_base + 21 + i,
                                color2[0],
                                color2[1],
                                color2[2],
                                255,
                                lamp_pos,
                                0.4,
                                50.0,
                                super::Coronas::TYPE_STAR,
                                super::Coronas::FLARE_NONE,
                                super::Coronas::REFLECTION_OFF,
                                super::Coronas::LOSCHECK_OFF,
                                super::Coronas::STREAK_OFF,
                                0.0,
                            ),
                            _ => coronas.update_corona_coors(
                                corona_base + 21 + i,
                                lamp_pos,
                                50.0,
                                0.0,
                            ),
                        }
                    }
                }
                let phase = now_ms & 0x3FF;
                let active = if phase < 512 { color1 } else { color2 };
                let mut fade = 1.0;
                let half_phase = now_ms & 0x1FF;
                if half_phase < 100 {
                    fade = half_phase as f32 / 100.0;
                } else if half_phase > 412 {
                    fade = (512 - half_phase) as f32 / 100.0;
                }
                let shadow_colour = active.map(|channel| (f32::from(channel / 6) * fade) as u8);
                if let Some(shadows) = shadows.as_deref_mut() {
                    let angle = (now_ms & 0x3FF) as f32 * std::f32::consts::TAU / 1023.0;
                    let (sine, cosine) = angle.sin_cos();
                    shadows.store_car_light_shadow(super::Shadows::CarLightShadowIntent {
                        key: corona_base + 21,
                        texture: 4,
                        position,
                        front: [8.0 * cosine, 8.0 * sine],
                        side: [8.0 * sine, -8.0 * cosine],
                        color: shadow_colour,
                    }, shadow_view, 8.0);
                }
                if let Some(point_lights) = point_lights.as_deref_mut() {
                    point_lights.add_light(
                        LIGHT_POINT,
                        CVector::new(
                            position.x + up.x * 2.0,
                            position.y + up.y * 2.0,
                            position.z + up.z * 2.0,
                        ),
                        CVector::ZERO,
                        12.0,
                        f32::from(shadow_colour[0]) * 0.02,
                        f32::from(shadow_colour[1]) * 0.02,
                        f32::from(shadow_colour[2]) * 0.02,
                        super::PointLights::FOG_NONE,
                        true,
                        camera_pos,
                    );
                }
            } else if model_index == MI_FBICAR {
                let lamp = matrix.transform_point(CVector::new(0.4, 0.6, 0.3));
                let to_camera = CVector::new(
                    position.x - camera_pos.x,
                    position.y - camera_pos.y,
                    position.z - camera_pos.z,
                );
                if now_ms & 0x100 != 0 && vec3_dot(forward, to_camera) < 0.0 {
                    if let Some(coronas) = coronas.as_deref_mut() {
                        coronas.register_corona(
                            corona_base + 21,
                            0,
                            0,
                            255,
                            255,
                            lamp,
                            0.4,
                            50.0,
                            super::Coronas::TYPE_STAR,
                            super::Coronas::FLARE_NONE,
                            super::Coronas::REFLECTION_OFF,
                            super::Coronas::LOSCHECK_OFF,
                            super::Coronas::STREAK_OFF,
                            0.0,
                        );
                    }
                }
            }
        }
        if matches!(model_index, MI_TAXI | MI_CABBIE | MI_BORGNINE) && car.taxi_light {
            let lamp = CVector::new(
                position.x + up.x * 0.95,
                position.y + up.y * 0.95,
                position.z + up.z * 0.95,
            );
            if let Some(coronas) = coronas.as_deref_mut() {
                coronas.register_corona(
                    corona_base + 21,
                    128,
                    128,
                    0,
                    255,
                    lamp,
                    0.8,
                    50.0,
                    super::Coronas::TYPE_NORMAL,
                    super::Coronas::FLARE_NONE,
                    super::Coronas::REFLECTION_ON,
                    super::Coronas::LOSCHECK_OFF,
                    super::Coronas::STREAK_OFF,
                    0.0,
                );
            }
            if let Some(point_lights) = point_lights.as_deref_mut() {
                point_lights.add_light(
                    LIGHT_POINT,
                    lamp,
                    CVector::ZERO,
                    10.0,
                    1.0,
                    1.0,
                    0.5,
                    super::PointLights::FOG_NONE,
                    true,
                    camera_pos,
                );
            }
        }

        if matches!(model_index, MI_RCBANDIT | MI_DODO | MI_RHINO) {
            continue;
        }
        let seed = vehicle.physical.entity.random_seed;
        let threshold = seed & 0x3F;
        let should_lights_be_on = hour > 20
            || (hour > 19 && u16::from(minute) > threshold)
            || hour < 7
            || (hour < 8 && u16::from(minute) < threshold)
            || f32::from(seed) / 50_000.0 < foggyness
            || f32::from(seed) / 50_000.0 < wet_roads;
        let status = vehicle.physical.entity.status;
        if should_lights_be_on != appearance.lights_on
            && status != eEntityStatus::STATUS_WRECKED
        {
            if status == eEntityStatus::STATUS_ABANDONED {
                let manhattan =
                    (camera_pos.x - position.x).abs() + (camera_pos.y - position.y).abs();
                if appearance.lights_on && manhattan > 100.0 {
                    appearance.lights_on = false;
                }
            } else {
                appearance.lights_on = should_lights_be_on;
            }
        }

        let Some(ModelInfoSlot::Vehicle(model)) =
            file_loader.model_info.get_model_info(model_index)
        else {
            light_actors.diagnostics.waiting_model += 1;
            continue;
        };
        let Some(lamps) = model.lamp_positions() else {
            light_actors.diagnostics.waiting_clump += 1;
            continue;
        };
        let Some(head_local) = lamps.headlights.position() else {
            light_actors.diagnostics.missing_headlights += 1;
            bevy::log::warn_once!(target: "rustgta3::vehicle_lighting", model_index,
                "required authored headlights dummy is absent or invalid");
            continue;
        };
        let Some(tail_local) = lamps.taillights else {
            light_actors.diagnostics.missing_taillights += 1;
            bevy::log::warn_once!(target: "rustgta3::vehicle_lighting", model_index,
                "required authored taillights dummy is absent or invalid");
            continue;
        };
        let Some(handling) = vehicle.handling_id
            .and_then(|id| usize::try_from(id).ok())
            .and_then(|id| handling_manager.get_handling_data(id))
        else {
            light_actors.diagnostics.missing_handling += 1;
            bevy::log::warn_once!(target: "rustgta3::vehicle_lighting", model_index,
                "required vehicle handling record is absent");
            continue;
        };
        light_actors.diagnostics.authored_lamps_resolved += 1;
        let halogen = handling.flags & HANDLING_HALOGEN_LIGHTS != 0;
        let head_r = matrix.transform_point(head_local);
        let head_l = CVector::new(
            head_r.x - right.x * 2.0 * head_local.x,
            head_r.y - right.y * 2.0 * head_local.x,
            head_r.z - right.z * 2.0 * head_local.x,
        );
        let tail_r = matrix.transform_point(tail_local);
        let tail_l = CVector::new(
            tail_r.x - right.x * 2.0 * tail_local.x,
            tail_r.y - right.y * 2.0 * tail_local.x,
            tail_r.z - right.z * 2.0 * tail_local.x,
        );
        let light_ok = |light: eLights| {
            car.damage.get_light_status(light as u8) == eLightStatus::LIGHT_STATUS_OK as u32
        };
        let to_car = CVector::new(
            position.x - camera_pos.x,
            position.y - camera_pos.y,
            position.z - camera_pos.z,
        );
        let cam_dist = to_car.magnitude();
        let look = if cam_dist > f32::EPSILON {
            CVector::new(
                to_car.x / cam_dist,
                to_car.y / cam_dist,
                to_car.z / cam_dist,
            )
        } else {
            CVector::new(1.0, 0.0, 0.0)
        };
        let behindness = vec3_dot(look, forward).clamp(-1.0, 1.0);
        let angle = behindness.acos().abs();
        let alarm = audio.alarm_state != 0
            && audio.alarm_state != -1
            && status != eEntityStatus::STATUS_WRECKED;
        let alarm_on = alarm && now_ms & 0x100 != 0;
        let alarm_off = alarm && !alarm_on;
        let lights_active =
            (drive.engine_on && appearance.lights_on) || alarm_on || alarm_off;

        if let Some(fx) = special_fx.as_deref_mut() {
            super::VehicleLightingAccess::register_automobile_bright_lights(
                fx, &matrix, camera_pos, [head_l, head_r], [tail_l, tail_r],
                [light_ok(eLights::VEHLIGHT_FRONT_LEFT), light_ok(eLights::VEHLIGHT_FRONT_RIGHT),
                 light_ok(eLights::VEHLIGHT_REAR_LEFT), light_ok(eLights::VEHLIGHT_REAR_RIGHT)],
                handling, lights_active, vehicle.physical.entity.flags.no_bright_head_lights,
                matches!(status, eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED),
                behindness, drive.gas_pedal, drive.brake_pedal,
            );
        }

        if lights_active {
            if behindness < 0.0 {
                let intensity = -0.5 * behindness + 0.3;
                let size = 1.0 - behindness;
                if let Some(coronas) = coronas.as_deref_mut() {
                    if behindness < -0.97 && cam_dist < 30.0 {
                        let bright = if halogen {
                            [150, 150, 195]
                        } else {
                            [160, 160, 140]
                        };
                        for (id_offset, pos, ok) in [
                            (6, head_l, light_ok(eLights::VEHLIGHT_FRONT_LEFT)),
                            (7, head_r, light_ok(eLights::VEHLIGHT_FRONT_RIGHT)),
                        ] {
                            if ok {
                                coronas.register_corona(
                                    corona_base + id_offset,
                                    bright[0],
                                    bright[1],
                                    bright[2],
                                    255,
                                    pos,
                                    1.2,
                                    45.0 * lod_dist,
                                    super::Coronas::TYPE_HEADLIGHT,
                                    super::Coronas::FLARE_NONE,
                                    super::Coronas::REFLECTION_ON,
                                    super::Coronas::LOSCHECK_OFF,
                                    super::Coronas::STREAK_OFF,
                                    angle,
                                );
                            }
                        }
                    }
                    let base_rgb = if halogen {
                        [190.0, 190.0, 255.0]
                    } else {
                        [210.0, 210.0, 195.0]
                    };
                    for (id_offset, pos, ok) in [
                        (0, head_l, light_ok(eLights::VEHLIGHT_FRONT_LEFT)),
                        (1, head_r, light_ok(eLights::VEHLIGHT_FRONT_RIGHT)),
                    ] {
                        if !ok {
                            continue;
                        }
                        if alarm_off {
                            coronas.register_corona(
                                corona_base + id_offset,
                                0,
                                0,
                                0,
                                0,
                                pos,
                                size,
                                50.0 * lod_dist,
                                super::Coronas::TYPE_STREAK,
                                super::Coronas::FLARE_NONE,
                                super::Coronas::REFLECTION_ON,
                                super::Coronas::LOSCHECK_OFF,
                                super::Coronas::STREAK_ON,
                                angle,
                            );
                        } else {
                            coronas.register_corona(
                                corona_base + id_offset,
                                (base_rgb[0] * intensity).clamp(0.0, 255.0) as u8,
                                (base_rgb[1] * intensity).clamp(0.0, 255.0) as u8,
                                (base_rgb[2] * intensity).clamp(0.0, 255.0) as u8,
                                255,
                                pos,
                                size,
                                50.0 * lod_dist,
                                super::Coronas::TYPE_STREAK,
                                super::Coronas::FLARE_NONE,
                                super::Coronas::REFLECTION_ON,
                                super::Coronas::LOSCHECK_OFF,
                                super::Coronas::STREAK_ON,
                                angle,
                            );
                        }
                    }
                }
            } else if let Some(coronas) = coronas.as_deref_mut() {
                if light_ok(eLights::VEHLIGHT_FRONT_LEFT) {
                    coronas.update_corona_coors(corona_base, head_l, 50.0 * lod_dist, angle);
                }
                if light_ok(eLights::VEHLIGHT_FRONT_RIGHT) {
                    coronas.update_corona_coors(corona_base + 1, head_r, 50.0 * lod_dist, angle);
                }
            }

            if behindness > 0.0 {
                let mut intensity = (behindness + 1.0) * 0.4;
                let mut size = (behindness + 1.0) * 0.5;
                let reverse = drive.gas_pedal < 0.0;
                if reverse || drive.brake_pedal > 0.0 {
                    intensity += 0.4;
                    size += 0.3;
                }
                let rgb = if reverse {
                    let c = (128.0 * intensity).clamp(0.0, 255.0) as u8;
                    [c, c, c]
                } else {
                    [(128.0 * intensity).clamp(0.0, 255.0) as u8, 0, 0]
                };
                if let Some(coronas) = coronas.as_deref_mut() {
                    for (id_offset, pos, ok) in [
                        (14, tail_l, light_ok(eLights::VEHLIGHT_REAR_LEFT)),
                        (15, tail_r, light_ok(eLights::VEHLIGHT_REAR_RIGHT)),
                    ] {
                        if ok {
                            coronas.register_corona(
                                corona_base + id_offset,
                                if alarm_off { 0 } else { rgb[0] },
                                if alarm_off { 0 } else { rgb[1] },
                                if alarm_off { 0 } else { rgb[2] },
                                if alarm_off { 0 } else { 255 },
                                pos,
                                size,
                                50.0 * lod_dist,
                                super::Coronas::TYPE_STREAK,
                                super::Coronas::FLARE_NONE,
                                super::Coronas::REFLECTION_ON,
                                super::Coronas::LOSCHECK_OFF,
                                super::Coronas::STREAK_ON,
                                angle,
                            );
                        }
                    }
                }
            }

            if !alarm_off {
                let mut fwd2 = CVector::new(forward.x, forward.y, 0.0);
                let len = (fwd2.x * fwd2.x + fwd2.y * fwd2.y).sqrt().max(f32::EPSILON);
                fwd2.x /= len;
                fwd2.y /= len;
                if let Some(shadows) = shadows.as_deref_mut() {
                    if light_ok(eLights::VEHLIGHT_FRONT_LEFT)
                        || light_ok(eLights::VEHLIGHT_FRONT_RIGHT)
                    {
                        let f = head_local.y + 6.0;
                        let intent = super::Shadows::CarLightShadowIntent {
                            key: corona_base + 22,
                            texture: 4,
                            position: CVector::new(
                                position.x + f * fwd2.x,
                                position.y + f * fwd2.y,
                                position.z + 2.0,
                            ),
                            front: [7.0 * fwd2.x, 7.0 * fwd2.y],
                            side: [7.0 * fwd2.y, -7.0 * fwd2.x],
                            color: [45, 45, 45],
                        };
                        shadows.store_car_light_shadow(intent, shadow_view, 7.0);
                    }
                    if light_ok(eLights::VEHLIGHT_REAR_LEFT)
                        || light_ok(eLights::VEHLIGHT_REAR_RIGHT)
                    {
                        let f = tail_local.y - 2.5;
                        let intent = super::Shadows::CarLightShadowIntent {
                            key: corona_base + 25,
                            texture: 3,
                            position: CVector::new(
                                position.x + f * fwd2.x,
                                position.y + f * fwd2.y,
                                position.z + 2.0,
                            ),
                            front: [3.0, 0.0],
                            side: [0.0, -3.0],
                            color: [35, 0, 0],
                        };
                        shadows.store_car_light_shadow(intent, shadow_view, 4.0);
                    }
                }
            }
        } else if status != eEntityStatus::STATUS_ABANDONED
            && status != eEntityStatus::STATUS_WRECKED
            && (drive.brake_pedal > 0.0 || drive.gas_pedal < 0.0)
            && behindness > 0.0
        {
            let rgb = if drive.gas_pedal < 0.0 {
                [120, 120, 120]
            } else {
                [120, 0, 0]
            };
            if let Some(coronas) = coronas.as_deref_mut() {
                for (id_offset, pos, ok) in [
                    (14, tail_l, light_ok(eLights::VEHLIGHT_REAR_LEFT)),
                    (15, tail_r, light_ok(eLights::VEHLIGHT_REAR_RIGHT)),
                ] {
                    if ok {
                        coronas.register_corona(
                            corona_base + id_offset,
                            rgb[0],
                            rgb[1],
                            rgb[2],
                            255,
                            pos,
                            1.2,
                            50.0 * lod_dist,
                            super::Coronas::TYPE_STAR,
                            super::Coronas::FLARE_NONE,
                            super::Coronas::REFLECTION_ON,
                            super::Coronas::LOSCHECK_OFF,
                            super::Coronas::STREAK_ON,
                            0.0,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod vehicle_lighting_lifecycle_tests {
    use super::*;
    use crate::{
        control::ScriptEntities::{ScriptEntityIndex, ScriptVehicleIdentity, ScriptVehicleKind},
        modelinfo::ModelIndices::MI_POLICE,
        renderer::VehicleLightingAccess::VehicleLightingDiagnostics,
        vehicles::{Automobile::CAutomobile, Vehicle::VehicleAppearance},
    };

    fn fixture() -> (App, Entity, i32) {
        let mut app = App::new();
        app.init_resource::<ScriptEntityIndex>()
            .init_resource::<VehicleLightingDiagnostics>()
            .init_resource::<CFileLoader>()
            .init_resource::<CTimer>()
            .init_resource::<CClock>()
            .init_resource::<CCamera>()
            .init_resource::<cHandlingDataMgr>()
            .init_resource::<crate::renderer::Weather::CWeather>()
            .add_systems(Update, update_explosion_and_vehicle_lights);
        app.world_mut()
            .resource_mut::<CClock>()
            .set_game_clock(22, 0, 0);
        let mut general = CGeneral::with_seed(29);
        let handle = 0x501;
        let (core, vitality, drive, appearance, entry, audio, car, audio_motion) = CAutomobile::new(
            MI_POLICE as i32,
            crate::vehicles::Vehicle::RANDOM_VEHICLE,
            0,
            &mut general,
            None,
            None,
            false,
        )
        .into_parts();
        let entity = app.world_mut().spawn((
            ScriptVehicleIdentity {
                handle,
                kind: ScriptVehicleKind::Automobile,
            },
            core, vitality, drive, appearance, entry, audio, car, audio_motion,
        )).id();
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .vehicles.insert(handle, entity);
        (app, entity, handle)
    }

    #[test]
    fn retired_index_stops_lighting_before_deferred_despawn() {
        let (mut app, entity, handle) = fixture();
        app.update();
        assert_eq!(app.world().resource::<VehicleLightingDiagnostics>().processed, 1);
        assert_eq!(app.world().resource::<VehicleLightingDiagnostics>().waiting_model, 1);
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .vehicles.remove(&handle);
        app.world_mut().get_mut::<VehicleAppearance>(entity).unwrap()
            .lights_on = false;
        app.update();
        let diagnostics = app.world().resource::<VehicleLightingDiagnostics>();
        assert_eq!(diagnostics.processed, 1);
        assert_eq!(diagnostics.retired_entities, 1);
        assert!(!app.world().get::<VehicleAppearance>(entity).unwrap().lights_on);
    }

    #[test]
    fn missing_weather_is_observable_and_does_not_supply_clear_weather() {
        let (mut app, entity, _) = fixture();
        app.world_mut().remove_resource::<crate::renderer::Weather::CWeather>();
        app.update();
        let diagnostics = app.world().resource::<VehicleLightingDiagnostics>();
        assert_eq!(diagnostics.missing_dependencies, 1);
        assert_eq!(diagnostics.processed, 0);
        assert!(!app.world().get::<VehicleAppearance>(entity).unwrap().lights_on);
        app.init_resource::<crate::renderer::Weather::CWeather>();
        app.update();
        assert_eq!(app.world().resource::<VehicleLightingDiagnostics>().processed, 1);
        assert!(app.world().get::<VehicleAppearance>(entity).unwrap().lights_on);
    }
}

fn vec3_distance(a: CVector, b: CVector) -> f32 {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z).magnitude()
}

fn vec3_dot(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn world_entity_direction(entity: &CWorldEntity, local: [f32; 3]) -> CVector {
    let point = entity_world_point(entity, local);
    CVector::new(
        point[0] - entity.position[0],
        point[1] - entity.position[1],
        point[2] - entity.position[2],
    )
}

#[allow(clippy::too_many_arguments)]
fn register_traffic_light_display(
    entity_index: usize,
    entity: &CWorldEntity,
    effects: &[GtaTwoDfx],
    time_ms: u32,
    hour: u8,
    foggyness: f32,
    camera_position: CVector,
    camera_forward: CVector,
    timecycle: &CTimeCycle,
    coronas: &mut CCoronas,
    mut point_lights: Option<&mut CPointLights>,
    mut shadows: Option<&mut CShadows>,
    mut special_fx: Option<&mut CSpecialFX>,
) {
    // re3 DisplayActualLight assumes six 2dfx positions authored into
    // trafficlight1.dff. A malformed replacement model simply has no custom
    // light display here rather than indexing outside the loaded asset data.
    if effects.len() < 6 {
        return;
    }

    let right = world_entity_direction(entity, [1.0, 0.0, 0.0]);
    let forward = world_entity_direction(entity, [0.0, 1.0, 0.0]);
    let up = world_entity_direction(entity, [0.0, 0.0, 1.0]);
    if up.z < 0.96 {
        return;
    }

    let phase = if CTrafficLights::find_traffic_light_type(entity) == 1 {
        CTrafficLights::light_for_cars1(time_ms)
    } else {
        CTrafficLights::light_for_cars2(time_ms)
    };

    let x = effects[0].position[0];
    let mut y_min = effects[0].position[1];
    let mut y_max = y_min;
    let mut z_min = effects[0].position[2];
    let mut z_max = z_min;
    for effect in &effects[1..6] {
        y_min = y_min.min(effect.position[1]);
        y_max = y_max.max(effect.position[1]);
        z_min = z_min.min(effect.position[2]);
        z_max = z_max.max(effect.position[2]);
    }

    let (r, g, z, id) = match phase {
        CAR_LIGHTS_GREEN => (0u8, 255u8, z_min, 0u32),
        CAR_LIGHTS_YELLOW => (255u8, 128u8, (z_min + z_max) * 0.5, 1u32),
        _ => (255u8, 0u8, z_max, 2u32),
    };
    let pos1_raw = entity_world_point(entity, [x, y_max, z]);
    let pos2_raw = entity_world_point(entity, [x, y_min, z]);
    let pos1 = CVector::new(pos1_raw[0], pos1_raw[1], pos1_raw[2]);
    let pos2 = CVector::new(pos2_raw[0], pos2_raw[1], pos2_raw[2]);

    if (hour > 19 || hour < 6 || foggyness > 0.05)
        && let Some(point_lights) = point_lights.as_deref_mut()
    {
        point_lights.add_light(
            LIGHT_POINT,
            pos1,
            CVector::ZERO,
            8.0,
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            0.0,
            super::PointLights::FOG_NORMAL,
            true,
            camera_position,
        );
    }

    if let Some(shadows) = shadows.as_deref_mut() {
        let brightness = timecycle.current.lights_on_ground_brightness;
        shadows.static_shadows.push(StaticShadowIntent {
            key: 0x5452_0000u32.wrapping_add(entity_index as u32),
            texture: 3, // gpShadowExplosionTex -> shad_exp
            position: pos1,
            front: [8.0, 0.0],
            side: [0.0, -8.0],
            intensity: 128,
            color: [
                (f32::from(r) * brightness / 8.0).clamp(0.0, 255.0) as u8,
                (f32::from(g) * brightness / 8.0).clamp(0.0, 255.0) as u8,
                0,
            ],
        });
    }

    let sprite_brightness = timecycle.current.sprite_brightness * 0.7;
    let corona_colour = [
        (f32::from(r) * sprite_brightness).clamp(0.0, 255.0) as u8,
        (f32::from(g) * sprite_brightness).clamp(0.0, 255.0) as u8,
    ];
    let show_first_side = vec3_dot(camera_forward, forward) < 0.0;
    let (corona_position, corona_side_offset) = if show_first_side {
        (pos1, 0u32)
    } else {
        (pos2, 3u32)
    };
    coronas.register_corona(
        0x5450_0000u32
            .wrapping_add((entity_index as u32).wrapping_mul(8))
            .wrapping_add(id)
            .wrapping_add(corona_side_offset),
        corona_colour[0],
        corona_colour[1],
        0,
        255,
        corona_position,
        1.75 * timecycle.current.sprite_size,
        50.0,
        super::Coronas::TYPE_STAR,
        super::Coronas::FLARE_NONE,
        super::Coronas::REFLECTION_ON,
        super::Coronas::LOSCHECK_OFF,
        super::Coronas::STREAK_OFF,
        0.0,
    );

    if let Some(special_fx) = special_fx.as_deref_mut() {
        let bright_type = match phase {
            CAR_LIGHTS_GREEN => BRIGHTLIGHT_TRAFFIC_GREEN,
            CAR_LIGHTS_YELLOW => BRIGHTLIGHT_TRAFFIC_YELLOW,
            _ => BRIGHTLIGHT_TRAFFIC_RED,
        };
        special_fx.register_traffic_bright_light(pos1, up, right, bright_type, camera_position);
        special_fx.register_traffic_bright_light(
            pos2,
            up,
            CVector::new(-right.x, -right.y, -right.z),
            bright_type,
            camera_position,
        );

        const TOP: f32 = -0.127;
        const BOT: f32 = -0.539;
        const MID: f32 = BOT + (TOP - BOT) / 3.0;
        const LEFT: f32 = 1.256;
        const RIGHT: f32 = 0.706;
        let ped_phase = CTrafficLights::light_for_peds(time_ms);
        let ped_quad = if ped_phase == PED_LIGHTS_DONT_WALK {
            Some((
                [
                    [2.7, RIGHT, TOP],
                    [2.7, LEFT, TOP],
                    [2.7, RIGHT, MID],
                    [2.7, LEFT, MID],
                ],
                [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                [255, 0, 0],
            ))
        } else if ped_phase == PED_LIGHTS_WALK || time_ms & 0x100 != 0 {
            Some((
                [
                    [2.7, RIGHT, MID],
                    [2.7, LEFT, MID],
                    [2.7, RIGHT, BOT],
                    [2.7, LEFT, BOT],
                ],
                [[1.0, 0.5], [0.0, 0.5], [1.0, 1.0], [0.0, 1.0]],
                [255, 255, 255],
            ))
        } else {
            None
        };
        if let Some((local_vertices, tex_coords, colour)) = ped_quad {
            let vertices = local_vertices.map(|point| {
                let world = entity_world_point(entity, point);
                CVector::new(world[0], world[1], world[2])
            });
            special_fx.register_walk_dont_text(vertices, tex_coords, colour, 60.0, camera_position);
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct MapObjectEffectAccess<'w, 's> {
    index: Option<Res<'w, crate::control::ScriptEntities::ScriptEntityIndex>>,
    objects: Query<'w, 's, &'static crate::objects::Object::CObject>,
    loader: Option<Res<'w, CFileLoader>>,
}

fn update_atmosphere_and_fx(
    // Same-frame camera eye: Transform is written by the Camera set earlier
    // this frame; the arena fallback camera keeps GlobalTransform because it
    // is not driven by the GTA camera schedule.
    camera_query: Query<&Transform, With<GtaSceneCamera>>,
    fallback_camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<GtaSceneCamera>)>,
    timecycle: Option<Res<CTimeCycle>>,
    camera: Option<Res<CCamera>>,
    mut general: Option<ResMut<CGeneral>>,
    timer: Option<Res<CTimer>>,
    weather: Option<Res<super::Weather::CWeather>>,
    clouds: Option<ResMut<CClouds>>,
    coronas: Option<ResMut<CCoronas>>,
    mut special_fx: Option<ResMut<CSpecialFX>>,
    traffic_lights: Option<Res<CTrafficLights>>,
    object_effects: MapObjectEffectAccess,
    clock: Option<Res<CClock>>,
    bridge: Option<Res<CBridge>>,
    mut point_lights: Option<ResMut<CPointLights>>,
    mut shadows: Option<ResMut<CShadows>>,
) {
    let file_loader = object_effects.loader;
    // The production camera carries GtaSceneCamera.  The insert/dev arena
    // deliberately owns its own Camera3d, so retain the same SpecialFX update
    // cadence there instead of silently dropping marker intents.
    let Some((camera_pos, camera_forward)) = camera_query
        .iter()
        .next()
        .map(|transform| (transform.translation, *transform.forward()))
        .or_else(|| {
            fallback_camera_query
                .iter()
                .next()
                .map(|transform| (transform.translation(), *transform.forward()))
        })
    else {
        return;
    };
    let camera_pos = CVector::from_bevy(camera_pos);
    let time_ms = timer
        .as_ref()
        .map(|t| t.time_in_milliseconds())
        .unwrap_or(0);

    if let Some(mut clouds) = clouds {
        let camera_orientation = camera.as_ref().map_or_else(
            || {
                let forward = CVector::from_bevy(camera_forward);
                forward.x.atan2(forward.y)
            },
            |camera| camera.gameplay_orientation(),
        );
        let (time_step, time_step_fix) = timer
            .as_ref()
            .map(|timer| (timer.time_step(), timer.get_time_step_fix()))
            .unwrap_or((1.0, 1.0));
        let (wind, foggyness, cloud_coverage) = weather
            .as_ref()
            .map(|weather| (weather.wind, weather.foggyness, weather.cloud_coverage))
            .unwrap_or((0.0, 0.0, 0.0));
        let (low_clouds, fluffy_top, fluffy_bottom) = timecycle
            .as_ref()
            .map(|timecycle| {
                (
                    timecycle.current.low_clouds,
                    timecycle.current.fluffy_clouds_top,
                    timecycle.current.fluffy_clouds_bottom,
                )
            })
            .unwrap_or(([0; 3], [0; 3], [0; 3]));
        clouds.update_and_build_intents(
            camera_pos,
            camera_orientation,
            time_step,
            time_step_fix,
            wind,
            foggyness,
            cloud_coverage,
            low_clouds,
            fluffy_top,
            fluffy_bottom,
        );
    }
    if let (Some(mut coronas), Some(timecycle)) = (coronas, timecycle.as_ref()) {
        // re3 `CCoronas::DoSunAndMoon`: use the interpolated timecycle sun
        // direction and colours, not a renderer-side approximation.
        let sun_dir =
            timecycle.vector_to_sun[timecycle.current_stored_value.rem_euclid(16) as usize];
        if sun_dir.z > -0.2 {
            let sun_pos = CVector::new(
                camera_pos.x + sun_dir.x * 150.0,
                camera_pos.y + sun_dir.y * 150.0,
                camera_pos.z + sun_dir.z * 150.0,
            );
            let random_size = general
                .as_deref_mut()
                .map_or(0, |general| (general.get_random_number() & 0xFF) as i32);
            let size = (random_size as f32 * 0.005 + 10.0) * timecycle.current.sun_size;
            let core = timecycle.current.sun_core;
            coronas.register_corona(
                1,
                core[0].clamp(0, 255) as u8,
                core[1].clamp(0, 255) as u8,
                core[2].clamp(0, 255) as u8,
                255,
                sun_pos,
                size,
                999_999.88,
                super::Coronas::TYPE_STAR,
                super::Coronas::FLARE_NONE,
                super::Coronas::REFLECTION_OFF,
                super::Coronas::LOSCHECK_OFF,
                super::Coronas::STREAK_OFF,
                0.0,
            );
            if sun_dir.z > 0.0 {
                let corona = timecycle.current.sun_corona;
                coronas.register_corona(
                    2,
                    corona[0].clamp(0, 255) as u8,
                    corona[1].clamp(0, 255) as u8,
                    corona[2].clamp(0, 255) as u8,
                    255,
                    sun_pos,
                    25.0 * timecycle.current.sun_size,
                    999_999.88,
                    super::Coronas::TYPE_STAR,
                    super::Coronas::FLARE_SUN,
                    super::Coronas::REFLECTION_OFF,
                    super::Coronas::LOSCHECK_ON,
                    super::Coronas::STREAK_OFF,
                    0.0,
                );
            }
        }
        if let (Some(file_loader), Some(clock)) = (file_loader.as_deref(), clock.as_deref()) {
            let hour = clock.hours();
            let is_night = hour > 18 || hour < 7;
            let wet_roads = weather.as_ref().map_or(0.0, |weather| weather.wet_roads);
            let foggyness = weather.as_ref().map_or(0.0, |weather| weather.foggyness);
            let sprite_brightness = timecycle.current.sprite_brightness;
            let traffic_light_model_index = traffic_lights
                .as_ref()
                .and_then(|traffic_lights| traffic_lights.model_index);
            let camera_forward_gta = CVector::from_bevy(camera_forward);
            // re3 traverses the camera's visible sectors rather than the IPL
            // file's insertion order. Traversing the full IPL by index lets
            // unrelated distant/offscreen effects claim all 128 corona slots
            // before a nearby streetlamp is even visited. Preserve one scan
            // but process eligible effects nearest to the camera first.
            let mut nearby_effect_entities = Vec::new();
            nearby_effect_entities.extend(file_loader.world.entities.iter().enumerate()
                .filter(|(_, entity)| {
                    entity.is_visible
                        && file_loader.model_info.get_2d_effects(entity.model_index).is_some()
                        && (entity.position[0] - camera_pos.x).powi(2)
                            + (entity.position[1] - camera_pos.y).powi(2) <= 300.0 * 300.0
                })
                .map(|(index, entity)| (index, std::borrow::Cow::Borrowed(entity))));
            // ConvertToRealObject transfers the IPL dummy to a live CObject.
            // The hidden placement is provenance, not the lamp's live pose or
            // visibility. Keep its corona ID across promotion/demotion.
            if let Some(index) = object_effects.index.as_deref() {
                for (&world_index, &handle) in &file_loader.world.ecs_lifecycle_objects {
                    let Some(owner) = index.object(handle) else { continue };
                    let Ok(object) = object_effects.objects.get(owner) else { continue };
                    let live = &object.physical.entity;
                    if !live.flags.is_visible { continue; }
                    let model_index = live.model_index.max(0) as usize;
                    if traffic_light_model_index == Some(model_index) && live.flags.render_damaged { continue; }
                    if file_loader.model_info.get_2d_effects(model_index).is_none() { continue; }
                    let position = live.get_position();
                    if (position.x - camera_pos.x).powi(2) + (position.y - camera_pos.y).powi(2) > 300.0 * 300.0 { continue; }
                    let Some(placement) = file_loader.world.entities.get(world_index) else { continue };
                    let mut effect_entity = placement.clone();
                    effect_entity.model_index = model_index;
                    effect_entity.position = [position.x, position.y, position.z];
                    let matrix = &live.matrix;
                    effect_entity.matrix_basis_override = Some([
                        [matrix.right.x, matrix.right.y, matrix.right.z],
                        [matrix.forward.x, matrix.forward.y, matrix.forward.z],
                        [matrix.up.x, matrix.up.y, matrix.up.z],
                    ]);
                    effect_entity.is_visible = true;
                    nearby_effect_entities.push((world_index, std::borrow::Cow::Owned(effect_entity)));
                }
            }
            nearby_effect_entities.sort_unstable_by(|a, b| {
                let distance_sq = |entity: &CWorldEntity| {
                    (entity.position[0] - camera_pos.x).powi(2)
                        + (entity.position[1] - camera_pos.y).powi(2)
                };
                distance_sq(&a.1).total_cmp(&distance_sq(&b.1))
                    .then_with(|| a.0.cmp(&b.0))
            });
            for (entity_index, entity) in &nearby_effect_entities {
                let entity_index = *entity_index;
                let entity = entity.as_ref();
                let Some(effects) = file_loader.model_info.get_2d_effects(entity.model_index) else {
                    continue;
                };
                if traffic_light_model_index == Some(entity.model_index) {
                    register_traffic_light_display(
                        entity_index,
                        entity,
                        effects,
                        time_ms,
                        hour,
                        foggyness,
                        camera_pos,
                        camera_forward_gta,
                        timecycle,
                        &mut coronas,
                        point_lights.as_deref_mut(),
                        shadows.as_deref_mut(),
                        special_fx.as_deref_mut(),
                    );
                    // re3 gives trafficlight1 its own DisplayActualLight path;
                    // the six authored marker effects are not six independent
                    // generic 2dfx bulbs.
                    continue;
                }
                let random_seed = (((entity_index as u32)
                    .wrapping_mul(1_103_515_245)
                    .wrapping_add(12_345)
                    >> 16)
                    & 0x7FFF) as u16;
                for (fx_index, effect) in effects.iter().enumerate() {
                    if effect.effect_type != 0 {
                        continue;
                    }
                    let Some(ref light) = effect.light else {
                        continue;
                    };
                    let (light_on, light_flickering) = two_dfx_light_state(
                        light.light_type,
                        is_night,
                        wet_roads,
                        time_ms,
                        random_seed,
                        fx_index,
                        bridge
                            .as_deref()
                            .is_some_and(CBridge::should_lights_be_flashing),
                    );
                    let pos = entity_world_point(entity, effect.position);
                    let world_pos = CVector::new(pos[0], pos[1], pos[2]);
                    let corona_id = (entity_index as u32)
                        .wrapping_mul(16)
                        .wrapping_add(fx_index as u32)
                        .wrapping_add(1000);
                    // re3 CCoronas::RegisterCorona rejects lights beyond
                    // each effect's authored 2D draw distance *before*
                    // reserving any of the 128 global corona slots. Without
                    // this gate, distant lamps exhaust the pool ahead of the
                    // visible streetlights around the camera.
                    let delta_x = world_pos.x - camera_pos.x;
                    let delta_y = world_pos.y - camera_pos.y;
                    let corona_in_range = delta_x * delta_x + delta_y * delta_y
                        < light.dist * light.dist;
                    if (light_on || light_flickering) && corona_in_range {
                        let rgb = if light_on {
                            effect.color
                        } else {
                            [0, 0, 0, effect.color[3]]
                        };
                        coronas.register_corona(
                            corona_id,
                            rgb[0],
                            rgb[1],
                            rgb[2],
                            255,
                            world_pos,
                            light.size,
                            light.dist,
                            corona_texture_type(&light.corona),
                            light.flare_type.clamp(i8::MIN as i32, i8::MAX as i32) as i8,
                            light.road_reflection.clamp(0, 255) as u8,
                            (light.flags & 1) as u8,
                            super::Coronas::STREAK_OFF,
                            0.0,
                        );
                    }

                    if let Some(ref mut pl) = point_lights {
                        if light.flags & 4 != 0 {
                            pl.add_light(
                                super::PointLights::LIGHT_FOGONLY_ALWAYS,
                                world_pos,
                                CVector::ZERO,
                                light.range,
                                effect.color[0] as f32 / 255.0,
                                effect.color[1] as f32 / 255.0,
                                effect.color[2] as f32 / 255.0,
                                super::PointLights::FOG_ALWAYS,
                                true,
                                camera_pos,
                            );
                        } else if light.flags & 2 != 0 && light_on && light.range == 0.0 {
                            pl.add_light(
                                super::PointLights::LIGHT_FOGONLY,
                                world_pos,
                                CVector::ZERO,
                                light.range,
                                effect.color[0] as f32 / 255.0,
                                effect.color[1] as f32 / 255.0,
                                effect.color[2] as f32 / 255.0,
                                super::PointLights::FOG_NORMAL,
                                true,
                                camera_pos,
                            );
                        } else if light_on && light.range != 0.0 {
                            let black = effect.color[0] == 0
                                && effect.color[1] == 0
                                && effect.color[2] == 0;
                            let brightness = if black { 1.0 } else { sprite_brightness };
                            pl.add_light(
                                super::PointLights::LIGHT_POINT,
                                world_pos,
                                CVector::ZERO,
                                light.range,
                                effect.color[0] as f32 * brightness / 255.0,
                                effect.color[1] as f32 * brightness / 255.0,
                                effect.color[2] as f32 * brightness / 255.0,
                                if light.flags & 2 != 0 {
                                    super::PointLights::FOG_NORMAL
                                } else {
                                    super::PointLights::FOG_NONE
                                },
                                true,
                                camera_pos,
                            );
                        }
                    }

                    if light.shadow_size != 0.0
                        && (light_on || light_flickering)
                        && delta_x * delta_x + delta_y * delta_y < 40.0 * 40.0
                        && let Some(shadows) = shadows.as_deref_mut()
                        && let Some(texture) = shadow_texture_type(shadows, &light.shadow)
                    {
                        let shadow_intensity = if light_on { 128 } else { 0 };
                        let shadow_scale = if light_on {
                            sprite_brightness * light.shadow_intensity as f32 / 255.0
                        } else {
                            0.0
                        };
                        shadows.static_shadows.push(StaticShadowIntent {
                            key: 0xD000_0000u32
                                .wrapping_add((entity_index as u32).wrapping_mul(32))
                                .wrapping_add(fx_index as u32),
                            texture,
                            position: world_pos,
                            front: [light.shadow_size, 0.0],
                            side: [0.0, -light.shadow_size],
                            intensity: shadow_intensity,
                            color: [
                                (effect.color[0] as f32 * shadow_scale).clamp(0.0, 255.0) as u8,
                                (effect.color[1] as f32 * shadow_scale).clamp(0.0, 255.0) as u8,
                                (effect.color[2] as f32 * shadow_scale).clamp(0.0, 255.0) as u8,
                            ],
                        });
                    }
                }
            }
        }
    }
    if let Some(special_fx) = special_fx.as_deref_mut() {
        special_fx.update(time_ms);
        special_fx.build_render_intents(time_ms);
    }
}

#[derive(Default)]
struct WorldRenderCandidateScratch {
    indices: Vec<usize>,
    scan_codes: Vec<u32>,
    scan_code: u32,
    #[cfg(test)]
    priority_indices: Vec<usize>,
    #[cfg(test)]
    priority_stream_requests: Vec<usize>,
    #[cfg(test)]
    priority_scan_codes: Vec<u32>,
    #[cfg(test)]
    priority_scan_code: u32,
    planned_indices: Vec<usize>,
    planned_passes: Vec<Option<GtaWorldRenderPass>>,
    planned_distance_fade: Vec<bool>,
    planned_scan_codes: Vec<u32>,
    planned_scan_code: u32,
}

impl WorldRenderCandidateScratch {
    #[cfg(test)]
    fn collect<'a>(&'a mut self, world: &CWorld, camera: CVector) -> &'a [usize] {
        self.collect_many(world, &[camera], LOD_DISTANCE, ELevelName::Generic)
    }

    #[cfg(test)]
    fn collect_many<'a>(
        &'a mut self,
        world: &CWorld,
        cameras: &[CVector],
        lod_distance: f32,
        collision_level: ELevelName,
    ) -> &'a [usize] {
        self.indices.clear();
        if self.scan_codes.len() < world.entities.len() {
            self.scan_codes.resize(world.entities.len(), 0);
        }
        self.scan_code = self.scan_code.wrapping_add(1);
        if self.scan_code == 0 {
            self.scan_codes.fill(0);
            self.scan_code = 1;
        }

        let scan_code = self.scan_code;
        let mut append = |list: &[usize]| {
            for &index in list {
                let Some(code) = self.scan_codes.get_mut(index) else {
                    continue;
                };
                if *code == scan_code {
                    continue;
                }
                *code = scan_code;
                self.indices.push(index);
            }
        };
        let radius = lod_distance + FADE_DISTANCE + STREAM_DISTANCE;
        for camera in cameras {
            let x0 = CWorld::get_sector_index_x(camera.x - radius);
            let x1 = CWorld::get_sector_index_x(camera.x + radius);
            let y0 = CWorld::get_sector_index_y(camera.y - radius);
            let y1 = CWorld::get_sector_index_y(camera.y + radius);
            for y in y0..=y1.min(NUMSECTORS_Y - 1) {
                for x in x0..=x1.min(NUMSECTORS_X - 1) {
                    let sector = &world.sectors[y * NUMSECTORS_X + x];
                    append(&sector.buildings);
                    append(&sector.buildings_overlap);
                    append(&sector.objects);
                    append(&sector.objects_overlap);
                    append(&sector.vehicles);
                    append(&sector.vehicles_overlap);
                    append(&sector.peds);
                    append(&sector.peds_overlap);
                    append(&sector.dummies);
                    append(&sector.dummies_overlap);
                }
            }
        }

        // Renderer.cpp ScanWorld scans only the collision-resident island's
        // big-building list, followed by LEVEL_GENERIC. Scanning all islands
        // made old-island LODs survive SCM/collision transitions and also
        // caused unnecessary stream/render churn.
        let collision_level_index = match collision_level {
            ELevelName::Industrial => 1,
            ELevelName::Commercial => 2,
            ELevelName::Suburban => 3,
            _ => 0,
        };
        if collision_level_index != 0 {
            append(&world.big_buildings_by_level[collision_level_index]);
        }
        append(&world.big_buildings_by_level[0]);
        &self.indices
    }

    /// re3 `CRenderer::ScanWorld` does not scan the whole LOD-radius square.
    /// It rasterises the camera/LOD triangle into world sectors and only then
    /// calls `ScanSectorList`. This distinction matters for unloaded models:
    /// `SetupEntityVisibility` can return `VIS_STREAMME` before the later
    /// on-screen test, so feeding it sectors behind the camera creates a large
    /// stream-request storm that can starve the road directly ahead.
    /// re3 `ScanWorld` uses one CWorld scan code for the priority triangle and
    /// the following ordinary frustum scan. Entities first seen by the centre
    /// triangle therefore cannot be processed a second time by
    /// `ScanSectorList`. Return that priority prefix length together with the
    /// combined candidate list so production preserves the same ordering.
    fn collect_scanworld_many<'a>(
        &'a mut self,
        world: &CWorld,
        cameras: &[CVector],
        priority_triangle: Option<[[f32; 2]; 3]>,
        scan_triangle: Option<[[f32; 2]; 3]>,
        topdown_rect: Option<[usize; 4]>,
        lod_distance: f32,
        collision_level: ELevelName,
    ) -> (&'a [usize], usize) {
        self.indices.clear();
        if self.scan_codes.len() < world.entities.len() {
            self.scan_codes.resize(world.entities.len(), 0);
        }
        self.scan_code = self.scan_code.wrapping_add(1);
        if self.scan_code == 0 {
            self.scan_codes.fill(0);
            self.scan_code = 1;
        }

        let scan_code = self.scan_code;
        if let Some(triangle) = priority_triangle {
            let mut append = |list: &[usize]| {
                for &index in list {
                    let Some(code) = self.scan_codes.get_mut(index) else {
                        continue;
                    };
                    if *code == scan_code {
                        continue;
                    }
                    *code = scan_code;
                    self.indices.push(index);
                }
            };
            append_sector_triangle(world, &triangle, &mut append);
        }
        let priority_count = self.indices.len();

        {
            let mut append = |list: &[usize]| {
                for &index in list {
                    let Some(code) = self.scan_codes.get_mut(index) else {
                        continue;
                    };
                    if *code == scan_code {
                        continue;
                    }
                    *code = scan_code;
                    self.indices.push(index);
                }
            };
            if let Some([x1, x2, y1, y2]) = topdown_rect {
                append_sector_rect(world, x1, x2, y1, y2, &mut append);
            } else {
                if let Some(triangle) = scan_triangle {
                    append_sector_triangle(world, &triangle, &mut append);
                } else if let Some(camera) = cameras.first() {
                    append_sector_square(world, *camera, lod_distance, &mut append);
                }
                for camera in cameras.iter().copied().skip(1) {
                    append_sector_square(world, camera, lod_distance, &mut append);
                }
            }

            let collision_level_index = match collision_level {
                ELevelName::Industrial => 1,
                ELevelName::Commercial => 2,
                ELevelName::Suburban => 3,
                _ => 0,
            };
            if collision_level_index != 0 {
                append(&world.big_buildings_by_level[collision_level_index]);
            }
            append(&world.big_buildings_by_level[0]);
        }
        (&self.indices, priority_count)
    }

    #[cfg(test)]
    fn collect_priority_triangle(&mut self, world: &CWorld, triangle: Option<[[f32; 2]; 3]>) {
        self.priority_indices.clear();
        self.priority_stream_requests.clear();
        if self.priority_scan_codes.len() < world.entities.len() {
            self.priority_scan_codes.resize(world.entities.len(), 0);
        }
        self.priority_scan_code = self.priority_scan_code.wrapping_add(1);
        if self.priority_scan_code == 0 {
            self.priority_scan_codes.fill(0);
            self.priority_scan_code = 1;
        }

        let Some(triangle) = triangle else {
            return;
        };
        let scan_code = self.priority_scan_code;
        scan_sector_triangle(&triangle, |x, y| {
            let sector = &world.sectors[y * NUMSECTORS_X + x];
            for list in [
                sector.buildings.as_slice(),
                sector.buildings_overlap.as_slice(),
                sector.objects.as_slice(),
                sector.objects_overlap.as_slice(),
                sector.vehicles.as_slice(),
                sector.vehicles_overlap.as_slice(),
                sector.peds.as_slice(),
                sector.peds_overlap.as_slice(),
                sector.dummies.as_slice(),
                sector.dummies_overlap.as_slice(),
            ] {
                for &index in list {
                    let Some(code) = self.priority_scan_codes.get_mut(index) else {
                        continue;
                    };
                    if *code == scan_code {
                        continue;
                    }
                    *code = scan_code;
                    self.priority_indices.push(index);
                }
            }
        });
    }

    fn begin_plan(&mut self, entity_count: usize) {
        self.planned_indices.clear();
        if self.planned_passes.len() < entity_count {
            self.planned_passes.resize(entity_count, None);
            self.planned_distance_fade.resize(entity_count, false);
            self.planned_scan_codes.resize(entity_count, 0);
        }
        self.planned_scan_code = self.planned_scan_code.wrapping_add(1);
        if self.planned_scan_code == 0 {
            self.planned_scan_codes.fill(0);
            self.planned_scan_code = 1;
        }
    }

    fn insert_plan(&mut self, entity_index: usize, pass: GtaWorldRenderPass) {
        let Some(scan_code) = self.planned_scan_codes.get_mut(entity_index) else {
            return;
        };
        if *scan_code != self.planned_scan_code {
            *scan_code = self.planned_scan_code;
            self.planned_indices.push(entity_index);
        }
        self.planned_passes[entity_index] = Some(pass);
        self.planned_distance_fade[entity_index] = false;
    }

    fn mark_distance_fade(&mut self, entity_index: usize) {
        if self
            .planned_scan_codes
            .get(entity_index)
            .is_some_and(|scan_code| *scan_code == self.planned_scan_code)
        {
            self.planned_distance_fade[entity_index] = true;
        }
    }

    fn contains_plan(&self, entity_index: usize) -> bool {
        self.planned_scan_codes
            .get(entity_index)
            .is_some_and(|scan_code| *scan_code == self.planned_scan_code)
    }
}

fn append_sector_square(
    world: &CWorld,
    camera: CVector,
    lod_distance: f32,
    append: &mut impl FnMut(&[usize]),
) {
    let radius = lod_distance + FADE_DISTANCE + STREAM_DISTANCE;
    let x0 = CWorld::get_sector_index_x(camera.x - radius);
    let x1 = CWorld::get_sector_index_x(camera.x + radius);
    let y0 = CWorld::get_sector_index_y(camera.y - radius);
    let y1 = CWorld::get_sector_index_y(camera.y + radius);
    for y in y0..=y1.min(NUMSECTORS_Y - 1) {
        for x in x0..=x1.min(NUMSECTORS_X - 1) {
            append_world_sector_lists(&world.sectors[y * NUMSECTORS_X + x], append);
        }
    }
}

fn append_sector_triangle(
    world: &CWorld,
    triangle: &[[f32; 2]; 3],
    append: &mut impl FnMut(&[usize]),
) {
    scan_sector_triangle(triangle, |x, y| {
        append_world_sector_lists(&world.sectors[y * NUMSECTORS_X + x], append);
    });
}

fn two_dfx_light_state(
    light_type: i32,
    is_night: bool,
    wet_roads: f32,
    time_ms: u32,
    random_seed: u16,
    effect_index: usize,
    bridge_flashing: bool,
) -> (bool, bool) {
    let flicker = |seed: u32| {
        let mut on = ((time_ms ^ seed) & 0x60) != 0;
        let flickering = !on;
        if (((time_ms >> 11) ^ seed) & 3) != 0 {
            on = true;
        }
        (on, flickering)
    };
    let flash1 = time_ms.wrapping_add((effect_index as u32).wrapping_mul(0x80)) & 0x200 != 0;
    let flash2 = time_ms.wrapping_add((effect_index as u32).wrapping_mul(0x100)) & 0x400 != 0;
    let flash3 = time_ms.wrapping_add((effect_index as u32).wrapping_mul(0x200)) & 0x800 != 0;
    match light_type {
        0 => (true, false),
        1 => (is_night, false),
        2 => flicker(u32::from(random_seed)),
        3 if is_night || wet_roads > 0.5 => flicker(u32::from(random_seed)),
        3 => (false, false),
        4 => (flash1, false),
        5 => (is_night && flash1, false),
        6 => (flash2, false),
        7 => (is_night && flash2, false),
        8 => (flash3, false),
        9 => (is_night && flash3, false),
        10 if random_seed > 16 => (true, false),
        10 => flicker(u32::from(random_seed) * 8),
        11 if !is_night => (false, false),
        11 if random_seed > 16 => (true, false),
        11 => flicker(u32::from(random_seed) * 8),
        13 => (bridge_flashing && time_ms & 0x200 != 0, false),
        14 => (bridge_flashing && (time_ms & 0x1FF) < 60, false),
        _ => (false, false),
    }
}

fn corona_texture_type(name: &str) -> u8 {
    super::Coronas::CORONA_SPRITE_NAMES
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(name))
        .map(|index| index as u8)
        .unwrap_or(super::Coronas::TYPE_STAR)
}

fn shadow_texture_type(shadows: &CShadows, name: &str) -> Option<u8> {
    shadows
        .texture_names
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(name))
        .and_then(|index| u8::try_from(index).ok())
}

/// re3 `CRadar::Draw3dMarkers`.
///
/// SCM owns only radar/blip state. This render-prep adapter resolves the live
/// Bevy entities and refreshes the transient C3dMarker arrow exactly where re3
/// does, keeping entity ownership one-way and avoiding a second gameplay pool.
fn emit_radar_3d_markers(
    file_loader: Option<Res<CFileLoader>>,
    script_entities: Option<RuntimeActorsRead>,
    camera: Option<Res<CCamera>>,
    players: CWorldPlayersRead,
    hud: Option<Res<CHud>>,
    timer: Option<Res<CTimer>>,
    mut special_fx: Option<ResMut<CSpecialFX>>,
) {
    let (Some(file_loader), Some(script_entities), Some(special_fx)) =
        (file_loader, script_entities, special_fx.as_deref_mut())
    else {
        return;
    };
    if hud
        .as_ref()
        .is_some_and(|hud| !hud.wants_to_draw_3d_markers)
    {
        for marker in &mut special_fx.markers {
            if marker.identifier & RADAR_3D_MARKER_MASK == RADAR_3D_MARKER_PREFIX {
                *marker = Default::default();
            }
        }
        return;
    }

    let player_pos = players
        .player(0)
        .filter(|p| p.active_in_world)
        .map(|p| p.ped.physical.entity.get_position())
        .or_else(|| camera.as_ref().map(|c| c.position))
        .unwrap_or(CVector::ZERO);
    let now = timer
        .as_ref()
        .map_or(0, |timer| timer.time_in_milliseconds());
    let mut active = Vec::with_capacity(crate::core::Radar::NUMRADARBLIPS);
    for (index, trace) in file_loader.radar.radar_trace.iter().enumerate() {
        use crate::core::Radar::{eBlipDisplay, eBlipType};
        if !trace.in_use
            || !matches!(
                trace.blip_display,
                eBlipDisplay::BLIP_DISPLAY_BOTH | eBlipDisplay::BLIP_DISPLAY_MARKER_ONLY
            )
        {
            continue;
        }

        let marker_id = RADAR_3D_MARKER_PREFIX
            | ((u32::from(trace.blip_index) & 0x0FFF) << 8)
            | (index as u32 & 0xFF);
        let marker = match trace.blip_type {
            eBlipType::BLIP_CAR => {
                script_entities
                    .vehicle_get(trace.entity_handle)
                    .map(|snapshot| {
                        let max_z = usize::try_from(snapshot.base().physical.entity.model_index)
                            .ok()
                            .and_then(|model| file_loader.model_info.col_model(model))
                            .map_or(0.0, |col| col.bounding_box.max.z);
                        let mut position = snapshot.base().physical.entity.get_position();
                        // re3 CRadar.cpp: pos.z += 1.2f * colModel->boundingBox.max.z + 2.5f;
                        position.z += 1.2 * max_z + 2.5;
                        (position, 2.5)
                    })
            }
            eBlipType::BLIP_CHAR => {
                script_entities
                    .ped_get(trace.entity_handle)
                    .and_then(|snapshot| {
                        let entity_position = script_entities
                            .relation_for_char(trace.entity_handle)
                            .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle))
                            .and_then(|handle| script_entities.vehicle_get(handle))
                            .map_or(snapshot.ped.physical.entity.get_position(), |vehicle| vehicle.base().physical.entity.get_position());
                        // re3 CRadar.cpp: pos.z += 3.0f;
                        Some((
                            CVector::new(
                                entity_position.x,
                                entity_position.y,
                                entity_position.z + 3.0,
                            ),
                            1.5,
                        ))
                    })
            }
            eBlipType::BLIP_OBJECT => {
                script_entities
                    .object_get(trace.entity_handle)
                    .map(|object| {
                        let entity = &object.object.physical.entity;
                        let max_z = usize::try_from(entity.model_index)
                            .ok()
                            .and_then(|model| file_loader.model_info.col_model(model))
                            .map_or(0.0, |col| col.bounding_box.max.z);
                        let mut position = entity.get_position();
                        // re3 CRadar.cpp: pos.z += colModel->boundingBox.max.z + 1.0f + 1.0f;
                        position.z += max_z + 2.0;
                        (position, 1.0)
                    })
            }
            eBlipType::BLIP_COORD | eBlipType::BLIP_CONTACT_POINT | eBlipType::BLIP_NONE => None,
        };
        let Some((position, base_size)) = marker else {
            continue;
        };
        active.push(marker_id);

        // re3 C3dMarkers::PlaceMarker:
        // float dist = Sqrt((pos.x - FindPlayerCentreOfWorld(0).x)^2 + (pos.y - FindPlayerCentreOfWorld(0).y)^2);
        // re3 CRadar::Draw3dMarkers:
        // C3dMarkers::PlaceMarker(..., MARKERTYPE_ARROW, pos, ..., 0, 128, 255, 255, 1024, 0.2f, 5);
        special_fx.place_marker(
            marker_id,
            MARKERTYPE_ARROW,
            position,
            base_size,
            0,
            128,
            255,
            255,
            1024,
            0.2,
            5,
            now,
            player_pos,
            timer.as_ref().map_or(0.0, |timer| timer.time_step()),
        );
    }

    for marker in &mut special_fx.markers {
        if marker.identifier & RADAR_3D_MARKER_MASK == RADAR_3D_MARKER_PREFIX
            && !active.contains(&marker.identifier)
        {
            *marker = Default::default();
        }
    }
}

/// re3 `CPed::PreRender -> CShadows::StoreShadowForPed` and
/// `CAutomobile::PreRender -> CShadows::StoreShadowForCar`.
///
/// The authoritative actors stay in ECS. This system only derives the
/// transient projected-shadow intents consumed by the renderer later in the
/// same frame.
fn prepare_runtime_contact_shadows(
    file_loader: Option<Res<CFileLoader>>,
    timecycle: Option<Res<CTimeCycle>>,
    camera: Option<Res<CCamera>>,
    players: CWorldPlayersRead,
    script_entities: Option<RuntimeActorsRead>,
    mut shadows: Option<ResMut<CShadows>>,
) {
    let (Some(file_loader), Some(timecycle), Some(camera), Some(shadows)) =
        (file_loader, timecycle, camera, shadows.as_deref_mut())
    else {
        return;
    };
    let base_strength = (timecycle.current.shadow_strength.clamp(0, 255) as f32).max(150.0) as i16;
    let slot = (timecycle.current_stored_value as usize) & 0xF;

    let probe_ground_z = |cx: f32, cy: f32, fallback_z: f32| -> f32 {
        file_loader
            .world
            .find_ground_z_for_3d_coord(cx, cy, fallback_z + 1.5, &file_loader.model_info)
            .or_else(|| {
                file_loader
                    .world
                    .process_vertical_line_static(
                        [cx, cy, fallback_z + 1.5],
                        fallback_z - 3.0,
                        &file_loader.model_info,
                    )
                    .map(|hit| hit.point.z)
            })
            .unwrap_or(fallback_z)
    };

    // Ped shadows: re3 intentionally uses a 26-unit nominal draw distance but
    // rejects at half that distance in StoreShadowForPedObject.
    for player_index in 0..crate::peds::PlayerPed::MAX_PLAYERS {
        if players.relation(player_index as i32).is_some() {
            continue;
        }
        let Some(player) = players.player(player_index as i32) else {
            continue;
        };
        if !player.active_in_world || !player.ped.physical.entity.flags.is_visible {
            continue;
        }
        let mut position = player.position();
        let dx = position.x - camera.position.x;
        let dy = position.y - camera.position.y;
        let distance_sq = dx * dx + dy * dy;
        if distance_sq >= 13.0 * 13.0 {
            continue;
        }
        let distance = distance_sq.sqrt();
        let multiplier = if distance >= 6.5 {
            1.0 - (4.0 / 26.0) * (distance - 6.5)
        } else {
            1.0
        };
        let strength = ((base_strength as f32) * multiplier)
            .round()
            .clamp(0.0, 255.0) as i16;
        if strength <= 0 {
            continue;
        }
        position.x += timecycle.shadow_displacement_x[slot];
        position.y += timecycle.shadow_displacement_y[slot];
        let ground_z = probe_ground_z(position.x, position.y, position.z);
        position.z = ground_z + 0.04;

        let front_x = timecycle.shadow_front_x[slot];
        let front_y = timecycle.shadow_front_y[slot];
        let side_x = timecycle.shadow_side_x[slot];
        let side_y = timecycle.shadow_side_y[slot];

        let c0_x = position.x + front_x - side_x;
        let c0_y = position.y + front_y - side_y;
        let c1_x = position.x + front_x + side_x;
        let c1_y = position.y + front_y + side_y;
        let c2_x = position.x - front_x + side_x;
        let c2_y = position.y - front_y + side_y;
        let c3_x = position.x - front_x - side_x;
        let c3_y = position.y - front_y - side_y;

        let z0 = probe_ground_z(c0_x, c0_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;
        let z1 = probe_ground_z(c1_x, c1_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;
        let z2 = probe_ground_z(c2_x, c2_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;
        let z3 = probe_ground_z(c3_x, c3_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;

        let corners = [
            CVector::new(c0_x, c0_y, z0),
            CVector::new(c1_x, c1_y, z1),
            CVector::new(c2_x, c2_y, z2),
            CVector::new(c3_x, c3_y, z3),
        ];

        shadows.script_shadows.push(ScriptShadow {
            // eShadowTextureType::SHADOWTEX_PED
            texture: 2,
            position,
            front: [front_x, front_y],
            side: [side_x, side_y],
            intensity: strength,
            color: [strength as u8; 3],
            corners: Some(corners),
        });
    }

    let Some(script_entities) = script_entities else {
        return;
    };
    for (_, vehicle) in script_entities.vehicles() {
        let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
            continue;
        };
        let base = &car.vehicle;
        let entity = &base.physical.entity;
        if !entity.flags.is_visible {
            continue;
        }
        let mut position = entity.get_position();
        let dx = position.x - camera.position.x;
        let dy = position.y - camera.position.y;
        let distance_sq = dx * dx + dy * dy;
        if distance_sq >= 18.0 * 18.0 {
            continue;
        }
        let distance = distance_sq.sqrt();
        let multiplier = if distance >= 13.5 {
            1.0 - (4.0 / 18.0) * (distance - 13.5)
        } else {
            1.0
        };
        let strength = ((base_strength as f32) * multiplier)
            .round()
            .clamp(0.0, 255.0) as i16;
        if strength <= 0 {
            continue;
        }
        let Some(model_index) = usize::try_from(entity.model_index).ok() else {
            continue;
        };
        let Some(col) = file_loader.model_info.col_model(model_index) else {
            continue;
        };
        let mut vehicle_height = col.bounding_box.max.y - col.bounding_box.min.y;
        let mut vehicle_width = col.bounding_box.max.x - col.bounding_box.min.x;
        if model_index == MI_DODO {
            vehicle_height *= 0.9;
            vehicle_width *= 0.4;
        }
        let forward = entity.get_forward();
        let right = entity.get_right();
        position.x -= forward.x * ((vehicle_height * 0.5) - col.bounding_box.max.y);
        position.y -= forward.y * ((vehicle_height * 0.5) - col.bounding_box.max.y);
        let ground_z = file_loader
            .world
            .find_ground_z_for_3d_coord(
                position.x,
                position.y,
                position.z + 1.0,
                &file_loader.model_info,
            )
            .or_else(|| {
                file_loader
                    .world
                    .process_vertical_line_static(
                        [position.x, position.y, position.z + 1.0],
                        position.z + col.bounding_box.min.z - 1.0,
                        &file_loader.model_info,
                    )
                    .map(|hit| hit.point.z)
            })
            .unwrap_or(position.z + col.bounding_box.min.z);
        position.z = ground_z + 0.04;
        let side_sign = if entity.get_up().z > 0.0 { 1.0 } else { -1.0 };
        let front_x = forward.x * vehicle_height * 0.5;
        let front_y = forward.y * vehicle_height * 0.5;
        let side_x = side_sign * right.x * vehicle_width * 0.5;
        let side_y = side_sign * right.y * vehicle_width * 0.5;

        let c0_x = position.x + front_x - side_x;
        let c0_y = position.y + front_y - side_y;
        let c1_x = position.x + front_x + side_x;
        let c1_y = position.y + front_y + side_y;
        let c2_x = position.x - front_x + side_x;
        let c2_y = position.y - front_y + side_y;
        let c3_x = position.x - front_x - side_x;
        let c3_y = position.y - front_y - side_y;

        let z0 = probe_ground_z(c0_x, c0_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;
        let z1 = probe_ground_z(c1_x, c1_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;
        let z2 = probe_ground_z(c2_x, c2_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;
        let z3 = probe_ground_z(c3_x, c3_y, ground_z).clamp(ground_z - 1.0, ground_z + 1.0) + 0.04;

        let corners = [
            CVector::new(c0_x, c0_y, z0),
            CVector::new(c1_x, c1_y, z1),
            CVector::new(c2_x, c2_y, z2),
            CVector::new(c3_x, c3_y, z3),
        ];

        shadows.script_shadows.push(ScriptShadow {
            // eShadowTextureType::SHADOWTEX_CAR
            texture: 1,
            position,
            front: [front_x, front_y],
            side: [side_x, side_y],
            intensity: strength,
            color: [strength as u8; 3],
            corners: Some(corners),
        });
    }
}

/// Exact Rust translation of re3 `CRenderer::ScanSectorPoly` for the triangle
/// used by `ScanWorld`.
///
/// The original does not perform an exact triangle-vs-cell intersection. It
/// rasterises inclusive horizontal sector spans with `Floor`, deliberately
/// touching a few conservative boundary cells. Replacing that with SAT made
/// the Bevy candidate set narrower than C++ and could hide a large road/world
/// entity whose primary/overlap list lived in one of those boundary sectors.
fn scan_sector_triangle(triangle: &[[f32; 2]; 3], mut scan: impl FnMut(usize, usize)) {
    #[inline]
    fn to_int(value: f32) -> i32 {
        value.floor() as i32
    }

    #[inline]
    fn calc_delta(a: [f32; 2], b: [f32; 2]) -> f32 {
        (b[0] - a[0]) / (b[1] - a[1])
    }

    const N: usize = 3;
    let mut min_y = triangle[0][1];
    let mut max_y = triangle[0][1];
    let mut a2 = 0usize;
    let mut x_start = 9999i32;
    let mut x_end = -9999i32;

    for (i, point) in triangle.iter().enumerate().skip(1) {
        if point[1] > max_y {
            max_y = point[1];
        }
        if point[1] < min_y {
            min_y = point[1];
            a2 = i;
        }
    }
    let mut y = to_int(min_y);
    let y_end = to_int(max_y);

    // Go left in poly to find first edge b.
    let mut b2 = a2;
    let mut b1 = b2;
    for _ in 0..N {
        b1 = b2;
        b2 = if b2 == 0 { N - 1 } else { b2 - 1 };
        x_start = x_start.min(to_int(triangle[b1][0]));
        if to_int(triangle[b1][1]) != to_int(triangle[b2][1]) {
            break;
        }
    }

    // Go right to find first edge a.
    let mut a1 = a2;
    for _ in 0..N {
        a1 = a2;
        a2 = (a2 + 1) % N;
        x_end = x_end.max(to_int(triangle[a1][0]));
        if to_int(triangle[a1][1]) != to_int(triangle[a2][1]) {
            break;
        }
    }

    // Prestep both edges to the next integer y, like RenderWare/re3.
    let mut delta_a = calc_delta(triangle[a1], triangle[a2]);
    let mut x_a = delta_a * (triangle[a1][1].ceil() - triangle[a1][1]) + triangle[a1][0];
    let mut delta_b = calc_delta(triangle[b1], triangle[b2]);
    let mut x_b = delta_b * (triangle[b1][1].ceil() - triangle[b1][1]) + triangle[b1][0];

    if y != y_end {
        if delta_b < 0.0 && to_int(x_b) < x_start {
            x_start = to_int(x_b);
        }
        if delta_a >= 0.0 && to_int(x_a) > x_end {
            x_end = to_int(x_a);
        }
    }

    while y <= y_end && y < NUMSECTORS_Y as i32 {
        if y >= 0 && x_start < NUMSECTORS_X as i32 {
            let first_x = x_start.max(0);
            let last_x = x_end.min(NUMSECTORS_X as i32 - 1);
            if first_x <= last_x {
                for x in first_x..=last_x {
                    scan(x as usize, y as usize);
                }
            }
        }

        y += 1;
        x_a += delta_a;
        x_b += delta_b;

        // Update left side.
        if y == to_int(triangle[b2][1]) {
            if y == y_end {
                if delta_b < 0.0 {
                    loop {
                        x_start = to_int(triangle[b2][0]);
                        b2 = if b2 == 0 { N - 1 } else { b2 - 1 };
                        if x_start <= to_int(triangle[b2][0]) {
                            break;
                        }
                    }
                } else {
                    x_start = to_int(x_b - delta_b);
                }
            } else {
                x_start = if delta_b < 0.0 {
                    to_int(triangle[b2][0])
                } else {
                    to_int(x_b - delta_b)
                };
                loop {
                    b1 = b2;
                    b2 = if b2 == 0 { N - 1 } else { b2 - 1 };
                    x_start = x_start.min(to_int(triangle[b1][0]));
                    if y != to_int(triangle[b2][1]) {
                        break;
                    }
                }
                delta_b = calc_delta(triangle[b1], triangle[b2]);
                x_b = delta_b * (triangle[b1][1].ceil() - triangle[b1][1]) + triangle[b1][0];
                if delta_b < 0.0 && to_int(x_b) < x_start {
                    x_start = to_int(x_b);
                }
            }
        } else if delta_b < 0.0 {
            x_start = to_int(x_b);
        } else {
            x_start = to_int(x_b - delta_b);
        }

        // Update right side.
        if y == to_int(triangle[a2][1]) {
            if y == y_end {
                if delta_a < 0.0 {
                    x_end = to_int(x_a - delta_a);
                } else {
                    loop {
                        x_end = to_int(triangle[a2][0]);
                        a2 = (a2 + 1) % N;
                        if x_end >= to_int(triangle[a2][0]) {
                            break;
                        }
                    }
                }
            } else {
                x_end = if delta_a < 0.0 {
                    to_int(x_a - delta_a)
                } else {
                    to_int(triangle[a2][0])
                };
                loop {
                    a1 = a2;
                    a2 = (a2 + 1) % N;
                    x_end = x_end.max(to_int(triangle[a1][0]));
                    if y != to_int(triangle[a2][1]) {
                        break;
                    }
                }
                delta_a = calc_delta(triangle[a1], triangle[a2]);
                x_a = delta_a * (triangle[a1][1].ceil() - triangle[a1][1]) + triangle[a1][0];
                if delta_a >= 0.0 && to_int(x_a) > x_end {
                    x_end = to_int(x_a);
                }
            }
        } else if delta_a < 0.0 {
            x_end = to_int(x_a - delta_a);
        } else {
            x_end = to_int(x_a);
        }
    }
}

fn append_world_sector_lists(
    sector: &crate::core::World::CWorldSector,
    append: &mut impl FnMut(&[usize]),
) {
    append(&sector.buildings);
    append(&sector.buildings_overlap);
    append(&sector.objects);
    append(&sector.objects_overlap);
    append(&sector.vehicles);
    append(&sector.vehicles_overlap);
    append(&sector.peds);
    append(&sector.peds_overlap);
    append(&sector.dummies);
    append(&sector.dummies_overlap);
}

fn append_sector_rect(
    world: &CWorld,
    x1: usize,
    x2: usize,
    y1: usize,
    y2: usize,
    append: &mut impl FnMut(&[usize]),
) {
    let x_end = x2.min(NUMSECTORS_X.saturating_sub(1));
    let y_end = y2.min(NUMSECTORS_Y.saturating_sub(1));
    for y in y1..=y_end {
        for x in x1..=x_end {
            append_world_sector_lists(&world.sectors[y * NUMSECTORS_X + x], append);
        }
    }
}

fn renderer_is_topdown(camera: &CCamera) -> bool {
    let Some(cam) = camera.cams.get(camera.active_cam) else {
        return false;
    };
    matches!(
        cam.mode,
        crate::core::Cam::eCamMode::MODE_TOPDOWN
            | crate::core::Cam::eCamMode::MODE_GTACLASSIC
            | crate::core::Cam::eCamMode::MODE_TOP_DOWN_PED
    )
}

fn renderer_topdown_sector_rect_from_basis(
    front: CVector,
    up_hint: CVector,
    source: CVector,
    horizontal_fov: f32,
    aspect_ratio: f32,
    far_clip: f32,
) -> Option<[usize; 4]> {
    if far_clip <= 0.0 || aspect_ratio <= f32::EPSILON {
        return None;
    }
    let front_len = front.magnitude();
    let up_len = up_hint.magnitude();
    if front_len <= f32::EPSILON || up_len <= f32::EPSILON {
        return None;
    }
    let front = CVector::new(
        front.x / front_len,
        front.y / front_len,
        front.z / front_len,
    );
    let up_hint = CVector::new(up_hint.x / up_len, up_hint.y / up_len, up_hint.z / up_len);
    // CCamera::Process writes the RenderWare camera frame with
    // `GetRight() = CrossProduct(CamUp, CamFront)` (Camera.cpp:657).  That
    // vector is intentionally the engine's matrix "right"/left axis. Using
    // Front x Up mirrors the frustum polygon winding; exact cell intersection
    // hid that mistake, but re3's ScanSectorPoly is winding-sensitive.
    let right_raw = up_hint.cross(front);
    let right_len = right_raw.magnitude();
    if right_len <= f32::EPSILON {
        return None;
    }
    let right = CVector::new(
        right_raw.x / right_len,
        right_raw.y / right_len,
        right_raw.z / right_len,
    );
    let up_raw = front.cross(right);
    let up_len = up_raw.magnitude();
    if up_len <= f32::EPSILON {
        return None;
    }
    let up = CVector::new(up_raw.x / up_len, up_raw.y / up_len, up_raw.z / up_len);

    let effective_fov = horizontal_fov.max(70.0);
    let effective_aspect = aspect_ratio.max(4.0 / 3.0);
    let view_window_x = (effective_fov.to_radians() * 0.5).tan();
    let view_window_y = view_window_x / effective_aspect;

    // re3 `CRenderer::ScanWorld`: in topdown modes, the four far frustum corners
    // are projected down to ground level `z = -100.0f` via `LimitFrustumVector`,
    // and all sectors within the resulting bounding rectangle are scanned.
    let f = far_clip;
    let corners = [
        // CORNER_FAR_TOPLEFT: -vw.x * f, vw.y * f, f
        CVector::new(
            source.x - right.x * view_window_x * f + up.x * view_window_y * f + front.x * f,
            source.y - right.y * view_window_x * f + up.y * view_window_y * f + front.y * f,
            source.z - right.z * view_window_x * f + up.z * view_window_y * f + front.z * f,
        ),
        // CORNER_FAR_TOPRIGHT: vw.x * f, vw.y * f, f
        CVector::new(
            source.x + right.x * view_window_x * f + up.x * view_window_y * f + front.x * f,
            source.y + right.y * view_window_x * f + up.y * view_window_y * f + front.y * f,
            source.z + right.z * view_window_x * f + up.z * view_window_y * f + front.z * f,
        ),
        // CORNER_FAR_BOTRIGHT: vw.x * f, -vw.y * f, f
        CVector::new(
            source.x + right.x * view_window_x * f - up.x * view_window_y * f + front.x * f,
            source.y + right.y * view_window_x * f - up.y * view_window_y * f + front.y * f,
            source.z + right.z * view_window_x * f - up.z * view_window_y * f + front.z * f,
        ),
        // CORNER_FAR_BOTLEFT: -vw.x * f, -vw.y * f, f
        CVector::new(
            source.x - right.x * view_window_x * f - up.x * view_window_y * f + front.x * f,
            source.y - right.y * view_window_x * f - up.y * view_window_y * f + front.y * f,
            source.z - right.z * view_window_x * f - up.z * view_window_y * f + front.z * f,
        ),
    ];

    let ground_l = -100.0f32;
    let mut min_x = source.x;
    let mut max_x = source.x;
    let mut min_y = source.y;
    let mut max_y = source.y;

    for mut pt in corners {
        let denom = pt.z - source.z;
        // re3 `LimitFrustumVector` is an unclamped line/plane projection.
        // The previous 0..10 clamp silently shortened high/steep top-down
        // frusta and could leave unscanned sector strips at the ground plane.
        let scale = (ground_l - source.z) / denom;
        pt.x = scale * (pt.x - source.x) + source.x;
        pt.y = scale * (pt.y - source.y) + source.y;
        min_x = min_x.min(pt.x);
        max_x = max_x.max(pt.x);
        min_y = min_y.min(pt.y);
        max_y = max_y.max(pt.y);
    }

    let x1 = CWorld::get_sector_index_x(min_x);
    let x2 = CWorld::get_sector_index_x(max_x);
    let y1 = CWorld::get_sector_index_y(min_y);
    let y2 = CWorld::get_sector_index_y(max_y);
    Some([x1, x2, y1, y2])
}

#[cfg(test)]
fn renderer_scan_sector_triangle(
    camera: &CCamera,
    source: CVector,
    horizontal_fov: f32,
    aspect_ratio: f32,
    lod_distance: f32,
    far_clip: f32,
) -> Option<[[f32; 2]; 3]> {
    let cam = camera.cams.get(camera.active_cam)?;
    renderer_scan_sector_triangle_from_basis(
        cam.front,
        cam.up,
        source,
        horizontal_fov,
        aspect_ratio,
        lod_distance,
        far_clip,
    )
}

fn renderer_scan_sector_triangle_from_basis(
    front: CVector,
    up: CVector,
    source: CVector,
    horizontal_fov: f32,
    aspect_ratio: f32,
    lod_distance: f32,
    far_clip: f32,
) -> Option<[[f32; 2]; 3]> {
    if far_clip <= 0.0 || lod_distance <= 0.0 || aspect_ratio <= f32::EPSILON {
        return None;
    }
    let distance = far_clip.min(lod_distance);
    renderer_sector_triangle_from_basis(
        front,
        up,
        source,
        horizontal_fov,
        aspect_ratio,
        distance,
        1.0,
        far_clip > lod_distance,
    )
}

fn renderer_sector_triangle_from_basis(
    front: CVector,
    up_hint: CVector,
    source: CVector,
    horizontal_fov: f32,
    aspect_ratio: f32,
    distance: f32,
    lateral_scale: f32,
    use_lod_corner_rule: bool,
) -> Option<[[f32; 2]; 3]> {
    if distance <= 0.0 || aspect_ratio <= f32::EPSILON {
        return None;
    }
    let front_len = front.magnitude();
    let up_len = up_hint.magnitude();
    if front_len <= f32::EPSILON || up_len <= f32::EPSILON {
        return None;
    }
    let front = CVector::new(
        front.x / front_len,
        front.y / front_len,
        front.z / front_len,
    );
    let up_hint = CVector::new(up_hint.x / up_len, up_hint.y / up_len, up_hint.z / up_len);
    // Match the actual RenderWare camera matrix used by
    // `RwV3dTransformPoints` in CRenderer::ScanWorld.
    let right_raw = up_hint.cross(front);
    let right_len = right_raw.magnitude();
    if right_len <= f32::EPSILON {
        return None;
    }
    let right = CVector::new(
        right_raw.x / right_len,
        right_raw.y / right_len,
        right_raw.z / right_len,
    );
    let up_raw = front.cross(right);
    let up_len = up_raw.magnitude();
    if up_len <= f32::EPSILON {
        return None;
    }
    let up = CVector::new(up_raw.x / up_len, up_raw.y / up_len, up_raw.z / up_len);

    // In narrow or tall viewports (aspect < 4:3), ensure sector scanning covers
    // at least the standard 4:3 frustum cone so adjacent sectors are never skipped.
    let effective_fov = horizontal_fov.max(70.0);
    let effective_aspect = aspect_ratio.max(4.0 / 3.0);
    let view_window_x = (effective_fov.to_radians() * 0.5).tan();
    let view_window_y = view_window_x / effective_aspect;
    let local_x = view_window_x * distance * lateral_scale;
    let local_y_sign = if use_lod_corner_rule && front.z > 0.0 {
        -1.0
    } else {
        1.0
    };
    let local_y = local_y_sign * view_window_y * distance * lateral_scale;
    let world_point = |x: f32| {
        CVector::new(
            source.x + right.x * x + up.x * local_y + front.x * distance,
            source.y + right.y * x + up.y * local_y + front.y * distance,
            source.z + right.z * x + up.z * local_y + front.z * distance,
        )
    };
    let left = world_point(-local_x);
    let right = world_point(local_x);
    let sector_point = |point: CVector| {
        [
            (point.x - WORLD_MIN_X) / SECTOR_SIZE_X,
            (point.y - WORLD_MIN_Y) / SECTOR_SIZE_Y,
        ]
    };
    Some([
        sector_point(source),
        sector_point(left),
        sector_point(right),
    ])
}

#[cfg(test)]
fn renderer_priority_sector_triangle(
    camera: &CCamera,
    source: CVector,
    horizontal_fov: f32,
    aspect_ratio: f32,
    lod_distance: f32,
    far_clip: f32,
) -> Option<[[f32; 2]; 3]> {
    let cam = camera.cams.get(camera.active_cam)?;
    renderer_priority_sector_triangle_from_basis(
        cam.front,
        cam.up,
        source,
        horizontal_fov,
        aspect_ratio,
        lod_distance,
        far_clip,
    )
}

fn renderer_priority_sector_triangle_from_basis(
    front: CVector,
    up: CVector,
    source: CVector,
    horizontal_fov: f32,
    aspect_ratio: f32,
    lod_distance: f32,
    far_clip: f32,
) -> Option<[[f32; 2]; 3]> {
    if far_clip <= lod_distance || lod_distance <= 0.0 || aspect_ratio <= f32::EPSILON {
        return None;
    }
    // Renderer.cpp ScanWorld builds CORNER_PRIO_LEFT/RIGHT from the LOD
    // frustum: X/Y are narrowed to 20%, while Z stays at LOD_DISTANCE.
    renderer_sector_triangle_from_basis(
        front,
        up,
        source,
        horizontal_fov,
        aspect_ratio,
        lod_distance,
        0.2,
        true,
    )
}

/// re3 `Renderer.cpp:281` (`IsRoad`): a building treadable. Culled roads keep
/// their model request in `ScanSectorList`/`ScanSectorList_Priority` without
/// entering the render lists.
fn is_scan_road_entity(entity: &CWorldEntity) -> bool {
    matches!(
        entity.entity_type,
        crate::core::World::EEntityType::Building
    ) && entity.treadable_pool_slot.is_some()
}

#[inline(always)]
fn nearest_camera_position(
    camera_positions: &[CVector],
    entity_pos: CVector,
    fallback: CVector,
) -> CVector {
    match camera_positions.len() {
        0 => fallback,
        1 => camera_positions[0],
        _ => camera_positions
            .iter()
            .copied()
            .min_by(|left, right| {
                let d1 = (entity_pos.x - left.x).powi(2)
                    + (entity_pos.y - left.y).powi(2)
                    + (entity_pos.z - left.z).powi(2);
                let d2 = (entity_pos.x - right.x).powi(2)
                    + (entity_pos.y - right.y).powi(2)
                    + (entity_pos.z - right.z).powi(2);
                d1.total_cmp(&d2)
            })
            .unwrap_or(fallback),
    }
}

#[inline(always)]
fn nearest_camera_frustum<'a>(
    camera_frustums: Option<&'a [CameraFrustum]>,
    entity_pos: CVector,
) -> Option<&'a CameraFrustum> {
    let frustums = camera_frustums?;
    match frustums.len() {
        0 => None,
        1 => Some(&frustums[0]),
        _ => frustums.iter().min_by(|left, right| {
            let d1 = (entity_pos.x - left.source.x).powi(2)
                + (entity_pos.y - left.source.y).powi(2)
                + (entity_pos.z - left.source.z).powi(2);
            let d2 = (entity_pos.x - right.source.x).powi(2)
                + (entity_pos.y - right.source.y).powi(2)
                + (entity_pos.z - right.source.z).powi(2);
            d1.total_cmp(&d2)
        }),
    }
}

/// The RenderWare renderer consumes the matrix and view window of the camera
/// that will actually rasterise the frame. At the Bevy boundary the local
/// Transform + PerspectiveProjection are that same source of truth.
///
/// Reconstructing these values from CCamera creates a second camera which is
/// usually identical during gameplay, but can diverge during spline,
/// interpolation and cutscene transitions. Anything visible through the live
/// raster must be admitted by ScanWorld and the on-screen test.
fn render_camera_basis(transform: &Transform) -> (CVector, CVector) {
    (
        CVector::from_bevy(transform.rotation * Vec3::NEG_Z),
        CVector::from_bevy(transform.rotation * Vec3::Y),
    )
}

fn perspective_horizontal_fov_degrees(perspective: &PerspectiveProjection) -> f32 {
    let half_vertical = perspective.fov * 0.5;
    (2.0 * (half_vertical.tan() * perspective.aspect_ratio.max(1.0e-4)).atan()).to_degrees()
}

fn render_camera_frustum(
    transform: &Transform,
    perspective: &PerspectiveProjection,
) -> Option<CameraFrustum> {
    let source = CVector::from_bevy(transform.translation);
    let (mut front, mut up) = render_camera_basis(transform);
    let front_len = front.magnitude();
    let up_len = up.magnitude();
    if front_len <= f32::EPSILON || up_len <= f32::EPSILON {
        return None;
    }
    front = CVector::new(
        front.x / front_len,
        front.y / front_len,
        front.z / front_len,
    );
    up = CVector::new(up.x / up_len, up.y / up_len, up.z / up_len);

    // CameraFrustum uses +X as camera-right. RenderWare stores the opposite
    // vector in its historically misnamed GetRight field.
    let mut right = front.cross(up);
    let right_len = right.magnitude();
    if right_len <= f32::EPSILON {
        return None;
    }
    right = CVector::new(
        right.x / right_len,
        right.y / right_len,
        right.z / right_len,
    );
    up = right.cross(front);
    let up_len = up.magnitude();
    if up_len <= f32::EPSILON {
        return None;
    }
    up = CVector::new(up.x / up_len, up.y / up_len, up.z / up_len);

    let half_h = perspective_horizontal_fov_degrees(perspective).to_radians() * 0.5;
    let half_v = perspective.fov * 0.5;
    let (sin_h, cos_h) = half_h.sin_cos();
    let (sin_v, cos_v) = half_v.sin_cos();

    Some(CameraFrustum {
        source,
        right,
        front,
        up,
        near_clip: perspective.near,
        far_clip: perspective.far,
        cos_h,
        sin_h,
        cos_v,
        sin_v,
    })
}

fn flush_world_render_pass(
    file_loader: Option<ResMut<CFileLoader>>,
    mut streaming: Option<ResMut<crate::core::Streaming::CStreaming>>,
    timer: Option<Res<CTimer>>,
    clock: Option<Res<CClock>>,
    timecycle: Option<Res<CTimeCycle>>,
    the_camera: Option<Res<CCamera>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    // `follow_script_player_camera` writes the local Transform in the Camera
    // schedule.  Bevy only propagates GlobalTransform in PostUpdate, after
    // this RenderPrep pass; reading GlobalTransform here therefore used the
    // previous frame's eye position while CCamera already held the current
    // one.  re3 constructs the render list after camera processing, so use the
    // just-written Transform at this boundary.
    camera_query: Query<(&Camera, &Transform, &Projection, &GtaSceneCamera), Without<GtaWorldMesh>>,
    mut pass_state: ResMut<GtaRenderPassPlanState>,
    mut world_stats: ResMut<GtaWorldRenderStats>,
    mut world_mesh_resources: WorldMeshRenderResources,
    mut commands: Commands,
    mut mesh_query: Query<(&mut Visibility, &mut GtaWorldMesh)>,
    mut transform_query: Query<&mut Transform, With<GtaWorldMesh>>,
    mut candidate_scratch: Local<WorldRenderCandidateScratch>,
) {
    crate::profile_scope!("flush_world_render_pass");
    let _audit_timer = RenderAuditTimer::start("flush_world_render_pass");
    let texture_cache = &mut world_mesh_resources.texture_cache;
    let model_assets = &mut world_mesh_resources.model_assets;
    let mesh_registry = &mut world_mesh_resources.mesh_registry;
    let mut meshes = &mut world_mesh_resources.meshes;
    let mut materials = &mut world_mesh_resources.materials;
    let mut images = &mut world_mesh_resources.images;
    let collision_level = world_mesh_resources
        .collision
        .as_deref()
        .map(|collision| collision.collision_in_memory)
        .unwrap_or(ELevelName::Generic);
    let Some(mut file_loader) = file_loader else {
        pass_state.last_plan = None;
        *world_stats = GtaWorldRenderStats::default();
        return;
    };
    // Renderer.cpp ScanWorld resets this every render scan. Without an active
    // camera there is no priority sector pass, so never leave the previous
    // frame's throttle latched.
    file_loader.renderer.loading_priority = false;
    if mesh_registry.rw_object_epoch != file_loader.world_rw_object_epoch {
        // re3 CStreaming::DeleteAllRwObjects removes entity instances but
        // leaves streamed model data resident. Keep the heavier Bevy mirrors
        // hidden/recyclable so the post-LOAD_SCENE render list can recreate
        // the lightweight RwObject ownership without rebuilding ECS/GPU state.
        let mut stale_slots = std::mem::take(&mut mesh_registry.slots);
        mesh_registry.clear_slot_presence();
        if !stale_slots.is_empty() {
            mesh_registry.slots_epoch = mesh_registry.slots_epoch.wrapping_add(1);
        }
        for (entity_index, slot) in stale_slots.iter_mut() {
            slot.is_visible = false;
            for bevy_entity in &slot.bevy_entities {
                if let Ok((mut visibility, _)) = mesh_query.get_mut(*bevy_entity)
                    && *visibility != Visibility::Hidden
                {
                    *visibility = Visibility::Hidden;
                }
            }
            // There can only be one recyclable mirror per IPL entity. If an
            // older LOAD_SCENE left one behind and this entity was recreated
            // since then, the newer mirror supersedes it.
            if let Some(previous) = mesh_registry.retired_slots.remove(entity_index) {
                despawn_world_mesh_slot(&mut commands, &mut materials, &previous);
            }
        }
        mesh_registry.retired_slots.append(&mut stale_slots);
        mesh_registry.rw_object_epoch = file_loader.world_rw_object_epoch;
    }
    let mut active_cameras = camera_query
        .iter()
        .filter(|(camera, _, _, _)| camera.is_active)
        .collect::<Vec<_>>();
    active_cameras.sort_unstable_by_key(|(_, _, _, marker)| marker.local_player);
    let Some((_, primary_transform, projection, _)) = active_cameras.first().copied() else {
        pass_state.last_plan = None;
        *world_stats = GtaWorldRenderStats::default();
        return;
    };
    {
        crate::profile_scope!("flush_world_render_pass:texture_cache");
        let _audit_timer = RenderAuditTimer::start("flush_world_render_pass:texture_cache");
        sync_world_texture_cache(&file_loader, texture_cache, &mut images);
    }

    let camera_positions = active_cameras
        .iter()
        .map(|(_, transform, _, _)| CVector::from_bevy(transform.translation))
        .collect::<Vec<_>>();
    let camera_position = camera_positions[0];
    let current_hour = clock.as_ref().map(|clock| clock.hours()).unwrap_or(12);
    let aspect_ratio = match projection {
        Projection::Perspective(perspective) => perspective.aspect_ratio,
        _ => 4.0 / 3.0,
    };
    let far_clip = match projection {
        Projection::Perspective(perspective) => perspective.far,
        _ => the_camera.as_ref().map_or(2000.0, |camera| camera.far_clip),
    };
    let horizontal_fov = the_camera
        .as_ref()
        .map(|camera| {
            if camera.script_mode == CameraScriptMode::FlyBy {
                camera.flyby_fov
            } else {
                camera
                    .cams
                    .get(camera.active_cam)
                    .map(|cam| cam.fov)
                    .unwrap_or(70.0)
            }
        })
        .unwrap_or(70.0);
    let cutscene_running = cutscene
        .as_ref()
        .is_some_and(|cutscene| cutscene.is_running());
    let viewport_horizontal_fov = match projection {
        Projection::Perspective(perspective) => perspective_horizontal_fov_degrees(perspective),
        _ => CCamera::viewport_horizontal_fov(horizontal_fov, aspect_ratio, cutscene_running),
    };
    let cutscene_uses_lod_multiplier = cutscene
        .as_ref()
        .is_some_and(|cutscene| cutscene.use_lod_multiplier());
    let renderer_scale = file_loader.renderer.renderer_lod_scale();
    let lod_multiplier = CCamera::calculate_lod_dist_multiplier(
        horizontal_fov,
        aspect_ratio,
        cutscene_running,
        cutscene_uses_lod_multiplier,
        renderer_scale,
    );
    let is_topdown = the_camera.as_deref().is_some_and(renderer_is_topdown);
    let (primary_front, primary_up) = render_camera_basis(primary_transform);
    let topdown_rect = if is_topdown {
        renderer_topdown_sector_rect_from_basis(
            primary_front,
            primary_up,
            camera_position,
            viewport_horizontal_fov,
            aspect_ratio,
            far_clip,
        )
    } else {
        None
    };
    let priority_triangle = if is_topdown {
        None
    } else {
        renderer_priority_sector_triangle_from_basis(
            primary_front,
            primary_up,
            camera_position,
            viewport_horizontal_fov,
            aspect_ratio,
            LOD_DISTANCE * lod_multiplier,
            far_clip,
        )
    };
    let scan_triangle = if is_topdown {
        None
    } else {
        renderer_scan_sector_triangle_from_basis(
            primary_front,
            primary_up,
            camera_position,
            viewport_horizontal_fov,
            aspect_ratio,
            LOD_DISTANCE * lod_multiplier,
            far_clip,
        )
    };
    file_loader
        .renderer
        .begin_construct_render_list_for_cameras(&camera_positions, current_hour, lod_multiplier);

    // `CRenderer::ScanWorld` rasterises the camera/LOD triangle into sectors.
    // Do the same here. A radius square is not equivalent for unloaded simple
    // models because `SetupEntityVisibility` can return VIS_STREAMME before
    // reaching its on-screen test, flooding CStreaming with off-frustum work.
    let (candidate_indices, priority_candidate_count) = candidate_scratch.collect_scanworld_many(
        &file_loader.world,
        &camera_positions,
        priority_triangle,
        scan_triangle,
        topdown_rect,
        LOD_DISTANCE * lod_multiplier,
        collision_level,
    );
    let camera_frustums = active_cameras
        .iter()
        .filter_map(|(_, transform, projection, _)| match projection {
            Projection::Perspective(perspective) => render_camera_frustum(transform, perspective),
            _ => the_camera.as_deref().map(|camera| {
                camera.frustum_for_source_and_fov(
                    CVector::from_bevy(transform.translation),
                    viewport_horizontal_fov,
                )
            }),
        })
        .collect::<Vec<_>>();
    let camera_frustums = (!camera_frustums.is_empty()).then_some(camera_frustums);
    // Reuse the previous pass-plan vectors.  The C++ renderer writes its
    // visibility arrays in place; retaining this allocation is the Bevy
    // equivalent and avoids a large Vec-of-snapshots plus two list clones per
    // render frame.
    let previous_plan_indices =
        if std::env::var_os("RUSTGTA3_RENDER_DIAG").is_some_and(|value| value != "0") {
            pass_state.last_plan.as_ref().map(|previous| {
                previous
                    .roads
                    .iter()
                    .chain(previous.everything_bar_roads.iter())
                    .chain(previous.fading_in_entities.iter())
                    .copied()
                    .collect::<BTreeSet<_>>()
            })
        } else {
            None
        };
    let mut plan = pass_state.last_plan.take().unwrap_or_default();
    let file_loader_ref: &mut CFileLoader = &mut file_loader;
    let CFileLoader {
        world,
        model_info,
        renderer,
        ..
    } = file_loader_ref;
    let renderer_camera = renderer.camera_position;
    let renderer_lod = renderer.lod_distance_multiplier();
    let disable_zone_cull = zone_cull_disabled(renderer);
    let render_inputs =
        candidate_indices
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(candidate_offset, index)| {
                let entity = world.entities.get(index)?;
                // re3 `ScanSectorList`/`ScanSectorList_Priority` (`Renderer.cpp:1528,
                // 1604`) skip entities failing `IsEntityCullZoneVisible`, except for
                // the culled-road branch (`Renderer.cpp:1574-1577,1653-1655`): a
                // culled treadable still runs `SetupEntityVisibility` purely to
                // re-request its model, never entering the render lists.
                let cull_visible = entity_cull_zone_visible(disable_zone_cull, entity);
                let accept_visibility = cull_visible;
                if !cull_visible && !is_scan_road_entity(entity) {
                    return None;
                }
                let entity_position =
                    CVector::new(entity.position[0], entity.position[1], entity.position[2]);
                let nearest_camera = if camera_positions.len() <= 1 {
                    renderer_camera
                } else {
                    nearest_camera_position(&camera_positions, entity_position, renderer_camera)
                };
                let frustum = nearest_camera_frustum(camera_frustums.as_deref(), entity_position);
                let entity_snapshot = world_entity_snapshot_for_camera(
                    entity,
                    model_info,
                    frustum,
                    mesh_registry.has_current_slot(index, entity.model_index),
                );
                let model_snapshot = world_model_snapshot(
                    entity.model_index,
                    entity.position,
                    model_info,
                    nearest_camera,
                    renderer_lod,
                    entity.is_big_building,
                )?;
                Some((
                    index,
                    entity_snapshot,
                    model_snapshot,
                    candidate_offset < priority_candidate_count,
                    accept_visibility,
                ))
            });
    renderer.construct_render_pass_plan_after_begin_with_scan_flags(
        render_inputs,
        &world.entities,
        &mut plan,
    );

    if std::env::var_os("RUSTGTA3_RENDER_DIAG").is_some_and(|value| value != "0") {
        if let Some(previous) = previous_plan_indices {
            let current = plan
                .roads
                .iter()
                .chain(plan.everything_bar_roads.iter())
                .chain(plan.fading_in_entities.iter())
                .copied()
                .collect::<BTreeSet<_>>();
            let removed = previous
                .difference(&current)
                .filter_map(|index| world.entities.get(*index).map(|entity| (*index, entity)))
                .take(12)
                .map(|(entity_index, entity)| {
                    let entity_pos =
                        CVector::new(entity.position[0], entity.position[1], entity.position[2]);
                    let frustum = nearest_camera_frustum(camera_frustums.as_deref(), entity_pos);
                    let entity_snapshot = world_entity_snapshot_for_camera(
                        entity,
                        model_info,
                        frustum,
                        mesh_registry.has_current_slot(entity_index, entity.model_index),
                    );
                    let model_snapshot = world_model_snapshot(
                        entity.model_index,
                        entity.position,
                        model_info,
                        renderer_camera,
                        renderer_lod,
                        entity.is_big_building,
                    );
                    (
                        entity.name.as_str(),
                        entity.model_index,
                        entity.position,
                        entity.is_visible,
                        entity.zone_culled,
                        entity.zone_culled2,
                        entity_snapshot.on_screen,
                        model_snapshot
                            .as_ref()
                            .map(|model| model.loaded_for_distance),
                        model_snapshot
                            .as_ref()
                            .map(|model| model.loaded_for_fade_distance),
                    )
                })
                .collect::<Vec<_>>();
            eprintln!("RENDER_DIAG removed_from_plan={removed:?}");
        }
        eprintln!(
            "RENDER_DIAG cameras={:?} cpp_camera={:?} cpp_front={:?} plan roads={} everything={} fading={} stream={} candidates={}",
            camera_positions,
            the_camera.as_ref().map(|camera| camera.position),
            the_camera
                .as_ref()
                .and_then(|camera| camera.cams.get(camera.active_cam).map(|cam| cam.front)),
            plan.roads.len(),
            plan.everything_bar_roads.len(),
            plan.fading_in_entities.len(),
            plan.stream_requests.len(),
            candidate_indices.len(),
        );
    }

    // Priority candidates are the prefix of the same scan-code traversal used
    // for the ordinary frustum, matching `ScanWorld`: they are evaluated once,
    // request first, and cannot be rediscovered by the normal sector scan.
    // re3 `ScanSectorList_Priority` raises `m_loadingPriority` only when a
    // priority STREAMME model is genuinely not loaded yet. Raising it for
    // already-loaded far models (STREAMME by distance band) would latch the
    // sector-walk throttle forever while driving and starve everything
    // outside the narrow priority cone.
    renderer.loading_priority =
        priority_stream_needs_loading(&plan.priority_stream_requests, streaming.as_deref());
    let loading_priority = renderer.loading_priority;

    // CRenderer::ScanWorld immediately forwards VIS_STREAMME results to
    // CStreaming::RequestModel.  The ordinary radius scan is deliberately not
    // a substitute: big buildings and related/base LOD entities can be outside
    // that radius while still being required by SetupEntityVisibility.
    if let Some(streaming) = streaming.as_mut() {
        if !plan.priority_stream_requests.is_empty() {
            file_loader.request_models_for_runtime(
                streaming,
                plan.priority_stream_requests.iter().copied(),
                0,
            );
        }
        if !plan.stream_requests.is_empty() {
            request_ordinary_renderer_stream_models(
                &file_loader,
                streaming,
                plan.stream_requests.iter().copied(),
                loading_priority,
            );
        }
    }

    // `SetupEntityVisibility` mutates the shared CSimpleModelInfo alpha even
    // when the entity ends up OFFSCREEN/INVISIBLE. Those side effects are
    // important streaming state: an off-screen resident road becomes fully
    // opaque before it enters the view instead of starting a late fade only
    // after the player reaches it.
    // GTA III's stock PC renderer advances simple-model fade alpha once per
    // rendered 30 Hz game frame. Bevy can render substantially faster than
    // that, so advancing on every display frame makes LOD swaps finish several
    // times too quickly. CTimer already tracks the reference 30 Hz logical
    // cadence used by re3's frame-dependent fixes; keep visibility scans at
    // display rate but only advance the shared model alpha when that cadence
    // ticks. Immediate alpha-forcing side effects still apply every frame.
    let advance_model_fade = timer
        .as_deref()
        .is_none_or(|timer| timer.logical_frames_passed() != 0);
    for (entity_index, increase_alpha, force_alpha_255) in plan.model_alpha_updates.iter().copied()
    {
        let Some(entity) = file_loader.world.entities.get(entity_index) else {
            continue;
        };
        let model_index = entity.model_index;
        if let Some(simple) = file_loader.model_info.simple_model_info_mut(model_index) {
            if increase_alpha && advance_model_fade && simple.alpha < 255 {
                simple.IncreaseAlpha();
            }
            if force_alpha_255 {
                simple.alpha = 255;
            }
        }
    }

    let mut stats = GtaWorldRenderStats {
        planned_entities: plan.roads.len()
            + plan.everything_bar_roads.len()
            + plan.fading_in_entities.len(),
        ..default()
    };

    let fog = timecycle
        .as_ref()
        .map(|tc| {
            let c = &tc.current;
            let sun = tc.vector_to_sun[tc.current_stored_value.rem_euclid(16) as usize]
                .to_bevy()
                .normalize_or_zero();
            GtaRenderLighting {
                fog_colour: srgb_i32_to_linear(c.fog_colour),
                fog_start: c.fog_start,
                far_clip: c.far_clip,
                ambient: c.ambient,
                directional: c.directional,
                sun_direction: if sun.length_squared() > 1.0e-6 {
                    sun.to_array()
                } else {
                    [0.0, 0.0, 1.0]
                },
                ambient_multiplier: 1.0,
                directional_enabled: true,
            }
        })
        .unwrap_or_default();
    candidate_scratch.begin_plan(file_loader.world.entities.len());
    for entity_index in &plan.roads {
        candidate_scratch.insert_plan(*entity_index, GtaWorldRenderPass::Roads);
    }
    for entity_index in &plan.everything_bar_roads {
        candidate_scratch.insert_plan(*entity_index, GtaWorldRenderPass::EverythingBarRoads);
    }
    for entity_index in &plan.fading_in_entities {
        candidate_scratch.insert_plan(*entity_index, GtaWorldRenderPass::FadingIn);
    }
    for entity_index in &plan.distance_fade_entities {
        candidate_scratch.mark_distance_fade(*entity_index);
    }

    // Streaming retirement is consumed before ScanWorld. These remaining
    // deletions are decisions made by SetupBigBuildingVisibility itself.
    {
        crate::profile_scope!("flush_world_render_pass:rw_object_deletes");
        let _audit_timer = RenderAuditTimer::start("flush_world_render_pass:rw_object_deletes");
        // re3 `SetupBigBuildingVisibility` itself owns two `DeleteRwObject()`
        // branches (no-fade out of range, and no faded atomic). They are not
        // streaming-sector deletions, so keep them in the render decision instead
        // of waiting for `DeleteFarAwayRwObjects`. Otherwise an obsolete island /
        // bridge LOD keeps its Bevy RwObject mirror and model/TXD ref alive.
        for entity_index in plan.rw_object_deletes.iter().copied() {
            if let Some(slot) = mesh_registry.slots.remove(&entity_index) {
                mesh_registry.set_slot_presence(entity_index, false);
                mesh_registry.slots_epoch = mesh_registry.slots_epoch.wrapping_add(1);
                despawn_world_mesh_slot(&mut commands, &mut materials, &slot);
            }
        }
    }

    // Off-screen/culled but still-near RwObject mirrors stay allocated and
    // retain their streaming refs. Hide their Bevy draw entities without
    // destroying the registry slot; `ensure_world_mesh_for_entity` unhides a
    // persistent slot as soon as it re-enters the re3 render plan.
    for (&entity_index, slot) in &mut mesh_registry.slots {
        if candidate_scratch.contains_plan(entity_index) {
            continue;
        }
        if !slot.is_visible {
            continue;
        }
        slot.is_visible = false;
        for bevy_entity in &slot.bevy_entities {
            if let Ok((mut vis, _)) = mesh_query.get_mut(*bevy_entity) {
                if *vis != Visibility::Hidden {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }

    // re3 creates an entity RwObject as soon as a resident atomic is selected,
    // before checking bIsVisible/GetIsOnScreen. Preserve that ownership for
    // scanned off-screen entities too. The hidden Bevy instance is the
    // CEntity::m_rwObject mirror; next frame `sync_runtime_entity_model_refs`
    // gives the source DFF/TXD the same streaming ref that C++ would retain.
    {
        crate::profile_scope!("flush_world_render_pass:ensure_hidden_rw_objects");
        let _audit_timer =
            RenderAuditTimer::start("flush_world_render_pass:ensure_hidden_rw_objects");
        for (entity_index, fade_fallback) in plan.rw_object_creates.iter().copied() {
            if candidate_scratch.contains_plan(entity_index) {
                continue;
            }
            let Some(entity) = file_loader.world.entities.get(entity_index) else {
                continue;
            };
            let pass = if fade_fallback {
                GtaWorldRenderPass::FadingIn
            } else if entity.treadable_pool_slot.is_some() {
                GtaWorldRenderPass::Roads
            } else {
                GtaWorldRenderPass::EverythingBarRoads
            };
            let entity_position =
                CVector::new(entity.position[0], entity.position[1], entity.position[2]);
            let camera_gta = if camera_positions.len() <= 1 {
                camera_position
            } else {
                nearest_camera_position(&camera_positions, entity_position, camera_position)
            };
            ensure_world_mesh_for_entity(
                entity_index,
                pass,
                fade_fallback,
                false,
                &file_loader,
                &mut commands,
                &mut meshes,
                &mut materials,
                texture_cache,
                &mut images,
                model_assets,
                mesh_registry,
                &mut mesh_query,
                &mut transform_query,
                &mut stats,
                camera_gta,
                fog,
                lod_multiplier,
            );
        }
    }

    // Keep render commands in entity-index order with one contiguous sort,
    // avoiding a tree allocation per visible entity on every render frame.
    candidate_scratch.planned_indices.sort_unstable();
    {
        crate::profile_scope!("flush_world_render_pass:ensure_visible_rw_objects");
        let _audit_timer =
            RenderAuditTimer::start("flush_world_render_pass:ensure_visible_rw_objects");
        for entity_index in candidate_scratch.planned_indices.iter().copied() {
            let Some(pass) = candidate_scratch.planned_passes[entity_index] else {
                continue;
            };
            let distance_fade = candidate_scratch.planned_distance_fade[entity_index];
            let camera_gta = if camera_positions.len() <= 1 {
                camera_position
            } else {
                file_loader
                    .world
                    .entities
                    .get(entity_index)
                    .map(|entity| {
                        let position = CVector::new(
                            entity.position[0],
                            entity.position[1],
                            entity.position[2],
                        );
                        nearest_camera_position(&camera_positions, position, camera_position)
                    })
                    .unwrap_or(camera_position)
            };
            ensure_world_mesh_for_entity(
                entity_index,
                pass,
                distance_fade,
                true,
                &file_loader,
                &mut commands,
                &mut meshes,
                &mut materials,
                texture_cache,
                &mut images,
                model_assets,
                mesh_registry,
                &mut mesh_query,
                &mut transform_query,
                &mut stats,
                camera_gta,
                fog,
                lod_multiplier,
            );
        }
    }

    // `DeleteAllRwObjects` is allowed to recycle matching mirrors only while
    // the cooperative LOAD_SCENE barrier is active. Once the scene is
    // resident, any unreclaimed mirror represents an RwObject that C++ has
    // destroyed. Retaining all of them forever kept hidden ECS entities and
    // Mesh/Material handles alive as the player visited more mission/cutscene
    // regions. Drain a small fixed batch per frame to avoid turning cleanup
    // itself into another hitch.
    {
        crate::profile_scope!("flush_world_render_pass:retired_slot_cleanup");
        let _audit_timer = RenderAuditTimer::start("flush_world_render_pass:retired_slot_cleanup");
        if !file_loader.scene_load_in_progress() {
            for _ in 0..64 {
                let Some((_entity_index, retired)) = mesh_registry.retired_slots.pop_first() else {
                    break;
                };
                despawn_world_mesh_slot(&mut commands, &mut materials, &retired);
            }
        }
    }

    // Keep shared model assets while their RenderWare model is resident, then
    // release them with the same streaming lifetime as the source geometry.
    // Streaming itself is incremental; scanning the cache every render frame
    // would reintroduce the allocation/tree walk this cache removes.
    {
        crate::profile_scope!("flush_world_render_pass:model_asset_prune");
        let _audit_timer = RenderAuditTimer::start("flush_world_render_pass:model_asset_prune");
        model_assets.prune_tick = model_assets.prune_tick.saturating_add(1);
        if model_assets.prune_tick >= 30 {
            model_assets.prune_tick = 0;
            let active_asset_keys = mesh_registry
                .slots
                .values()
                .map(|slot| slot.asset_key)
                .collect::<BTreeSet<_>>();
            let stale_asset_keys = model_assets
                .entries
                .keys()
                .copied()
                .filter(|key| {
                    !active_asset_keys.contains(key)
                        && (!runtime_model_geometry_is_resident(&file_loader, key.model_index)
                            || !world_model_asset_txd_generation_is_current(&file_loader, *key))
                })
                .collect::<Vec<_>>();
            for key in stale_asset_keys {
                if let Some(assets) = model_assets.entries.remove(&key) {
                    // Mesh assets are shared with the TXD-independent geometry
                    // map; only material assets die with the TXD generation.
                    for material in assets.materials {
                        materials.remove(material.id());
                    }
                }
            }
            // Geometry follows model residency, not TXD generations: drop Mesh
            // assets only when the streamed model is evicted and no live slot
            // renders this (model, LOD) anymore.
            let active_geometry_keys = mesh_registry
                .slots
                .values()
                .map(|slot| (slot.model_index, slot.asset_key.lod_index))
                .collect::<BTreeSet<_>>();
            let geometry_keys = model_assets.geometry.keys().copied().collect::<Vec<_>>();
            let mut deferred_geometry = 0usize;
            let mut evicted_meshes = 0usize;
            for key in geometry_keys {
                let active = active_geometry_keys.contains(&key);
                let source_resident = runtime_model_geometry_is_resident(&file_loader, key.0);
                if active || source_resident {
                    model_assets.geometry_idle_prunes.remove(&key);
                    continue;
                }

                let should_evict = if world_geometry_cache_can_grace(key.0) {
                    let idle_prunes = model_assets.geometry_idle_prunes.entry(key).or_default();
                    *idle_prunes = idle_prunes.saturating_add(1);
                    if *idle_prunes < WORLD_GEOMETRY_CACHE_GRACE_PRUNES {
                        deferred_geometry += 1;
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if !should_evict {
                    continue;
                }
                model_assets.geometry_idle_prunes.remove(&key);
                if let Some(geometry) = model_assets.geometry.remove(&key) {
                    evicted_meshes += geometry.meshes.len();
                    for mesh in geometry.meshes {
                        meshes.remove(mesh.id());
                    }
                }
            }
            #[cfg(feature = "profiling")]
            if deferred_geometry != 0 || evicted_meshes != 0 {
                eprintln!(
                    "RENDER_CACHE deferred_geometry={} evicted_meshes={} cached_geometry={}",
                    deferred_geometry,
                    evicted_meshes,
                    model_assets.geometry.len()
                );
            }
            #[cfg(not(feature = "profiling"))]
            let _ = (deferred_geometry, evicted_meshes);
        }
    }

    pass_state.last_plan = Some(plan);
    *world_stats = stats;
}

fn despawn_world_mesh_slot(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
    slot: &GtaWorldMeshSlot,
) {
    for entity in &slot.bevy_entities {
        // Best-effort asset cleanup — entity may already be gone.
        commands.entity(*entity).despawn();
    }
    for material in &slot.fade_materials {
        materials.remove(material.id());
    }
}

#[cfg(test)]
fn world_mesh_model_is_current(world: &CWorld, entity_index: usize, model_index: usize) -> bool {
    world
        .entities
        .get(entity_index)
        .is_some_and(|entity| entity.model_index == model_index)
}

fn world_model_txd_generation_and_revision(
    file_loader: &CFileLoader,
    model_index: usize,
) -> (u64, u64) {
    let Some(model) = file_loader.model_info.get_model_info(model_index) else {
        return (0, 0);
    };
    let Ok(slot) = usize::try_from(model.base().txd_slot) else {
        return (0, 0);
    };
    file_loader
        .txd_store
        .slot(slot)
        .map(|txd| (txd.generation, txd.texture_revision))
        .unwrap_or((0, 0))
}

fn world_model_txd_generation(file_loader: &CFileLoader, model_index: usize) -> u64 {
    world_model_txd_generation_and_revision(file_loader, model_index).0
}

#[allow(dead_code)]
fn world_model_txd_revision(file_loader: &CFileLoader, model_index: usize) -> u64 {
    world_model_txd_generation_and_revision(file_loader, model_index).1
}

fn world_model_asset_txd_generation_is_current(
    file_loader: &CFileLoader,
    key: GtaWorldModelAssetKey,
) -> bool {
    let (generation, revision) =
        world_model_txd_generation_and_revision(file_loader, key.model_index);
    key.txd_generation == generation && key.txd_revision == revision
}

fn vehicle_colour_remap_from_indices(
    colour1: u8,
    colour2: u8,
    table: Option<&crate::modelinfo::VehicleModelInfo::CVehicleColours>,
) -> VehicleColourRemap {
    let Some(table) = table else {
        return VehicleColourRemap {
            colour1_index: Some(colour1),
            colour2_index: Some(colour2),
            ..VehicleColourRemap::default()
        };
    };
    let c1 = table
        .vehicle_colour_table
        .get(colour1 as usize)
        .map(|c| [c.red, c.green, c.blue, c.alpha]);
    let c2 = table
        .vehicle_colour_table
        .get(colour2 as usize)
        .map(|c| [c.red, c.green, c.blue, c.alpha]);
    VehicleColourRemap {
        colour1: c1,
        colour2: c2,
        colour1_index: Some(colour1),
        colour2_index: Some(colour2),
        wheel_state: None,
        damage_state: None,
        door_state: None,
        optional_components: None,
        heli_rotor_rotation: None,
    }
}

/// re3 `CPacManPickups::Render`: particle TXD corona texture six, additive
/// colour (100, 50, 5), a 1024 ms horizontal sine pulse, and a 100-unit
/// camera-depth cutoff.
fn flush_pacman_billboards(
    pacman: Option<Res<CPacManPickups>>,
    timer: Res<CTimer>,
    // Same-frame camera basis (see sync_sky_horizon_mesh). Without<> keeps
    // this disjoint from the &mut Transform slot writer below.
    camera_query: Query<&Transform, (With<GtaSceneCamera>, Without<GtaPersistentImmediate>)>,
    texture_cache: Res<GtaWorldTextureCache>,
    file_loader: Option<Res<CFileLoader>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
) {
    let Some(pacman) = pacman else {
        hide_persistent_billboard_slots(&immediate_cache.pacman_entities, 0, &mut persistent_query);
        return;
    };
    if !pacman.active {
        hide_persistent_billboard_slots(&immediate_cache.pacman_entities, 0, &mut persistent_query);
        return;
    }
    let Some(camera_transform) = camera_query.iter().next() else {
        hide_persistent_billboard_slots(&immediate_cache.pacman_entities, 0, &mut persistent_query);
        return;
    };
    let camera_rotation = camera_transform.rotation;
    let camera_position = camera_transform.translation;
    let camera_forward = *camera_transform.forward();
    let texture = file_loader
        .as_ref()
        .and_then(|loader| loader.txd_store.find_txd_slot("particle"))
        .and_then(|slot| {
            texture_cache
                .images
                .get(&(slot, "coronaring".to_owned()))
                .cloned()
        });
    let phase = (timer.time_in_milliseconds() % 1024) as f32;
    let horizontal = (phase * 6.28 / 1024.0).sin();
    let billboard_mesh = unit_billboard_mesh_handle(&mut immediate_cache, &mut meshes);
    let mut active_count = 0;

    for pickup in &pacman.pickups {
        if pickup.pickup_type == PACMAN_NONE {
            continue;
        }
        let world_position = pickup.pos.to_bevy();
        let depth = (world_position - camera_position).dot(camera_forward);
        if !(0.0..100.0).contains(&depth) {
            continue;
        }
        let transform = Transform::from_translation(world_position)
            .with_rotation(camera_rotation)
            .with_scale(Vec3::new(1.6 * horizontal, 1.6, 1.0));
        let entity = update_persistent_billboard_slot(
            immediate_cache.pacman_entities.get(active_count).copied(),
            transform,
            gta_billboard_material(Color::srgba_u8(100, 50, 5, 255), texture.clone(), true),
            &billboard_mesh,
            &mut persistent_query,
            &mut materials,
            &mut commands,
        );
        if let Some(slot) = immediate_cache.pacman_entities.get_mut(active_count) {
            *slot = entity;
        } else {
            immediate_cache.pacman_entities.push(entity);
        }
        active_count += 1;
    }
    hide_persistent_billboard_slots(
        &immediate_cache.pacman_entities,
        active_count,
        &mut persistent_query,
    );
}

fn flush_cloud_billboards(
    clouds: Option<ResMut<CClouds>>,
    immediate_cache: Res<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
) {
    let Some(mut clouds) = clouds else {
        hide_persistent_billboard_slots(&immediate_cache.cloud_entities, 0, &mut persistent_query);
        return;
    };
    // Clouds are rendered by ParticleSprite's re3-style pre-projected pass.
    // Retire the old world-space StandardMaterial slots so they cannot depth
    // test/write or show up as coloured cards behind the correct sprite path.
    let _ = clouds.drain_render_intents();
    hide_persistent_billboard_slots(&immediate_cache.cloud_entities, 0, &mut persistent_query);
}

fn build_traffic_bright_light_mesh(intent: &BrightLightRenderIntent) -> Mesh {
    const SIDE: [f32; 6] = [-0.09, 0.09, 0.162, 0.09, -0.09, -0.162];
    const UP: [f32; 6] = [0.162, 0.162, 0.0, -0.162, -0.162, 0.0];
    let positions = (0..6)
        .map(|index| {
            CVector::new(
                intent.position.x + SIDE[index] * intent.side.x + UP[index] * intent.up.x,
                intent.position.y + SIDE[index] * intent.side.y + UP[index] * intent.up.y,
                intent.position.z + SIDE[index] * intent.side.z + UP[index] * intent.up.z,
            )
            .to_bevy()
            .to_array()
        })
        .collect::<Vec<_>>();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 5, 1, 2, 3, 1, 3, 4, 1, 4, 5]))
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 6])
}

fn build_shiny_text_mesh(intent: &ShinyTextRenderIntent) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 2, 1, 3]))
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        intent
            .vertices
            .into_iter()
            .map(|vertex| vertex.to_bevy().to_array())
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, intent.tex_coords.to_vec())
}

/// CBulletTraces::Render: six vertices facing the camera, shad_exp raster,
/// additive blend, and no depth writes. Width is 1/20 GTA world unit.
fn build_bullet_trace_mesh(
    trace: &crate::renderer::SpecialFX::BulletTraceRenderIntent,
    front: Vec3,
) -> Mesh {
    let start = trace.start.to_bevy();
    let end = trace.end.to_bevy();
    let center = (start + end) * 0.5;
    let width = front.cross(end - start).normalize_or_zero() * trace.thickness;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 2, 1, 1, 2, 3, 2, 4, 3, 3, 4, 5]))
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        [
            start + width,
            start - width,
            center + width,
            center - width,
            end + width,
            end - width,
        ]
        .map(|p| p.to_array())
        .to_vec(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![(-front).to_array(); 6])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 0.5],
            [1.0, 0.5],
            [0.0, 1.0],
            [1.0, 1.0],
        ],
    )
}

fn flush_special_fx_meshes(
    special_fx: Option<ResMut<CSpecialFX>>,
    camera: Option<Res<CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    mut general: Option<ResMut<CGeneral>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
    mut bright_lights: Local<car_light_render::BrightLightMeshBatch>,
) {
    let Some(mut special_fx) = special_fx else {
        hide_persistent_billboard_slots(
            &immediate_cache.special_fx_entities,
            0,
            &mut persistent_query,
        );
        return;
    };
    let mut active_count = 0;
    bright_lights.clear();

    for intent in special_fx.drain_bright_light_intents() {
        let flicker = general.as_deref_mut().map_or(0, |general| {
            ((general.get_random_number() & 0xFF) as f32 * 0.2) as u8
        });
        let (mesh, [r, g, b, alpha]) = if let Some(mesh) = car_light_render::build_vehicle_bright_light_mesh(&intent) {
            let Some(colour) = car_light_render::vehicle_bright_light_colour(&intent, flicker) else { continue; };
            (mesh, colour)
        } else {
            let [r, g, b] = match intent.light_type {
                BRIGHTLIGHT_TRAFFIC_GREEN => [flicker, 255, flicker],
                BRIGHTLIGHT_TRAFFIC_YELLOW => [255, 128, flicker],
                BRIGHTLIGHT_TRAFFIC_RED => [255, flicker, flicker],
                _ => continue,
            };
            let alpha = if intent.camera_distance < 45.0 { 255 } else {
                (255.0 * (1.0 - (intent.camera_distance - 45.0) / 15.0)).clamp(0.0, 255.0) as u8
            };
            (build_traffic_bright_light_mesh(&intent), [r, g, b, alpha])
        };
        bright_lights.append(mesh, Color::srgba_u8(r, g, b, alpha));
    }
    if let Some(mesh) = bright_lights.mesh() {
        let entity = update_persistent_mesh_slot(
            immediate_cache.special_fx_entities.first().copied(),
            mesh,
            StandardMaterial {
                base_color: Color::WHITE, unlit: true, alpha_mode: AlphaMode::Blend,
                double_sided: true, cull_mode: None, ..default()
            },
            Transform::IDENTITY, &mut persistent_query, &mut meshes, &mut materials, &mut commands,
        );
        if let Some(slot) = immediate_cache.special_fx_entities.first_mut() {
            *slot = entity;
        } else {
            immediate_cache.special_fx_entities.push(entity);
        }
        active_count = 1;
    }

    let particle_slot = file_loader
        .as_deref()
        .and_then(|loader| loader.txd_store.find_txd_slot("particle"));
    if let (Some(slot), Some(loader)) = (particle_slot, file_loader.as_deref()) {
        let _ = sync_txd_slot_texture_cache(slot, loader, &mut texture_cache, &mut images);
    }
    let walk_dont_texture = particle_slot.and_then(|slot| {
        texture_cache_lookup(&texture_cache, slot, "walk_dont").map(|(handle, _)| handle)
    });
    for intent in special_fx.drain_shiny_text_intents() {
        let Some(texture) = walk_dont_texture.clone() else {
            continue;
        };
        let entity = update_persistent_mesh_slot(
            immediate_cache
                .special_fx_entities
                .get(active_count)
                .copied(),
            build_shiny_text_mesh(&intent),
            StandardMaterial {
                base_color: Color::srgba_u8(
                    intent.colour[0],
                    intent.colour[1],
                    intent.colour[2],
                    255,
                ),
                base_color_texture: Some(texture),
                unlit: true,
                alpha_mode: AlphaMode::Add,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            Transform::IDENTITY,
            &mut persistent_query,
            &mut meshes,
            &mut materials,
            &mut commands,
        );
        if let Some(slot) = immediate_cache.special_fx_entities.get_mut(active_count) {
            *slot = entity;
        } else {
            immediate_cache.special_fx_entities.push(entity);
        }
        active_count += 1;
    }

    for trace in special_fx.drain_bullet_intents() {
        let texture = particle_slot.and_then(|slot| {
            texture_cache_lookup(&texture_cache, slot, "shad_exp").map(|(handle, _)| handle)
        });
        let Some(texture) = texture else {
            warn_once!(target: "rustgta3::weapons", "bullet trace particle/shad_exp raster unavailable");
            continue;
        };
        let Some(front) = camera.as_ref().map(|camera| camera.orientation.to_bevy()) else {
            warn_once!(target: "rustgta3::weapons", "bullet trace camera unavailable");
            continue;
        };
        let colour = Color::srgba_u8(
            trace.colour[0],
            trace.colour[1],
            trace.colour[2],
            trace.colour[3],
        );
        let entity = update_persistent_mesh_slot(
            immediate_cache
                .special_fx_entities
                .get(active_count)
                .copied(),
            build_bullet_trace_mesh(&trace, front),
            gta_billboard_material(colour, Some(texture), true),
            Transform::IDENTITY,
            &mut persistent_query,
            &mut meshes,
            &mut materials,
            &mut commands,
        );
        if let Some(slot) = immediate_cache.special_fx_entities.get_mut(active_count) {
            *slot = entity;
        } else {
            immediate_cache.special_fx_entities.push(entity);
        }
        active_count += 1;
    }
    hide_persistent_billboard_slots(
        &immediate_cache.special_fx_entities,
        active_count,
        &mut persistent_query,
    );
}

fn flush_fluff_intents(
    moving_things: Option<ResMut<CMovingThings>>,
    timer: Option<Res<CTimer>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
) {
    let Some(mut moving_things) = moving_things else {
        hide_persistent_billboard_slots(&immediate_cache.fluff_entities, 0, &mut persistent_query);
        return;
    };
    let frame_counter = timer
        .as_ref()
        .map(|timer| timer.frame_counter())
        .unwrap_or(0);
    moving_things.update(frame_counter);
    moving_things.render();

    let intents = moving_things.drain_render_intents();
    let mut active_count = 0;
    for intent in intents {
        let Some((position, size, colour, scale)) = fluff_intent_shape(&moving_things, intent)
        else {
            continue;
        };
        let width = (size.x.abs() * scale).max(0.05);
        let height = (size.z.abs().max(size.y.abs()) * scale).max(0.05);
        let entity = update_persistent_mesh_slot(
            immediate_cache.fluff_entities.get(active_count).copied(),
            build_world_quad_mesh(width, height, colour),
            StandardMaterial {
                base_color: Color::WHITE,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            Transform::from_translation(position.to_bevy()),
            &mut persistent_query,
            &mut meshes,
            &mut materials,
            &mut commands,
        );
        commands.entity(entity).insert(GtaFluffMesh { intent });
        if let Some(slot) = immediate_cache.fluff_entities.get_mut(active_count) {
            *slot = entity;
        } else {
            immediate_cache.fluff_entities.push(entity);
        }
        active_count += 1;
    }

    hide_persistent_billboard_slots(
        &immediate_cache.fluff_entities,
        active_count,
        &mut persistent_query,
    );
}

fn flush_render_buffer_batches(
    render_buffer: Option<ResMut<RenderBuffer>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
) {
    let Some(mut render_buffer) = render_buffer else {
        hide_persistent_billboard_slots(
            &immediate_cache.render_buffer_entities,
            0,
            &mut persistent_query,
        );
        return;
    };
    render_buffer.render_stuff_in_buffer();
    let batches = std::mem::take(&mut render_buffer.flushed_batches);
    let mut active_count = 0;
    for batch in batches {
        if batch.vertices.is_empty() || batch.indices.is_empty() {
            continue;
        }
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(
            batch.indices.iter().map(|index| *index as u32).collect(),
        ))
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            batch
                .vertices
                .iter()
                .map(|vertex| vertex.position.to_bevy().to_array())
                .collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0.0, 1.0, 0.0]; batch.vertices.len()],
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            batch
                .vertices
                .iter()
                .map(|vertex| vertex.uv)
                .collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            batch
                .vertices
                .iter()
                .map(|vertex| rgba_u8_to_f32(vertex.colour))
                .collect::<Vec<_>>(),
        );
        let entity = update_persistent_mesh_slot(
            immediate_cache
                .render_buffer_entities
                .get(active_count)
                .copied(),
            mesh,
            StandardMaterial {
                base_color: Color::WHITE,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            },
            Transform::default(),
            &mut persistent_query,
            &mut meshes,
            &mut materials,
            &mut commands,
        );
        if let Some(slot) = immediate_cache.render_buffer_entities.get_mut(active_count) {
            *slot = entity;
        } else {
            immediate_cache.render_buffer_entities.push(entity);
        }
        active_count += 1;
    }
    hide_persistent_billboard_slots(
        &immediate_cache.render_buffer_entities,
        active_count,
        &mut persistent_query,
    );
}

/// Build the exact entity-side visibility snapshot consumed by the runtime
/// render pass. Kept public so headless parity/probe binaries can diagnose the
/// production plan without duplicating renderer rules.
pub fn world_entity_snapshot(
    entity: &CWorldEntity,
    model_info: &CModelInfo,
) -> RendererEntitySnapshot {
    let rw_object_loaded = model_info
        .get_model_info(entity.model_index)
        .is_some_and(|model| model.base().rw_object_loaded);
    world_entity_snapshot_for_camera(entity, model_info, None, rw_object_loaded)
}

/// Runtime variant of [`world_entity_snapshot`] that applies the same first
/// visibility gate as re3 `CRenderer::ScanSectorList`:
/// `IsEntityCullZoneVisible` (`renderer/Renderer.cpp:1761`) followed by
/// `CEntity::GetIsOnScreen` (`entities/Entity.cpp:411`).  The public helper
/// above intentionally keeps its data-only contract for callers that do not
/// own the live camera resource; production rendering always passes the
/// current `CCamera`.
fn world_entity_snapshot_for_camera(
    entity: &CWorldEntity,
    model_info: &CModelInfo,
    frustum: Option<&CameraFrustum>,
    rw_object_loaded: bool,
) -> RendererEntitySnapshot {
    let simple = model_info.simple_model_info(entity.model_index);
    let col_model = model_info.col_model(entity.model_index);
    let bound_center = col_model
        .map(|col| {
            let center = col.bounding_sphere.center;
            let world = entity_world_point(entity, [center.x, center.y, center.z]);
            CVector::new(world[0], world[1], world[2])
        })
        .unwrap_or_else(|| {
            CVector::new(entity.position[0], entity.position[1], entity.position[2])
        });
    let bound_radius = col_model
        .map(|col| col.bounding_sphere.radius)
        .unwrap_or(0.0);
    let on_screen = match frustum {
        Some(frustum) => frustum.is_sphere_visible(bound_center, bound_radius),
        None => true,
    };
    let on_screen_complex = if entity.is_big_building {
        match (frustum, col_model) {
            (Some(frustum), Some(col)) => {
                if frustum.is_point_visible(bound_center) {
                    true
                } else if let Some([left, right, top, bottom]) = entity.get_bound_rect(model_info) {
                    // CEntity::GetIsOnScreenComplex uses the transformed XY bound
                    // rect, but Z is intentionally entity position + the local
                    // collision-box min/max. Preserve that exact re3 construction.
                    let min_z = entity.position[2] + col.bounding_box.min.z;
                    let max_z = entity.position[2] + col.bounding_box.max.z;
                    let corners = [
                        CVector::new(left, bottom, min_z),
                        CVector::new(left, top, min_z),
                        CVector::new(right, bottom, min_z),
                        CVector::new(right, top, min_z),
                        CVector::new(left, bottom, max_z),
                        CVector::new(left, top, max_z),
                        CVector::new(right, bottom, max_z),
                        CVector::new(right, top, max_z),
                    ];
                    frustum.is_box_visible(&corners)
                } else {
                    on_screen
                }
            }
            _ => on_screen,
        }
    } else {
        on_screen
    };
    RendererEntitySnapshot {
        entity_type: entity.entity_type,
        model_index: entity.model_index,
        position: CVector::new(entity.position[0], entity.position[1], entity.position[2]),
        // `CEntity::m_rwObject` is instance state. Shared CModelInfo residency
        // only says the DFF is available to create this entity's object; it
        // does not mean this particular world entity currently owns one.
        rw_object_loaded,
        // re3 `SetupEntityVisibility` reads only `bIsVisible`; `bZoneCulled`
        // never enters the visibility decision. Zone membership gates the call
        // itself (`IsEntityCullZoneVisible`, `Renderer.cpp:1528`, with the
        // culled-road stream branch at :1574), mirrored here by the
        // `accept_visibility` scan flag. Keep the authored bit pure so the
        // decision replay matches C++ and the debug zone-cull bypass works.
        visible: entity.is_visible,
        on_screen,
        on_screen_complex,
        draw_last: simple.is_some_and(|simple| simple.flags.draw_last),
        render_damaged: false,
        first_person_player_vehicle: false,
        first_person_camera_forward: false,
        first_person_big_vehicle_exception: false,
        temporary_object: false,
        big_building: entity.is_big_building,
    }
}

fn entity_world_point(entity: &CWorldEntity, point: [f32; 3]) -> [f32; 3] {
    if point[0] == 0.0 && point[1] == 0.0 && point[2] == 0.0 {
        return entity.position;
    }
    if let Some([right, forward, up]) = entity.matrix_basis_override {
        return [
            entity.position[0] + right[0] * point[0] + forward[0] * point[1] + up[0] * point[2],
            entity.position[1] + right[1] * point[0] + forward[1] * point[1] + up[1] * point[2],
            entity.position[2] + right[2] * point[0] + forward[2] * point[1] + up[2] * point[2],
        ];
    }
    if entity.rotation_degrees.abs() <= 1e-4 {
        return [
            entity.position[0] + point[0],
            entity.position[1] + point[1],
            entity.position[2] + point[2],
        ];
    }
    let axis = entity.rotation_axis;
    let axis_length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if axis_length <= f32::EPSILON {
        return [
            entity.position[0] + point[0],
            entity.position[1] + point[1],
            entity.position[2] + point[2],
        ];
    }
    let unit = [
        axis[0] / axis_length,
        axis[1] / axis_length,
        axis[2] / axis_length,
    ];
    let radians = entity.rotation_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    let dot = unit[0] * point[0] + unit[1] * point[1] + unit[2] * point[2];
    let cross = [
        unit[1] * point[2] - unit[2] * point[1],
        unit[2] * point[0] - unit[0] * point[2],
        unit[0] * point[1] - unit[1] * point[0],
    ];
    [
        entity.position[0] + point[0] * cos + cross[0] * sin + unit[0] * dot * (1.0 - cos),
        entity.position[1] + point[1] * cos + cross[1] * sin + unit[1] * dot * (1.0 - cos),
        entity.position[2] + point[2] * cos + cross[2] * sin + unit[2] * dot * (1.0 - cos),
    ]
}

/// re3 `CRenderer::SetupEntityVisibility` bumps multi-atomic simple models to
/// their largest LOD distance once they are beyond the authored LOD frustum.
/// Atomic residency checks and the eventual geometry swap both use this
/// adjusted distance, not the raw entity-origin distance.
fn renderer_simple_distance(
    distance: f32,
    largest_lod_distance: f32,
    lod_multiplier: f32,
    is_big_building: bool,
) -> f32 {
    // re3 `Renderer.h:6` (FIX_BUGS): `LOD_DISTANCE` is scaled, `STREAM_DISTANCE` is not.
    // re3 `Renderer.cpp:802` bump lives only in `SetupEntityVisibility`;
    // `SetupBigBuildingVisibility` uses the raw distance.
    if is_big_building {
        return distance;
    }
    if LOD_DISTANCE * lod_multiplier + STREAM_DISTANCE < distance && distance < largest_lod_distance
    {
        largest_lod_distance
    } else {
        distance
    }
}

/// Build the exact model-side visibility snapshot consumed by the runtime
/// render pass. This intentionally observes current streaming residency.
pub fn world_model_snapshot(
    model_index: usize,
    entity_position: [f32; 3],
    model_info: &CModelInfo,
    camera_position: CVector,
    lod_multiplier: f32,
    is_big_building: bool,
) -> Option<RendererModelSnapshot> {
    let model = model_info.get_model_info(model_index)?;
    let base = model.base();
    let simple = model_info.simple_model_info(model_index);
    let distance = world_distance(
        CVector::new(entity_position[0], entity_position[1], entity_position[2]),
        camera_position,
    );
    let related_id = simple.and_then(|simple| simple.related_model_id);
    let other_time_id = simple.and_then(|simple| simple.other_time_model_id);
    let largest_lod_distance = simple
        .map(|simple| simple.largest_lod_distance(lod_multiplier, false))
        .unwrap_or(0.0);
    let simple_distance = renderer_simple_distance(
        distance,
        largest_lod_distance,
        lod_multiplier,
        is_big_building,
    );
    let loaded_for_distance = simple
        .filter(|simple| !simple.atomic_bindings.is_empty())
        .map(|simple| {
            simple
                .atomic_binding_from_distance(simple_distance, lod_multiplier, false)
                .is_some()
        })
        .unwrap_or(base.rw_object_loaded);
    let loaded_for_fade_distance = simple
        .filter(|simple| !simple.atomic_bindings.is_empty())
        .map(|simple| {
            simple
                .atomic_binding_from_distance(
                    simple_distance - FADE_DISTANCE,
                    lod_multiplier,
                    false,
                )
                .is_some()
        })
        .unwrap_or(base.rw_object_loaded);
    let related_model = related_id.and_then(|related_id| model_info.get_model_info(related_id));
    let related_simple = related_id.and_then(|related_id| model_info.simple_model_info(related_id));
    let related_time_other_loaded = related_simple
        .and_then(|related| related.other_time_model_id)
        .is_some_and(|other_id| model_info.rw_object_loaded(other_id));
    Some(RendererModelSnapshot {
        model_type: renderer_model_type(base.model_type),
        loaded_for_distance,
        loaded_for_fade_distance,
        largest_lod_distance,
        near_distance: simple
            .map(|simple| simple.lod_distances[2] * lod_multiplier)
            .unwrap_or(0.0),
        no_fade: simple.is_some_and(|simple| simple.flags.no_fade),
        draw_last: simple.is_some_and(|simple| simple.flags.draw_last),
        alpha: simple.map(|simple| simple.alpha).unwrap_or(255),
        time_on: simple.and_then(|simple| simple.time_on.map(|hour| hour as u8)),
        time_off: simple.and_then(|simple| simple.time_off.map(|hour| hour as u8)),
        other_time_model_exists: other_time_id.is_some(),
        other_time_model_loaded: other_time_id
            .is_some_and(|other_id| model_info.rw_object_loaded(other_id)),
        related_model_exists: related_model.is_some(),
        related_model_loaded: related_id
            .is_some_and(|related_id| model_info.rw_object_loaded(related_id)),
        related_model_alpha: related_simple.map(|related| related.alpha).unwrap_or(255),
        related_time_other_loaded,
    })
}

fn renderer_model_type(model_type: ModelInfoType) -> RendererModelType {
    match model_type {
        ModelInfoType::Simple => RendererModelType::Simple,
        ModelInfoType::Time => RendererModelType::Time,
        ModelInfoType::Vehicle => RendererModelType::Vehicle,
        ModelInfoType::Ped => RendererModelType::Ped,
        ModelInfoType::Clump | ModelInfoType::Mlo | ModelInfoType::ExtraComps => {
            RendererModelType::Clump
        }
        ModelInfoType::Na => RendererModelType::Other,
    }
}

fn material_base_color(
    material: Option<&rustgta3_formats::formats::ClumpRead::RwMaterialDescriptor>,
    vehicle_colours: VehicleColourRemap,
) -> Color {
    let Some(material) = material else {
        return Color::WHITE;
    };
    let [r, g, b, a] = match material.vehicle_colour_slot() {
        Some(1) => vehicle_colours.colour1.unwrap_or([
            material.color[0],
            material.color[1],
            material.color[2],
            material.color[3],
        ]),
        Some(2) => vehicle_colours.colour2.unwrap_or([
            material.color[0],
            material.color[1],
            material.color[2],
            material.color[3],
        ]),
        _ => material.color,
    };
    // Remapped vehicle materials use white base + colour tex when `@`/nil;
    // textured non-`@` paint keeps material colour × texture (re3 SetVehicleColour).
    Color::srgba_u8(r, g, b, a)
}

fn color_needs_blend(color: Color) -> bool {
    // Bevy Color → sRGBA alpha channel (vehicle / material transparency).
    color.to_srgba().alpha < 0.999
}

/// Resolve the same model/LOD choice as `CSimpleModelInfo::GetAtomicFromDistance`
/// so every world instance can share the immutable geometry asset for that LOD.
fn world_model_asset_key(
    model_index: usize,
    distance: f32,
    use_fade_geometry: bool,
    is_big_building: bool,
    file_loader: &CFileLoader,
) -> Option<GtaWorldModelAssetKey> {
    fn resolve(
        model_index: usize,
        distance: f32,
        use_fade_geometry: bool,
        is_big_building: bool,
        file_loader: &CFileLoader,
    ) -> Option<GtaWorldModelAssetKey> {
        let (txd_generation, txd_revision) =
            world_model_txd_generation_and_revision(file_loader, model_index);
        if let Some(simple) = file_loader.model_info.simple_model_info(model_index) {
            if !simple.atomic_bindings.is_empty() {
                let lod_multiplier = file_loader.renderer.lod_distance_multiplier();
                let largest_lod_distance = simple.largest_lod_distance(lod_multiplier, false);
                let distance = renderer_simple_distance(
                    distance,
                    largest_lod_distance,
                    lod_multiplier,
                    is_big_building,
                );
                // re3 `RenderFadingAtomic` renders the atomic selected at
                // `camdist - FADE_DISTANCE` while the entity is genuinely
                // distance-fading. Draw-last objects also use the alpha list,
                // but never take this shifted LOD.
                let atomic_distance = if use_fade_geometry {
                    distance - FADE_DISTANCE
                } else {
                    distance
                };
                let binding =
                    simple.atomic_binding_from_distance(atomic_distance, lod_multiplier, false);
                if let Some(binding) = binding {
                    return Some(GtaWorldModelAssetKey {
                        model_index,
                        lod_index: binding.lod_index,
                        txd_generation,
                        txd_revision,
                    });
                }
            }
            return None;
        }
        let model = file_loader.model_info.get_model_info(model_index)?;
        let summary = model.base().clump_summary.as_ref()?;
        if summary.atomics.is_empty() && summary.geometries.is_empty() {
            return None;
        }
        Some(GtaWorldModelAssetKey {
            model_index,
            lod_index: usize::MAX,
            txd_generation,
            txd_revision,
        })
    }
    resolve(
        model_index,
        distance,
        use_fade_geometry,
        is_big_building,
        file_loader,
    )
}

/// re3 `CVisibilityPlugins::RenderFadingAtomic`: combine the shared
/// CSimpleModelInfo fade-in alpha with the final 20-unit distance fade.
/// Additive models use their authored additive pass without alpha modulation.
fn world_distance_fade_alpha(
    model_index: usize,
    distance: f32,
    is_big_building: bool,
    file_loader: &CFileLoader,
) -> Option<u8> {
    let simple = file_loader.model_info.simple_model_info(model_index)?;
    if simple.flags.additive {
        return None;
    }
    let lod_multiplier = file_loader.renderer.lod_distance_multiplier();
    let largest_lod_distance = simple.largest_lod_distance(lod_multiplier, false);
    let distance = renderer_simple_distance(
        distance,
        largest_lod_distance,
        lod_multiplier,
        is_big_building,
    );
    let fade_factor =
        ((largest_lod_distance - (distance - FADE_DISTANCE)) / FADE_DISTANCE).clamp(0.0, 1.0);
    Some((f32::from(simple.alpha) * fade_factor).clamp(0.0, 255.0) as u8)
}

fn world_model_lod_range(
    model_index: usize,
    lod_index: usize,
    lod_multiplier: f32,
    is_big_building: bool,
    file_loader: &CFileLoader,
) -> (f32, f32) {
    let Some(simple) = file_loader.model_info.simple_model_info(model_index) else {
        return (0.0, f32::INFINITY);
    };
    if simple.atomic_bindings.is_empty() {
        return (0.0, 0.0);
    }
    let max_allowed_lod = if simple.first_damaged == 0 {
        simple.num_atomics as usize
    } else {
        (simple.first_damaged as usize).min(simple.num_atomics as usize)
    };
    if lod_index >= max_allowed_lod {
        return (0.0, 0.0);
    }
    let min_dist = if lod_index == 0 {
        0.0
    } else {
        simple.lod_distances[lod_index - 1] * lod_multiplier
    };
    let far = simple.lod_distances[lod_index] * lod_multiplier;
    let max_dist = if is_big_building {
        far
    } else {
        let bump = LOD_DISTANCE * lod_multiplier + STREAM_DISTANCE;
        far.min(bump)
    };
    (min_dist, max_dist)
}

/// Build the exact simple-model atomic selected by `world_model_asset_key`.
/// Reselecting from the raw distance here can disagree with re3's adjusted
/// multi-atomic distance or its fade fallback and cache geometry under the
/// wrong LOD key.
fn build_world_model_material_drawables_for_asset_key(
    asset_key: GtaWorldModelAssetKey,
    distance: f32,
    file_loader: &CFileLoader,
) -> Vec<ModelMaterialDrawable> {
    if asset_key.lod_index != usize::MAX {
        let Some(simple) = file_loader
            .model_info
            .simple_model_info(asset_key.model_index)
        else {
            return Vec::new();
        };
        let Some(binding) = simple
            .atomic_bindings
            .iter()
            .find(|binding| binding.lod_index == asset_key.lod_index)
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        append_geometry_drawables(
            asset_key.model_index,
            &binding.geometry,
            VehicleColourRemap::default(),
            None,
            &mut out,
        );
        return out;
    }

    build_model_material_drawables(
        asset_key.model_index,
        distance,
        false,
        file_loader,
        VehicleColourRemap::default(),
    )
}

/// Material handles for a full asset key, reusing TXD-generation-independent
/// geometry. C++ rebinds textures on the persistent RwObject when its TXD
/// cycles; geometry assets and entities are untouched, so a TXD reload must
/// not decode, re-upload or respawn anything here either.
#[allow(clippy::too_many_arguments)]
fn ensure_world_materials_for_key(
    asset_key: GtaWorldModelAssetKey,
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    materials: &mut Assets<GtaUnlitMaterial>,
    model_assets: &mut GtaWorldModelAssetCache,
    lighting: GtaRenderLighting,
) -> Option<(Vec<Handle<GtaUnlitMaterial>>, usize, usize)> {
    if let Some(entry) = model_assets.entries.get(&asset_key) {
        if entry.materials.len() == entry.meshes.len() && !entry.materials.is_empty() {
            return Some((
                entry.materials.clone(),
                entry.textured_count,
                entry.missing_texture_count,
            ));
        }
    }
    let geometry = model_assets
        .geometry
        .get(&(asset_key.model_index, asset_key.lod_index))?;
    if geometry.meshes.is_empty() || geometry.meshes.len() != geometry.recipes.len() {
        return None;
    }
    let additive = file_loader
        .model_info
        .simple_model_info(asset_key.model_index)
        .is_some_and(|simple| simple.flags.additive);
    let mut built = Vec::with_capacity(geometry.recipes.len());
    let mut textured_count = 0usize;
    let mut missing_texture_count = 0usize;
    for recipe in &geometry.recipes {
        let (base_color_texture, texture_alpha, base_color) = resolve_drawable_material_bind(
            recipe,
            file_loader,
            texture_cache,
            images,
            VehicleColourRemap::default(),
        );
        if base_color_texture.is_some() {
            textured_count += 1;
        } else if recipe.texture_name.is_some() {
            missing_texture_count += 1;
        }
        built.push(materials.add(gta_world_material(
            base_color,
            base_color_texture,
            texture_alpha,
            additive,
            recipe.surface_ambient,
            recipe.surface_diffuse,
            recipe.has_prelit,
            lighting.without_directional(),
        )));
    }
    let meshes = geometry.meshes.clone();
    model_assets.entries.insert(
        asset_key,
        GtaWorldModelAssets {
            meshes,
            materials: built.clone(),
            textured_count,
            missing_texture_count,
        },
    );
    Some((built, textured_count, missing_texture_count))
}

fn sync_world_slot_fade_materials(
    slot: &mut GtaWorldMeshSlot,
    base_materials: &[Handle<GtaUnlitMaterial>],
    fade_alpha: Option<u8>,
    commands: &mut Commands,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
) {
    if base_materials.len() != slot.bevy_entities.len() {
        return;
    }

    let fade_alpha = fade_alpha.filter(|alpha| *alpha < 255);
    if let Some(alpha) = fade_alpha {
        let alpha = f32::from(alpha) / 255.0;
        let mut variants = Vec::with_capacity(base_materials.len());
        for base_handle in base_materials {
            let Some(mut material) = materials.get(base_handle).cloned() else {
                return;
            };
            // RenderWare's SetAlphaCB replaces the material alpha for the
            // fading draw, then restores it after the callback.
            material.uniforms.colour.w = alpha;
            material.alpha_mode = AlphaMode::Blend;
            variants.push(material);
        }

        let reuse_variants = slot.fade_materials.len() == variants.len()
            && slot
                .fade_materials
                .iter()
                .all(|handle| materials.get(handle).is_some());
        if reuse_variants {
            for (handle, variant) in slot.fade_materials.iter().zip(variants) {
                if materials.get(handle) != Some(&variant)
                    && let Some(mut material) = materials.get_mut(handle)
                {
                    *material = variant;
                }
            }
        } else {
            for handle in slot.fade_materials.drain(..) {
                materials.remove(handle.id());
            }
            slot.fade_materials
                .extend(variants.into_iter().map(|material| materials.add(material)));
        }

        // Asset contents carry changing fade alpha. An unchanged handle
        // must not mark every instance for material specialization again.
        if !reuse_variants {
            for (entity, material) in slot.bevy_entities.iter().zip(&slot.fade_materials) {
                commands.entity(*entity).insert(MeshMaterial3d(material.clone()));
            }
        }
    } else if !slot.fade_materials.is_empty() {
        for (entity, material) in slot.bevy_entities.iter().zip(base_materials) {
            commands
                .entity(*entity)
                .insert(MeshMaterial3d(material.clone()));
        }
        for handle in slot.fade_materials.drain(..) {
            materials.remove(handle.id());
        }
    }
}

/// Persistent world mesh: update transform/pass if the model/LOD still match,
/// otherwise rebuild the RenderWare instance mirror.
fn ensure_world_mesh_for_entity(
    entity_index: usize,
    pass: GtaWorldRenderPass,
    distance_fade: bool,
    draw_visible: bool,
    file_loader: &CFileLoader,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    model_assets: &mut GtaWorldModelAssetCache,
    registry: &mut GtaWorldMeshRegistry,
    mesh_query: &mut Query<(&mut Visibility, &mut GtaWorldMesh)>,
    transform_query: &mut Query<&mut Transform, With<GtaWorldMesh>>,
    stats: &mut GtaWorldRenderStats,
    camera_gta: CVector,
    lighting: GtaRenderLighting,
    lod_multiplier: f32,
) {
    let Some(entity) = file_loader.world.entities.get(entity_index) else {
        stats.skipped_missing_model += 1;
        return;
    };
    let model_index = entity.model_index;
    let draw_visible = draw_visible && entity.is_visible;
    // IPL positions are bounded by GTA's world; corrupt records must not
    // create enormous frustum-spanning polygons in Bevy.
    if entity
        .position
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 100_000.0)
    {
        if let Some(old) = registry.slots.remove(&entity_index) {
            registry.set_slot_presence(entity_index, false);
            registry.slots_epoch = registry.slots_epoch.wrapping_add(1);
            despawn_world_mesh_slot(commands, materials, &old);
        }
        stats.skipped_missing_geometry += 1;
        return;
    }
    let entity_position = CVector::new(entity.position[0], entity.position[1], entity.position[2]);
    let distance = world_distance(entity_position, camera_gta);
    let fade_alpha = if distance_fade && draw_visible {
        world_distance_fade_alpha(model_index, distance, entity.is_big_building, file_loader)
    } else {
        None
    };
    let normal_asset_key = world_model_asset_key(
        model_index,
        distance,
        false,
        entity.is_big_building,
        file_loader,
    );
    let use_fade_geometry = should_use_fade_geometry(
        pass,
        distance_fade,
        draw_visible,
        fade_alpha,
        normal_asset_key.is_some(),
    );
    let render_transform = world_entity_transform(entity);
    let transform_changed = registry
        .slots
        .get(&entity_index)
        .is_some_and(|slot| slot.render_transform != render_transform);
    let mut is_dynamic = transform_changed
        || entity.matrix_basis_override.is_some()
        || entity.render_position_offset != [0.0, 0.0, 0.0];

    // A cached distance interval cannot override the current native atomic
    // selection or keep geometry visible after its original model disappeared.
    if !is_dynamic && !distance_fade && !use_fade_geometry {
        if let Some(slot) = registry.slots.get(&entity_index) {
            if slot.model_index == model_index
                && normal_asset_key == Some(slot.asset_key)
                && slot.pass == pass
                && slot.is_visible == draw_visible
                && slot.fade_materials.is_empty()
                && !slot.bevy_entities.is_empty()
                && slot.txd_store_revision == file_loader.txd_store.store_revision
                && slot.lod_multiplier == lod_multiplier
                && distance >= slot.lod_min_dist
                && distance < slot.lod_max_dist
            {
                stats.spawned_meshes += slot.submesh_count;
                stats.textured_meshes += slot.textured_count;
                stats.skipped_missing_texture += slot.missing_texture_count;
                return;
            }
        }
    }

    let asset_key = if use_fade_geometry {
        world_model_asset_key(
            model_index,
            distance,
            true,
            entity.is_big_building,
            file_loader,
        )
    } else {
        normal_asset_key
    };
    let Some(asset_key) = asset_key else {
        if let Some(slot) = registry.slots.get_mut(&entity_index) {
            if slot.is_visible {
                slot.is_visible = false;
                for bevy_entity in &slot.bevy_entities {
                    if let Ok((mut vis, _)) = mesh_query.get_mut(*bevy_entity) {
                        if *vis != Visibility::Hidden {
                            *vis = Visibility::Hidden;
                        }
                    }
                }
            }
        }
        if registry
            .slots
            .get(&entity_index)
            .is_some_and(|slot| slot.model_index != model_index)
            && let Some(old) = registry.slots.remove(&entity_index)
        {
            registry.set_slot_presence(entity_index, false);
            registry.slots_epoch = registry.slots_epoch.wrapping_add(1);
            despawn_world_mesh_slot(commands, materials, &old);
        }
        stats.skipped_missing_geometry += 1;
        return;
    };

    // LOAD_SCENE may have retired this exact drawable earlier in the frame.
    // Reattach it when model+LOD still agree. A TXD generation mismatch is
    // deliberately allowed: the normal material refresh below will swap the
    // texture bindings in place while retaining the Mesh/GPU geometry.
    if !registry.has_slot(entity_index)
        && let Some(retired) = registry.retired_slots.remove(&entity_index)
    {
        let reusable = retired.model_index == model_index
            && retired.asset_key.lod_index == asset_key.lod_index
            && !retired.bevy_entities.is_empty()
            && world_model_geometry_ready(model_assets, meshes, model_index, asset_key.lod_index)
            && retired
                .bevy_entities
                .iter()
                .all(|bevy_entity| mesh_query.get(*bevy_entity).is_ok());
        if reusable {
            registry.slots.insert(entity_index, retired);
            registry.set_slot_presence(entity_index, true);
            registry.slots_epoch = registry.slots_epoch.wrapping_add(1);
            is_dynamic = is_dynamic
                || registry
                    .slots
                    .get(&entity_index)
                    .is_some_and(|slot| slot.render_transform != render_transform);
        } else {
            despawn_world_mesh_slot(commands, materials, &retired);
        }
    }

    // Secondary fast path: static entity whose slot, LOD, pass, and visibility match
    // after resolving asset_key (e.g. at transition or boundary).
    if !is_dynamic && !distance_fade {
        if let Some(slot) = registry.slots.get_mut(&entity_index) {
            if slot.model_index == model_index
                && slot.asset_key == asset_key
                && slot.pass == pass
                && slot.is_visible == draw_visible
                && slot.fade_materials.is_empty()
                && !slot.bevy_entities.is_empty()
            {
                // A global TXD-store revision can advance because an unrelated
                // dictionary streamed in/out. `asset_key` contains this model's
                // own TXD generation + revision, so equality above proves its
                // material binding is still current. Acknowledge the new global
                // revision here so this static slot returns to the ultra-fast
                // path on the next frame instead of resolving its asset key for
                // the rest of the session.
                slot.txd_store_revision = file_loader.txd_store.store_revision;
                stats.spawned_meshes += slot.submesh_count;
                stats.textured_meshes += slot.textured_count;
                stats.skipped_missing_texture += slot.missing_texture_count;
                return;
            }
        }
    }

    let mut cache_ready = world_model_assets_ready(model_assets, meshes, materials, asset_key);
    // TXD-generation-independent geometry (see GtaWorldModelGeometry): a TXD
    // evict+reload must refresh textures, never geometry or entities.
    let geom_key = (model_index, asset_key.lod_index);
    model_assets.geometry_idle_prunes.remove(&geom_key);
    let mut geometry_ready =
        world_model_geometry_ready(model_assets, meshes, model_index, asset_key.lod_index);

    let reused = registry.slots.get(&entity_index).and_then(|slot| {
        if slot.model_index != model_index
            || slot.asset_key != asset_key
            || !cache_ready
            || slot.bevy_entities.is_empty()
        {
            return None;
        }
        let mut alive = 0usize;
        let target_visibility = if draw_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for bevy_entity in &slot.bevy_entities {
            if let Ok((mut vis, mut marker)) = mesh_query.get_mut(*bevy_entity) {
                if is_dynamic {
                    if let Ok(mut t) = transform_query.get_mut(*bevy_entity) {
                        if *t != render_transform {
                            *t = render_transform;
                        }
                    }
                }
                if marker.pass != pass {
                    marker.pass = pass;
                }
                if *vis != target_visibility {
                    *vis = target_visibility;
                }
                alive += 1;
            }
        }
        (alive == slot.bevy_entities.len()).then_some((
            slot.submesh_count,
            slot.textured_count,
            slot.missing_texture_count,
        ))
    });
    if let Some((submesh_count, textured_count, missing_texture_count)) = reused {
        let base_materials = model_assets
            .entries
            .get(&asset_key)
            .map(|assets| assets.materials.clone());
        if let Some(slot) = registry.slots.get_mut(&entity_index) {
            slot.pass = pass;
            slot.is_visible = draw_visible;
            slot.txd_store_revision = file_loader.txd_store.store_revision;
            slot.render_transform = render_transform;
            if let Some(base_materials) = base_materials.as_deref() {
                sync_world_slot_fade_materials(
                    slot,
                    base_materials,
                    fade_alpha,
                    commands,
                    materials,
                );
            }
        }
        stats.spawned_meshes += submesh_count;
        stats.textured_meshes += textured_count;
        stats.skipped_missing_texture += missing_texture_count;
        return;
    }
    // TXD-only refresh on the live slot: same model/LOD with intact entities
    // and cached geometry. re3 rebinds textures on the persistent RwObject
    // here instead of recreating it, so swap material handles in place and
    // keep the entities and Mesh assets (no despawn, no GPU re-upload).
    if geometry_ready {
        if let Some(slot) = registry.slots.get(&entity_index) {
            if slot.model_index == model_index
                && slot.asset_key.lod_index == asset_key.lod_index
                && !slot.bevy_entities.is_empty()
                && slot
                    .bevy_entities
                    .iter()
                    .all(|bevy_entity| mesh_query.get(*bevy_entity).is_ok())
            {
                if let Some((refreshed, textured_count, missing_texture_count)) =
                    ensure_world_materials_for_key(
                        asset_key,
                        file_loader,
                        texture_cache,
                        images,
                        materials,
                        model_assets,
                        lighting,
                    )
                {
                    if refreshed.len() == slot.bevy_entities.len() {
                        let entities = slot.bevy_entities.clone();
                        let target_visibility = if draw_visible {
                            Visibility::Visible
                        } else {
                            Visibility::Hidden
                        };
                        let recipes = model_assets.geometry.get(&geom_key)
                            .map(|geometry| &geometry.recipes);
                        for (index, (bevy_entity, material)) in entities.iter().zip(refreshed.iter()).enumerate() {
                            commands
                                .entity(*bevy_entity)
                                .insert(MeshMaterial3d(material.clone()));
                            // Refresh MatFX DUAL / optional Neo texture handles
                            // when the GTA TXD dictionary is reloaded, without
                            // re-decoding or reallocating the resident mesh.
                            if let Some(recipe) = recipes.and_then(|recipes| recipes.get(index)) {
                                commands.entity(*bevy_entity).insert(
                                    crate::renderer::NeoRender::surface_for_drawable(
                                        recipe, file_loader, texture_cache,
                                    ),
                                );
                            }
                            if is_dynamic {
                                if let Ok(mut slot_transform) =
                                    transform_query.get_mut(*bevy_entity)
                                {
                                    if *slot_transform != render_transform {
                                        *slot_transform = render_transform;
                                    }
                                }
                            }
                            if let Ok((mut vis, mut marker)) = mesh_query.get_mut(*bevy_entity) {
                                if marker.pass != pass {
                                    marker.pass = pass;
                                }
                                if *vis != target_visibility {
                                    *vis = target_visibility;
                                }
                            }
                        }
                        let (lod_min_dist, lod_max_dist) = world_model_lod_range(
                            model_index,
                            asset_key.lod_index,
                            lod_multiplier,
                            entity.is_big_building,
                            file_loader,
                        );
                        if let Some(slot) = registry.slots.get_mut(&entity_index) {
                            slot.asset_key = asset_key;
                            slot.pass = pass;
                            slot.is_visible = draw_visible;
                            slot.textured_count = textured_count;
                            slot.missing_texture_count = missing_texture_count;
                            slot.txd_store_revision = file_loader.txd_store.store_revision;
                            slot.lod_multiplier = lod_multiplier;
                            slot.lod_min_dist = lod_min_dist;
                            slot.lod_max_dist = lod_max_dist;
                            slot.render_transform = render_transform;
                            sync_world_slot_fade_materials(
                                slot, &refreshed, fade_alpha, commands, materials,
                            );
                        }
                        stats.spawned_meshes += entities.len();
                        stats.textured_meshes += textured_count;
                        stats.skipped_missing_texture += missing_texture_count;
                        return;
                    }
                }
                // Material refresh impossible (geometry entry inconsistent with
                // its recipes); drop it so the full path below rebuilds.
                model_assets.geometry.remove(&geom_key);
                geometry_ready = false;
            }
        }
    }
    if let Some(old) = registry.slots.remove(&entity_index) {
        registry.set_slot_presence(entity_index, false);
        registry.slots_epoch = registry.slots_epoch.wrapping_add(1);
        // Model/LOD changed or one of the mirrored Bevy entities disappeared.
        despawn_world_mesh_slot(commands, materials, &old);
    }

    // TXD-only refresh without a live slot (or after the slot above was
    // dropped for a shape change): materials resolve against the live TXD
    // while cached geometry is reused. No drawable assembly, no mesh upload.
    if !cache_ready && geometry_ready {
        cache_ready = ensure_world_materials_for_key(
            asset_key,
            file_loader,
            texture_cache,
            images,
            materials,
            model_assets,
            lighting,
        )
        .is_some();
        if !cache_ready {
            // Geometry entry inconsistent with its recipes; drop it so the
            // full path below rebuilds from the streamed DFF.
            model_assets.geometry.remove(&geom_key);
        }
    }

    if !cache_ready {
        crate::profile_scope!("ensure_world_mesh_for_entity:asset_build");
        let _audit_timer = RenderAuditTimer::start("ensure_world_mesh_for_entity:asset_build");
        let additive = file_loader
            .model_info
            .simple_model_info(asset_key.model_index)
            .is_some_and(|s| s.flags.additive);
        let drawables =
            build_world_model_material_drawables_for_asset_key(asset_key, distance, file_loader);
        if drawables.is_empty() {
            if file_loader.model_info.get_model_info(model_index).is_some() {
                stats.skipped_missing_geometry += 1;
            } else {
                stats.skipped_missing_model += 1;
            }
            return;
        }
        let mut cached = GtaWorldModelAssets {
            meshes: Vec::with_capacity(drawables.len()),
            materials: Vec::with_capacity(drawables.len()),
            textured_count: 0,
            missing_texture_count: 0,
        };
        // Material recipes for TXD-only refreshes (see GtaWorldModelGeometry):
        // the drawable's mesh moves into the GPU asset, the rest is retained
        // verbatim for texture rebinding without geometry work.
        let mut recipes = Vec::with_capacity(drawables.len());
        for mut drawable in drawables {
            let (base_color_texture, texture_alpha, base_color) = resolve_drawable_material_bind(
                &drawable,
                file_loader,
                texture_cache,
                images,
                VehicleColourRemap::default(),
            );
            if base_color_texture.is_some() {
                cached.textured_count += 1;
            } else if drawable.texture_name.is_some() {
                cached.missing_texture_count += 1;
            }
            let mesh = meshes.add(std::mem::replace(
                &mut drawable.mesh,
                Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                ),
            ));
            let material = materials.add(gta_world_material(
                base_color,
                base_color_texture,
                texture_alpha,
                additive,
                drawable.surface_ambient,
                drawable.surface_diffuse,
                drawable.has_prelit,
                lighting.without_directional(),
            ));
            cached.meshes.push(mesh);
            cached.materials.push(material);
            recipes.push(drawable);
        }
        model_assets.geometry.insert(
            geom_key,
            GtaWorldModelGeometry {
                meshes: cached.meshes.clone(),
                recipes,
            },
        );
        model_assets.entries.insert(asset_key, cached);
    }

    let Some(assets) = model_assets.entries.get(&asset_key) else {
        stats.skipped_missing_geometry += 1;
        return;
    };
    let base_materials = assets.materials.clone();
    let mut bevy_entities = Vec::with_capacity(assets.meshes.len());
    let recipes = model_assets.geometry.get(&geom_key).map(|geometry| &geometry.recipes);
    for (index, (mesh, material)) in assets.meshes.iter().zip(&assets.materials).enumerate() {
        let neo_surface = recipes
            .and_then(|recipes| recipes.get(index))
            .map(|recipe| crate::renderer::NeoRender::surface_for_drawable(
                recipe, file_loader, texture_cache,
            ))
            .unwrap_or_default();
        let id = commands
            .spawn((
                GtaWorldMesh { entity_index, pass },
                neo_surface,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                // C++ `CRenderer::ScanSectorList`/`CEntity::IsVisible` has
                // already applied the authoritative GTA frustum test.  The
                // Bevy mesh-AABB test is a second, axis-aligned cull and can
                // reject transformed GTA road/building meshes even while
                // their collision remains active.  Keep visibility owned by
                // the re3 render plan and avoid this false-negative pass.
                NoFrustumCulling,
                if draw_visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                render_transform,
                GlobalTransform::default(),
            ))
            .id();
        bevy_entities.push(id);
    }
    let submesh_count = bevy_entities.len();
    stats.spawned_meshes += submesh_count;
    stats.textured_meshes += assets.textured_count;
    stats.skipped_missing_texture += assets.missing_texture_count;

    let (lod_min_dist, lod_max_dist) = world_model_lod_range(
        model_index,
        asset_key.lod_index,
        lod_multiplier,
        entity.is_big_building,
        file_loader,
    );
    registry.set_slot_presence(entity_index, true);
    registry.slots.insert(
        entity_index,
        GtaWorldMeshSlot {
            bevy_entities,
            fade_materials: Vec::new(),
            asset_key,
            model_index,
            pass,
            submesh_count,
            is_visible: draw_visible,
            textured_count: assets.textured_count,
            missing_texture_count: assets.missing_texture_count,
            txd_store_revision: file_loader.txd_store.store_revision,
            lod_multiplier,
            lod_min_dist,
            lod_max_dist,
            render_transform,
        },
    );
    if let Some(slot) = registry.slots.get_mut(&entity_index) {
        sync_world_slot_fade_materials(slot, &base_materials, fade_alpha, commands, materials);
    }
    registry.slots_epoch = registry.slots_epoch.wrapping_add(1);
}

fn should_use_fade_geometry(
    pass: GtaWorldRenderPass,
    distance_fade: bool,
    draw_visible: bool,
    fade_alpha: Option<u8>,
    normal_asset_available: bool,
) -> bool {
    // Both SetupEntityVisibility and SetupBigBuildingVisibility explicitly
    // swap to GetAtomicFromDistance(dist - FADE_DISTANCE) when the direct
    // atomic is absent. Big buildings keep that fallback geometry opaque, so
    // geometry selection must not depend solely on bDistanceFade.
    (pass == GtaWorldRenderPass::FadingIn && !normal_asset_available)
        || (distance_fade && (!draw_visible || fade_alpha.is_some_and(|alpha| alpha < 255)))
}

struct RenderAuditTimer {
    name: &'static str,
    started: Option<Instant>,
}

impl RenderAuditTimer {
    fn start(name: &'static str) -> Self {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        Self {
            name,
            started: ENABLED
                .get_or_init(|| {
                    cfg!(feature = "profiling")
                        || std::env::var_os("RUSTGTA3_RENDER_AUDIT")
                            .is_some_and(|value| value != "0")
                })
                .then(Instant::now),
        }
    }
}

impl Drop for RenderAuditTimer {
    fn drop(&mut self) {
        let Some(started) = self.started else {
            return;
        };
        let elapsed = started.elapsed();
        if elapsed.as_millis() >= 5 {
            #[cfg(feature = "profiling")]
            eprintln!(
                "RENDER_AUDIT system={} elapsed_ms={:.3}",
                self.name,
                elapsed.as_secs_f64() * 1_000.0
            );
            #[cfg(not(feature = "profiling"))]
            info!(
                target: "rustgta3_render_audit",
                system = self.name,
                elapsed_ms = elapsed.as_secs_f64() * 1_000.0,
                "render system timing"
            );
        }
    }
}

/// Opt-in state probe for real BET/JB/free-roam runs. It is dormant in normal
/// builds; set `RUSTGTA3_RENDER_AUDIT=1` while collecting a parity trace.
fn log_render_audit(
    timer: Option<Res<CTimer>>,
    camera: Option<Res<CCamera>>,
    timecycle: Option<Res<CTimeCycle>>,
    bridge: Option<Res<ScriptGameplayBridge>>,
    script_entities: Option<RuntimeActorsRead>,
    cutscene: Option<Res<CCutsceneMgr>>,
    world_stats: Option<Res<GtaWorldRenderStats>>,
    scene_camera: Query<&Transform, With<GtaSceneCamera>>,
    runtime_meshes: Query<&GtaRuntimePoolMesh>,
    mut last_log_ms: Local<u32>,
    mut enabled: Local<Option<bool>>,
) {
    let enabled = *enabled.get_or_insert_with(|| {
        std::env::var_os("RUSTGTA3_RENDER_AUDIT").is_some_and(|value| value != "0")
    });
    if !enabled {
        return;
    }
    let now_ms = timer
        .as_ref()
        .map(|timer| timer.time_in_milliseconds())
        .unwrap_or(0);
    if now_ms.wrapping_sub(*last_log_ms) < 1_000 {
        return;
    }
    *last_log_ms = now_ms;

    let mut runtime_counts = BTreeMap::<GtaRuntimePoolMeshSource, usize>::new();
    for runtime in &runtime_meshes {
        *runtime_counts.entry(runtime.source).or_default() += 1;
    }
    let bevy_camera = scene_camera
        .iter()
        .next()
        .map(|transform| (transform.translation, transform.forward().as_vec3()));
    let script_peds = script_entities.as_ref().map(|world| {
        world
            .peds()
            .map(|(handle, ped)| {
                (
                    handle,
                    crate::core::Pools::CPoolHandle(handle).index(),
                    ped.model_id,
                    ped.ped.physical.entity.get_position(),
                    ped.ped.char_created_by,
                    world.relation_for_char(handle).is_some(),
                    world
                        .current_vehicle_handle_for_char(handle)
                        .map(|vehicle_handle| {
                            crate::core::Pools::CPoolHandle(vehicle_handle).index()
                        }),
                    ped.ped.ped_state,
                    ped.ped.physical.entity.flags.is_visible,
                )
            })
            .collect::<Vec<_>>()
    });
    let script_vehicles = script_entities.as_ref().map(|world| {
        world
            .vehicles()
            .map(|(handle, vehicle)| {
                let base = vehicle.base();
                let occupants = world
                    .vehicle_entity(handle)
                    .map(|vehicle_entity| {
                        world.relation_occupants(vehicle_entity).collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                (
                    handle,
                    crate::core::Pools::CPoolHandle(handle).index(),
                    base.physical.entity.model_index,
                    base.physical.entity.get_position(),
                    base.vehicle_created_by,
                    occupants.len(),
                    occupants,
                    base.physical.entity.status,
                    base.physical.entity.flags.is_visible,
                )
            })
            .collect::<Vec<_>>()
    });
    info!(
        target: "rustgta3_render_audit",
        now_ms,
        camera = ?camera.as_ref().map(|camera| (
            camera.script_mode,
            camera.position,
            camera.look_at,
            camera.fade_value,
            camera.fading,
            camera.fade_direction,
            camera.fade_colour,
        )),
        bevy_camera = ?bevy_camera,
        timecycle = ?timecycle.as_ref().map(|timecycle| (
            timecycle.current.fog_start,
            timecycle.current.far_clip,
            timecycle.current.fog_colour,
            timecycle.current.ambient,
            timecycle.current.directional,
        )),
        bridge = ?bridge.as_ref().map(|bridge| (
            bridge.player_position,
            bridge.player_visible,
            bridge.player_control_enabled,
            bridge.cutscene_started,
            bridge.playing_intro,
        )),
        cutscene = ?cutscene.as_ref().map(|cutscene| (
            cutscene.has_loaded(),
            cutscene.is_running(),
            cutscene.cutscene_objects.len(),
        )),
        world_stats = ?world_stats.as_deref(),
        runtime_counts = ?runtime_counts,
        script_peds = ?script_peds,
        script_vehicles = ?script_vehicles,
        "render audit"
    );
}

fn srgb_i32_to_linear(rgb: [i32; 3]) -> [f32; 3] {
    let linear = Color::srgb_u8(
        rgb[0].clamp(0, 255) as u8,
        rgb[1].clamp(0, 255) as u8,
        rgb[2].clamp(0, 255) as u8,
    )
    .to_linear();
    [linear.red, linear.green, linear.blue]
}

/// World/vehicle/ped mesh material — WESL `GtaUnlitMaterial` (colour × tex × prelit × fog).
fn gta_world_material(
    base_color: Color,
    base_color_texture: Option<Handle<Image>>,
    texture_alpha: RwTextureAlphaMode,
    additive: bool,
    surface_ambient: f32,
    surface_diffuse: f32,
    has_prelit: bool,
    lighting: GtaRenderLighting,
) -> GtaUnlitMaterial {
    GtaUnlitMaterial::from_gta_lit(
        base_color,
        base_color_texture,
        texture_alpha,
        additive,
        lighting.fog_colour,
        lighting.fog_start,
        lighting.far_clip,
        lighting.ambient,
        lighting.directional,
        lighting.sun_direction,
        surface_ambient,
        surface_diffuse,
        has_prelit,
        lighting.ambient_multiplier,
        lighting.directional_enabled,
    )
}

/// Particles/sprites/lines/coronas billboards.
fn gta_billboard_material(
    base_color: Color,
    base_color_texture: Option<Handle<Image>>,
    additive: bool,
) -> StandardMaterial {
    let alpha_mode = if additive {
        AlphaMode::Add
    } else if color_needs_blend(base_color) || base_color_texture.is_some() {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };
    StandardMaterial {
        base_color,
        base_color_texture,
        unlit: true,
        alpha_mode,
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.0,
        ..default()
    }
}

fn world_entity_transform(entity: &CWorldEntity) -> Transform {
    let translation = CVector::new(
        entity.position[0] + entity.render_position_offset[0],
        entity.position[1] + entity.render_position_offset[1],
        entity.position[2] + entity.render_position_offset[2],
    )
    .to_bevy();
    if let Some([right, forward, up]) = entity.matrix_basis_override {
        let right = CVector::new(right[0], right[1], right[2]).to_bevy();
        let forward = CVector::new(forward[0], forward[1], forward[2]).to_bevy();
        let up = CVector::new(up[0], up[1], up[2]).to_bevy();

        // Converted Bevy-local columns are [right, up, -forward]. A reflected
        // re3 basis (used by rotating garage doors) needs one negative scale;
        // regular direct matrices such as the crusher lid do not.
        let desired_basis = Mat3::from_cols(right, up, -forward);
        let reflected = desired_basis.determinant() < 0.0;
        let rotation_basis = if reflected {
            Mat3::from_cols(-right, up, -forward)
        } else {
            desired_basis
        };
        return Transform {
            translation,
            rotation: Quat::from_mat3(&rotation_basis).normalize(),
            scale: if reflected {
                Vec3::new(-1.0, 1.0, 1.0)
            } else {
                Vec3::ONE
            },
        };
    }
    let rotation = if entity.rotation_degrees.abs() <= 1e-4 {
        Quat::IDENTITY
    } else {
        let axis = CVector::new(
            entity.rotation_axis[0],
            entity.rotation_axis[1],
            entity.rotation_axis[2],
        )
        .to_bevy();
        if axis.length_squared() > f32::EPSILON {
            Quat::from_axis_angle(axis.normalize(), entity.rotation_degrees.to_radians())
        } else {
            Quat::IDENTITY
        }
    };
    Transform {
        translation,
        rotation,
        // GTA III's `CFileLoader::LoadObjectInstance` parses the IPL scale
        // triplet but never applies it to the RenderWare matrix. Applying it
        // here changes both render and collision alignment relative to re3.
        scale: Vec3::ONE,
    }
}

fn compatible_model_index(model_index: i16) -> Option<usize> {
    usize::try_from(model_index).ok()
}

fn ped_record_transform(record: &CompatiblePedSave) -> Transform {
    Transform::from_translation(record.base.position.to_bevy())
}

fn vehicle_record_transform(record: &CompatibleVehicleSave) -> Transform {
    vehicle_matrix_transform(record.base.matrix)
}

fn object_record_transform(record: &CompatibleObjectSave) -> Transform {
    compressed_matrix_transform(record.matrix)
}

fn vehicle_matrix_transform(matrix: CompatibleVehicleMatrix) -> Transform {
    let right = matrix.right.to_bevy();
    let forward = matrix.forward.to_bevy();
    let up = matrix.up.to_bevy();
    Transform {
        translation: matrix.position.to_bevy(),
        rotation: basis_rotation(right, up, -forward),
        scale: Vec3::ONE,
    }
}

/// re3 `CMatrix` (GTA right/forward/up) → Bevy Y-up transform.
///
/// GTA gameplay axes: X right, Y forward, Z up.
/// Bevy: X right, Y up, Z back — `CVector::to_bevy` already swaps Y/Z components;
/// basis columns become (right, up, -forward) in Bevy space.
fn entity_matrix_to_bevy_transform(matrix: &crate::math::Matrix::CMatrix) -> Transform {
    let right = matrix.right.to_bevy();
    let forward = matrix.forward.to_bevy();
    let up = matrix.up.to_bevy();
    Transform {
        translation: matrix.position.to_bevy(),
        rotation: basis_rotation(right, up, -forward),
        scale: Vec3::ONE,
    }
}

fn compressed_matrix_transform(matrix: CompatibleCompressedMatrix) -> Transform {
    let rotation = compressed_matrix_rotation(matrix.right, matrix.up);
    Transform {
        translation: matrix.position.to_bevy(),
        rotation,
        scale: Vec3::ONE,
    }
}

fn replay_matrix_transform(
    matrix: crate::control::Replay::CCompressedMatrixNotAlignedReplay,
) -> Transform {
    let rotation = compressed_matrix_rotation(matrix.right, matrix.up);
    Transform {
        translation: CVector::new(matrix.position[0], matrix.position[1], matrix.position[2])
            .to_bevy(),
        rotation,
        scale: Vec3::ONE,
    }
}

/// re3 `CCompressedMatrixNotAligned::DecompressIntoFullMatrix`.
///
/// Despite its historic member name `m_up*`, the second serialized vector is
/// `CMatrix::GetForward()`. Reconstructing it as GTA up rotated every saved or
/// replayed entity onto the wrong pair of axes. Reorthogonalise in GTA space
/// first, then perform the common GTA-Z-up → Bevy-Y-up basis conversion.
fn compressed_matrix_rotation(right: [i8; 3], stored_forward: [i8; 3]) -> Quat {
    let decompress_axis = |axis: [i8; 3]| {
        CVector::new(
            axis[0] as f32 / 127.0,
            axis[1] as f32 / 127.0,
            axis[2] as f32 / 127.0,
        )
    };
    let mut matrix = crate::math::Matrix::CMatrix::new();
    matrix.right = decompress_axis(right);

    matrix.forward = decompress_axis(stored_forward);
    matrix.reorthogonalise();
    basis_rotation(
        matrix.right.to_bevy(),
        matrix.up.to_bevy(),
        -matrix.forward.to_bevy(),
    )
}

fn basis_rotation(right: Vec3, up: Vec3, back: Vec3) -> Quat {
    let right = right.normalize_or_zero();
    let up = up.normalize_or_zero();
    let back = back.normalize_or_zero();
    if right.length_squared() <= f32::EPSILON
        || up.length_squared() <= f32::EPSILON
        || back.length_squared() <= f32::EPSILON
    {
        Quat::IDENTITY
    } else {
        Quat::from_mat3(&Mat3::from_cols(right, up, back))
    }
}

fn world_distance(left: CVector, right: CVector) -> f32 {
    let x = left.x - right.x;
    let y = left.y - right.y;
    let z = left.z - right.z;
    (x * x + y * y + z * z).sqrt()
}

fn prepare_legacy_street_fx_render(
    timer: Res<CTimer>,
    camera: Option<Res<CCamera>>,
    timecycle: Option<Res<CTimeCycle>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    mut general: ResMut<CGeneral>,
    weather: Option<ResMut<CWeather>>,
    player_state: Option<Res<CWorldPlayerState>>,
    players: Query<(&CPed, Option<&InVehicle>), With<PlayerPedState>>,
    vehicles: crate::control::ScriptEntities::VehicleReadStore,
    antennas: Option<ResMut<CAntennas>>,
    skidmarks: Option<ResMut<CSkidmarks>>,
    rubbish: Option<ResMut<CRubbish>>,
    water_cannons: Option<ResMut<CWaterCannons>>,
    mut lines: ResMut<CLines>,
) {
    if let (Some(mut weather), Some(camera), Some(timecycle)) =
        (weather, camera.as_ref(), timecycle.as_ref())
    {
        let player_speed = player_state
            .as_ref()
            .and_then(|state| state.entities.get(state.player_in_focus).copied().flatten())
            .and_then(|entity| players.get(entity).ok())
            .map(|(player, relation)| {
                relation
                    .and_then(|relation| vehicles.get(relation.vehicle).ok())
                    .map_or(player.physical.move_speed, |vehicle| {
                        vehicle.base().physical.move_speed
                    })
            })
            .unwrap_or(CVector::ZERO);
        weather.render_rain_streaks(
            timer.time_in_milliseconds(),
            timer.get_is_code_paused(),
            timer.get_is_paused(),
            timecycle.fog_reduction,
            camera,
            player_speed,
            cutscene
                .as_ref()
                .is_some_and(|cutscene| cutscene.is_running()),
            &mut general,
        );
    }
    if let Some(mut antennas) = antennas {
        antennas.render();
        lines.lines.extend(antennas.drain_render_lines());
    }
    if let Some(mut skidmarks) = skidmarks {
        skidmarks.render(timer.time_in_milliseconds());
    }
    if let (Some(mut rubbish), Some(camera)) = (rubbish, camera.as_ref()) {
        rubbish.render(timer.time_in_milliseconds(), camera.position);
    }
    if let (Some(mut water_cannons), Some(camera)) = (water_cannons, camera.as_ref()) {
        water_cannons.render(camera.orientation, &mut || {
            u32::from(general.get_random_number())
        });
    }
}

fn flush_legacy_street_fx_meshes(
    file_loader: Res<CFileLoader>,
    texture_cache: Res<GtaWorldTextureCache>,
    weather: Option<ResMut<CWeather>>,
    rubbish: Option<ResMut<CRubbish>>,
    skidmarks: Option<ResMut<CSkidmarks>>,
    water_cannons: Option<ResMut<CWaterCannons>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
) {
    let particle_slot = file_loader.txd_store.find_txd_slot("particle");
    let mut active_count = 0;
    let mut submit = |texture_name: &str,
                      vertices: Vec<crate::renderer::RenderBuffer::RwIm3DVertexIntent>,
                      indices: Vec<u16>,
                      additive: bool| {
        let Some((mesh, material)) = build_street_fx_batch(
            texture_name,
            vertices,
            indices,
            particle_slot,
            additive,
            &texture_cache,
        ) else {
            return;
        };
        let entity = update_persistent_mesh_slot(
            immediate_cache
                .street_fx_entities
                .get(active_count)
                .copied(),
            mesh,
            material,
            Transform::default(),
            &mut persistent_query,
            &mut meshes,
            &mut materials,
            &mut commands,
        );
        commands.entity(entity).insert(GtaStreetFxMesh);
        if let Some(slot) = immediate_cache.street_fx_entities.get_mut(active_count) {
            *slot = entity;
        } else {
            immediate_cache.street_fx_entities.push(entity);
        }
        active_count += 1;
    };

    if let Some(mut weather) = weather {
        for batch in std::mem::take(&mut weather.rain_render_batches) {
            submit(batch.texture_name, batch.vertices, batch.indices, true);
        }
    }
    if let Some(mut rubbish) = rubbish {
        for batch in std::mem::take(&mut rubbish.render_batches) {
            submit(batch.texture_name, batch.vertices, batch.indices, false);
        }
    }
    if let Some(mut skidmarks) = skidmarks {
        for batch in std::mem::take(&mut skidmarks.render_batches) {
            submit(batch.texture_name, batch.vertices, batch.indices, false);
        }
    }
    if let Some(mut water_cannons) = water_cannons {
        for batch in std::mem::take(&mut water_cannons.render_batches) {
            submit("water_old", batch.vertices, batch.indices, false);
        }
    }
    drop(submit);
    hide_persistent_billboard_slots(
        &immediate_cache.street_fx_entities,
        active_count,
        &mut persistent_query,
    );
}

fn fx_runtime_diagnostics_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("RUSTGTA3_FX_DIAG").ok().is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    })
}

/// Opt-in production proof for the effects pipeline. This intentionally reads
/// the same ECS/render resources the real game uses instead of a parity/fake
/// scene. Enable with `RUSTGTA3_FX_DIAG=1`.
fn report_fx_runtime_diagnostics(
    timer: Option<Res<CTimer>>,
    particle_frame: Res<crate::renderer::ParticleSprite::GtaParticleSpriteFrame>,
    live_particles: Query<(), With<GtaParticleBillboard>>,
    skidmarks: Option<Res<CSkidmarks>>,
    file_loader: Option<Res<CFileLoader>>,
    texture_cache: Res<GtaWorldTextureCache>,
    clock: Option<Res<CClock>>,
    point_lights: Option<Res<CPointLights>>,
    coronas: Option<Res<CCoronas>>,
) {
    if !fx_runtime_diagnostics_enabled() {
        return;
    }
    let frame_counter = timer
        .as_ref()
        .map(|timer| timer.frame_counter())
        .unwrap_or(0);
    if frame_counter % 60 != 0 {
        return;
    }
    let active_skidmarks = skidmarks.as_ref().map_or(0, |skidmarks| {
        skidmarks
            .skidmarks
            .iter()
            .filter(|mark| mark.state != 0 && mark.last >= 1)
            .count()
    });
    let particle_slot = file_loader
        .as_ref()
        .and_then(|loader| loader.txd_store.find_txd_slot("particle"));
    let particle_txd_loaded = particle_slot.is_some_and(|slot| {
        file_loader
            .as_ref()
            .is_some_and(|loader| loader.txd_store.is_txd_loaded(slot))
    });
    let particle_txd_textures = particle_slot
        .and_then(|slot| {
            file_loader
                .as_ref()?
                .txd_store
                .slot(slot)?
                .texture_summary
                .as_ref()
                .map(|summary| summary.textures.len())
        })
        .unwrap_or(0);
    let has_texture = |name: &str| {
        particle_slot.is_some_and(|slot| {
            texture_cache
                .images
                .contains_key(&(slot, name.to_ascii_lowercase()))
        })
    };
    eprintln!(
        "FX_DIAG frame={} live_particles={} particle_vertices={} cloud_vertices={} sprite_batches={} active_skidmarks={} particle_slot={:?} particle_txd_loaded={} particle_txd_textures={} tex_rubber={} tex_water={} tex_skid={} tex_cloud={}/{}",
        frame_counter,
        live_particles.iter().count(),
        particle_frame.particle_vertex_count,
        particle_frame.cloud_vertex_count,
        particle_frame.batches.len(),
        active_skidmarks,
        particle_slot,
        particle_txd_loaded,
        particle_txd_textures,
        has_texture("rubber1"),
        has_texture("waterspray1"),
        has_texture("particleskid"),
        has_texture("cloud1"),
        has_texture("cloudmasked"),
    );
    let (near_streetlamps, visible_lamps, registered_lamp_coronas, lamp_samples) = file_loader.as_ref().map_or((0, 0, 0, Vec::new()), |loader| {
        let cam = loader.renderer.camera_position;
        let mut near = 0;
        let mut visible = 0;
        let mut registered = 0;
        let mut samples = Vec::new();
        for (entity_index, ent) in loader.world.entities.iter().enumerate() {
            if !(1342..=1404).contains(&ent.model_index)
                || (ent.position[0] - cam.x).abs() >= 100.0
                || (ent.position[1] - cam.y).abs() >= 100.0
            {
                continue;
            }
            let Some(effects) = loader.model_info.get_2d_effects(ent.model_index) else { continue };
            near += 1;
            if !ent.is_visible { continue }
            visible += 1;
            for (fx_index, effect) in effects.iter().enumerate() {
                if samples.len() < 5 {
                    let pos = entity_world_point(ent, effect.position);
                    let dx = pos[0] - cam.x;
                    let dy = pos[1] - cam.y;
                    samples.push(format!("{} fx={} type={} light={:?} d={:.1}",
                        ent.model_index, fx_index, effect.effect_type,
                        effect.light.as_ref().map(|l| (l.light_type, l.dist, l.range)),
                        (dx * dx + dy * dy).sqrt()));
                }
                let id = (entity_index as u32).wrapping_mul(16).wrapping_add(fx_index as u32).wrapping_add(1000);
                if coronas.as_ref().is_some_and(|lights| lights.coronas.iter().any(|c| c.id == id && c.fade_alpha > 0)) {
                    registered += 1;
                }
            }
        }
        (near, visible, registered, samples)
    });
    let particle_corona_texture = particle_slot.map_or(false, |slot| {
        texture_cache.images.contains_key(&(slot, "coronastar".to_owned()))
    });
    eprintln!(
        "LIGHT_DIAG hour={:?} near_streetlamps={} visible_lamps={} registered_lamp_coronas={} world_2dfx={} point_lights={} coronas={} corona_texture={} camera={:?} lamp_samples={:?}",
        clock.as_ref().map(|clock| clock.hours()),
        near_streetlamps,
        visible_lamps,
        registered_lamp_coronas,
        file_loader.as_ref().map_or(0, |loader| loader.model_info.two_d_effects.len()),
        point_lights.as_ref().map_or(0, |lights| lights.lights.len()),
        coronas.as_ref().map_or(0, |lights| lights.active_coronas().count()),
        particle_corona_texture,
        file_loader.as_ref().map(|loader| loader.renderer.camera_position),
        lamp_samples,
    );
}

fn build_street_fx_batch(
    texture_name: &str,
    vertices: Vec<crate::renderer::RenderBuffer::RwIm3DVertexIntent>,
    indices: Vec<u16>,
    particle_slot: Option<usize>,
    additive: bool,
    texture_cache: &GtaWorldTextureCache,
) -> Option<(Mesh, StandardMaterial)> {
    if vertices.is_empty() || indices.is_empty() {
        return None;
    }
    let positions: Vec<[f32; 3]> = vertices
        .iter()
        .map(|vertex| vertex.position.to_bevy().to_array())
        .collect();
    let colours: Vec<[f32; 4]> = vertices
        .iter()
        .map(|vertex| rgba_u8_to_f32(vertex.colour))
        .collect();
    let uvs: Vec<[f32; 2]> = vertices.iter().map(|vertex| vertex.uv).collect();
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices.into_iter().map(u32::from).collect()));
    let Some(texture) = particle_slot.and_then(|slot| {
        texture_cache
            .images
            .get(&(slot, texture_name.to_ascii_lowercase()))
            .cloned()
    }) else {
        return None;
    };
    Some((
        mesh,
        StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(texture),
            alpha_mode: if additive {
                AlphaMode::Add
            } else {
                AlphaMode::Blend
            },
            unlit: true,
            cull_mode: None,
            ..default()
        },
    ))
}

fn flush_glass_meshes(
    glass: Option<ResMut<CGlass>>,
    timer: Option<Res<CTimer>>,
    camera: Option<Res<CCamera>>,
    file_loader: Res<CFileLoader>,
    texture_cache: Res<GtaWorldTextureCache>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
    glass_panes: Query<&super::Glass::CFallingGlassPane>,
) {
    let (Some(mut glass), Some(timer), Some(camera)) = (glass, timer, camera) else {
        hide_persistent_billboard_slots(&immediate_cache.glass_entities, 0, &mut persistent_query);
        return;
    };
    glass.render(
        timer.time_in_milliseconds(),
        camera.position,
        camera.orientation,
        &glass_panes,
    );
    let particle_slot = file_loader.txd_store.find_txd_slot("particle");
    let mut active_count = 0;
    for batch in std::mem::take(&mut glass.render_batches) {
        if batch.vertices.is_empty() || batch.indices.is_empty() {
            continue;
        }
        let Some(texture) = particle_slot.and_then(|slot| {
            texture_cache
                .images
                .get(&(slot, batch.texture_name.to_ascii_lowercase()))
                .cloned()
        }) else {
            warn_once!("glass_render_missing_original_raster name={}", batch.texture_name);
            continue;
        };
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            batch
                .vertices
                .iter()
                .map(|vertex| vertex.position.to_bevy().to_array())
                .collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            batch
                .vertices
                .iter()
                .map(|vertex| rgba_u8_to_f32(vertex.colour))
                .collect::<Vec<_>>(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            batch
                .vertices
                .iter()
                .map(|vertex| vertex.uv)
                .collect::<Vec<_>>(),
        )
        .with_inserted_indices(Indices::U32(
            batch.indices.into_iter().map(u32::from).collect(),
        ));
        let entity = update_persistent_mesh_slot(
            immediate_cache.glass_entities.get(active_count).copied(),
            mesh,
            StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(texture),
                alpha_mode: match batch.blend {
                    GlassBlend::Additive => AlphaMode::Add,
                    GlassBlend::Alpha => AlphaMode::Blend,
                },
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            Transform::default(),
            &mut persistent_query,
            &mut meshes,
            &mut materials,
            &mut commands,
        );
        commands.entity(entity).insert(GtaGlassMesh);
        if let Some(slot) = immediate_cache.glass_entities.get_mut(active_count) {
            *slot = entity;
        } else {
            immediate_cache.glass_entities.push(entity);
        }
        active_count += 1;
    }
    hide_persistent_billboard_slots(
        &immediate_cache.glass_entities,
        active_count,
        &mut persistent_query,
    );
}

fn flush_line_intents(
    lines: Option<ResMut<CLines>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
) {
    let Some(mut lines) = lines else {
        hide_persistent_billboard_slots(&immediate_cache.line_entities, 0, &mut persistent_query);
        return;
    };
    let line_intents = lines.drain_lines();
    let has_lines = !line_intents.is_empty();
    // The mesh below is a pure function of the intents with constant
    // material/transform, so identical intents leave identical slot state and
    // only the (now change-guarded) hide tail still runs.
    if line_intents != immediate_cache.last_line_intents {
        immediate_cache.last_line_intents.clone_from(&line_intents);
        if has_lines {
            let mut positions = Vec::with_capacity(line_intents.len() * 2);
            let mut colours = Vec::with_capacity(line_intents.len() * 2);
            for line in line_intents {
                positions.push(line.start.position.to_bevy().to_array());
                positions.push(line.end.position.to_bevy().to_array());
                colours.push(rgba_u8_to_f32(line.start.colour));
                colours.push(rgba_u8_to_f32(line.end.colour));
            }

            let mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours);
            let entity = update_persistent_mesh_slot(
                immediate_cache.line_entities.first().copied(),
                mesh,
                StandardMaterial {
                    base_color: Color::WHITE,
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                },
                Transform::default(),
                &mut persistent_query,
                &mut meshes,
                &mut materials,
                &mut commands,
            );
            if let Some(slot) = immediate_cache.line_entities.first_mut() {
                *slot = entity;
            } else {
                immediate_cache.line_entities.push(entity);
            }
        }
    }

    hide_persistent_billboard_slots(
        &immediate_cache.line_entities,
        usize::from(has_lines),
        &mut persistent_query,
    );
}

fn flush_sprite_batches(
    sprites: Option<ResMut<CSprite>>,
    mut commands: Commands,
    // Same-frame camera eye (see sync_sky_horizon_mesh). viewport_to_world
    // needs a GlobalTransform, so rebuild one from the fresh Transform.
    // Without<> keeps this disjoint from the &mut Transform slot writer below.
    camera_query: Query<
        (&Camera, &Transform),
        (With<GtaSceneCamera>, Without<GtaPersistentImmediate>),
    >,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
    intent_query: Query<&GtaSpriteBatchIntent>,
) {
    let Some(mut sprites) = sprites else {
        hide_persistent_billboard_slots(&immediate_cache.sprite_entities, 0, &mut persistent_query);
        return;
    };
    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        hide_persistent_billboard_slots(&immediate_cache.sprite_entities, 0, &mut persistent_query);
        return;
    };
    let Some(window) = window_query.iter().next() else {
        hide_persistent_billboard_slots(&immediate_cache.sprite_entities, 0, &mut persistent_query);
        return;
    };

    sprites.flush_sprite_buffer();
    let batches = sprites.drain_flushed_batches();
    // The built mesh is a pure function of the batches plus the view key
    // (SpriteViewKey documents the exact viewport_to_world inputs), so equal
    // batches under an equal view leave identical slot state. Empty batches
    // ignore the view: nothing is built either way.
    let view_key = sprite_view_key(camera, camera_transform, window);
    let unchanged = batches == immediate_cache.last_sprite_batches
        && (batches.is_empty() || immediate_cache.last_sprite_view == Some(view_key));
    let mut active_count = immediate_cache.last_sprite_active_count;
    if !unchanged {
        immediate_cache.last_sprite_batches.clone_from(&batches);
        immediate_cache.last_sprite_view = Some(view_key);
        active_count = 0;
        for batch in batches {
            if batch.vertices.is_empty() {
                continue;
            }
            let Some(mesh) = build_sprite_mesh(
                &batch,
                camera,
                &GlobalTransform::from(*camera_transform),
                window,
            ) else {
                continue;
            };
            let entity = update_persistent_mesh_slot(
                immediate_cache.sprite_entities.get(active_count).copied(),
                mesh,
                gta_billboard_material(Color::WHITE, None, false),
                Transform::default(),
                &mut persistent_query,
                &mut meshes,
                &mut materials,
                &mut commands,
            );
            // Unconditional inserts tick change detection (and render
            // extraction) every frame; the intent is constant per slot.
            let intent = GtaSpriteBatchIntent {
                z_test_disabled: batch.z_test_disabled,
            };
            if !intent_query
                .get(entity)
                .is_ok_and(|existing| *existing == intent)
            {
                commands.entity(entity).insert((GtaSpriteMesh, intent));
            }
            if let Some(slot) = immediate_cache.sprite_entities.get_mut(active_count) {
                *slot = entity;
            } else {
                immediate_cache.sprite_entities.push(entity);
            }
            active_count += 1;
        }
        immediate_cache.last_sprite_active_count = active_count;
    }
    hide_persistent_billboard_slots(
        &immediate_cache.sprite_entities,
        active_count,
        &mut persistent_query,
    );
}

fn flush_postfx_intents(postfx: Option<ResMut<CPostFX>>) {
    let Some(mut postfx) = postfx else {
        return;
    };
    // re3 first captures the camera raster into front/back buffers, then blends
    // those captured textures in CPostFX::Render. A flat coloured screen quad is
    // not an equivalent substitute and creates an alpha mask over the frame.
    // Keep the production intent queue bounded until the real WESL render-graph
    // postprocess pass owns those camera textures.
    let _ = postfx.drain_intents();
}

fn flush_mblur_intents(mblur: Option<ResMut<CMBlur>>) {
    let Some(mut mblur) = mblur else {
        return;
    };
    // CMBlur::OverlayRender has the same camera-raster dependency as CPostFX.
    // Do not turn its RGBA intent into an unrelated translucent screen overlay.
    let _ = mblur.drain_render_intents();
}

fn flush_screen_droplets(
    droplets: Option<ResMut<ScreenDroplets>>,
    mut frame: ResMut<super::ScreenDropletRender::GtaScreenDropletFrame>,
    camera_query: Query<(&Transform, &GtaSceneCamera)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    frame.clear();
    let Some(mut droplets) = droplets else {
        return;
    };
    if !droplets.enabled || !droplets.initialised {
        return;
    }
    let Some((camera_transform, _)) = camera_query
        .iter()
        .find(|(_, scene)| scene.local_player == 0) else {
        return;
    };
    let Some(window) = window_query.iter().next() else {
        return;
    };

    // re3 main.cpp: ScreenDroplets::Process runs every frame before Render so
    // drops slide with the camera. Feed it the same-frame camera basis; the
    // previous port never called process() at all, freezing rain on the lens.
    droplets.process_camera_movement(
        CVector::from_bevy(camera_transform.translation),
        CVector::from_bevy(*camera_transform.right()),
        CVector::from_bevy(*camera_transform.up()),
        CVector::from_bevy(*camera_transform.forward()),
    );
    droplets.process();
    droplets.render();
    let viewport = Vec2::new(window.width(), window.height());
    for drop_index in droplets.render_list.iter().copied() {
        if let Some(drop) = droplets.drops.get(drop_index) {
            frame.append(drop, viewport);
        }
    }
}

fn flush_point_lights(
    point_lights: Option<Res<CPointLights>>,
    mut commands: Commands,
    mut pool: ResMut<GtaPointLightPool>,
    mut existing: Query<
        (
            &mut GtaPointLightIntent,
            &mut PointLight,
            &mut Transform,
            &mut Visibility,
        ),
        With<GtaPersistentPointLight>,
    >,
) {
    let Some(point_lights) = point_lights else {
        for entity in pool.entities.iter().copied() {
            if let Ok((_, _, _, mut visibility)) = existing.get_mut(entity) {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let mut active_count = 0;
    for light in &point_lights.lights {
        if light.light_type != LIGHT_POINT {
            continue;
        }
        let colour = [
            light.red.clamp(0.0, 1.0),
            light.green.clamp(0.0, 1.0),
            light.blue.clamp(0.0, 1.0),
        ];
        let strongest_channel = colour[0].max(colour[1]).max(colour[2]);
        if light.radius <= 0.0 || strongest_channel <= 0.0 {
            continue;
        }
        let intent = GtaPointLightIntent {
            radius: light.radius,
            colour,
            cast_extra_shadows: light.cast_extra_shadows,
        };
        let point_light = PointLight {
            color: Color::srgb(colour[0], colour[1], colour[2]),
            intensity: strongest_channel * 4_000.0,
            range: light.radius,
            radius: 0.0,
            shadow_maps_enabled: false,
            ..default()
        };
        let transform = Transform::from_translation(light.coors.to_bevy());
        let entity = pool.entities.get(active_count).copied();
        let entity = if let Some(entity) = entity
            && let Ok((mut old_intent, mut old_light, mut old_transform, mut visibility)) =
                existing.get_mut(entity)
        {
            if *old_intent != intent {
                *old_intent = intent;
            }
            if old_light.color != point_light.color
                || old_light.intensity != point_light.intensity
                || old_light.range != point_light.range
                || old_light.radius != point_light.radius
                || old_light.shadow_maps_enabled != point_light.shadow_maps_enabled
            {
                *old_light = point_light;
            }
            if *old_transform != transform {
                *old_transform = transform;
            }
            if *visibility != Visibility::Visible {
                *visibility = Visibility::Visible;
            }
            entity
        } else {
            commands
                .spawn((
                    GtaPointLight,
                    GtaPersistentPointLight,
                    intent,
                    point_light,
                    transform,
                    GlobalTransform::default(),
                    Visibility::Visible,
                ))
                .id()
        };
        if let Some(slot) = pool.entities.get_mut(active_count) {
            *slot = entity;
        } else {
            pool.entities.push(entity);
        }
        active_count += 1;
    }
    for entity in pool.entities.iter().skip(active_count).copied() {
        if let Ok((_, _, _, mut visibility)) = existing.get_mut(entity) {
            *visibility = Visibility::Hidden;
        }
    }
}

/// re3 `CPointLights::RenderFogEffect`. The light resource produces the exact
/// authored world/grid samples; this Bevy adapter resolves the original
/// `particle/pointlight` raster and submits camera-facing additive billboards.
fn flush_point_light_fog(
    point_lights: Option<Res<CPointLights>>,
    weather: Option<Res<super::Weather::CWeather>>,
    timer: Option<Res<CTimer>>,
    file_loader: Option<Res<CFileLoader>>,
    camera_query: Query<&Transform, (With<GtaSceneCamera>, Without<GtaPersistentImmediate>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut immediate_cache: ResMut<GtaImmediateMeshCache>,
    mut persistent_query: PersistentBillboardQuery,
    mut fog_sprites: Local<Vec<super::PointLights::PointLightFogSpriteIntent>>,
) {
    let (Some(point_lights), Some(file_loader), Some(camera_transform)) =
        (point_lights, file_loader, camera_query.iter().next())
    else {
        hide_persistent_billboard_slots(
            &immediate_cache.point_light_fog_entities,
            0,
            &mut persistent_query,
        );
        return;
    };

    let camera_position = CVector::from_bevy(camera_transform.translation);
    point_lights.render_fog_effect_into(
        camera_position,
        weather.as_ref().map_or(0.0, |weather| weather.foggyness),
        timer
            .as_ref()
            .map_or(0, |timer| timer.time_in_milliseconds()),
        |top, lower_z| {
            file_loader
                .world
                .process_vertical_line_static(
                    [top.x, top.y, top.z],
                    lower_z,
                    &file_loader.model_info,
                )
                .map(|hit| hit.point.z)
        },
        &mut fog_sprites,
    );

    let Some((texture, _)) = resolve_txd_texture_asset(
        &file_loader,
        &mut texture_cache,
        &mut images,
        "particle",
        "pointlight",
    ) else {
        hide_persistent_billboard_slots(
            &immediate_cache.point_light_fog_entities,
            0,
            &mut persistent_query,
        );
        return;
    };
    let billboard_mesh = unit_billboard_mesh_handle(&mut immediate_cache, &mut meshes);
    let camera_rotation = camera_transform.rotation;
    let camera_world_position = camera_transform.translation;
    let camera_forward = *camera_transform.forward();
    let mut active_count = 0;

    for intent in fog_sprites.iter().copied() {
        let world_position = intent.position.to_bevy();
        let view_depth = (world_position - camera_world_position).dot(camera_forward);
        let Some(colour) = point_light_fog_colour(intent, view_depth) else {
            continue;
        };
        let transform = Transform::from_translation(world_position)
            // RenderOneXLUSprite_Rotate_Aspect rotates in screen coordinates,
            // whose +Y is opposite Bevy billboard-local +Y.
            .with_rotation(camera_rotation * Quat::from_rotation_z(-intent.rotation))
            .with_scale(Vec3::new(intent.size * 2.0, intent.size * 2.0, 1.0));
        let entity = update_persistent_billboard_slot(
            immediate_cache
                .point_light_fog_entities
                .get(active_count)
                .copied(),
            transform,
            gta_billboard_material(colour, Some(texture.clone()), true),
            &billboard_mesh,
            &mut persistent_query,
            &mut materials,
            &mut commands,
        );
        if let Some(slot) = immediate_cache
            .point_light_fog_entities
            .get_mut(active_count)
        {
            *slot = entity;
        } else {
            immediate_cache.point_light_fog_entities.push(entity);
        }
        active_count += 1;
    }
    hide_persistent_billboard_slots(
        &immediate_cache.point_light_fog_entities,
        active_count,
        &mut persistent_query,
    );
}

fn point_light_fog_colour(
    intent: super::PointLights::PointLightFogSpriteIntent,
    view_depth: f32,
) -> Option<Color> {
    // CalcScreenCoors rejects near+1, while the unbuffered rotated sprite has
    // its own 1.5..3.0 near fade. Preserve the latter's double modulation of
    // both colour and `intens` before the final RGB multiply.
    if view_depth <= 1.5 {
        return None;
    }
    let mut intensity = intent.intensity.clamp(0.0, i16::MAX as f32) as i32;
    let mut channels = intent
        .light_colour
        .map(|channel| (channel * intent.intensity).clamp(0.0, 255.0) as i32);
    if view_depth < 3.0 {
        let fade = (((view_depth - 1.5) / 1.5) * 255.0).clamp(0.0, 255.0) as i32;
        for channel in &mut channels {
            *channel = fade * *channel >> 8;
        }
        intensity = fade * intensity >> 8;
    }
    let mut rgb = [0_u8; 3];
    for (out, channel) in rgb.iter_mut().zip(channels) {
        *out = ((channel * intensity) >> 8).clamp(0, 255) as u8;
    }
    if rgb == [0, 0, 0] {
        return None;
    }
    Some(Color::srgba_u8(rgb[0], rgb[1], rgb[2], 255))
}

fn fluff_intent_shape(
    moving_things: &CMovingThings,
    intent: FluffRenderIntent,
) -> Option<(CVector, CVector, [u8; 4], f32)> {
    match intent {
        FluffRenderIntent::ScrollBar(index) => {
            let scroll_bar = moving_things.scroll_bars.get(index)?;
            let alpha = (scroll_bar.intensity.clamp(0.0, 1.0) * 255.0) as u8;
            Some((
                scroll_bar.position,
                scroll_bar.size,
                [scroll_bar.red, scroll_bar.green, scroll_bar.blue, alpha],
                scroll_bar.scale.max(0.01),
            ))
        }
        FluffRenderIntent::TowerClock(index) => {
            let clock = moving_things.tower_clocks.get(index)?;
            let alpha = (clock.intensity.clamp(0.0, 1.0) * 255.0) as u8;
            Some((
                clock.position,
                clock.size,
                [clock.red, clock.green, clock.blue, alpha],
                clock.scale.max(0.01),
            ))
        }
        FluffRenderIntent::DigitalClock(index) => {
            let clock = moving_things.digital_clocks.get(index)?;
            let alpha = (clock.intensity.clamp(0.0, 1.0) * 255.0) as u8;
            Some((
                clock.position,
                clock.size,
                [clock.red, clock.green, clock.blue, alpha],
                clock.scale.max(0.01),
            ))
        }
    }
}

fn build_world_quad_mesh(width: f32, height: f32, colour: [u8; 4]) -> Mesh {
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-half_width, 0.0, -half_height],
            [-half_width, 0.0, half_height],
            [half_width, 0.0, half_height],
            [half_width, 0.0, -half_height],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![rgba_u8_to_f32(colour); 4])
}

fn build_sprite_mesh(
    batch: &super::Sprite::SpriteBatch,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<Mesh> {
    let viewport_size = Vec2::new(window.width(), window.height());
    if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
        return None;
    }

    let mut positions = Vec::with_capacity(batch.vertices.len());
    let mut normals = Vec::with_capacity(batch.vertices.len());
    let mut uvs = Vec::with_capacity(batch.vertices.len());
    let mut colours = Vec::with_capacity(batch.vertices.len());
    for vertex in &batch.vertices {
        let screen = Vec2::new(
            vertex.screen[0].clamp(0.0, viewport_size.x),
            vertex.screen[1].clamp(0.0, viewport_size.y),
        );
        let Ok(ray) = camera.viewport_to_world(camera_transform, screen) else {
            continue;
        };
        positions.push(ray.get_point(vertex.camera_z.max(0.1)).to_array());
        normals.push([0.0, 0.0, 1.0]);
        uvs.push(vertex.uv);
        colours.push(rgba_u8_to_f32([
            vertex.colour.red,
            vertex.colour.green,
            vertex.colour.blue,
            vertex.colour.alpha,
        ]));
    }
    if positions.len() < 3 {
        return None;
    }

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours),
    )
}

fn build_unit_billboard_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, -0.5, 0.0],
            [-0.5, 0.5, 0.0],
            [0.5, 0.5, 0.0],
            [0.5, -0.5, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0, 1.0, 1.0, 1.0]; 4])
}

fn build_sky_horizon_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-1.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, -1.0, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, -1.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
    )
}

fn particle_colour(
    intent_colour: super::Particle::ParticleColour,
    system_colour: Option<crate::objects::ParticleObject::CRGBA>,
) -> [u8; 4] {
    if intent_colour.alpha != 0 {
        [
            intent_colour.red,
            intent_colour.green,
            intent_colour.blue,
            intent_colour.alpha,
        ]
    } else if let Some(colour) = system_colour {
        [colour.red, colour.green, colour.blue, colour.alpha]
    } else {
        [255, 255, 255, 255]
    }
}

#[allow(dead_code)]
fn scale_channel(channel: u8, intensity: u8) -> u8 {
    ((u16::from(channel) * u16::from(intensity)) >> 8) as u8
}

fn normalised_degrees(mut degrees: f32) -> f32 {
    while degrees >= 360.0 {
        degrees -= 360.0;
    }
    while degrees < 0.0 {
        degrees += 360.0;
    }
    degrees
}

impl Default for GtaRenderFrameState {
    fn default() -> Self {
        Self {
            sky_top: [0; 3],
            sky_bottom: [0; 3],
            far_clip: 0.0,
            fog_start: 0.0,
        }
    }
}

fn rgb_i32_to_u8(rgb: [i32; 3]) -> [u8; 3] {
    [
        rgb[0].clamp(0, 255) as u8,
        rgb[1].clamp(0, 255) as u8,
        rgb[2].clamp(0, 255) as u8,
    ]
}

fn gta_to_bevy_position(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z, -y]
}

fn rgba_u8_to_f32(colour: [u8; 4]) -> [f32; 4] {
    [
        colour[0] as f32 / 255.0,
        colour[1] as f32 / 255.0,
        colour[2] as f32 / 255.0,
        colour[3] as f32 / 255.0,
    ]
}

#[cfg(test)]
mod compressed_matrix_render_tests {
    use super::*;

    fn assert_direction(actual: Vec3, expected: Vec3) {
        assert!(
            actual.normalize_or_zero().dot(expected.normalize_or_zero()) > 0.999_99,
            "actual {actual:?} did not match expected {expected:?}"
        );
    }

    #[test]
    fn compressed_identity_preserves_re3_forward_and_up_axes() {
        let transform = compressed_matrix_transform(CompatibleCompressedMatrix {
            position: CVector::new(3.0, 4.0, 5.0),
            right: [127, 0, 0],
            up: [0, 127, 0],
        });
        assert_eq!(transform.translation, Vec3::new(3.0, 5.0, -4.0));
        assert_direction(transform.right().as_vec3(), Vec3::X);
        assert_direction(transform.up().as_vec3(), Vec3::Y);
        assert_direction(transform.forward().as_vec3(), Vec3::NEG_Z);
    }

    #[test]
    fn compressed_positive_gta_heading_matches_full_matrix_conversion() {
        let compressed = compressed_matrix_transform(CompatibleCompressedMatrix {
            position: CVector::ZERO,
            right: [0, 127, 0],
            up: [-127, 0, 0],
        });
        let mut full = crate::math::Matrix::CMatrix::unity();
        full.set_rotate_z_only(std::f32::consts::FRAC_PI_2);
        let full = entity_matrix_to_bevy_transform(&full);
        assert_direction(compressed.right().as_vec3(), full.right().as_vec3());
        assert_direction(compressed.up().as_vec3(), full.up().as_vec3());
        assert_direction(compressed.forward().as_vec3(), full.forward().as_vec3());
    }

    #[test]
    fn replay_and_save_compressed_matrices_share_the_re3_basis() {
        let save = compressed_matrix_transform(CompatibleCompressedMatrix {
            position: CVector::new(1.0, 2.0, 3.0),
            right: [91, 90, 0],
            up: [-90, 91, 0],
        });
        let replay =
            replay_matrix_transform(crate::control::Replay::CCompressedMatrixNotAlignedReplay {
                position: [1.0, 2.0, 3.0],
                right: [91, 90, 0],
                up: [-90, 91, 0],
            });
        assert_eq!(save.translation, replay.translation);
        assert_direction(save.right().as_vec3(), replay.right().as_vec3());
        assert_direction(save.up().as_vec3(), replay.up().as_vec3());
        assert_direction(save.forward().as_vec3(), replay.forward().as_vec3());
    }
}

#[cfg(test)]
mod world_render_candidate_tests {
    use super::*;
    use crate::core::World::EEntityType;
    use rustgta3_formats::formats::TexRead::{
        RW_PLATFORM_D3D8, RwTexDictionarySummary, RwTextureNativeDescriptor,
    };

    #[test]
    fn promoted_lamp_effects_follow_live_object_and_retire_with_identity() {
        use crate::control::ScriptEntities::ScriptEntityIndex;
        use crate::objects::Object::CObject;
        use rustgta3_formats::formats::FileLoader::GtaTwoDfxLight;
        let mut app = App::new();
        let mut loader = CFileLoader::default();
        let mut placement = world_entity([100.0, 100.0, 0.0], false);
        placement.model_index = 724;
        placement.is_visible = false;
        loader.world.entities.push(placement);
        loader.world.ecs_lifecycle_objects.insert(0, 0x501);
        loader.model_info.add_2d_effect(GtaTwoDfx {
            id: 724, position: [0.0, 0.0, 5.0], color: [255, 220, 120, 255],
            effect_type: 0, raw: String::new(),
            light: Some(GtaTwoDfxLight {
                dist: 80.0, range: 10.0, size: 1.0, shadow_size: 0.0,
                shadow_intensity: 0, light_type: 0, road_reflection: 0,
                flare_type: 0, flags: 0, corona: "corona".into(), shadow: String::new(),
            }),
        });
        let mut timecycle = super::super::Timecycle::empty_timecycle();
        timecycle.current.sprite_brightness = 1.0;
        let mut clock = CClock::default();
        clock.set_game_clock(22, 0, 0);
        app.insert_resource(loader).insert_resource(timecycle).insert_resource(clock)
            .insert_resource(CCoronas::init()).init_resource::<CPointLights>()
            .init_resource::<ScriptEntityIndex>()
            .add_systems(Update, (update_coronas_lifecycle, update_atmosphere_and_fx).chain());
        app.world_mut().spawn((GtaSceneCamera { local_player: 0 }, Transform::default()));
        let mut object = CObject::new_with_model(724, false, &mut CGeneral::with_seed(1));
        object.physical.entity.set_position(CVector::new(2.0, 3.0, 0.0));
        let owner = app.world_mut().spawn(object).id();
        app.world_mut().resource_mut::<ScriptEntityIndex>().objects.insert(0x501, owner);
        app.update();
        let lights = app.world().resource::<CPointLights>();
        assert_eq!(lights.lights.len(), 1);
        assert_eq!(lights.lights[0].coors, CVector::new(2.0, 3.0, 5.0));
        assert!(app.world().resource::<CCoronas>().coronas.iter().any(|c| c.id == 1000));
        app.world_mut().get_mut::<CObject>(owner).unwrap().physical.entity.flags.is_visible = false;
        app.world_mut().resource_mut::<CPointLights>().init_per_frame();
        app.update();
        assert!(app.world().resource::<CPointLights>().lights.is_empty());
        app.world_mut().get_mut::<CObject>(owner).unwrap().physical.entity.flags.is_visible = true;
        app.world_mut().resource_mut::<ScriptEntityIndex>().objects.remove(&0x501);
        app.update();
        assert!(app.world().resource::<CPointLights>().lights.is_empty());
    }

    fn dummy_world_material_recipe() -> ModelMaterialDrawable {
        ModelMaterialDrawable {
            mesh: Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            ),
            base_color: Color::WHITE,
            texture_name: None,
            dual_texture_name: None,
            texture_model_index: 0,
            texture_txd_slot: None,
            vertex_count: 0,
            index_count: 0,
            vehicle_colour_slot: None,
            specular: 0.0,
            surface_ambient: 1.0,
            surface_diffuse: 1.0,
            has_prelit: false,
            dynamic_vehicle_component: false,
            vehicle_component_transform: None,
        }
    }

    #[test]
    fn world_geometry_cache_is_not_ready_after_mesh_asset_is_evicted() {
        let mut meshes = Assets::<Mesh>::default();
        let mesh = meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        ));
        let mut cache = GtaWorldModelAssetCache::default();
        cache.geometry.insert(
            (77, 0),
            GtaWorldModelGeometry {
                meshes: vec![mesh.clone()],
                recipes: vec![dummy_world_material_recipe()],
            },
        );

        assert!(world_model_geometry_ready(&cache, &meshes, 77, 0));
        meshes.remove(mesh.id());
        assert!(
            !world_model_geometry_ready(&cache, &meshes, 77, 0),
            "a retired world slot must not be reclaimed when its Mesh asset was pruned"
        );
    }

    #[test]
    fn world_asset_cache_is_not_ready_after_material_asset_is_evicted() {
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<GtaUnlitMaterial>::default();
        let mesh = meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        ));
        let material = materials.add(GtaUnlitMaterial::from_gta(
            Color::WHITE,
            None,
            false,
            false,
            [0.0; 3],
            0.0,
        ));
        let key = GtaWorldModelAssetKey {
            model_index: 91,
            lod_index: 0,
            txd_generation: 1,
            txd_revision: 1,
        };
        let mut cache = GtaWorldModelAssetCache::default();
        cache.entries.insert(
            key,
            GtaWorldModelAssets {
                meshes: vec![mesh],
                materials: vec![material.clone()],
                textured_count: 0,
                missing_texture_count: 0,
            },
        );

        assert!(world_model_assets_ready(&cache, &meshes, &materials, key));
        materials.remove(material.id());
        assert!(
            !world_model_assets_ready(&cache, &meshes, &materials, key),
            "a world slot must rebuild when its material asset no longer exists"
        );
    }

    pub(super) fn world_entity(position: [f32; 3], big: bool) -> CWorldEntity {
        CWorldEntity {
            entity_type: EEntityType::Building,
            model_index: 0,
            name: String::new(),
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
            is_big_building: big,
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

    fn add_empty_simple_binding(
        file_loader: &mut CFileLoader,
        model_index: usize,
        lod_index: usize,
    ) {
        use rustgta3_formats::formats::ClumpRead::{RwAtomicDescriptor, RwGeometryDescriptor};

        let frame_name = format!("lod{lod_index}");
        assert!(file_loader.model_info.set_simple_atomic_binding(
            model_index,
            lod_index,
            &frame_name,
            RwAtomicDescriptor {
                frame_index: lod_index as i32,
                geometry_index: lod_index as i32,
                flags: 0,
                frame_name: Some(frame_name.clone()),
            },
            RwGeometryDescriptor {
                flags: 0,
                raw_flags: 0,
                num_triangles: 0,
                num_vertices: 0,
                num_morph_targets: 1,
                num_materials: 0,
                num_tex_coord_sets: 0,
                chunk_version: 0,
                chunk_length: 0,
                decoded: None,
                materials: Vec::new(),
                skin: None,
            },
        ));
    }

    fn one_pixel_txd(texture_name: &str, bgra: [u8; 4]) -> RwTexDictionarySummary {
        RwTexDictionarySummary {
            num_textures: 1,
            first_phase_textures: 1,
            texture_native_chunks: 1,
            textures: vec![RwTextureNativeDescriptor {
                platform_id: RW_PLATFORM_D3D8,
                chunk_version: 0,
                chunk_length: 0,
                filter_addressing: 0,
                name: texture_name.to_owned(),
                mask: String::new(),
                raster_format: 0,
                d3d_format: None,
                has_alpha: bgra[3] != 255,
                width: 1,
                height: 1,
                depth: 32,
                num_levels: 1,
                raster_type: 4,
                compression_or_flags: 0,
                palette: Vec::new(),
                mip_level_sizes: vec![4],
                mip_level_payloads: vec![bgra.to_vec()],
            }],
            bevy_images: Vec::new(),
        }
    }

    #[test]
    fn renderer_candidates_scan_near_sectors_deduplicate_and_keep_big_buildings() {
        let mut world = CWorld::default();
        world.entities.push(world_entity([0.0, 0.0, 0.0], false));
        world
            .entities
            .push(world_entity([-1_900.0, -1_900.0, 0.0], false));
        world
            .entities
            .push(world_entity([1_500.0, 1_500.0, 0.0], true));
        let x = CWorld::get_sector_index_x(0.0);
        let y = CWorld::get_sector_index_y(0.0);
        let near = &mut world.sectors[y * NUMSECTORS_X + x];
        near.buildings.push(0);
        near.buildings_overlap.push(0);
        world.sectors[0].buildings.push(1);
        world.big_buildings_by_level[0].push(2);
        let mut scratch = WorldRenderCandidateScratch::default();
        assert_eq!(scratch.collect(&world, CVector::ZERO), &[0, 2]);
    }

    #[test]
    fn render_plan_preserves_big_building_delete_rw_object_side_effect() {
        let mut file_loader = CFileLoader::default();
        let model_index = 60;
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.lod_distances = [100.0, 0.0, 0.0];
            simple.num_atomics = 1;
            simple.alpha = 255;
            simple.flags.no_fade = true;
        }
        let mut entity = world_entity([500.0, 0.0, 0.0], true);
        entity.model_index = model_index;
        let entity_snapshot =
            world_entity_snapshot_for_camera(&entity, &file_loader.model_info, None, true);
        let model_snapshot = world_model_snapshot(
            model_index,
            entity.position,
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            true,
        )
        .expect("big-building model snapshot");
        let mut renderer = CRenderer::default();
        renderer.begin_construct_render_list(CVector::ZERO, 12, 1.0);
        let mut plan = RendererPassPlan::default();
        renderer.construct_render_pass_plan_after_begin(
            [(0, entity_snapshot, model_snapshot)],
            &[entity],
            &mut plan,
        );

        assert_eq!(plan.rw_object_deletes, vec![0]);
        assert!(plan.roads.is_empty());
        assert!(plan.everything_bar_roads.is_empty());
    }

    #[test]
    fn render_plan_preserves_offscreen_simple_rw_object_and_alpha_side_effects() {
        let entity = world_entity([40.0, 0.0, 0.0], false);
        let mut entity_snapshot = RendererEntitySnapshot {
            entity_type: entity.entity_type,
            model_index: entity.model_index,
            position: CVector::new(40.0, 0.0, 0.0),
            rw_object_loaded: false,
            visible: true,
            on_screen: false,
            on_screen_complex: false,
            draw_last: false,
            render_damaged: false,
            first_person_player_vehicle: false,
            first_person_camera_forward: false,
            first_person_big_vehicle_exception: false,
            temporary_object: false,
            big_building: false,
        };
        entity_snapshot.model_index = entity.model_index;
        let model_snapshot = RendererModelSnapshot {
            model_type: RendererModelType::Simple,
            loaded_for_distance: true,
            loaded_for_fade_distance: true,
            largest_lod_distance: 100.0,
            near_distance: 0.0,
            no_fade: false,
            draw_last: false,
            alpha: 64,
            time_on: None,
            time_off: None,
            other_time_model_exists: false,
            other_time_model_loaded: false,
            related_model_exists: false,
            related_model_loaded: false,
            related_model_alpha: 255,
            related_time_other_loaded: false,
        };
        let mut renderer = CRenderer::default();
        renderer.begin_construct_render_list(CVector::ZERO, 12, 1.0);
        let mut plan = RendererPassPlan::default();
        renderer.construct_render_pass_plan_after_begin(
            [(0, entity_snapshot, model_snapshot)],
            &[entity],
            &mut plan,
        );

        assert_eq!(plan.rw_object_creates, vec![(0, false)]);
        assert_eq!(plan.model_alpha_updates, vec![(0, true, true)]);
        assert_eq!(plan.invisible_prerender, vec![0]);
        assert!(plan.roads.is_empty());
        assert!(plan.everything_bar_roads.is_empty());
    }

    #[test]
    fn render_plan_separates_distance_fade_from_draw_last_alpha_sorting() {
        let fade_entity = world_entity([40.0, 0.0, 0.0], false);
        let mut draw_last_entity = world_entity([50.0, 0.0, 0.0], false);
        draw_last_entity.model_index = fade_entity.model_index;

        let make_entity_snapshot = |position: f32, draw_last: bool| RendererEntitySnapshot {
            entity_type: fade_entity.entity_type,
            model_index: fade_entity.model_index,
            position: CVector::new(position, 0.0, 0.0),
            rw_object_loaded: true,
            visible: true,
            on_screen: true,
            on_screen_complex: true,
            draw_last,
            render_damaged: false,
            first_person_player_vehicle: false,
            first_person_camera_forward: false,
            first_person_big_vehicle_exception: false,
            temporary_object: false,
            big_building: false,
        };
        let make_model_snapshot = |alpha: u8| RendererModelSnapshot {
            model_type: RendererModelType::Simple,
            loaded_for_distance: true,
            loaded_for_fade_distance: true,
            largest_lod_distance: 100.0,
            near_distance: 0.0,
            no_fade: false,
            draw_last: false,
            alpha,
            time_on: None,
            time_off: None,
            other_time_model_exists: false,
            other_time_model_loaded: false,
            related_model_exists: false,
            related_model_loaded: false,
            related_model_alpha: 255,
            related_time_other_loaded: false,
        };

        let mut renderer = CRenderer::default();
        renderer.begin_construct_render_list(CVector::ZERO, 12, 1.0);
        let mut plan = RendererPassPlan::default();
        renderer.construct_render_pass_plan_after_begin(
            [
                (
                    0,
                    make_entity_snapshot(40.0, false),
                    make_model_snapshot(64),
                ),
                (
                    1,
                    make_entity_snapshot(50.0, true),
                    make_model_snapshot(255),
                ),
            ],
            &[fade_entity, draw_last_entity],
            &mut plan,
        );

        assert_eq!(plan.fading_in_entities, vec![1, 0]);
        assert_eq!(plan.distance_fade_entities, vec![0]);
    }

    #[test]
    fn priority_cull_hidden_road_requests_without_entering_render_lists() {
        let mut entity = world_entity([40.0, 0.0, 0.0], false);
        entity.treadable_pool_slot = Some(0);
        let model_index = entity.model_index;
        let entity_snapshot = RendererEntitySnapshot {
            entity_type: entity.entity_type,
            model_index,
            position: CVector::new(40.0, 0.0, 0.0),
            rw_object_loaded: false,
            visible: false,
            on_screen: true,
            on_screen_complex: true,
            draw_last: false,
            render_damaged: false,
            first_person_player_vehicle: false,
            first_person_camera_forward: false,
            first_person_big_vehicle_exception: false,
            temporary_object: false,
            big_building: false,
        };
        let model_snapshot = RendererModelSnapshot {
            model_type: RendererModelType::Simple,
            loaded_for_distance: false,
            loaded_for_fade_distance: false,
            largest_lod_distance: 100.0,
            near_distance: 0.0,
            no_fade: false,
            draw_last: false,
            alpha: 255,
            time_on: None,
            time_off: None,
            other_time_model_exists: false,
            other_time_model_loaded: false,
            related_model_exists: false,
            related_model_loaded: false,
            related_model_alpha: 255,
            related_time_other_loaded: false,
        };
        let mut renderer = CRenderer::default();
        renderer.begin_construct_render_list(CVector::ZERO, 12, 1.0);
        let mut plan = RendererPassPlan::default();
        renderer.construct_render_pass_plan_after_begin_with_scan_flags(
            [(0, entity_snapshot, model_snapshot, true, false)],
            &[entity],
            &mut plan,
        );

        assert_eq!(plan.priority_stream_requests, vec![model_index]);
        assert!(plan.stream_requests.is_empty());
        assert!(plan.roads.is_empty());
        assert!(plan.everything_bar_roads.is_empty());
        assert!(plan.invisible_prerender.is_empty());
    }

    #[test]
    fn zone_culled_entity_keeps_authored_visible_bit_in_snapshot() {
        // re3 `SetupEntityVisibility` reads only `bIsVisible`; the zone gate
        // lives outside it (`Renderer.cpp:1528`). The snapshot must stay pure.
        let file_loader = CFileLoader::default();
        let mut entity = world_entity([10.0, 0.0, 0.0], false);
        entity.zone_culled = true;
        entity.zone_culled2 = true;
        let snapshot =
            world_entity_snapshot_for_camera(&entity, &file_loader.model_info, None, true);
        assert!(snapshot.visible);
    }

    #[test]
    fn zone_gate_bypass_matches_re3_debug_flag() {
        // re3 `gbDisableZoneCull` bypasses `IsEntityCullZoneVisible`.
        let entity = world_entity([10.0, 0.0, 0.0], false);
        let mut culled = entity;
        culled.zone_culled = true;
        assert!(!entity_cull_zone_visible(false, &culled));
        assert!(entity_cull_zone_visible(true, &culled));
    }

    #[test]
    fn renderer_candidates_scan_only_collision_resident_island_and_generic_big_buildings() {
        let mut world = CWorld::default();
        for _ in 0..4 {
            world
                .entities
                .push(world_entity([1_500.0, 1_500.0, 0.0], true));
        }
        world.big_buildings_by_level[0].push(0);
        world.big_buildings_by_level[1].push(1);
        world.big_buildings_by_level[2].push(2);
        world.big_buildings_by_level[3].push(3);

        let mut scratch = WorldRenderCandidateScratch::default();
        assert_eq!(
            scratch.collect_many(
                &world,
                &[CVector::ZERO],
                LOD_DISTANCE,
                ELevelName::Commercial,
            ),
            &[2, 0]
        );
    }

    #[test]
    fn renderer_priority_triangle_is_disabled_inside_lod_distance() {
        let camera = CCamera::default();
        assert!(
            renderer_priority_sector_triangle(
                &camera,
                CVector::ZERO,
                70.0,
                4.0 / 3.0,
                LOD_DISTANCE,
                LOD_DISTANCE,
            )
            .is_none()
        );
    }

    #[test]
    fn downward_flyby_view_does_not_enter_cpp_topdown_scan_path() {
        let mut camera = CCamera::default();
        camera.script_mode = CameraScriptMode::FlyBy;
        camera.cams[camera.active_cam].mode = crate::core::Cam::eCamMode::MODE_FLYBY;
        camera.cams[camera.active_cam].front = CVector::new(0.0, 0.1, -0.99);

        assert!(
            !renderer_is_topdown(&camera),
            "re3 selects the top-down ScanWorld branch by CCam mode, not by a downward FlyBy pitch"
        );

        camera.cams[camera.active_cam].mode = crate::core::Cam::eCamMode::MODE_TOPDOWN;
        assert!(renderer_is_topdown(&camera));
    }

    #[test]
    fn renderer_priority_triangle_scans_forward_center_sectors_only() {
        let camera = CCamera::default();
        let triangle = renderer_priority_sector_triangle(
            &camera,
            CVector::ZERO,
            70.0,
            4.0 / 3.0,
            LOD_DISTANCE,
            2000.0,
        )
        .expect("priority triangle");
        let mut world = CWorld::default();
        for position in [[0.0, 220.0, 0.0], [220.0, 220.0, 0.0], [0.0, -80.0, 0.0]] {
            world.entities.push(world_entity(position, false));
        }
        let center_x = CWorld::get_sector_index_x(0.0);
        let forward_y = CWorld::get_sector_index_y(220.0);
        let side_x = CWorld::get_sector_index_x(220.0);
        let behind_y = CWorld::get_sector_index_y(-80.0);
        world.sectors[forward_y * NUMSECTORS_X + center_x]
            .buildings
            .push(0);
        world.sectors[forward_y * NUMSECTORS_X + side_x]
            .buildings
            .push(1);
        world.sectors[behind_y * NUMSECTORS_X + center_x]
            .buildings
            .push(2);

        let mut scratch = WorldRenderCandidateScratch::default();
        scratch.collect_priority_triangle(&world, Some(triangle));
        assert_eq!(scratch.priority_indices, vec![0]);
    }

    #[test]
    fn renderer_world_candidates_follow_scanworld_frustum_instead_of_radius_square() {
        let camera = CCamera::default();
        let triangle = renderer_scan_sector_triangle(
            &camera,
            CVector::ZERO,
            70.0,
            4.0 / 3.0,
            LOD_DISTANCE,
            2000.0,
        )
        .expect("scan triangle");
        let mut world = CWorld::default();
        for position in [[0.0, 220.0, 0.0], [220.0, 220.0, 0.0], [0.0, -80.0, 0.0]] {
            world.entities.push(world_entity(position, false));
        }
        let center_x = CWorld::get_sector_index_x(0.0);
        let forward_y = CWorld::get_sector_index_y(220.0);
        let side_x = CWorld::get_sector_index_x(220.0);
        let behind_y = CWorld::get_sector_index_y(-80.0);
        world.sectors[forward_y * NUMSECTORS_X + center_x]
            .buildings
            .push(0);
        world.sectors[forward_y * NUMSECTORS_X + side_x]
            .buildings
            .push(1);
        world.sectors[behind_y * NUMSECTORS_X + center_x]
            .buildings
            .push(2);

        let mut scratch = WorldRenderCandidateScratch::default();
        let (indices, priority_count) = scratch.collect_scanworld_many(
            &world,
            &[CVector::ZERO],
            None,
            Some(triangle),
            None,
            LOD_DISTANCE,
            ELevelName::Generic,
        );
        assert_eq!(priority_count, 0);
        assert_eq!(indices, &[0]);
    }

    #[test]
    fn renderer_scanworld_priority_prefix_is_not_rescanned_by_normal_frustum() {
        let camera = CCamera::default();
        let priority = renderer_priority_sector_triangle(
            &camera,
            CVector::ZERO,
            70.0,
            4.0 / 3.0,
            LOD_DISTANCE,
            2000.0,
        )
        .expect("priority triangle");
        let normal = renderer_scan_sector_triangle(
            &camera,
            CVector::ZERO,
            70.0,
            4.0 / 3.0,
            LOD_DISTANCE,
            2000.0,
        )
        .expect("normal triangle");
        let mut world = CWorld::default();
        for position in [[0.0, 220.0, 0.0], [100.0, 220.0, 0.0]] {
            world.entities.push(world_entity(position, false));
        }
        for (index, position) in [[0.0, 220.0], [100.0, 220.0]].into_iter().enumerate() {
            let x = CWorld::get_sector_index_x(position[0]);
            let y = CWorld::get_sector_index_y(position[1]);
            world.sectors[y * NUMSECTORS_X + x].buildings.push(index);
        }

        let mut scratch = WorldRenderCandidateScratch::default();
        let (indices, priority_count) = scratch.collect_scanworld_many(
            &world,
            &[CVector::ZERO],
            Some(priority),
            Some(normal),
            None,
            LOD_DISTANCE,
            ELevelName::Generic,
        );
        assert_eq!(priority_count, 1);
        assert_eq!(indices[0], 0);
        assert_eq!(indices.iter().filter(|&&index| index == 0).count(), 1);
        assert!(indices.contains(&1));
    }

    #[test]
    fn renderer_topdown_sector_rect_scans_ground_bounding_box_without_slits() {
        let mut world = CWorld::default();
        // Place entities across a 4x4 sector area centered around the origin.
        let mut entity_idx = 0;
        let mut expected = Vec::new();
        for iy in 48..=52 {
            for ix in 48..=52 {
                world.entities.push(world_entity([0.0, 0.0, 0.0], false));
                world.sectors[iy * NUMSECTORS_X + ix]
                    .buildings
                    .push(entity_idx);
                expected.push(entity_idx);
                entity_idx += 1;
            }
        }

        let mut scratch = WorldRenderCandidateScratch::default();
        let topdown_rect = [48, 52, 48, 52];
        let (indices, priority_count) = scratch.collect_scanworld_many(
            &world,
            &[CVector::new(0.0, 0.0, 50.0)],
            None,
            None,
            Some(topdown_rect),
            LOD_DISTANCE,
            ELevelName::Generic,
        );

        assert_eq!(priority_count, 0);
        for id in expected {
            assert!(
                indices.contains(&id),
                "topdown ground rect must contain sector entity {id}"
            );
        }
    }

    #[test]
    fn scm_model_swap_invalidates_the_persistent_world_mesh_model() {
        let mut world = CWorld::default();
        let mut entity = world_entity([0.0, 0.0, 0.0], false);
        entity.model_index = 1143;
        world.entities.push(entity);
        assert!(world_mesh_model_is_current(&world, 0, 1143));

        world.entities[0].model_index = 1147;
        assert!(!world_mesh_model_is_current(&world, 0, 1143));
        assert!(world_mesh_model_is_current(&world, 0, 1147));
    }

    #[test]
    fn ordinary_instance_deletion_releases_refs_without_forcing_model_eviction() {
        for initial_refs in [1, 2] {
            let mut loader = crate::core::FileLoader::CFileLoader::default();
            let model = loader.model_info.add_simple_model(1000);
            model.base.ref_count = initial_refs;
            model.base.rw_object_loaded = true;
            loader.model_info.add_simple_model(1001);
            let mut building = world_entity([0.0; 3], false);
            building.entity_type = crate::core::World::EEntityType::Building;
            building.model_index = 1000;
            loader.world.entities.push(building);
            let mut streaming = crate::core::Streaming::CStreaming::default();
            streaming.info_for_model[1000].load_state = crate::core::Streaming::StreamLoadState::Loaded;
            let mut refs = GtaRuntimeEntityModelRefs::default();
            refs.owned.insert((GtaRuntimeModelRefSource::World, 0), 1000);
            let mut app = App::new();
            app.insert_resource(loader).insert_resource(streaming).insert_resource(refs)
                .init_resource::<GtaWorldMeshRegistry>()
                .add_systems(Update, sync_runtime_entity_model_refs);
            app.update();
            assert!(app.world().resource::<GtaRuntimeEntityModelRefs>().owned.is_empty());
            let loader = app.world().resource::<crate::core::FileLoader::CFileLoader>();
            assert_eq!(loader.model_info.get_model_info(1000).unwrap().base().ref_count, initial_refs - 1);
            assert_eq!(app.world().resource::<crate::core::Streaming::CStreaming>().info_for_model[1000].load_state,
                crate::core::Streaming::StreamLoadState::Loaded);
        }
    }

    #[test]
    fn scm_swap_nearest_building_model_does_not_stream_non_big_building() {
        use crate::control::Script::{
            CTheScripts, ScriptCommandEffect, ScriptGameplayBridge,
            apply_script_effects_to_world_with_streaming,
        };
        use crate::core::FileLoader::CFileLoader;
        use crate::core::Streaming::{CStreaming, STREAMFLAGS_DONT_REMOVE, StreamLoadState};
        use crate::core::World::EEntityType;
        use rustgta3_core::core::Coordinates::CVector;
        use rustgta3_formats::formats::Script::ScmFile;

        let mut file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        let mut bridge = ScriptGameplayBridge::default();
        let mut game = rustgta3_core::core::GameState::CGameState::default();

        // 1143 is nbbridgerdb (near bridge piece, draw distance 299, not a big building)
        let simple1143 = file_loader.model_info.add_simple_model(1143);
        simple1143.lod_distances[0] = 299.0;
        simple1143.is_big_building = false;

        // 1147 is damgbridgerdb (damaged bridge piece, draw distance 299, not a big building)
        let simple1147 = file_loader.model_info.add_simple_model(1147);
        simple1147.lod_distances[0] = 299.0;
        simple1147.is_big_building = false;

        let entity_pos = [702.75, -939.94, 38.69];
        let entity_index = file_loader.world.entities.len();
        let mut entity = world_entity(entity_pos, false);
        entity.entity_type = EEntityType::Building;
        entity.model_index = 1143;
        entity.is_big_building = false;
        file_loader.world.entities.push(entity);
        file_loader.world.add(entity_index, &file_loader.model_info);

        assert_eq!(file_loader.world.entities[entity_index].model_index, 1143);
        assert!(!file_loader.world.entities[entity_index].is_big_building);

        scripts
            .pending_effects
            .push(ScriptCommandEffect::SwapNearestBuildingModel {
                position: CVector::new(702.75, -939.94, 38.69),
                radius: 20.0,
                old_model: 1143,
                new_model: 1147,
            });

        apply_script_effects_to_world_with_streaming(
            &mut scripts,
            &mut bridge,
            Some(&mut game),
            None,
            None,
            Some(&mut streaming),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&mut file_loader),
            None,
            None,
            0,
            None,
        );

        // 1. World entity model is updated to 1147
        assert_eq!(file_loader.world.entities[entity_index].model_index, 1147);
        // 2. re3 CBuilding::ReplaceWithNewModel only requests a replacement
        // model when the *building entity* is a big building. Ordinary bridge
        // pieces rely on the SCM REQUEST_MODEL / LOAD_ALL_MODELS_NOW sequence
        // that precedes the swap; the swap itself must not drain/reseed
        // streaming.
        assert_eq!(
            streaming.info_for_model[1147].load_state,
            StreamLoadState::NotLoaded
        );
        assert_eq!(
            streaming.info_for_model[1147].flags & STREAMFLAGS_DONT_REMOVE,
            0
        );
        assert!(!file_loader.world.entities[entity_index].is_big_building);
        assert!(!file_loader.world.entities[entity_index].streaming_dont_delete);
        // 3. The deferred Bevy render bridge owns the RwObject hand-off. The
        // generic far-object deletion queue must not tear the old drawable
        // down before the replacement DFF becomes resident.
        assert!(
            !file_loader
                .take_pending_world_rw_object_deletes()
                .contains(&entity_index)
        );
        // 4. The old model begins non-resident in this isolated test.
        assert_eq!(
            streaming.info_for_model[1143].load_state,
            StreamLoadState::NotLoaded
        );
        // 5. Script building_swaps records the swap
        assert!(
            scripts
                .building_swaps
                .iter()
                .any(|s| s.building_ref == Some(entity_index as i32) && s.new_model == 1147)
        );
    }

    #[test]
    fn txd_reload_with_same_texture_count_replaces_slot_cache_generation() {
        let mut file_loader = CFileLoader::default();
        let slot = file_loader.txd_store.add_txd_slot("roads");
        file_loader
            .txd_store
            .mark_loaded(slot, Some(one_pixel_txd("oldroad", [10, 20, 30, 255])));
        let first_generation = file_loader
            .txd_store
            .slot(slot)
            .expect("roads txd")
            .generation;
        let mut cache = GtaWorldTextureCache::default();
        let mut images = Assets::<Image>::default();
        assert!(sync_txd_slot_texture_cache(
            slot,
            &file_loader,
            &mut cache,
            &mut images,
        ));
        assert!(texture_cache_lookup(&cache, slot, "oldroad").is_some());
        assert_eq!(
            cache.synced_txd_generations.get(&slot).copied(),
            Some(first_generation)
        );

        file_loader.txd_store.remove_txd(slot);
        file_loader
            .txd_store
            .mark_loaded(slot, Some(one_pixel_txd("newroad", [40, 50, 60, 255])));
        let second_generation = file_loader
            .txd_store
            .slot(slot)
            .expect("roads txd")
            .generation;
        assert!(second_generation > first_generation);
        assert!(sync_txd_slot_texture_cache(
            slot,
            &file_loader,
            &mut cache,
            &mut images,
        ));

        assert!(
            texture_cache_lookup(&cache, slot, "oldroad").is_none(),
            "an old dictionary name must not survive a same-count TXD replacement"
        );
        assert!(texture_cache_lookup(&cache, slot, "newroad").is_some());
        assert_eq!(
            cache.synced_txd_generations.get(&slot).copied(),
            Some(second_generation)
        );
    }

    #[test]
    fn txd_finish_phase_refreshes_textures_without_changing_dictionary_identity() {
        let mut file_loader = CFileLoader::default();
        let slot = file_loader.txd_store.add_txd_slot("roads");
        let first_half = one_pixel_txd("road_a", [10, 20, 30, 255]);
        file_loader
            .txd_store
            .mark_started(slot, Some(first_half.clone()));
        let started = file_loader
            .txd_store
            .slot(slot)
            .expect("started txd")
            .clone();

        let mut cache = GtaWorldTextureCache::default();
        let mut images = Assets::<Image>::default();
        assert!(sync_txd_slot_texture_cache(
            slot,
            &file_loader,
            &mut cache,
            &mut images,
        ));
        let road_a_before = texture_cache_lookup(&cache, slot, "road_a")
            .expect("first-phase road_a")
            .0;
        let first_phase_image_count = images.len();
        assert!(texture_cache_lookup(&cache, slot, "road_b").is_none());

        let mut full = first_half;
        let road_b = one_pixel_txd("road_b", [40, 50, 60, 255]);
        full.num_textures = 2;
        full.first_phase_textures = 1;
        full.texture_native_chunks = 2;
        full.textures.extend(road_b.textures);
        file_loader.txd_store.mark_loaded(slot, Some(full));
        let loaded = file_loader.txd_store.slot(slot).expect("loaded txd");
        assert_eq!(loaded.generation, started.generation);
        assert_ne!(loaded.texture_revision, started.texture_revision);

        assert!(sync_txd_slot_texture_cache(
            slot,
            &file_loader,
            &mut cache,
            &mut images,
        ));
        let road_a_after = texture_cache_lookup(&cache, slot, "road_a")
            .expect("finished road_a")
            .0;
        assert_eq!(
            road_a_after, road_a_before,
            "FinishLoadTxd must keep already-decoded first-phase image handles"
        );
        assert!(
            texture_cache_lookup(&cache, slot, "road_b").is_some(),
            "FinishLoadTxd must make second-phase rasters visible to cached world materials"
        );
        assert_eq!(
            images.len(),
            first_phase_image_count + 1,
            "FinishLoadTxd should upload only the newly appended second-phase raster"
        );
        assert_eq!(
            cache.synced_txd_revisions.get(&slot).copied(),
            Some(loaded.texture_revision)
        );
    }

    #[test]
    fn world_model_asset_key_changes_when_its_txd_is_reloaded() {
        let mut file_loader = CFileLoader::default();
        let model_index = 47;
        let txd_slot = file_loader.txd_store.add_txd_slot("roads");
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.base.txd_slot = txd_slot as i16;
            simple.lod_distances = [100.0, 0.0, 0.0];
            simple.num_atomics = 1;
            simple.alpha = 255;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);
        file_loader
            .txd_store
            .mark_loaded(txd_slot, Some(one_pixel_txd("road", [1, 2, 3, 255])));
        let first = world_model_asset_key(model_index, 10.0, false, false, &file_loader)
            .expect("first asset key");
        assert!(world_model_asset_txd_generation_is_current(
            &file_loader,
            first
        ));

        file_loader.txd_store.remove_txd(txd_slot);
        file_loader
            .txd_store
            .mark_loaded(txd_slot, Some(one_pixel_txd("road", [4, 5, 6, 255])));
        let second = world_model_asset_key(model_index, 10.0, false, false, &file_loader)
            .expect("second asset key");

        assert_ne!(first.txd_generation, second.txd_generation);
        assert!(!world_model_asset_txd_generation_is_current(
            &file_loader,
            first
        ));
        assert!(world_model_asset_txd_generation_is_current(
            &file_loader,
            second
        ));
    }

    #[test]
    fn world_model_asset_key_changes_when_txd_finish_adds_rasters() {
        let mut file_loader = CFileLoader::default();
        let model_index = 49;
        let txd_slot = file_loader.txd_store.add_txd_slot("roads");
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.base.txd_slot = txd_slot as i16;
            simple.lod_distances = [100.0, 0.0, 0.0];
            simple.num_atomics = 1;
            simple.alpha = 255;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);
        file_loader
            .txd_store
            .mark_started(txd_slot, Some(one_pixel_txd("road_a", [1, 2, 3, 255])));
        let first = world_model_asset_key(model_index, 10.0, false, false, &file_loader)
            .expect("started-phase asset key");

        file_loader
            .txd_store
            .mark_loaded(txd_slot, Some(one_pixel_txd("road_a", [1, 2, 3, 255])));
        let second = world_model_asset_key(model_index, 10.0, false, false, &file_loader)
            .expect("finished-phase asset key");

        assert_eq!(first.txd_generation, second.txd_generation);
        assert_ne!(first.txd_revision, second.txd_revision);
        assert!(!world_model_asset_txd_generation_is_current(
            &file_loader,
            first
        ));
        assert!(world_model_asset_txd_generation_is_current(
            &file_loader,
            second
        ));
    }

    /// re3 keeps entity geometry across TXD evict+reload cycles and only
    /// rebinds textures. A TXD reload must therefore produce new material
    /// assets while creating zero Mesh assets and keeping the same geometry
    /// handles.
    #[test]
    fn txd_reload_rebinds_materials_without_rebuilding_geometry() {
        let mut file_loader = CFileLoader::default();
        let model_index = 61;
        let txd_slot = file_loader.txd_store.add_txd_slot("roads");
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.base.txd_slot = txd_slot as i16;
            simple.lod_distances = [100.0, 0.0, 0.0];
            simple.num_atomics = 1;
            simple.alpha = 255;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);
        file_loader
            .txd_store
            .mark_loaded(txd_slot, Some(one_pixel_txd("road", [1, 2, 3, 255])));
        let mut texture_cache = GtaWorldTextureCache::default();
        let mut images = Assets::<Image>::default();
        sync_world_texture_cache(&file_loader, &mut texture_cache, &mut images);

        let mut meshes = Assets::<Mesh>::default();
        let mesh_handle = meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        ));
        let recipe = ModelMaterialDrawable {
            mesh: Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            ),
            base_color: Color::WHITE,
            texture_name: Some("road".to_owned()),
            dual_texture_name: None,
            texture_model_index: model_index,
            texture_txd_slot: None,
            vertex_count: 0,
            index_count: 0,
            vehicle_colour_slot: None,
            specular: 0.0,
            surface_ambient: 1.0,
            surface_diffuse: 1.0,
            has_prelit: false,
            dynamic_vehicle_component: false,
            vehicle_component_transform: None,
        };
        let mut model_assets = GtaWorldModelAssetCache::default();
        model_assets.geometry.insert(
            (model_index, 0),
            GtaWorldModelGeometry {
                meshes: vec![mesh_handle.clone()],
                recipes: vec![recipe],
            },
        );
        let mut materials = Assets::<GtaUnlitMaterial>::default();
        let lighting = GtaRenderLighting::default();

        let first = world_model_asset_key(model_index, 10.0, false, false, &file_loader)
            .expect("first key");
        let (first_materials, _, _) = ensure_world_materials_for_key(
            first,
            &file_loader,
            &mut texture_cache,
            &mut images,
            &mut materials,
            &mut model_assets,
            lighting,
        )
        .expect("first materials");
        let mesh_count = meshes.len();

        // Streamed TXD replacement (evict + reload), exactly the driving churn.
        file_loader.txd_store.remove_txd(txd_slot);
        file_loader
            .txd_store
            .mark_loaded(txd_slot, Some(one_pixel_txd("road", [4, 5, 6, 255])));
        sync_world_texture_cache(&file_loader, &mut texture_cache, &mut images);
        let second = world_model_asset_key(model_index, 10.0, false, false, &file_loader)
            .expect("second key");
        assert_ne!(first.txd_generation, second.txd_generation);
        let (second_materials, _, _) = ensure_world_materials_for_key(
            second,
            &file_loader,
            &mut texture_cache,
            &mut images,
            &mut materials,
            &mut model_assets,
            lighting,
        )
        .expect("second materials");

        assert_ne!(
            first_materials, second_materials,
            "textures must rebind to the reloaded dictionary"
        );
        assert_eq!(
            meshes.len(),
            mesh_count,
            "TXD reload must not create Mesh assets"
        );
        let entry = model_assets.entries.get(&second).expect("refreshed entry");
        assert_eq!(
            entry.meshes,
            vec![mesh_handle],
            "geometry handles survive TXD cycles"
        );
    }

    #[test]
    fn world_entity_rw_object_state_comes_from_the_instance_slot() {
        let mut file_loader = CFileLoader::default();
        let model_index = 48;
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.base.rw_object_loaded = true;
            simple.lod_distances = [100.0, 0.0, 0.0];
            simple.num_atomics = 1;
            simple.alpha = 255;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);
        let mut entity = world_entity([10.0, 0.0, 0.0], false);
        entity.model_index = model_index;
        let model = world_model_snapshot(
            model_index,
            entity.position,
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            false,
        )
        .expect("model snapshot");
        file_loader
            .renderer
            .begin_construct_render_list(CVector::ZERO, 12, 1.0);

        let missing_instance =
            world_entity_snapshot_for_camera(&entity, &file_loader.model_info, None, false);
        assert!(!missing_instance.rw_object_loaded);
        assert!(
            file_loader
                .renderer
                .setup_entity_visibility(&missing_instance, &model)
                .side_effects
                .create_rw_object
        );

        let live_instance =
            world_entity_snapshot_for_camera(&entity, &file_loader.model_info, None, true);
        assert!(live_instance.rw_object_loaded);
        assert!(
            !file_loader
                .renderer
                .setup_entity_visibility(&live_instance, &model)
                .side_effects
                .create_rw_object
        );

        let recreated_after_delete =
            world_entity_snapshot_for_camera(&entity, &file_loader.model_info, None, false);
        assert!(
            file_loader
                .renderer
                .setup_entity_visibility(&recreated_after_delete, &model)
                .side_effects
                .create_rw_object
        );
    }

    #[test]
    fn fading_world_mesh_uses_the_same_distance_fallback_as_renderer() {
        use rustgta3_formats::formats::ClumpRead::{RwAtomicDescriptor, RwGeometryDescriptor};

        let mut file_loader = CFileLoader::default();
        let model_index = 42;
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.lod_distances = [40.0, 0.0, 0.0];
            simple.num_atomics = 1;
        }
        assert!(file_loader.model_info.set_simple_atomic_binding(
            model_index,
            0,
            "lod0",
            RwAtomicDescriptor {
                frame_index: 0,
                geometry_index: 0,
                flags: 0,
                frame_name: Some("lod0".to_owned()),
            },
            RwGeometryDescriptor {
                flags: 0,
                raw_flags: 0,
                num_triangles: 0,
                num_vertices: 0,
                num_morph_targets: 1,
                num_materials: 0,
                num_tex_coord_sets: 0,
                chunk_version: 0,
                chunk_length: 0,
                decoded: None,
                materials: Vec::new(),
                skin: None,
            },
        ));
        file_loader.renderer.camera_lod_dist_multiplier = 1.0;

        let distance = 40.0 + FADE_DISTANCE * 0.5;
        assert!(world_model_asset_key(model_index, distance, false, false, &file_loader).is_none());
        assert_eq!(
            world_model_asset_key(model_index, distance, true, false, &file_loader)
                .map(|key| key.lod_index),
            Some(0),
        );
    }

    #[test]
    fn distance_fading_world_mesh_keeps_the_shifted_lod_during_fade() {
        let mut file_loader = CFileLoader::default();
        let model_index = 43;
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.lod_distances = [40.0, 80.0, 0.0];
            simple.num_atomics = 2;
            simple.alpha = 128;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);
        add_empty_simple_binding(&mut file_loader, model_index, 1);
        file_loader.renderer.camera_lod_dist_multiplier = 1.0;

        // At 45 units the ordinary draw has already switched to LOD1, while
        // re3 RenderFadingAtomic samples at 45 - FADE_DISTANCE and keeps LOD0
        // for the transparent transition.
        assert_eq!(
            world_model_asset_key(model_index, 45.0, false, false, &file_loader)
                .map(|key| key.lod_index),
            Some(1),
        );
        assert_eq!(
            world_model_asset_key(model_index, 45.0, true, false, &file_loader)
                .map(|key| key.lod_index),
            Some(0),
        );
        assert_eq!(
            world_distance_fade_alpha(model_index, 45.0, false, &file_loader),
            Some(128),
        );
    }

    #[test]
    fn multi_atomic_world_mesh_uses_cpp_adjusted_distance_for_visibility_and_lod() {
        let mut file_loader = CFileLoader::default();
        let model_index = 44;
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.lod_distances = [200.0, 400.0, 500.0];
            simple.num_atomics = 3;
        }
        for lod_index in 0..3 {
            add_empty_simple_binding(&mut file_loader, model_index, lod_index);
        }
        file_loader.renderer.camera_lod_dist_multiplier = 1.0;

        // re3 bumps 350 to the largest LOD distance (500), so the direct
        // atomic is nil and the fade lookup at 480 selects LOD2. Selecting
        // from the raw 350 distance would incorrectly choose LOD1.
        let distance = LOD_DISTANCE + STREAM_DISTANCE + 20.0;
        let snapshot = world_model_snapshot(
            model_index,
            [distance, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            false,
        )
        .expect("simple model snapshot");
        assert!(!snapshot.loaded_for_distance);
        assert!(snapshot.loaded_for_fade_distance);
        assert!(world_model_asset_key(model_index, distance, false, false, &file_loader).is_none());
        assert_eq!(
            world_model_asset_key(model_index, distance, true, false, &file_loader)
                .map(|key| key.lod_index),
            Some(2),
        );
    }

    #[test]
    fn time_model_with_loaded_alternate_still_runs_normal_distance_streaming() {
        use crate::renderer::Renderer::RendererVisibility;

        let mut file_loader = CFileLoader::default();
        let time_model_index = 44;
        let alternate_model_index = 45;
        {
            let time = file_loader.model_info.add_time_model(time_model_index);
            time.lod_distances = [100.0, 0.0, 0.0];
            time.num_atomics = 1;
            time.time_on = Some(6);
            time.time_off = Some(20);
            time.other_time_model_id = Some(alternate_model_index);
        }
        {
            let alternate = file_loader.model_info.add_time_model(alternate_model_index);
            alternate.lod_distances = [100.0, 0.0, 0.0];
            alternate.num_atomics = 1;
        }
        add_empty_simple_binding(&mut file_loader, time_model_index, 0);
        add_empty_simple_binding(&mut file_loader, alternate_model_index, 0);

        let distance = 120.0;
        let model = world_model_snapshot(
            time_model_index,
            [distance, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            false,
        )
        .expect("time model snapshot");
        assert!(model.other_time_model_exists);
        assert!(model.other_time_model_loaded);

        let mut entity = world_entity([distance, 0.0, 0.0], false);
        entity.model_index = time_model_index;
        let entity = world_entity_snapshot(&entity, &file_loader.model_info);
        file_loader
            .renderer
            .begin_construct_render_list(CVector::ZERO, 12, 1.0);
        assert_eq!(
            file_loader
                .renderer
                .setup_entity_visibility(&entity, &model)
                .visibility,
            RendererVisibility::StreamMe,
        );
    }

    #[test]
    fn time_model_without_alternate_is_culled_outside_its_time_range() {
        use crate::renderer::Renderer::RendererVisibility;

        let mut file_loader = CFileLoader::default();
        let model_index = 46;
        {
            let time = file_loader.model_info.add_time_model(model_index);
            time.lod_distances = [100.0, 0.0, 0.0];
            time.num_atomics = 1;
            time.time_on = Some(6);
            time.time_off = Some(20);
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);

        let model = world_model_snapshot(
            model_index,
            [10.0, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            false,
        )
        .expect("time model snapshot");
        assert!(!model.other_time_model_exists);

        let mut entity = world_entity([10.0, 0.0, 0.0], false);
        entity.model_index = model_index;
        let entity = world_entity_snapshot(&entity, &file_loader.model_info);
        file_loader
            .renderer
            .begin_construct_render_list(CVector::ZERO, 22, 1.0);
        assert_eq!(
            file_loader
                .renderer
                .setup_entity_visibility(&entity, &model)
                .visibility,
            RendererVisibility::Invisible,
        );
    }

    #[test]
    fn time_big_building_without_alternate_is_culled_outside_its_time_range() {
        use crate::renderer::Renderer::RendererVisibility;

        let mut file_loader = CFileLoader::default();
        let model_index = 47;
        {
            let time = file_loader.model_info.add_time_model(model_index);
            time.lod_distances = [400.0, 0.0, 0.0];
            time.num_atomics = 1;
            time.time_on = Some(6);
            time.time_off = Some(20);
            time.is_big_building = true;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);

        let model = world_model_snapshot(
            model_index,
            [10.0, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            true,
        )
        .expect("time big-building model snapshot");
        assert!(!model.other_time_model_exists);

        let mut entity = world_entity([10.0, 0.0, 0.0], true);
        entity.model_index = model_index;
        let entity = world_entity_snapshot(&entity, &file_loader.model_info);
        file_loader
            .renderer
            .begin_construct_render_list(CVector::ZERO, 22, 1.0);
        assert_eq!(
            file_loader
                .renderer
                .setup_big_building_visibility(&entity, &model)
                .visibility,
            RendererVisibility::Invisible,
        );
    }

    #[test]
    fn fading_pass_keeps_shifted_geometry_when_direct_lod_is_missing() {
        assert!(should_use_fade_geometry(
            GtaWorldRenderPass::FadingIn,
            false,
            true,
            None,
            false,
        ));
        assert!(should_use_fade_geometry(
            GtaWorldRenderPass::FadingIn,
            true,
            true,
            Some(255),
            false,
        ));
        assert!(!should_use_fade_geometry(
            GtaWorldRenderPass::FadingIn,
            false,
            true,
            None,
            true,
        ));
    }

    #[test]
    fn scaled_lod_threshold_keeps_mid_range_simple_models_loaded() {
        // re3 `Renderer.h:6` (FIX_BUGS): `LOD_DISTANCE` is 300 * multiplier.
        // At mult 1.2 the bump threshold is 390, so dist 350 stays raw and
        // loaded; at mult 1.0 the threshold is 330, so the same dist bumps
        // to the largest LOD and reads as unloaded. The previous constant
        // threshold wrongly unloaded mid-range roads/buildings at 330-390m.
        let mut file_loader = CFileLoader::default();
        let model_index = 61;
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.lod_distances = [500.0, 0.0, 0.0];
            simple.num_atomics = 1;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);
        let near = world_model_snapshot(
            model_index,
            [350.0, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.2,
            false,
        )
        .expect("scaled snapshot");
        assert!(near.loaded_for_distance);
        let far = world_model_snapshot(
            model_index,
            [350.0, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            false,
        )
        .expect("unscaled snapshot");
        assert!(!far.loaded_for_distance);
    }

    #[test]
    fn big_building_snapshot_skips_cpp_distance_bump() {
        // re3 `SetupBigBuildingVisibility` calls `GetAtomicFromDistance`
        // with the raw distance; the `Renderer.cpp:802` bump belongs only
        // to the simple-model path. Bumping big LODs pushed them past
        // their largest distance and deleted bridges/island connectors.
        let mut file_loader = CFileLoader::default();
        let model_index = 62;
        {
            let simple = file_loader.model_info.add_simple_model(model_index);
            simple.lod_distances = [400.0, 0.0, 0.0];
            simple.num_atomics = 1;
        }
        add_empty_simple_binding(&mut file_loader, model_index, 0);
        let simple = world_model_snapshot(
            model_index,
            [350.0, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            false,
        )
        .expect("simple snapshot");
        assert!(!simple.loaded_for_distance);
        let big = world_model_snapshot(
            model_index,
            [350.0, 0.0, 0.0],
            &file_loader.model_info,
            CVector::ZERO,
            1.0,
            true,
        )
        .expect("big snapshot");
        assert!(big.loaded_for_distance);
    }

    #[test]
    fn big_building_near_check_uses_scaled_lod_distance() {
        use crate::renderer::Renderer::{
            RendererEntitySnapshot, RendererModelSnapshot, RendererVisibility,
        };
        // dist 370 sits below near 420. With mult 1.2 the scaled
        // `LOD_DISTANCE + STREAM_DISTANCE` gate is 390, so the LOD stays
        // hidden while its non-LOD fades in; with mult 1.0 the gate is
        // 330 and the same entity takes the normal loaded path.
        let entity = RendererEntitySnapshot {
            entity_type: crate::core::World::EEntityType::Building,
            model_index: 0,
            position: CVector::new(370.0, 0.0, 0.0),
            rw_object_loaded: true,
            visible: true,
            on_screen: true,
            on_screen_complex: true,
            draw_last: false,
            render_damaged: false,
            first_person_player_vehicle: false,
            first_person_camera_forward: false,
            first_person_big_vehicle_exception: false,
            temporary_object: false,
            big_building: true,
        };
        let model = RendererModelSnapshot {
            model_type: crate::renderer::Renderer::RendererModelType::Simple,
            loaded_for_distance: true,
            loaded_for_fade_distance: true,
            largest_lod_distance: 3000.0,
            near_distance: 420.0,
            no_fade: false,
            draw_last: false,
            alpha: 255,
            time_on: None,
            time_off: None,
            other_time_model_exists: false,
            other_time_model_loaded: false,
            related_model_exists: false,
            related_model_loaded: false,
            related_model_alpha: 0,
            related_time_other_loaded: false,
        };
        let mut renderer = CRenderer::default();
        renderer.begin_construct_render_list(CVector::ZERO, 12, 1.2);
        assert_eq!(
            renderer
                .setup_big_building_visibility(&entity, &model)
                .visibility,
            RendererVisibility::Invisible,
        );
        renderer.begin_construct_render_list(CVector::ZERO, 12, 1.0);
        assert_eq!(
            renderer
                .setup_big_building_visibility(&entity, &model)
                .visibility,
            RendererVisibility::Visible,
        );
    }
}
