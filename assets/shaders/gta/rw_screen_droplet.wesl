// re3 extras/screendroplets.cpp + screenDroplet_PS.hlsl.
// The original second sampler is a copy of the camera raster. The first is a
// generated white circular mask, evaluated analytically to avoid a texture.
import super::rw_unlit_util::{linear_to_rw_rgb, rw_to_linear_rgb};

@group(0) @binding(0) var captured_scene: texture_2d<f32>;
@group(0) @binding(1) var captured_sampler: sampler;

struct DropletVertex {
    @location(0) position: vec2<f32>,
    @location(1) mask_uv: vec2<f32>,
    @location(2) capture_uv: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct DropletOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) mask_uv: vec2<f32>,
    @location(1) capture_uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

@vertex
fn droplet_vertex(input: DropletVertex) -> DropletOutput {
    var output: DropletOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.mask_uv = input.mask_uv;
    output.capture_uv = input.capture_uv;
    output.color = input.color;
    return output;
}

@fragment
fn copy_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = position.xy / vec2<f32>(textureDimensions(captured_scene));
    return textureSample(captured_scene, captured_sampler, uv);
}

@fragment
fn droplet_fragment(input: DropletOutput) -> @location(0) vec4<f32> {
    let mask_position = input.mask_uv * 2.0 - vec2<f32>(1.0);
    if dot(mask_position, mask_position) >= 1.0 {
        discard;
    }
    let sample_color = textureSample(captured_scene, captured_sampler, input.capture_uv);
    // HLSL: input.Color * mask_tex * screen_tex (D3D8 encoded RGB).
    var rgb = sample_color.rgb;
    @if(!GTA_ENCODED_TARGET) {
        rgb = linear_to_rw_rgb(rgb);
    }
    rgb *= input.color.rgb;
    @if(!GTA_ENCODED_TARGET) {
        rgb = rw_to_linear_rgb(rgb);
    }
    return vec4<f32>(rgb, sample_color.a * input.color.a);
}
