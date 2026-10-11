// Shared RenderWare frame state. Rust counterpart:
// RendererMaterials.rs::GtaGlobalLighting.
struct GtaGlobalLighting {
    fog_colour_and_start: vec4<f32>,
    ambient: vec4<f32>,
    directional: vec4<f32>,
    light_direction_and_far_clip: vec4<f32>,
    postfx_blur_colour: vec4<f32>,
}

fn rw_decode_channel(value: f32) -> f32 {
    if value <= 0.04045 {
        return value / 12.92;
    }
    return pow((value + 0.055) / 1.055, 2.4);
}

fn rw_encode_channel(value: f32) -> f32 {
    let clamped_value = max(value, 0.0);
    if clamped_value <= 0.0031308 {
        return clamped_value * 12.92;
    }
    return 1.055 * pow(clamped_value, 1.0 / 2.4) - 0.055;
}

/// Bevy samples Rgba8UnormSrgb into linear values; D3D8 RenderWare performed
/// material/light/fog modulation on the encoded 0..255 channel values.
fn linear_to_rw_rgb(value: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        rw_encode_channel(value.r),
        rw_encode_channel(value.g),
        rw_encode_channel(value.b),
    );
}

fn rw_to_linear_rgb(value: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        rw_decode_channel(value.r),
        rw_decode_channel(value.g),
        rw_decode_channel(value.b),
    );
}

fn apply_vertex_colour(base: vec4<f32>, vertex_colour: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(base.rgb * vertex_colour.rgb, base.a * vertex_colour.a);
}

fn apply_fog(colour: vec4<f32>, fog_colour: vec3<f32>, fog_factor: f32) -> vec4<f32> {
    let f = clamp(fog_factor, 0.0, 1.0);
    return vec4<f32>(mix(colour.rgb, fog_colour, f), colour.a);
}

fn apply_colour_filter_iii(colour: vec4<f32>, blur_colour: vec4<f32>) -> vec4<f32> {
    let a = clamp(blur_colour.a, 0.0, 1.0);
    let dst = colour;
    var prev = dst;
    for (var i = 0; i < 5; i = i + 1) {
        let tmp = dst * (1.0 - a) + prev * blur_colour * a;
        prev = clamp(tmp, vec4<f32>(0.0), vec4<f32>(1.0));
    }
    return vec4<f32>(prev.rgb, colour.a);
}
