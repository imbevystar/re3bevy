//! Camera-raster lens droplets. Bevy 0.20 Core3d pass corresponding to re3's
//! `ScreenDroplets::Render` and `screenDroplet_PS.hlsl`.

use super::{PostFxRender::GtaPostFxSettings, ShaderAssets};
use crate::extras::screendroplets::ScreenDrop;
use bevy::{
    core_pipeline::{Core3d, Core3dSystems, FullscreenShader},
    mesh::VertexBufferLayout,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            binding_types::{sampler, texture_2d},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        view::ViewTarget,
    },
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DropletVertex {
    position: [f32; 2],
    mask_uv: [f32; 2],
    capture_uv: [f32; 2],
    colour: [f32; 4],
}

#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub struct GtaScreenDropletFrame {
    vertices: Vec<DropletVertex>,
}

impl GtaScreenDropletFrame {
    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    pub fn append(&mut self, drop: &ScreenDrop, screen: Vec2) {
        if !drop.active || screen.min_element() <= 0.0 {
            return;
        }
        // re3 SCREEN_SCALE_X/Y, AddToRenderList. Screen-space screen capture
        // uses the actual colour attachment extent instead of a power-of-two
        // Raster, so crop coordinates are divided by the viewport dimensions.
        let scale = 0.5 * drop.size * screen.x / 640.0;
        let mag_size = (drop.magnification * 260.0 + 40.0) * screen.y / 448.0;
        let ul = ((drop.x - mag_size) / screen.x).clamp(0.0, 1.0);
        let ur = ((drop.x + mag_size) / screen.x).clamp(0.0, 1.0);
        let vt = ((drop.y - mag_size) / screen.y).clamp(0.0, 1.0);
        let vb = ((drop.y + mag_size) / screen.y).clamp(0.0, 1.0);
        let colour = [
            drop.color.red as f32 / 255.0,
            drop.color.green as f32 / 255.0,
            drop.color.blue as f32 / 255.0,
            drop.color.alpha as f32 / 255.0,
        ];
        let corners = [
            (-1.0, -1.0, 0.0, 0.0, ul, vb),
            (-1.0, 1.0, 0.0, 1.0, ul, vt),
            (1.0, 1.0, 1.0, 1.0, ur, vt),
            (1.0, -1.0, 1.0, 0.0, ur, vb),
        ];
        let mut quad = [DropletVertex {
            position: [0.0; 2],
            mask_uv: [0.0; 2],
            capture_uv: [0.0; 2],
            colour,
        }; 4];
        for (vertex, (x, y, u, v, u2, v2)) in quad.iter_mut().zip(corners) {
            let px = drop.x + x * scale;
            let py = drop.y + y * scale;
            vertex.position = [2.0 * px / screen.x - 1.0, 1.0 - 2.0 * py / screen.y];
            vertex.mask_uv = [u, v];
            vertex.capture_uv = [u2, v2];
        }
        for i in [0usize, 1, 2, 2, 3, 0] {
            self.vertices.push(quad[i]);
        }
    }
}

#[derive(Resource, Default)]
struct DropletGpuBuffer {
    buffer: Option<Buffer>,
    capacity: usize,
}

#[derive(Resource)]
struct DropletPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipelines: Vec<(
        TextureFormat,
        CachedRenderPipelineId,
        CachedRenderPipelineId,
    )>,
}

fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "gta_screen_droplets_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    let source = ShaderAssets::RW_SCREEN_DROPLET;
    // Match ViewTarget: encoded UNORM preserves native camera-raster blending;
    // sRGB and HDR attachments retain their linear shader output.
    let pipelines = [
        TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Rgba8Unorm,
        TextureFormat::Bgra8UnormSrgb,
        TextureFormat::Bgra8Unorm,
        TextureFormat::Rgba16Float,
    ]
    .into_iter()
    .map(|format| {
        let shader_defs = vec![bevy::shader::ShaderDefVal::Bool(
            "GTA_ENCODED_TARGET".into(),
            matches!(
                format,
                TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm
            ),
        )];
        let target = ColorTargetState {
            format,
            blend: None,
            write_mask: ColorWrites::ALL,
        };
        let copy = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("gta_screen_droplets_copy".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: source.clone(),
                entry_point: Some("copy_fragment".into()),
                shader_defs: shader_defs.clone(),
                targets: vec![Some(target.clone())],
                ..default()
            }),
            ..default()
        });
        let vertices = VertexBufferLayout {
            array_stride: std::mem::size_of::<DropletVertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 8,
                    shader_location: 1,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 16,
                    shader_location: 2,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 24,
                    shader_location: 3,
                },
            ],
        };
        let droplets = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("gta_screen_droplets_overlays".into()),
            layout: vec![layout.clone()],
            vertex: VertexState {
                shader: source.clone(),
                entry_point: Some("droplet_vertex".into()),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertices],
                ..default()
            },
            primitive: PrimitiveState {
                cull_mode: None,
                ..default()
            },
            fragment: Some(FragmentState {
                shader: source.clone(),
                entry_point: Some("droplet_fragment".into()),
                shader_defs: shader_defs.clone(),
                targets: vec![Some(ColorTargetState {
                    blend: Some(BlendState::ALPHA_BLENDING),
                    ..target
                })],
                ..default()
            }),
            ..default()
        });
        (format, copy, droplets)
    })
    .collect();
    commands.insert_resource(DropletPipeline {
        layout,
        sampler: device.create_sampler(&SamplerDescriptor::default()),
        pipelines,
    });
}

fn draw_screen_droplets(
    view: ViewQuery<(&ViewTarget, &GtaPostFxSettings)>,
    frame: Option<Res<GtaScreenDropletFrame>>,
    pipeline: Option<Res<DropletPipeline>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut gpu: ResMut<DropletGpuBuffer>,
    mut ctx: RenderContext,
) {
    let (target, settings) = view.into_inner();
    // ScreenDroplets belongs to the primary re3 camera, not the second
    // local-co-op viewport.
    if settings.contrast_mult.w > 0.5 {
        return;
    }
    let Some(frame) = frame else { return };
    if frame.vertices.is_empty() {
        return;
    }
    let Some(pipeline) = pipeline else { return };
    let Some((_, copy_id, droplets_id)) = pipeline
        .pipelines
        .iter()
        .find(|(format, _, _)| *format == target.main_texture_format())
    else {
        return;
    };
    let (Some(copy), Some(droplets)) = (
        cache.get_render_pipeline(*copy_id),
        cache.get_render_pipeline(*droplets_id),
    ) else {
        return;
    };
    let bytes = bytemuck::cast_slice(frame.vertices.as_slice());
    if gpu.capacity < bytes.len() {
        gpu.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("gta_screen_droplet_vertices"),
            contents: bytes,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        }));
        gpu.capacity = bytes.len();
    } else if let Some(buffer) = gpu.buffer.as_ref() {
        queue.write_buffer(buffer, 0, bytes);
    }
    let Some(buffer) = gpu.buffer.as_ref() else {
        return;
    };
    let post = target.post_process_write();
    let bind_group = device.create_bind_group(
        "gta_screen_droplets_bind_group",
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((post.source, &pipeline.sampler)),
    );
    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("gta_screen_droplets_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(copy);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
    // Unlike the fullscreen copy, lens overlays affect only player one's
    // viewport when local co-op is active.
    if settings.view_info.z > 0.0 && settings.view_info.w > 0.0 {
        pass.set_scissor_rect(
            settings.view_info.x as u32,
            settings.view_info.y as u32,
            settings.view_info.z as u32,
            settings.view_info.w as u32,
        );
    }
    pass.set_pipeline(droplets);
    pass.set_vertex_buffer(0, (**buffer).slice(..));
    pass.draw(0..frame.vertices.len() as u32, 0..1);
}

pub struct GtaScreenDropletRenderPlugin;

impl Plugin for GtaScreenDropletRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GtaScreenDropletFrame>()
            .add_plugins(ExtractResourcePlugin::<GtaScreenDropletFrame>::default());
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<DropletGpuBuffer>()
                .add_systems(RenderStartup, init_pipeline)
                .add_systems(
                    Core3d,
                    draw_screen_droplets
                        .in_set(Core3dSystems::PostProcess)
                        .before(super::PostFxRender::postfx_pass),
                );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn droplets_use_the_original_six_vertex_quad_and_secondary_capture_uvs() {
        let drop = ScreenDrop {
            x: 320.0,
            y: 224.0,
            size: 40.0,
            magnification: 1.0,
            lifetime: 1000.0,
            time: 0.0,
            active: true,
            fades: false,
            color: crate::objects::ParticleObject::CRGBA {
                red: 255,
                green: 128,
                blue: 64,
                alpha: 160,
            },
        };
        let mut frame = GtaScreenDropletFrame::default();
        frame.append(&drop, Vec2::new(640.0, 448.0));
        assert_eq!(frame.vertices.len(), 6);
        assert_eq!(frame.vertices[0].position[0], -0.0625);
        assert!((frame.vertices[0].position[1] - 40.0 / 448.0).abs() < 0.000001);
        assert_eq!(frame.vertices[1].mask_uv, [0.0, 1.0]);
        assert_eq!(frame.vertices[2].mask_uv, [1.0, 1.0]);
        assert_eq!(frame.vertices[0].capture_uv, [20.0 / 640.0, 1.0]);
        assert_eq!(frame.vertices[1].capture_uv, [20.0 / 640.0, 0.0]);
        assert_eq!(frame.vertices[0].colour[3], 160.0 / 255.0);

        frame.clear();
        assert!(frame.vertices.is_empty());
    }

    #[test]
    fn inactive_or_invalid_drops_are_never_uploaded() {
        let drop = ScreenDrop::default();
        let mut frame = GtaScreenDropletFrame::default();
        frame.append(&drop, Vec2::new(640.0, 448.0));
        assert!(frame.vertices.is_empty());
        frame.append(
            &ScreenDrop {
                active: true,
                ..drop
            },
            Vec2::ZERO,
        );
        assert!(frame.vertices.is_empty());
    }
}
