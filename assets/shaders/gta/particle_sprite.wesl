// re3 reference: references/re-GTA/src/renderer/Sprite.cpp
// re3 reference: references/re-GTA/src/renderer/Particle.cpp (CParticle::Render)
//
// Screen-space particle sprites. Vertex positions arrive pre-projected in NDC
// (x/y) plus a wgpu depth value in z, replicating the RwIm2D pre-transformed
// vertex stream. Blend state (rwBLENDSRCALPHA/INVSRCALPHA, BLENDONE/BLENDONE,
// SRCALPHA/BLENDONE) lives in the pipeline, matching CParticle::Render's
// DRAW_OPAQUE / DRAW_DARK / additive switch.
import super::rw_unlit_util::{linear_to_rw_rgb, rw_to_linear_rgb};

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // Depth test runs against the scene depth buffer with writes disabled,
    // like rwRENDERSTATEZWRITEENABLE=FALSE + rwRENDERSTATEZTESTENABLE=TRUE.
    output.position = vec4<f32>(input.position.xy, input.position.z, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite_texture, sprite_sampler, input.uv);
    // DefinedState / SetAlphaRef(2): alpha test follows texture/vertex
    // modulation (D3D8 GREATER 2, librw GREATEREQUAL 3).
    let alpha = texel.a * input.color.a;
    if alpha < 3.0 / 255.0 {
        discard;
    }
    // D3D8 modulated material*texture*vertex colour on encoded channel values;
    // an encoded compositing target also adds these values directly. Decoding
    // before ONE/ONE blending loses the bright yellow core of overlapping fire.
    let tex_rw = linear_to_rw_rgb(texel.rgb);
    let rgb_rw = tex_rw * input.color.rgb;
    @if(PARTICLE_ENCODED_TARGET) {
        return vec4<f32>(rgb_rw, alpha);
    } @else {
        return vec4<f32>(rw_to_linear_rgb(rgb_rw), alpha);
    }
}
