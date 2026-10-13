import bevy_render::view::View;
import bevy_pbr::render::forward_io::{Vertex, decompress_vertex};
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::{get_world_from_local, mesh_position_local_to_world};
import super::rw_unlit_util::{GtaGlobalLighting, apply_fog, linear_to_rw_rgb, rw_target_rgb, rw_object_local_lights};

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
    actor_origin: vec4<f32>,
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

// librw default.vert/default_VS.hlsl light and clamp each vertex before
// interpolation. Evaluating the 32-light selection in every fragment both
// changed that shading and repeated actor-invariant work over every pixel.
struct GtaVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) fog: f32,
    @if(BINDLESS)
    @location(3) @interpolate(flat) material_slot: u32,
}

@vertex
fn vertex(input: Vertex) -> GtaVertexOutput {
    let source = decompress_vertex(input, input.instance_index);
    let world = get_world_from_local(input.instance_index);
    let world_position = mesh_position_local_to_world(world, vec4<f32>(source.position, 1.0));
    @if(VERTEX_NORMALS)
    let normal = (world * vec4<f32>(source.normal, 0.0)).xyz;
    @if(!VERTEX_NORMALS)
    let normal = vec3<f32>(0.0);
    @if(BINDLESS)
    let material_slot = mesh[input.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    @if(BINDLESS)
    let bindings = material_indices[material_slot];
    @if(BINDLESS)
    let material = material_data[bindings.data];
    @if(BINDLESS)
    let global_lighting = lighting_buffers[bindings.lighting];
    @if(!BINDLESS)
    let material = material_single;
    @if(!BINDLESS)
    let global_lighting = lighting_single;
    @if(VERTEX_COLORS)
    let source_colour = source.color;
    @if(!VERTEX_COLORS)
    let source_colour = vec4<f32>(1.0);
    var vertex_colour = source_colour;
    let mode = material.light_direction_and_prelit.w;
    if mode >= 0.0 {
        let has_prelight = step(0.5, mode) * (1.0 - step(1.5, mode));
        let batched_material_colour = step(1.5, mode);
        // RenderWare dots the transformed authored normal directly, including
        // zero normals, without fragment-stage renormalization.
        let sun = normalize(global_lighting.light_direction_and_far_clip.xyz);
        let diffuse = max(dot(normal, sun), 0.0);
        // C3dMarker::Render installs SetBrightMarkerColours around its draw.
        var ambient = global_lighting.ambient.rgb;
        if material.lighting_options.x > 1.0 {
            ambient = global_lighting.actor_ambient.rgb / material.lighting_options.x;
        }
        var directional = global_lighting.directional.rgb;
        if material.lighting_options.z > 0.5 {
            ambient = vec3<f32>(0.6);
            directional = (vec3<f32>(1.0) - directional) * 0.4 + directional;
        }
        var rw_light =
            source_colour.rgb * has_prelight
            + ambient
                * (material.surface.x * material.lighting_options.x)
            + directional
                * (material.surface.y * diffuse * material.lighting_options.y);
        if material.lighting_options.w > 0.5 {
            let origin = select(world_position.xyz, material.actor_origin.xyz, material.actor_origin.w > 0.5);
            let local = rw_object_local_lights(global_lighting, origin, normal);
            if local.w != 1.0 {
                // SetupLighting replaces ambient/direct colours only; DFF
                // prelight and the extra directional lights are untouched.
                rw_light = source_colour.rgb * has_prelight
                    + global_lighting.ambient.rgb * (material.surface.x * local.w)
                    + directional * (material.surface.y * diffuse * material.lighting_options.y * local.w);
            }
            rw_light += local.rgb * material.surface.y;
        }
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

    var output: GtaVertexOutput;
    output.position = gta_view.clip_from_world * world_position;
    output.uv = vec2<f32>(0.0);
    @if(VERTEX_UVS_A)
    output.uv = source.uv;
    output.colour = material.colour * vertex_colour;
    output.fog = 0.0;
    @if(BINDLESS)
    output.material_slot = material_slot;
    let fog_start = global_lighting.fog_colour_and_start.w;
    let far_clip = global_lighting.light_direction_and_far_clip.w;
    // CShadows::RenderStoredShadows disables fog for additive projected
    // lights, even when the receiver's ordinary world material is fogged.
    if material.flags.w < 0.5 && far_clip > fog_start {
        let view_depth = -(gta_view.view_from_world * world_position).z;
        output.fog = clamp((view_depth - fog_start) / (far_clip - fog_start), 0.0, 1.0);
    }
    return output;
}

@fragment
fn fragment(input: GtaVertexOutput) -> @location(0) vec4<f32> {
    @if(BINDLESS)
    let bindings = material_indices[input.material_slot];
    @if(BINDLESS)
    let material = material_data[bindings.data];
    @if(BINDLESS)
    let global_lighting = lighting_buffers[bindings.lighting];
    @if(BINDLESS)
    let tex = textureSample(material_textures[bindings.texture], material_samplers[bindings.sampler], input.uv);
    @if(!BINDLESS)
    let material = material_single;
    @if(!BINDLESS)
    let global_lighting = lighting_single;
    @if(!BINDLESS)
    let tex = textureSample(base_color_texture, base_color_sampler, input.uv);
    var lit = input.colour * vec4<f32>(linear_to_rw_rgb(tex.rgb), tex.a);
    lit = apply_fog(lit, global_lighting.fog_colour_and_start.rgb, input.fog);

    let cutoff = material.flags.y;
    if cutoff > 0.0 && lit.a <= cutoff {
        discard;
    }

    // GtaCarLightMaterial's ONE/ONE pass reproduces D3D8 encoded-channel
    // addition on a linear HDR attachment by supplying encoded source RGB.
    // Decoding small source bytes first would erase the streetlamp road glow.
    if material.flags.w > 0.5 {
        return vec4<f32>(lit.rgb, lit.a);
    }
    // PASS_ADD's pipeline applies SRCALPHA/ONE; do not premultiply here.
    return vec4<f32>(rw_target_rgb(lit.rgb), lit.a);
}
