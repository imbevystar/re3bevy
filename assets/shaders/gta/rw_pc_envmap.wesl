// Reference-derived GTA III PC MatFX adapter. re3 VehicleModelInfo.cpp and
// Game.cpp; librw 5501c4fdc7425ff926be59369a13593bb6c81b54's
// matfx_env.vert/.frag and matfx_env_VS.hlsl/matfx_env_PS.hlsl.
// GTA's identity env frame uses normal XY, not Neo's eye/Fresnel reflection.
import bevy_render::view::View;
import bevy_pbr::render::forward_io::{Vertex, decompress_vertex};
import bevy_pbr::render::mesh_bindings::mesh;
import bevy_pbr::render::mesh_functions::{get_world_from_local, mesh_position_local_to_world};
import super::rw_unlit_util::{GtaGlobalLighting, apply_fog, linear_to_rw_rgb, rw_target_rgb, rw_object_local_lights};

@if(BINDLESS)
enable wgpu_binding_array;

@group(0) @binding(0) var<uniform> gta_view: View;

struct BaseParams {
    colour: vec4<f32>,
    flags: vec4<f32>,
    surface: vec4<f32>,
    light_direction_and_prelit: vec4<f32>,
    lighting_options: vec4<f32>,
    actor_origin: vec4<f32>,
}
struct PcEnvParams {
    base: BaseParams,
    environment: vec4<f32>,
}
@if(!BINDLESS)
@group(3) @binding(0) var<uniform> params_single: PcEnvParams;
@if(!BINDLESS)
@group(3) @binding(1) var diffuse: texture_2d<f32>;
@if(!BINDLESS)
@group(3) @binding(2) var diffuse_sampler: sampler;
@if(!BINDLESS)
@group(3) @binding(3) var environment: texture_2d<f32>;
@if(!BINDLESS)
@group(3) @binding(4) var environment_sampler: sampler;
@if(!BINDLESS)
@group(3) @binding(5) var<storage, read> lighting_single: GtaGlobalLighting;

@if(BINDLESS)
struct MaterialBindings {
    data: u32,
    diffuse_texture: u32,
    diffuse_sampler: u32,
    environment_texture: u32,
    environment_sampler: u32,
    lighting: u32,
}
@if(BINDLESS)
@group(3) @binding(0) var<storage, read> material_indices: array<MaterialBindings>;
@if(BINDLESS)
@group(3) @binding(1) var material_samplers: binding_array<sampler>;
@if(BINDLESS)
@group(3) @binding(5) var material_textures: binding_array<texture_2d<f32>>;
@if(BINDLESS)
@group(3) @binding(10) var<storage, read> material_data: array<PcEnvParams>;
@if(BINDLESS)
@group(3) @binding(11) var<storage, read> lighting_buffers: binding_array<GtaGlobalLighting>;

struct PcEnvVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) diffuse_uv: vec2<f32>,
    @location(1) environment_uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) fog: f32,
    @if(BINDLESS)
    @location(4) @interpolate(flat) material_slot: u32,
}

@vertex
fn vertex(input: Vertex) -> PcEnvVertexOutput {
    // ModelDrawables owns CPU-skinned/frame-separated, uncompressed DFF
    // attributes. Use Bevy's vertex interface and resident instance transform.
    let source = decompress_vertex(input, input.instance_index);
    let world = get_world_from_local(input.instance_index);
    let position = mesh_position_local_to_world(world, vec4<f32>(source.position, 1.0));
    // librw maps the transformed normal directly, with no per-fragment eye
    // vector or normalization. Zero normals consequently map to the centre.
    let normal = (world * vec4<f32>(source.normal, 0.0)).xyz;
    @if(BINDLESS)
    let material_slot = mesh[input.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    @if(BINDLESS)
    let bindings = material_indices[material_slot];
    @if(BINDLESS)
    let params = material_data[bindings.data];
    @if(BINDLESS)
    let lighting = lighting_buffers[bindings.lighting];
    @if(!BINDLESS)
    let params = params_single;
    @if(!BINDLESS)
    let lighting = lighting_single;
    @if(VERTEX_COLORS)
    let prelight = source.color;
    @if(!VERTEX_COLORS)
    let prelight = vec4<f32>(1.0);
    let base = params.base;
    var colour = prelight;
    if base.light_direction_and_prelit.w >= 0.0 {
        let has_prelight = select(0.0, 1.0, base.light_direction_and_prelit.w > 0.5);
        let sun = normalize(lighting.light_direction_and_far_clip.xyz);
        let diffuse_intensity = max(dot(normal, sun), 0.0);
        var ambient = lighting.ambient.rgb * base.lighting_options.x;
        if base.lighting_options.x > 1.0 {
            ambient = lighting.actor_ambient.rgb;
        }
        var light = prelight.rgb * has_prelight
            + ambient * base.surface.x
            + lighting.directional.rgb * (base.surface.y * diffuse_intensity * base.lighting_options.y);
        if base.lighting_options.w > 0.5 {
            let origin = select(position.xyz, base.actor_origin.xyz, base.actor_origin.w > 0.5);
            let local = rw_object_local_lights(lighting, origin, normal);
            if local.w != 1.0 {
                light = prelight.rgb * has_prelight
                    + lighting.ambient.rgb * (base.surface.x * local.w)
                    + lighting.directional.rgb * (base.surface.y * diffuse_intensity * base.lighting_options.y * local.w);
            }
            light += local.rgb * base.surface.y;
        }
        // Native MatFX clamps light/prelight before material colour and
        // interpolates that vertex colour, rather than relighting each pixel.
        colour = vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)), prelight.a);
    }
    var output: PcEnvVertexOutput;
    output.position = gta_view.clip_from_world * position;
    output.diffuse_uv = source.uv;
    @if(BINDLESS)
    output.material_slot = material_slot;
    // CVector::to_bevy: GTA (x,y,z) -> Bevy (x,z,-y). Native matrix
    // (0.5*N.x+0.5, -0.5*N.y+0.5) therefore uses Bevy (x,z).
    output.environment_uv = normal.xz * vec2<f32>(0.5) + vec2<f32>(0.5);
    output.colour = colour * base.colour;
    let fog_start = lighting.fog_colour_and_start.w;
    let far_clip = lighting.light_direction_and_far_clip.w;
    let depth = -(gta_view.view_from_world * position).z;
    output.fog = 0.0;
    if far_clip > fog_start {
        output.fog = clamp((depth - fog_start) / (far_clip - fog_start), 0.0, 1.0);
    }
    return output;
}

@fragment
fn fragment(input: PcEnvVertexOutput) -> @location(0) vec4<f32> {
    @if(BINDLESS)
    let bindings = material_indices[input.material_slot];
    @if(BINDLESS)
    let params = material_data[bindings.data];
    @if(BINDLESS)
    let lighting = lighting_buffers[bindings.lighting];
    @if(BINDLESS)
    let diffuse_sample = textureSample(material_textures[bindings.diffuse_texture], material_samplers[bindings.diffuse_sampler], input.diffuse_uv);
    @if(BINDLESS)
    let environment_sample = textureSample(material_textures[bindings.environment_texture], material_samplers[bindings.environment_sampler], input.environment_uv);
    @if(!BINDLESS)
    let params = params_single;
    @if(!BINDLESS)
    let lighting = lighting_single;
    @if(!BINDLESS)
    let diffuse_sample = textureSample(diffuse, diffuse_sampler, input.diffuse_uv);
    @if(!BINDLESS)
    let environment_sample = textureSample(environment, environment_sampler, input.environment_uv);
    var pass1 = input.colour;
    if params.base.flags.x > 0.5 {
        pass1 *= vec4<f32>(linear_to_rw_rgb(diffuse_sample.rgb), diffuse_sample.a);
    }
    pass1 = apply_fog(pass1, lighting.fog_colour_and_start.rgb, input.fog);
    let reflection = linear_to_rw_rgb(environment_sample.rgb);
    // Game.cpp: applyLight/useMatColor/flipU=false; envMapColor=(128,128,128).
    // VehicleModelInfo.cpp: coefficient=surface specular*0.5; fbAlpha=false.
    // Thus pass2 is independent of prelight, material/diffuse colour and alpha.
    let pass2 = reflection * params.environment.x * params.environment.y * (1.0 - input.fog);
    if params.base.flags.y > 0.0 && pass1.a <= params.base.flags.y {
        discard;
    }
    // librw simulates the diffuse SRCALPHA/INVSRCALPHA pass followed by
    // reflection ONE/ONE in one draw: RGB=diffuse*alpha+env, alpha=diffuse.a.
    // The material pipeline uses ONE/INVSRCALPHA and retains vehicle Z write.
    return vec4<f32>(rw_target_rgb(pass1.rgb * pass1.a + pass2), pass1.a);
}
