// Reference-derived GTA III PC MatFX adapter. re3 VehicleModelInfo.cpp and
// Game.cpp; librw 5501c4fdc7425ff926be59369a13593bb6c81b54's
// matfx_env.vert/.frag and matfx_env_VS.hlsl/matfx_env_PS.hlsl.
// GTA's identity env frame uses normal XY, not Neo's eye/Fresnel reflection.
import bevy_render::view::View;
import bevy_pbr::render::forward_io::Vertex;
import bevy_pbr::render::mesh_types::Mesh;
import bevy_render::maths::affine3_to_square;
import super::rw_unlit_util::{GtaGlobalLighting, apply_fog, linear_to_rw_rgb, rw_target_rgb, rw_object_local_lights};

@group(0) @binding(0) var<uniform> gta_view: View;
@group(2) @binding(0) var<storage, read> meshes: array<Mesh>;

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
@group(3) @binding(0) var<uniform> params: PcEnvParams;
@group(3) @binding(1) var diffuse: texture_2d<f32>;
@group(3) @binding(2) var diffuse_sampler: sampler;
@group(3) @binding(3) var environment: texture_2d<f32>;
@group(3) @binding(4) var environment_sampler: sampler;
@group(3) @binding(5) var<storage, read> lighting: GtaGlobalLighting;

struct PcEnvVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) diffuse_uv: vec2<f32>,
    @location(1) environment_uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) fog: f32,
}

@vertex
fn vertex(input: Vertex) -> PcEnvVertexOutput {
    // ModelDrawables owns CPU-skinned/frame-separated, uncompressed DFF
    // attributes. Use Bevy's vertex interface and resident instance transform.
    let world = affine3_to_square(meshes[input.instance_index].world_from_local);
    let position = world * vec4<f32>(input.position, 1.0);
    // librw maps the transformed normal directly, with no per-fragment eye
    // vector or normalization. Zero normals consequently map to the centre.
    let normal = (world * vec4<f32>(input.normal, 0.0)).xyz;
    @if(VERTEX_COLORS)
    let prelight = input.color;
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
    output.diffuse_uv = input.uv;
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
    let diffuse_sample = textureSample(diffuse, diffuse_sampler, input.diffuse_uv);
    var pass1 = input.colour;
    if params.base.flags.x > 0.5 {
        pass1 *= vec4<f32>(linear_to_rw_rgb(diffuse_sample.rgb), diffuse_sample.a);
    }
    pass1 = apply_fog(pass1, lighting.fog_colour_and_start.rgb, input.fog);
    let reflection = linear_to_rw_rgb(textureSample(environment, environment_sampler, input.environment_uv).rgb);
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
