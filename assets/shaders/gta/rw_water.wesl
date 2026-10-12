import bevy_render::view::View;
import bevy_pbr::render::forward_io::VertexOutput;
import super::rw_unlit_util::{GtaGlobalLighting, apply_fog, linear_to_rw_rgb, rw_target_rgb};

@group(0) @binding(0) var<uniform> gta_view: View;

@group(3) @binding(0) var<uniform> colour: vec4<f32>;
@group(3) @binding(1) var<storage, read> global_lighting: GtaGlobalLighting;
@group(3) @binding(2) var<uniform> postfx_blur_colour: vec4<f32>;
@group(3) @binding(3) var water_texture: texture_2d<f32>;
@group(3) @binding(4) var water_sampler: sampler;
@group(3) @binding(5) var<uniform> uv_offset: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(water_texture, water_sampler, mesh.uv + uv_offset.xy);
    let texture_rw = linear_to_rw_rgb(tex.rgb);
    // RenderOneWavySector supplies original RW prelight, including boat wakes.
    // Flat sectors use the frame colour; uv_offset.z selects the vertex stream.
    @if(VERTEX_COLORS)
    let prelight = mix(colour.rgb, mesh.color.rgb, uv_offset.z);
    @if(!VERTEX_COLORS)
    let prelight = colour.rgb;
    let water_colour = vec4<f32>(prelight * texture_rw, colour.a * tex.a);
    let fog_start = global_lighting.fog_colour_and_start.w;
    let far_clip = global_lighting.light_direction_and_far_clip.w;
    let view_depth = -(gta_view.view_from_world * mesh.world_position).z;
    let f = clamp((view_depth - fog_start) / max(far_clip - fog_start, 0.00001), 0.0, 1.0);
    let fogged = apply_fog(water_colour, global_lighting.fog_colour_and_start.rgb, f);
    return vec4<f32>(rw_target_rgb(fogged.rgb), fogged.a);
}
