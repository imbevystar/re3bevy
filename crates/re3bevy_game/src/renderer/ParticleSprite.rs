//! re3 reference: references/re-GTA/src/renderer/Particle.cpp (`CParticle::Render`)
//! re3 reference: references/re-GTA/src/renderer/Sprite.cpp
//!   (`CSprite::CalcScreenCoors`, `CSprite::RenderBufferedOneXLUSprite`,
//!    `CSprite::RenderBufferedOneXLUSprite_Rotate_Dimension`)
//!
//! Screen-space particle sprite pass. Particles are drawn exactly like re3:
//! buffered screen-space textured quads with per-system blend state
//! (additive default, DRAW_OPAQUE -> SRCALPHA/INVSRCALPHA, DRAW_DARK ->
//! SRCALPHA/ONE), depth test against the scene depth buffer with depth writes
//! disabled, wrap addressing, drawn in PARTICLE.CFG system order with buffer
//! flushes on raster/state change.
//!
//! Platform adaptation (documented): re3 projects the particle centre through
//! the RW camera view matrix whose projection rows fold the frustum into
//! device pixel space (`librw/src/camera.cpp` cameraSync). This port projects
//! the centre through the live Bevy camera view-projection instead, so sprite
//! centres stay glued to the world geometry exactly the way re3's sprites stay
//! glued to its RW camera raster. Sprite sizes, rotation, trail math,
//! clipping/culling, colour/intensity modulation, blend states and draw
//! ordering follow Sprite.cpp/Particle.cpp verbatim in device pixel space.

use std::{
    collections::HashMap,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::mesh::VertexBufferLayout;
use bevy::render::Extract;
use bevy::{
    asset::{AssetId, Handle},
    camera::CameraProjection,
    core_pipeline::core_3d::main_transparent_pass_3d,
    prelude::*,
    render::{
        ExtractSchedule, RenderApp,
        render_asset::RenderAssets,
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferInitDescriptor,
            BufferUsages, CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction,
            DepthStencilState, FragmentState, MultisampleState, PipelineCache, PrimitiveState,
            PrimitiveTopology, RenderPassDescriptor, RenderPipelineDescriptor, SamplerBindingType,
            ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines, StoreOp,
            TextureFormat, TextureSampleType, VertexAttribute, VertexFormat, VertexState,
            VertexStepMode,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        texture::GpuImage,
        view::{ViewDepthStencilTexture, ViewTarget},
    },
};

use super::{
    BevyRender::{
        GtaParticleBillboard, GtaRenderFrameState, GtaWorldTextureCache,
        sync_txd_slot_texture_cache,
    },
    Clouds::CClouds,
    Draw::CDraw,
    ParticleMgr::{
        DRAW_DARK, DRAW_OPAQUE, DRAWTOP2D, SCREEN_TRAIL, SPEED_TRAIL, VERT_TRAIL,
        cParticleSystemMgr,
    },
    ParticleType::{
        PARTICLE_BOAT_WAKE, PARTICLE_RAINDROP_SMALL, tParticleType,
    },
};
use crate::{
    core::{Camera::CCamera, FileLoader::CFileLoader},
    extras::postfx::CPostFX,
};
use re3bevy_core::core::Timer::CTimer;

/// re3 `DefaultFOV` (`references/re-GTA/src/core/Camera.h:43`).
const RE3_DEFAULT_FOV: f32 = 70.0;
/// Fallback for `CDraw::GetFOV`; re3 runs gameplay at `SetFOV(120)`
/// (`core/Game.cpp:541`, `core/Game.cpp:748`) which the production
/// `CDraw::initialise_for_game` mirrors.
const RE3_GAME_FOV_FALLBACK: f32 = 120.0;
/// re3 `DEFAULT_ASPECT_RATIO` (`core/common.h:171`).
const RE3_DEFAULT_ASPECT_RATIO: f32 = 4.0 / 3.0;
/// re3 PC_PARTICLE `fParticleScaleLimit` (`renderer/Particle.cpp:227`).
const PARTICLE_SCALE_LIMIT: f32 = 0.5;

fn fx_runtime_diagnostics_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("RE3BEVY_FX_DIAG").ok().is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    })
}

/// re3 `PC_PARTICLE` systems culled early when they grow beyond half the
/// screen (`renderer/Particle.cpp:1510-1517`).
use super::Particle::is_pc_smoke_particle as is_pc_particle_banned;

/// Blend mode derived from `psystem->Flags` (`renderer/Particle.cpp:1530-1543`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleBlendMode {
    /// Default: `rwBLENDONE` / `rwBLENDONE`. re3 does not premultiply by
    /// alpha here; additive particles fade through intensity fade-to-black.
    Additive,
    /// `DRAW_OPAQUE`: `rwBLENDSRCALPHA` / `rwBLENDINVSRCALPHA`.
    StandardAlpha,
    /// `DRAW_DARK`: `rwBLENDSRCALPHA` / `rwBLENDONE`.
    DarkAlpha,
}

impl ParticleBlendMode {
    fn blend_state(self) -> Option<BlendState> {
        let add = BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        };
        match self {
            Self::Additive => Some(BlendState {
                color: add,
                alpha: add,
            }),
            Self::StandardAlpha => Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            }),
            Self::DarkAlpha => Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
        }
    }
}

/// One packed sprite vertex: NDC position (+ wgpu depth), UV, encoded RGBA.
/// `pub(crate)` mirrors the `pub(crate)` frame resource below; a private
/// vertex type behind a `pub` frame field triggers `private_interfaces`.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PackedSpriteVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) colour: [u8; 4],
}

const VERTEX_SIZE: usize = std::mem::size_of::<PackedSpriteVertex>();
const SPRITE_VERTEX_COUNT: u32 = 6;

/// One flush group: identical texture raster + blend state + z-test flag,
/// mirroring re3's buffer flushes on raster or blend change
/// (`renderer/Particle.cpp:1525-1557`).
#[derive(Debug, Clone)]
pub struct ParticleSpriteBatch {
    pub image: AssetId<Image>,
    pub blend: ParticleBlendMode,
    /// False only for `DRAWTOP2D` particles, which re3 draws with the z-test
    /// temporarily disabled (`Sprite.cpp:73-77`).
    pub z_test_enabled: bool,
    pub first_vertex: u32,
    pub vertex_count: u32,
}

/// Frame-local sprite geometry built on the main world every tick.
#[derive(Default, Clone, Resource)]
pub struct GtaParticleSpriteFrame {
    pub batches: Vec<ParticleSpriteBatch>,
    pub(crate) vertices: Vec<PackedSpriteVertex>,
    /// Vertices produced by `CParticle::Render`-equivalent particle sprites.
    pub particle_vertex_count: usize,
    /// Vertices produced by the `CClouds::Render` screen-space cloud slice.
    pub cloud_vertex_count: usize,
}

type BatchKey = (AssetId<Image>, ParticleBlendMode, bool);

fn push_sprite(
    frame: &mut GtaParticleSpriteFrame,
    batch_key: &mut Option<BatchKey>,
    image: AssetId<Image>,
    blend: ParticleBlendMode,
    z_test_enabled: bool,
    corners: [[f32; 2]; 4],
    uvs: [[f32; 2]; 4],
    rgb: [u8; 3],
    alpha: u8,
    depth_ndc: f32,
    viewport: Vec2,
) {
    if batch_key.is_none()
        || batch_key
            .map(|key| key.0 != image || key.1 != blend || key.2 != z_test_enabled)
            .unwrap_or(false)
    {
        frame.batches.push(ParticleSpriteBatch {
            image,
            blend,
            z_test_enabled,
            first_vertex: frame.vertices.len() as u32,
            vertex_count: 0,
        });
        *batch_key = Some((image, blend, z_test_enabled));
    }

    // librw im2DSetXform + im2d_vert_src: top-left-origin device pixels ->
    // clip space, Y down.
    let to_ndc = |p: [f32; 2]| [p[0] * 2.0 / viewport.x - 1.0, 1.0 - p[1] * 2.0 / viewport.y];

    // Sprite.cpp indices { 0, 1, 2, 3, 0, 2 }.
    for index in [0usize, 1, 2, 3, 0, 2] {
        frame.vertices.push(PackedSpriteVertex {
            position: [
                to_ndc(corners[index])[0],
                to_ndc(corners[index])[1],
                depth_ndc,
            ],
            uv: uvs[index],
            colour: [rgb[0], rgb[1], rgb[2], alpha],
        });
    }
    if let Some(batch) = frame.batches.last_mut() {
        batch.vertex_count += SPRITE_VERTEX_COUNT;
    }
}

fn apply_postfx_rgb(rgb: [u8; 3], multiplier: [f32; 3]) -> [u8; 3] {
    [
        (f32::from(rgb[0]) * multiplier[0])
            .round()
            .clamp(0.0, 255.0) as u8,
        (f32::from(rgb[1]) * multiplier[1])
            .round()
            .clamp(0.0, 255.0) as u8,
        (f32::from(rgb[2]) * multiplier[2])
            .round()
            .clamp(0.0, 255.0) as u8,
    ]
}

/// Unrotated buffered sprite with screen-edge UV clipping
/// (`Sprite.cpp:205-267`).
#[allow(clippy::too_many_arguments)]
fn emit_plain_sprite(
    frame: &mut GtaParticleSpriteFrame,
    batch_key: &mut Option<BatchKey>,
    centre: Vec2,
    half: Vec2,
    viewport: Vec2,
    rgb: [u8; 3],
    alpha: u8,
    depth_ndc: f32,
    image: AssetId<Image>,
    blend: ParticleBlendMode,
    z_test_enabled: bool,
) {
    if half.x <= 0.0 || half.y <= 0.0 {
        return;
    }
    let mut xs = [
        centre.x - half.x,
        centre.x - half.x,
        centre.x + half.x,
        centre.x + half.x,
    ];
    let mut ys = [
        centre.y - half.y,
        centre.y + half.y,
        centre.y + half.y,
        centre.y - half.y,
    ];
    let mut us = [0.0f32, 0.0, 1.0, 1.0];
    let mut vs = [0.0f32, 1.0, 1.0, 0.0];

    for i in 0..4 {
        if xs[i] < 0.0 {
            us[i] = -xs[i] / (2.0 * half.x);
            xs[i] = 0.0;
        }
        if xs[i] > viewport.x {
            us[i] = 1.0 - (xs[i] - viewport.x) / (2.0 * half.x);
            xs[i] = viewport.x;
        }
        if ys[i] < 0.0 {
            vs[i] = -ys[i] / (2.0 * half.y);
            ys[i] = 0.0;
        }
        if ys[i] > viewport.y {
            vs[i] = 1.0 - (ys[i] - viewport.y) / (2.0 * half.y);
            ys[i] = viewport.y;
        }
    }

    push_sprite(
        frame,
        batch_key,
        image,
        blend,
        z_test_enabled,
        [
            [xs[0], ys[0]],
            [xs[1], ys[1]],
            [xs[2], ys[2]],
            [xs[3], ys[3]],
        ],
        [
            [us[0], vs[0]],
            [us[1], vs[1]],
            [us[2], vs[2]],
            [us[3], vs[3]],
        ],
        rgb,
        alpha,
        depth_ndc,
        viewport,
    );
}

/// Rotated buffered sprite; whole-sprite culling only, no UV clipping
/// (`Sprite.cpp:270-320`). `rotation_degrees` is degrees.
#[allow(clippy::too_many_arguments)]
fn emit_rotated_sprite(
    frame: &mut GtaParticleSpriteFrame,
    batch_key: &mut Option<BatchKey>,
    centre: Vec2,
    half_w: f32,
    half_h: f32,
    rotation_degrees: f32,
    viewport: Vec2,
    rgb: [u8; 3],
    alpha: u8,
    depth_ndc: f32,
    image: AssetId<Image>,
    blend: ParticleBlendMode,
    z_test_enabled: bool,
) -> bool {
    if half_w <= 0.0 && half_h <= 0.0 {
        return false;
    }
    let rotation = rotation_degrees.to_radians();
    let c = rotation.cos();
    let s = rotation.sin();

    let xs = [
        centre.x - c * half_w - s * half_h,
        centre.x - c * half_w + s * half_h,
        centre.x + c * half_w + s * half_h,
        centre.x + c * half_w - s * half_h,
    ];
    let ys = [
        centre.y - c * half_h + s * half_w,
        centre.y + c * half_h + s * half_w,
        centre.y + c * half_h - s * half_w,
        centre.y - c * half_h - s * half_w,
    ];

    // No clipping, just whole-sprite culling (Sprite.cpp:294-299).
    if xs.iter().all(|x| *x < 0.0) || ys.iter().all(|y| *y < 0.0) {
        return false;
    }
    if xs.iter().all(|x| *x > viewport.x) || ys.iter().all(|y| *y > viewport.y) {
        return false;
    }

    push_sprite(
        frame,
        batch_key,
        image,
        blend,
        z_test_enabled,
        [
            [xs[0], ys[0]],
            [xs[1], ys[1]],
            [xs[2], ys[2]],
            [xs[3], ys[3]],
        ],
        [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        rgb,
        alpha,
        depth_ndc,
        viewport,
    );
    true
}

/// Screen-space trail rotation helper shared by SCREEN_TRAIL/SPEED_TRAIL
/// (`Particle.cpp:1645-1665`, `1696-1716`): returns degrees in [0, 360).
fn trail_rotation_degrees(dist_vec: Vec2) -> f32 {
    let dist = dist_vec.length().max(f32::EPSILON);
    let rot = (dist_vec.x / dist).asin();
    let mut rotation = rot;
    if dist_vec.y < 0.0 {
        rotation = -rot + std::f32::consts::PI;
    }
    let mut degrees = rotation.to_degrees();
    if degrees < 0.0 {
        degrees += 360.0;
    }
    degrees
}

fn trail_length_multiplier(
    psystems: Option<&cParticleSystemMgr>,
    particle_type: tParticleType,
) -> f32 {
    psystems
        .and_then(|mgr| mgr.particle(particle_type))
        .map(|system| system.trail_length_multiplier)
        .unwrap_or(0.0)
}

/// Main-world system: converts the simulated `GtaParticleBillboard` state into
/// re3-style buffered sprite geometry for this frame.
#[allow(clippy::too_many_arguments)]
pub fn build_particle_sprite_frame(
    mut frame: ResMut<GtaParticleSpriteFrame>,
    mut particles: Query<(Entity, &Transform, &mut GtaParticleBillboard)>,
    psystems: Option<Res<cParticleSystemMgr>>,
    draw: Option<Res<CDraw>>,
    timer: Option<Res<CTimer>>,
    frame_state: Option<Res<GtaRenderFrameState>>,
    camera_state: Option<Res<CCamera>>,
    postfx: Option<Res<CPostFX>>,
    // Same-frame camera projection: the Camera set writes Transform earlier
    // this frame; GlobalTransform only propagates in PostUpdate.
    cameras: Query<(&Camera, &Transform, &Projection), With<Camera3d>>,
    windows: Query<&Window>,
    mut ordered_buf: Local<Vec<(i32, u64, Entity)>>,
) {
    frame.batches.clear();
    frame.vertices.clear();
    frame.particle_vertex_count = 0;
    frame.cloud_vertex_count = 0;

    let Some((camera, camera_transform, projection)) = cameras
        .iter()
        .find(|(cam, _, _)| cam.is_active)
        .or_else(|| cameras.iter().next())
        .map(|(cam, transform, proj)| (cam, *transform, proj))
    else {
        return;
    };
    let Projection::Perspective(perspective) = projection else {
        return;
    };

    let Some(viewport_size) = camera
        .viewport
        .as_ref()
        .map(|viewport| {
            Vec2::new(
                viewport.physical_size.x as f32,
                viewport.physical_size.y as f32,
            )
        })
        .or_else(|| {
            windows
                .iter()
                .next()
                .map(|window| Vec2::new(window.width(), window.height()))
        })
    else {
        return;
    };
    if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
        return;
    }

    let world_from_camera = camera_transform.to_matrix();
    // Bevy's 3D pipeline uses an infinite reverse-Z projection. Building
    // a conventional `Mat4::perspective_rh` here produced ordinary depth while
    // the scene buffer contained reverse depth, so the custom sprite pass
    // rejected every world particle. Use the live camera projection verbatim.
    let clip_from_world = perspective.get_clip_from_view() * world_from_camera.inverse();
    let far_clip = frame_state
        .as_ref()
        .map(|state| state.far_clip)
        .filter(|far| *far > 0.0)
        .unwrap_or(perspective.far);
    let near_clip = perspective.near;
    let fov_scale = RE3_DEFAULT_FOV
        / draw
            .as_ref()
            .map(|draw| draw.fov)
            .unwrap_or(RE3_GAME_FOV_FALLBACK);
    let aspect_ratio_scale = RE3_DEFAULT_ASPECT_RATIO / (viewport_size.x / viewport_size.y);
    let time_step = timer.map(|timer| timer.time_step()).unwrap_or(1.0);
    let postfx_multiplier = postfx
        .as_deref()
        .zip(camera_state.as_deref())
        .map_or([1.0; 3], |(postfx, camera)| {
            postfx.effective_colour_multiplier(camera)
        });

    // Project a world point into re3's device-pixel sprite space:
    // returns (screen px, ndc depth, view depth). CalcScreenCoors rejects
    // behind-near(+1) and past-far-clip points (Sprite.cpp:31-32).
    let project = |position: Vec3| -> Option<(Vec2, f32, f32)> {
        let clip = clip_from_world * position.extend(1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let view = world_from_camera.inverse() * position.extend(1.0);
        let view_depth = -view.z;
        if view_depth <= near_clip + 1.0 || view_depth >= far_clip {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let screen = Vec2::new(
            (ndc.x * 0.5 + 0.5) * viewport_size.x,
            (1.0 - (ndc.y * 0.5 + 0.5)) * viewport_size.y,
        );
        Some((screen, ndc.z.clamp(0.0, 1.0), view_depth))
    };

    // re3 iterates systems in enum order and walks each system list in
    // insertion order (Particle.cpp:1498-1790). Sorting by (type, sequence)
    // reproduces both levels of ordering. The buffer is reused across frames
    // to avoid a per-frame order-list allocation on this hot path.
    ordered_buf.clear();
    ordered_buf.extend(
        particles
            .iter()
            .map(|(entity, _, particle)| (particle.particle_type, particle.spawn_sequence, entity)),
    );
    ordered_buf.sort_by_key(|&(particle_type, sequence, _)| (particle_type, sequence));

    let mut batch_key: Option<BatchKey> = None;
    // `CParticle::Render` keeps the last bound raster in RenderWare state.
    // GUNSMOKE and SPLASH intentionally have no raster of their own and draw
    // with this value; retaining it here removes the old invisible-particle
    // regression without adding a fabricated fallback texture.
    let mut last_bound_image: Option<AssetId<Image>> = None;

    for (_, _, entity) in ordered_buf.iter().copied() {
        let Ok((_, trans, mut particle)) = particles.get_mut(entity) else {
            continue;
        };
        let world_translation = trans.translation;
        if particle.alpha == 0 {
            continue;
        }
        let image_id = particle
            .animation_frames
            .get(particle.current_frame)
            .map(|handle| handle.id())
            .or(last_bound_image);
        let Some(image_id) = image_id else { continue };
        if let Some(current) = particle
            .animation_frames
            .get(particle.current_frame)
            .map(|handle| handle.id())
        {
            last_bound_image = Some(current);
        }

        let blend = if particle.flags & DRAW_OPAQUE != 0 {
            ParticleBlendMode::StandardAlpha
        } else if particle.flags & DRAW_DARK != 0 {
            ParticleBlendMode::DarkAlpha
        } else {
            ParticleBlendMode::Additive
        };
        let is_top2d = particle.flags & DRAWTOP2D != 0;

        // RwIm2DVertexSetIntRGBA: r*intens >> 8 (Sprite.cpp:260).
        let rgb = apply_postfx_rgb(
            [
                ((u16::from(particle.colour[0]) * u16::from(particle.intensity)) >> 8) as u8,
                ((u16::from(particle.colour[1]) * u16::from(particle.intensity)) >> 8) as u8,
                ((u16::from(particle.colour[2]) * u16::from(particle.intensity)) >> 8) as u8,
            ],
            postfx_multiplier,
        );

        // DRAWTOP2D particles carry their position directly in device pixel
        // space and draw with size*63 dimensions (Particle.cpp:1579-1606).
        if is_top2d {
            let size = particle.size * 63.0;
            let centre = Vec2::new(world_translation.x, world_translation.y);
            if particle.rotation_degrees != 0.0 {
                emit_rotated_sprite(
                    &mut frame,
                    &mut batch_key,
                    centre,
                    size,
                    size,
                    particle.rotation_degrees,
                    viewport_size,
                    rgb,
                    particle.alpha,
                    0.0,
                    image_id,
                    blend,
                    false,
                );
            } else {
                emit_plain_sprite(
                    &mut frame,
                    &mut batch_key,
                    centre,
                    Vec2::splat(size),
                    viewport_size,
                    rgb,
                    particle.alpha,
                    0.0,
                    image_id,
                    blend,
                    false,
                );
            }
            continue;
        }

        let Some((coors, depth_ndc, view_depth)) = project(world_translation) else {
            continue;
        };
        // CalcScreenCoors base sizes in device pixels (Sprite.cpp:38-44):
        // outw = fovScale * SCREEN_SCALE_AR(recip) * SCREEN_WIDTH,
        // outh = fovScale * recip * SCREEN_HEIGHT.
        let recip_z = 1.0 / view_depth.max(f32::EPSILON);
        let base_w = fov_scale * aspect_ratio_scale * recip_z * viewport_size.x;
        let base_h = fov_scale * recip_z * viewport_size.y;

        // PC_PARTICLE whole-screen scale limit (Particle.cpp:1617-1619); the
        // check uses the un-multiplied CalcScreenCoors sizes.
        if is_pc_particle_banned(particle.particle_type)
            && (viewport_size.x * PARTICLE_SCALE_LIMIT < base_w
                || viewport_size.y * PARTICLE_SCALE_LIMIT < base_h)
        {
            continue;
        }

        if particle.rotation_degrees != 0.0 {
            emit_rotated_sprite(
                &mut frame,
                &mut batch_key,
                coors,
                particle.size * base_w,
                particle.size * base_h,
                particle.rotation_degrees,
                viewport_size,
                rgb,
                particle.alpha,
                depth_ndc,
                image_id,
                blend,
                true,
            );
        } else if particle.flags & SCREEN_TRAIL != 0 {
            // Trail from the previous frame's screen position
            // (Particle.cpp:1633-1687).
            let mut trail_length = 0.0;
            let mut rotation = 0.0;
            if particle.screen_position.x != 0.0 {
                let dist_vec = coors - particle.screen_position;
                trail_length = dist_vec.length();
                rotation = trail_rotation_degrees(dist_vec);
                let speed = particle.velocity.length();
                // Particle.cpp:1669 uses CTimer::GetTimeStep().
                let new_trail_length = speed * time_step * base_w * 2.0;
                if trail_length > new_trail_length {
                    trail_length = new_trail_length;
                }
            }
            let multiplier = trail_length_multiplier(psystems.as_deref(), particle.particle_type);
            emit_rotated_sprite(
                &mut frame,
                &mut batch_key,
                coors,
                particle.size * base_w,
                particle.size * base_h + trail_length * multiplier,
                rotation,
                viewport_size,
                rgb,
                particle.alpha,
                depth_ndc,
                image_id,
                blend,
                true,
            );
            // particle->m_vecScreenPosition = coors (Particle.cpp:1686).
            particle.screen_position = coors;
        } else if particle.flags & SPEED_TRAIL != 0 {
            // Trail toward pos - velocity projected on screen
            // (Particle.cpp:1688-1734).
            let prev_position = world_translation - particle.velocity;
            let mut trail_length = 0.0;
            let mut rotation = 0.0;
            if let Some((prev_screen, _, _)) = project(prev_position) {
                let dist_vec = coors - prev_screen;
                trail_length = dist_vec.length();
                rotation = trail_rotation_degrees(dist_vec);
            }
            let multiplier = trail_length_multiplier(psystems.as_deref(), particle.particle_type);
            emit_rotated_sprite(
                &mut frame,
                &mut batch_key,
                coors,
                particle.size * base_w,
                particle.size * base_h + trail_length * multiplier,
                rotation,
                viewport_size,
                rgb,
                particle.alpha,
                depth_ndc,
                image_id,
                blend,
                true,
            );
        } else if particle.flags & VERT_TRAIL != 0 {
            // Vertical stretch from GTA Z velocity (Bevy Y),
            // Particle.cpp:1735-1748.
            let trail_length = particle.velocity.y.abs() * 10.0;
            let multiplier = trail_length_multiplier(psystems.as_deref(), particle.particle_type);
            emit_plain_sprite(
                &mut frame,
                &mut batch_key,
                coors,
                Vec2::new(
                    particle.size * base_w,
                    (particle.size + trail_length * multiplier) * base_h,
                ),
                viewport_size,
                rgb,
                particle.alpha,
                depth_ndc,
                image_id,
                blend,
                true,
            );
        } else if particle.particle_type == PARTICLE_RAINDROP_SMALL {
            // Particle.cpp:1749-1760.
            emit_plain_sprite(
                &mut frame,
                &mut batch_key,
                coors,
                Vec2::new(particle.size * base_w * 0.05, particle.size * base_h),
                viewport_size,
                rgb,
                particle.alpha,
                depth_ndc,
                image_id,
                blend,
                true,
            );
        } else if particle.particle_type == PARTICLE_BOAT_WAKE {
            // Height comes straight from the system default radius
            // (Particle.cpp:1761-1772).
            let height = psystems
                .as_deref()
                .and_then(|mgr| mgr.particle(particle.particle_type))
                .map(|system| system.default_initial_radius)
                .unwrap_or(particle.size);
            emit_plain_sprite(
                &mut frame,
                &mut batch_key,
                coors,
                Vec2::new(particle.size * base_w, height * base_h),
                viewport_size,
                rgb,
                particle.alpha,
                depth_ndc,
                image_id,
                blend,
                true,
            );
        } else {
            // Plain buffered sprite (Particle.cpp:1773-1784).
            emit_plain_sprite(
                &mut frame,
                &mut batch_key,
                coors,
                Vec2::new(particle.size * base_w, particle.size * base_h),
                viewport_size,
                rgb,
                particle.alpha,
                depth_ndc,
                image_id,
                blend,
                true,
            );
        }
    }
    frame.particle_vertex_count = frame.vertices.len();
}

/// Packs the frame vertices into upload-ready bytes.
fn pack_vertices(frame: &GtaParticleSpriteFrame) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(frame.vertices.len() * VERTEX_SIZE);
    for vertex in &frame.vertices {
        bytes.extend_from_slice(&vertex.position[0].to_le_bytes());
        bytes.extend_from_slice(&vertex.position[1].to_le_bytes());
        bytes.extend_from_slice(&vertex.position[2].to_le_bytes());
        bytes.extend_from_slice(&vertex.uv[0].to_le_bytes());
        bytes.extend_from_slice(&vertex.uv[1].to_le_bytes());
        bytes.extend_from_slice(&vertex.colour);
    }
    bytes
}

/// Render-world mirror of [`GtaParticleSpriteFrame`].
#[derive(Default, Resource)]
struct ExtractedGtaParticleSprites {
    batches: Vec<ParticleSpriteBatch>,
    vertices_bytes: Vec<u8>,
}

/// Render-world extraction of the built frame.
fn extract_particle_sprite_frame(
    frame: Extract<Res<GtaParticleSpriteFrame>>,
    mut commands: Commands,
) {
    commands.insert_resource(ExtractedGtaParticleSprites {
        batches: frame.batches.clone(),
        vertices_bytes: pack_vertices(&frame),
    });
}

#[derive(Resource, Default)]
struct GtaParticleSpriteBuffers {
    buffer: Option<Buffer>,
    capacity: usize,
}

#[derive(Resource)]
struct GtaParticleSpritePipeline {
    shader: Handle<Shader>,
    bind_group_layout_descriptor: BindGroupLayoutDescriptor,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct ParticleSpritePipelineKey {
    blend: ParticleBlendMode,
    z_test_enabled: bool,
    color_format: TextureFormat,
    depth_format: TextureFormat,
    sample_count: u32,
}

impl SpecializedRenderPipeline for GtaParticleSpritePipeline {
    type Key = ParticleSpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: VERTEX_SIZE as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 12,
                    shader_location: 1,
                },
                VertexAttribute {
                    format: VertexFormat::Unorm8x4,
                    offset: 20,
                    shader_location: 2,
                },
            ],
        };
        RenderPipelineDescriptor {
            label: Some("gta_particle_sprite_pipeline".into()),
            layout: vec![self.bind_group_layout_descriptor.clone()],
            immediate_size: 0,
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: Vec::new(),
                constants: Default::default(),
                entry_point: Some("vs_main".into()),
                buffers: vec![vertex_buffer_layout],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: key.depth_format,
                // rwRENDERSTATEZWRITEENABLE = FALSE.
                depth_write_enabled: Some(false),
                depth_compare: if key.z_test_enabled {
                    // Core3d uses reverse-Z (`MeshPipeline`).
                    // This remains the RenderWare Z-test semantically; only
                    // the backend depth convention is reversed.
                    Some(CompareFunction::GreaterEqual)
                } else {
                    None
                },
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: key.sample_count,
                ..Default::default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: Vec::new(),
                constants: Default::default(),
                entry_point: Some("fs_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.color_format,
                    blend: key.blend.blend_state(),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
}

/// Core3d pass system drawing all particle sprite batches after scene
/// transparency, matching re3's late `RenderEffects` placement
/// (`core/main.cpp:1455`).
fn particle_sprite_pass(
    view: ViewQuery<(&ViewTarget, &ViewDepthStencilTexture)>,
    extracted: Option<Res<ExtractedGtaParticleSprites>>,
    images: Res<RenderAssets<GpuImage>>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<GtaParticleSpritePipeline>,
    mut specialized: ResMut<SpecializedRenderPipelines<GtaParticleSpritePipeline>>,
    mut buffers: ResMut<GtaParticleSpriteBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ctx: RenderContext,
) {
    let Some(extracted) = extracted else {
        return;
    };
    if extracted.batches.is_empty() {
        return;
    }
    let (view_target, view_depth) = view.into_inner();

    // `capacity` must mirror the GPU buffer's real byte size.  The previous
    // `next_power_of_two()` bookkeeping let a later larger frame pass the
    // capacity check but still `write_buffer` into the smaller allocation,
    // which wgpu rejects as a validation error and Bevy turns into a hard
    // application quit the moment car-collision dust grows the batch.
    if buffers.capacity < extracted.vertices_bytes.len() {
        buffers.buffer = Some(
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("gta_particle_sprite_vertices"),
                contents: &extracted.vertices_bytes,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }),
        );
        buffers.capacity = extracted.vertices_bytes.len();
    } else if let Some(buffer) = buffers.buffer.as_ref() {
        render_queue.write_buffer(buffer, 0, &extracted.vertices_bytes);
    }
    let Some(vertex_buffer) = buffers.buffer.as_ref() else {
        return;
    };

    let color_format = view_target.main_texture_format();
    let depth_format = view_depth.texture().format();
    let sample_count = view_depth.texture().sample_count();

    let layout = pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout_descriptor);
    let mut bind_groups: HashMap<AssetId<Image>, BindGroup> = HashMap::new();
    let mut drawn_batches = 0usize;
    let mut drawn_vertices = 0u32;
    for batch in &extracted.batches {
        if bind_groups.contains_key(&batch.image) {
            continue;
        }
        let Some(gpu_image) = images.get(batch.image) else {
            continue;
        };
        let group = render_device.create_bind_group(
            "gta_particle_sprite_bind_group",
            &layout,
            &BindGroupEntries::sequential((&gpu_image.texture_view, &gpu_image.sampler)),
        );
        bind_groups.insert(batch.image, group);
    }

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("gta_particle_sprite_pass"),
        color_attachments: &[Some(view_target.get_color_attachment())],
        // Load the scene depth; writes stay disabled via pipeline state,
        // matching rwRENDERSTATEZWRITEENABLE=FALSE + ZTESTENABLE=TRUE.
        depth_stencil_attachment: Some(view_depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    for batch in &extracted.batches {
        let Some(bind_group) = bind_groups.get(&batch.image) else {
            bevy::log::warn_once!(
                "particle sprite texture {:?} not yet uploaded; skipping batch",
                batch.image
            );
            continue;
        };
        let pipeline_id: CachedRenderPipelineId = specialized.specialize(
            &pipeline_cache,
            &pipeline,
            ParticleSpritePipelineKey {
                blend: batch.blend,
                z_test_enabled: batch.z_test_enabled,
                color_format,
                depth_format,
                sample_count,
            },
        );
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
            continue;
        };
        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(
            batch.first_vertex..batch.first_vertex + batch.vertex_count,
            0..1,
        );
        drawn_batches += 1;
        drawn_vertices = drawn_vertices.saturating_add(batch.vertex_count);
    }
    if fx_runtime_diagnostics_enabled() {
        static PASS_COUNTER: AtomicU64 = AtomicU64::new(0);
        let pass = PASS_COUNTER.fetch_add(1, Ordering::Relaxed);
        if pass % 60 == 0 {
            eprintln!(
                "FX_GPU_DIAG pass={} submitted_batches={} submitted_vertices={} requested_batches={} uploaded_textures={}",
                pass,
                drawn_batches,
                drawn_vertices,
                extracted.batches.len(),
                bind_groups.len(),
            );
        }
    }
}

/// Appends the visible low/fluffy-cloud portion of `CClouds::Render` to the
/// same pre-projected sprite frame used by particles.
///
/// re3 disables both Z testing and Z writes for this pass and calls
/// `CSprite::CalcScreenCoors(..., false)`, which keeps the near/behind-camera
/// rejection but deliberately does not reject clouds beyond the normal far
/// clip. Rendering these as ordinary world-space Bevy quads therefore cannot
/// reproduce the original behavior.
#[allow(clippy::too_many_arguments)]
pub fn append_cloud_sprite_frame(
    mut frame: ResMut<GtaParticleSpriteFrame>,
    clouds: Option<Res<CClouds>>,
    draw: Option<Res<CDraw>>,
    camera_state: Option<Res<CCamera>>,
    postfx: Option<Res<CPostFX>>,
    file_loader: Option<Res<CFileLoader>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut images: ResMut<Assets<Image>>,
    // Same-frame camera projection (see build_particle_sprite_frame).
    cameras: Query<(&Camera, &Transform, &Projection), With<Camera3d>>,
    windows: Query<&Window>,
) {
    frame.cloud_vertex_count = 0;
    let start_vertex = frame.vertices.len();
    let Some(clouds) = clouds else {
        return;
    };
    if clouds.render_intents.is_empty() {
        return;
    }
    let Some(file_loader) = file_loader else {
        return;
    };
    let Some(particle_slot) = file_loader.txd_store.find_txd_slot("particle") else {
        return;
    };
    if !sync_txd_slot_texture_cache(particle_slot, &file_loader, &mut texture_cache, &mut images) {
        return;
    }

    let Some((camera, camera_transform, projection)) = cameras
        .iter()
        .find(|(cam, _, _)| cam.is_active)
        .or_else(|| cameras.iter().next())
        .map(|(cam, transform, proj)| (cam, *transform, proj))
    else {
        return;
    };
    let Projection::Perspective(perspective) = projection else {
        return;
    };
    let Some(viewport_size) = camera
        .viewport
        .as_ref()
        .map(|viewport| {
            Vec2::new(
                viewport.physical_size.x as f32,
                viewport.physical_size.y as f32,
            )
        })
        .or_else(|| {
            windows
                .iter()
                .next()
                .map(|window| Vec2::new(window.width(), window.height()))
        })
    else {
        return;
    };
    if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
        return;
    }

    let world_from_camera = camera_transform.to_matrix();
    let view_from_world = world_from_camera.inverse();
    let clip_from_world = perspective.get_clip_from_view() * view_from_world;
    let near_clip = perspective.near;
    let fov_scale = RE3_DEFAULT_FOV
        / draw
            .as_ref()
            .map(|draw| draw.fov)
            .unwrap_or(RE3_GAME_FOV_FALLBACK);
    let aspect_ratio_scale = RE3_DEFAULT_ASPECT_RATIO / (viewport_size.x / viewport_size.y);
    let postfx_multiplier = postfx
        .as_deref()
        .zip(camera_state.as_deref())
        .map_or([1.0; 3], |(postfx, camera)| {
            postfx.effective_colour_multiplier(camera)
        });

    // `CalcScreenCoors(..., false)`: reject behind/near-camera points, but do
    // not apply the far clip used by the normal particle path.
    let project = |position: Vec3| -> Option<(Vec2, f32, f32)> {
        let clip = clip_from_world * position.extend(1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let view = view_from_world * position.extend(1.0);
        let view_depth = -view.z;
        if view_depth <= near_clip + 1.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let screen = Vec2::new(
            (ndc.x * 0.5 + 0.5) * viewport_size.x,
            (1.0 - (ndc.y * 0.5 + 0.5)) * viewport_size.y,
        );
        Some((screen, ndc.z.clamp(0.0, 1.0), view_depth))
    };

    let mut batch_key = frame
        .batches
        .last()
        .map(|batch| (batch.image, batch.blend, batch.z_test_enabled));
    for intent in &clouds.render_intents {
        let texture_name = clouds
            .texture_names
            .get(intent.texture_index as usize)
            .copied()
            .unwrap_or("cloud1")
            .to_ascii_lowercase();
        let Some(image) = texture_cache
            .images
            .get(&(particle_slot, texture_name))
            .map(|handle| handle.id())
        else {
            continue;
        };
        let Some((screen, _depth_ndc, view_depth)) = project(intent.position.to_bevy()) else {
            continue;
        };
        let recip_z = 1.0 / view_depth.max(f32::EPSILON);
        let base_w = fov_scale * aspect_ratio_scale * recip_z * viewport_size.x;
        let base_h = fov_scale * recip_z * viewport_size.y;
        let blend = if intent.additive {
            ParticleBlendMode::Additive
        } else {
            // Clouds.cpp:231-233 explicitly switches fluffy clouds to
            // SRCALPHA / INVSRCALPHA.
            ParticleBlendMode::StandardAlpha
        };
        let right_gta =
            re3bevy_core::core::Coordinates::CVector::from_bevy(*camera_transform.right());
        let horizontal_right = (right_gta.x * right_gta.x + right_gta.y * right_gta.y)
            .sqrt()
            .min(1.0);
        let camera_roll_degrees = horizontal_right.acos().copysign(right_gta.z).to_degrees();
        let filtered_colour = apply_postfx_rgb(
            [intent.colour[0], intent.colour[1], intent.colour[2]],
            postfx_multiplier,
        );
        emit_rotated_sprite(
            &mut frame,
            &mut batch_key,
            screen,
            base_w * intent.width,
            base_h * intent.height,
            intent.rotation_degrees + camera_roll_degrees,
            viewport_size,
            filtered_colour,
            intent.colour[3],
            // re3 draws clouds before the world with Z test/write disabled.
            // This custom pass runs after the world, so reverse-Z depth 0 with
            // GreaterEqual reproduces the same visible ordering: clouds only
            // touch pixels whose scene depth is still the clear value.
            0.0,
            image,
            blend,
            true,
        );
    }
    frame.cloud_vertex_count = frame.vertices.len().saturating_sub(start_vertex);
}

pub struct GtaParticleSpritePlugin;

impl Plugin for GtaParticleSpritePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GtaParticleSpriteFrame>();

        let shader = super::ShaderAssets::PARTICLE_SPRITE;

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<GtaParticleSpriteBuffers>()
            // `particle_sprite_pass` takes this as `ResMut`; without it the
            // render schedule aborts parameter validation and takes the whole
            // app down on the first frame that draws a particle batch.
            .init_resource::<SpecializedRenderPipelines<GtaParticleSpritePipeline>>()
            .insert_resource(GtaParticleSpritePipeline {
                shader,
                bind_group_layout_descriptor: BindGroupLayoutDescriptor::new(
                    "gta_particle_sprite_bind_group_layout",
                    &BindGroupLayoutEntries::sequential(
                        ShaderStages::FRAGMENT,
                        (
                            bevy::render::render_resource::binding_types::texture_2d(
                                TextureSampleType::Float { filterable: true },
                            ),
                            bevy::render::render_resource::binding_types::sampler(
                                SamplerBindingType::Filtering,
                            ),
                        ),
                    ),
                ),
            })
            .add_systems(ExtractSchedule, extract_particle_sprite_frame)
            .add_systems(
                Core3d,
                particle_sprite_pass
                    .in_set(Core3dSystems::MainPass)
                    .after(main_transparent_pass_3d),
            );
    }
}
