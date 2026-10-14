//! Bevy 0.20 Core3d postprocessing for re3's camera-raster colour filters.
//! Reference: extras/postfx.cpp, colourfilterIII_PS.hlsl, contrastPS.hlsl.

use bevy::{
    core_pipeline::{Core3d, Core3dSystems, FullscreenShader},
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{ExtractedView, ResolveCompositingSpaces, ResolvedCompositingSpace, ViewTarget},
    },
    window::PrimaryWindow,
};

use super::{BevyRender::GtaSceneCamera, ShaderAssets};
use crate::{
    core::Camera::CCamera,
    extras::postfx::{CPostFX, POSTFX_MOBILE, POSTFX_NORMAL, POSTFX_SIMPLE},
};

/// GPU inputs for a camera's postprocess. One extracted component per scene
/// camera makes split-screen views independent, without a second gameplay state.
#[derive(Component, Clone, Copy, Default, PartialEq, ExtractComponent, ShaderType)]
#[extract_app(RenderApp)]
pub struct GtaPostFxSettings {
    pub colour: Vec4,
    pub contrast_mult: Vec4,
    pub contrast_add_and_mode: Vec4,
    /// xy: camera physical viewport origin; zw: its size in physical pixels.
    /// Postprocessing copies other viewports unchanged for split-screen safety.
    pub view_info: Vec4,
}

fn sync_postfx_settings(
    mut commands: Commands,
    postfx: Option<Res<CPostFX>>,
    camera: Option<Res<CCamera>>,
    mut views: Query<(
        Entity,
        &Camera,
        &GtaSceneCamera,
        Option<&mut GtaPostFxSettings>,
    )>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut settings = GtaPostFxSettings::default();
    if let (Some(postfx), Some(camera)) = (postfx, camera) {
        let rgba = [
            camera.blur_red.clamp(0, 255) as f32,
            camera.blur_green.clamp(0, 255) as f32,
            camera.blur_blue.clamp(0, 255) as f32,
            camera.motion_blur.clamp(0, 255) as f32,
        ];
        match postfx.effect_switch {
            POSTFX_NORMAL if !postfx.motion_blur_on => {
                if let Some(colour) = postfx.effective_colour_filter(&camera) {
                    settings.colour = Vec4::from_array(colour);
                    settings.contrast_add_and_mode.w = 1.0;
                }
            }
            POSTFX_MOBILE => {
                settings.contrast_mult = Vec4::new(
                    (rgba[0] - 64.0) / 384.0 + 1.14,
                    (rgba[1] - 64.0) / 384.0 + 1.14,
                    (rgba[2] - 64.0) / 384.0 + 1.14,
                    0.0,
                );
                settings.contrast_add_and_mode =
                    Vec4::new(rgba[0] / 1536.0, rgba[1] / 1536.0, rgba[2] / 1536.0, 2.0);
            }
            POSTFX_SIMPLE => {
                settings.colour = Vec4::from_array(rgba.map(|v| v / 255.0));
                settings.contrast_add_and_mode.w = 3.0;
            }
            _ => {}
        }
    }
    let default_size = windows.iter().next().map(|window| {
        Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        )
    });
    for (entity, camera_view, scene_camera, current) in &mut views {
        let mut settings = settings;
        settings.contrast_mult.w = scene_camera.local_player as f32;
        if let Some(rect) = camera_view.physical_viewport_rect() {
            settings.view_info = Vec4::new(
                rect.min.x as f32,
                rect.min.y as f32,
                rect.width() as f32,
                rect.height() as f32,
            );
        } else if let Some(viewport) = camera_view.viewport.as_ref() {
            settings.view_info = Vec4::new(
                viewport.physical_position.x as f32,
                viewport.physical_position.y as f32,
                viewport.physical_size.x as f32,
                viewport.physical_size.y as f32,
            );
        } else if let Some(size) = default_size {
            settings.view_info = Vec4::new(0.0, 0.0, size.x, size.y);
        }
        if let Some(mut current) = current {
            if *current != settings {
                *current = settings;
            }
        } else {
            commands.entity(entity).insert(settings);
        }
    }
}

#[derive(Resource)]
pub(super) struct GtaPostFxPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipelines: Vec<(TextureFormat, CachedRenderPipelineId)>,
}

fn init_postfx_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "gta_postfx_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<GtaPostFxSettings>(true),
            ),
        ),
    );
    let sampler = device.create_sampler(&SamplerDescriptor::default());
    // The camera boundary owns the format. Never force an sRGB attachment here:
    // an encoded UNORM raster must stay encoded through postprocessing.
    let pipelines = [
        TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Rgba8Unorm,
        TextureFormat::Bgra8UnormSrgb,
        TextureFormat::Bgra8Unorm,
        TextureFormat::Rgba16Float,
    ]
    .into_iter()
    .map(|format| {
        let pipeline_id = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("gta_postfx_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: ShaderAssets::RW_POSTFX,
                shader_defs: vec![bevy::shader::ShaderDefVal::Bool(
                    "GTA_ENCODED_TARGET".into(),
                    matches!(
                        format,
                        TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm
                    ),
                )],
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
        (format, pipeline_id)
    })
    .collect();
    commands.insert_resource(GtaPostFxPipeline {
        layout,
        sampler,
        pipelines,
    });
}

pub(super) fn postfx_pass(
    view: ViewQuery<(
        &ViewTarget,
        &DynamicUniformIndex<GtaPostFxSettings>,
        &GtaPostFxSettings,
    )>,
    pipeline: Option<Res<GtaPostFxPipeline>>,
    uniforms: Res<ComponentUniforms<GtaPostFxSettings>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    mut ctx: RenderContext,
) {
    let (target, index, settings) = view.into_inner();
    // No effect: avoid an unnecessary full-screen copy and ping-pong swap.
    if settings.contrast_add_and_mode.w < 0.5 {
        return;
    }
    let Some(pipeline) = pipeline else { return };
    let Some((_, pipeline_id)) = pipeline
        .pipelines
        .iter()
        .find(|(format, _)| *format == target.main_texture_format())
    else {
        return;
    };
    let Some(render_pipeline) = cache.get_render_pipeline(*pipeline_id) else {
        return;
    };
    let Some(uniform_binding) = uniforms.uniforms().binding() else {
        return;
    };
    let post_process = target.post_process_write();
    let bind_group = device.create_bind_group(
        "gta_postfx_bind_group",
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((post_process.source, &pipeline.sampler, uniform_binding)),
    );
    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("gta_postfx_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    // Draw the entire attachment. The shader passthroughs pixels belonging to
    // the other local player's camera instead of clobbering the ping-pong
    // destination outside this camera's viewport.
    pass.set_pipeline(render_pipeline);
    pass.set_bind_group(0, &bind_group, &[index.index()]);
    pass.draw(0..3, 0..1);
}

pub struct GtaPostFxRenderPlugin;

impl Plugin for GtaPostFxRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<GtaPostFxSettings>::default(),
            UniformComponentPlugin::<GtaPostFxSettings>::default(),
        ));
        app.add_systems(
            Update,
            sync_postfx_settings.in_set(rust_gta_3_core::core::Schedules::GtaProcessSet::RenderPrep),
        );
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(RenderStartup, init_postfx_pipeline)
                .add_systems(Core3d, postfx_pass.in_set(Core3dSystems::PostProcess));
        }
    }
}

/// Extracted only from GTA scene cameras. UI and unrelated cameras keep Bevy's
/// normal linear rendering; split GTA views retain matching shared raster keys.
#[derive(Component, Clone, Copy, ExtractComponent)]
#[extract_app(RenderApp)]
#[extract_component_filter(With<GtaSceneCamera>)]
struct GtaEncodedRaster;

fn sync_encoded_raster_markers(
    mut commands: Commands,
    added: Query<Entity, (With<GtaSceneCamera>, Without<GtaEncodedRaster>)>,
    removed: Query<Entity, (With<GtaEncodedRaster>, Without<GtaSceneCamera>)>,
) {
    for entity in &added {
        commands.entity(entity).insert(GtaEncodedRaster);
    }
    for entity in &removed {
        commands.entity(entity).remove::<GtaEncodedRaster>();
    }
}

fn prepare_gta_raster_format(mut views: Query<&mut ExtractedView, With<GtaEncodedRaster>>) {
    for mut view in &mut views {
        view.target_format = TextureFormat::Rgba8Unorm;
    }
}

fn resolve_gta_raster_encoding(
    mut views: Query<&mut ResolvedCompositingSpace, With<GtaEncodedRaster>>,
) {
    for mut space in &mut views {
        *space = ResolvedCompositingSpace(Some(bevy::camera::CompositingSpace::Srgb));
    }
}

/// Install only after every GTA world pipeline (including PC MatFX) supports
/// encoded target output. This plugin deliberately is not enabled by postfx.
pub struct GtaEncodedRasterPlugin;

impl Plugin for GtaEncodedRasterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<GtaEncodedRaster>::default());
        app.add_systems(Update, sync_encoded_raster_markers);
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                Render,
                (
                    // Mesh view keys are cached in PrepareAssets, before CreateViews.
                    prepare_gta_raster_format
                        .in_set(RenderSystems::PrepareAssets)
                        .before(bevy::pbr::check_views_need_specialization),
                    // Camera3d requests are rejected by the stock resolver. The GTA
                    // shaders explicitly implement this camera-raster convention.
                    resolve_gta_raster_encoding
                        .in_set(RenderSystems::CreateViews)
                        .after(ResolveCompositingSpaces),
                ),
            );
        }
    }
}

#[cfg(test)]
mod encoded_raster_tests {
    use super::*;

    #[test]
    fn encoding_override_is_scoped_to_gta_views() {
        let mut app = App::new();
        let gta = app
            .world_mut()
            .spawn((GtaEncodedRaster, ResolvedCompositingSpace(None)))
            .id();
        let ui = app.world_mut().spawn(ResolvedCompositingSpace(None)).id();
        app.add_systems(Update, resolve_gta_raster_encoding);
        app.update();
        assert_eq!(
            app.world().get::<ResolvedCompositingSpace>(gta).unwrap().0,
            Some(bevy::camera::CompositingSpace::Srgb)
        );
        assert_eq!(
            app.world().get::<ResolvedCompositingSpace>(ui).unwrap().0,
            None
        );
    }
}
