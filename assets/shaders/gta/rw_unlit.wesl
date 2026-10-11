import bevy_render::view::View;
import bevy_pbr::render::mesh_types::Mesh;
import bevy_pbr::render::forward_io::VertexOutput;
import super::rw_unlit_util::{GtaGlobalLighting, apply_colour_filter_iii, apply_fog, linear_to_rw_rgb, rw_to_linear_rgb};

@if(BINDLESS)
enable wgpu_binding_array;

@group(0) @binding(0) var<uniform> gta_view: View;

// Must match `GtaUnlitUniforms` / `#[uniform(0)]` in RendererMaterials.rs
struct GtaUnlitUniforms {
    colour: vec4<f32>,
    flags: vec4<f32>,
    surface: vec4<f32>,
    light_direction_and_prelit: vec4<f32>,
    lighting_options: vec4<f32>,
}

// Non-bindless backends retain the original material layout.
@if(!BINDLESS)
@group(3) @binding(0) var<uniform> material_single: GtaUnlitUniforms;
@if(!BINDLESS)
@group(3) @binding(1) var base_color_texture: texture_2d<f32>;
@if(!BINDLESS)
@group(3) @binding(2) var base_color_sampler: sampler;
@if(!BINDLESS)
@group(3) @binding(3) var<storage, read> lighting_single: GtaGlobalLighting;

@if(BINDLESS)
struct MaterialBindings {
    data: u32,
    texture: u32,
    sampler: u32,
    lighting: u32,
}
@if(BINDLESS)
@group(3) @binding(0) var<storage, read> material_indices: array<MaterialBindings>;
@if(BINDLESS)
@group(3) @binding(1) var material_samplers: binding_array<sampler>;
@if(BINDLESS)
@group(3) @binding(5) var material_textures: binding_array<texture_2d<f32>>;
@if(BINDLESS)
@group(3) @binding(10) var<storage, read> material_data: array<GtaUnlitUniforms>;
@if(BINDLESS)
@group(3) @binding(11) var<storage, read> lighting_buffers: binding_array<GtaGlobalLighting>;

@if(BINDLESS)
@group(2) @binding(0) var<storage, read> mesh_instances: array<Mesh>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    @if(BINDLESS)
    let bindings = material_indices[mesh_instances[mesh.instance_index].material_and_lightmap_bind_group_slot & 0xffffu];
    @if(BINDLESS)
    let material = material_data[bindings.data];
    @if(BINDLESS)
    let global_lighting = lighting_buffers[bindings.lighting];
    @if(BINDLESS)
    let tex = textureSample(material_textures[bindings.texture], material_samplers[bindings.sampler], mesh.uv);
    @if(!BINDLESS)
    let material = material_single;
    @if(!BINDLESS)
    let global_lighting = lighting_single;
    @if(!BINDLESS)
    let tex = textureSample(base_color_texture, base_color_sampler, mesh.uv);
    let texture_rw = linear_to_rw_rgb(tex.rgb);

    @if(VERTEX_COLORS)
    let source_colour = mesh.color;
    @if(!VERTEX_COLORS)
    let source_colour = vec4<f32>(1.0);
    var vertex_colour = source_colour;
    let mode = material.light_direction_and_prelit.w;
    if mode >= 0.0 {
        let has_prelight = step(0.5, mode) * (1.0 - step(1.5, mode));
        let batched_material_colour = step(1.5, mode);
        // Bevy's mesh vertex stage already supplies the transformed normal.
        // re3/librw's default shader dots that value directly; normalizing a
        // zero RW normal here produced NaNs even when directional light was
        // disabled for static world entities.
        let normal = mesh.world_normal;
        let sun = normalize(global_lighting.light_direction_and_far_clip.xyz);
        let diffuse = max(dot(normal, sun), 0.0);
        // C3dMarker::Render installs SetBrightMarkerColours around its draw.
        var ambient = global_lighting.ambient.rgb;
        var directional = global_lighting.directional.rgb;
        if material.lighting_options.z > 0.5 {
            ambient = vec3<f32>(0.6);
            directional = (vec3<f32>(1.0) - directional) * 0.4 + directional;
        }
        let rw_light =
            source_colour.rgb * has_prelight
            + ambient
                * (material.surface.x * material.lighting_options.x)
            + directional
                * (material.surface.y * diffuse * material.lighting_options.y);
        let clamped_light = clamp(rw_light, vec3<f32>(0.0), vec3<f32>(1.0));
        // Arena's resident vehicle batching stores the original RW material
        // colour in the vertex stream for mode 2. re3's default_UV2_VS first
        // clamps lighting, then multiplies by matCol; preserve that order.
        let lit_vertex_rgb = mix(
            clamped_light,
            clamped_light * source_colour.rgb,
            batched_material_colour,
        );
        vertex_colour = vec4<f32>(lit_vertex_rgb, source_colour.a);
    }

    var lit = vec4<f32>(
        material.colour.rgb * texture_rw * vertex_colour.rgb,
        material.colour.a * tex.a * vertex_colour.a,
    );

    let fog_start = global_lighting.fog_colour_and_start.w;
    let far_clip = global_lighting.light_direction_and_far_clip.w;
    if far_clip > fog_start {
        let view_depth = -(gta_view.view_from_world * mesh.world_position).z;
        let fog_f = clamp((view_depth - fog_start) / (far_clip - fog_start), 0.0, 1.0);
        lit = apply_fog(lit, global_lighting.fog_colour_and_start.rgb, fog_f);
    }

    let cutoff = material.flags.y;
    if cutoff > 0.0 && lit.a <= cutoff {
        discard;
    }

    if material.flags.z > 0.5 {
        // Additive: scale RGB by alpha for Add blend.
        lit = vec4<f32>(lit.rgb * lit.a, lit.a);
    }
    lit = apply_colour_filter_iii(lit, global_lighting.postfx_blur_colour);
    return vec4<f32>(rw_to_linear_rgb(lit.rgb), lit.a);
}
