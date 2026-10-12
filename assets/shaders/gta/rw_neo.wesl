// re3 extras/custompipes shaders: default_UV2_VS, neoVehicle_VS/PS,
// neoWorldIII_PS, neoGloss_VS/PS, neoRim_VS and neoRimSkin_VS.
// Bevy's mesh vertex stage replaces the original matrix/skinning boilerplate;
// decoded DFF UV1, world position, transformed normals and vertex colours are
// carried by forward_io::VertexOutput. Both static and CPU-skinned peds share
// the rim shading path after their meshes have been transformed.
import bevy_render::view::View;
import bevy_pbr::render::forward_io::VertexOutput;
import super::rw_unlit_util::{GtaGlobalLighting, apply_fog, linear_to_rw_rgb, rw_target_rgb, rw_object_local_lights};

@group(0) @binding(0) var<uniform> gta_view: View;

struct NeoParams {
    colour: vec4<f32>,
    surface: vec4<f32>,         // ambient, diffuse, specular, mode
    prelight: vec4<f32>,        // x: prelight mode, y: alpha cutoff
    reflection: vec4<f32>,      // fresnel, light strength, shininess, specularity
    spec_color_and_power: vec4<f32>,
    lightmap: vec4<f32>,        // lightmap modulation RGBA
    rim_start: vec4<f32>,
    rim_end: vec4<f32>,
    rim_data: vec4<f32>,        // offset, scale, scaling, gloss multiplier
    actor_origin: vec4<f32>,
}

@group(3) @binding(0) var<uniform> params: NeoParams;
@group(3) @binding(1) var diffuse: texture_2d<f32>;
@group(3) @binding(2) var diffuse_sampler: sampler;
@group(3) @binding(3) var reflection_tex: texture_2d<f32>;
@group(3) @binding(4) var reflection_sampler: sampler;
@group(3) @binding(5) var lightmap_tex: texture_2d<f32>;
@group(3) @binding(6) var lightmap_sampler: sampler;
@group(3) @binding(7) var gloss_tex: texture_2d<f32>;
@group(3) @binding(8) var gloss_sampler: sampler;
@group(3) @binding(9) var<storage, read> lighting: GtaGlobalLighting;

fn safe_normal(value: vec3<f32>) -> vec3<f32> {
    return normalize(select(vec3<f32>(0.0, 0.0, 1.0), value,
                            dot(value, value) > 0.000001));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let mode = params.surface.w;
    let normal = safe_normal(mesh.world_normal);
    let eye = safe_normal(gta_view.world_position - mesh.world_position.xyz);
    let light = safe_normal(lighting.light_direction_and_far_clip.xyz);
    let t0 = textureSample(diffuse, diffuse_sampler, mesh.uv);
    let tex = linear_to_rw_rgb(t0.rgb);

    @if(VERTEX_COLORS)
    let vertex_colour = mesh.color;
    @if(!VERTEX_COLORS)
    let vertex_colour = vec4<f32>(1.0);
    let diffuse_intensity = max(dot(normal, light), 0.0);
    // re3/RenderWare: prelight + ambient + directional, clamp first, then
    // multiply material colour (default_UV2_VS).
    var ambient = lighting.ambient.rgb;
    if mode < 0.5 || (mode > 2.5 && mode < 3.5) {
        ambient = lighting.actor_ambient.rgb / 1.3;
    }
    var lit = ambient * params.surface.x
            + lighting.directional.rgb * (params.surface.y * diffuse_intensity);
    if mode < 0.5 {
        lit *= params.reflection.y;
    }
    if params.prelight.x > 0.5 {
        lit += vertex_colour.rgb;
    }
    // Neo vehicle and ped rim passes share the live re3 local light list.
    // World lightmaps and gloss overlays do not receive actor lights.
    if mode < 0.5 || (mode > 2.5 && mode < 3.5) {
        let origin = select(mesh.world_position.xyz, params.actor_origin.xyz, params.actor_origin.w > 0.5);
        let local = rw_object_local_lights(lighting, origin, normal);
        lit = lit * local.w + local.rgb * params.surface.y;
    }
    lit = clamp(lit, vec3<f32>(0.0), vec3<f32>(1.0));

    // Neo Rim (static + skinned): lerp(rampEnd, rampStart,
    // offset - scale * dot(normal, view)) and clamp after scaling.
    if mode > 2.5 && mode < 3.5 {
        let f = params.rim_data.x - params.rim_data.y * dot(normal, eye);
        let rim = clamp(mix(params.rim_end, params.rim_start, f) * params.rim_data.z,
                        vec4<f32>(0.0), vec4<f32>(1.0));
        lit = clamp(lit + rim.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    var colour = vec4<f32>(
        lit * params.colour.rgb * tex,
        params.colour.a * t0.a * vertex_colour.a,
    );

    let fog_start = lighting.fog_colour_and_start.w;
    let fog_end = lighting.light_direction_and_far_clip.w;
    let depth = -(gta_view.view_from_world * mesh.world_position).z;
    let fog = clamp((depth - fog_start) / max(fog_end - fog_start, 0.00001), 0.0, 1.0);

    // Neo Vehicle: reflected eye vector in the XY environment map with a
    // fifth-power Fresnel term and specular highlight from the active sun.
    if mode < 0.5 {
        let reflection_dir = 2.0 * dot(eye, normal) * normal - eye;
        let reflection_uv = reflection_dir.xy * 0.5 + vec2<f32>(0.5);
        let environment = linear_to_rw_rgb(textureSample(reflection_tex, reflection_sampler,
                                                         reflection_uv).rgb);
        let edge = 1.0 - clamp(dot(eye, normal), 0.0, 1.0);
        let fresnel = mix(pow(edge, 5.0), 1.0, params.reflection.x)
                      * params.reflection.z;
        let reflected = mix(colour.rgb, environment, clamp(fresnel, 0.0, 1.0));
        let halfway = safe_normal(eye + light);
        let specular = pow(max(dot(normal, halfway), 0.0),
                           max(params.spec_color_and_power.w, 1.0));
        let highlight = params.spec_color_and_power.rgb * specular
                        * params.reflection.y * params.reflection.w;
        // neoVehicle_PS: fog the diffuse+environment pass first, multiply its
        // RGB by alpha, and add the separately fog-attenuated specular pass.
        let fogged = apply_fog(vec4<f32>(reflected, colour.a),
                              lighting.fog_colour_and_start.rgb, fog);
        colour = vec4<f32>(fogged.rgb * colour.a + highlight * (1.0 - fog), colour.a);
        if params.prelight.y > 0.0 && colour.a <= params.prelight.y {
            discard;
        }
        return vec4<f32>(rw_target_rgb(colour.rgb), colour.a);
    }

    // Neo World: lightmap is sampled at UV1; no fabricated lightmap is used
    // for models without a second authored UV channel.
    if mode > 0.5 && mode < 1.5 {
        @if(VERTEX_UVS_B)
        let light_uv = mesh.uv_b;
        @if(!VERTEX_UVS_B)
        let light_uv = mesh.uv;
        let light_sample = linear_to_rw_rgb(textureSample(lightmap_tex, lightmap_sampler, light_uv).rgb);
        colour = vec4<f32>(
            colour.rgb * (vec3<f32>(1.0) + params.lightmap.rgb * (2.0 * light_sample - vec3<f32>(1.0))),
            colour.a,
        );
        colour = vec4<f32>(colour.rgb, colour.a * params.lightmap.a);
    }

    // Neo Gloss is rendered as an additive overlay, using the source's
    // eighth-power cosine response and the authored _gloss raster.
    if mode > 1.5 && mode < 2.5 {
        let gloss_light = safe_normal(eye - light);
        let s = dot(vec3<f32>(0.0, 0.0, 1.0), gloss_light);
        let gloss_sample = textureSample(gloss_tex, gloss_sampler, mesh.uv);
        colour = vec4<f32>(linear_to_rw_rgb(gloss_sample.rgb) * pow(s, 8.0)
                             * (1.0 - fog) * params.rim_data.w,
                            gloss_sample.a);
    } else {
        colour = apply_fog(colour, lighting.fog_colour_and_start.rgb, fog);
    }

    if params.prelight.y > 0.0 && colour.a <= params.prelight.y {
        discard;
    }
    return vec4<f32>(rw_target_rgb(colour.rgb), colour.a);
}
