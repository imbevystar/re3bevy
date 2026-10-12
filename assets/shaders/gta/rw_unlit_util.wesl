// Shared RenderWare frame state. Rust counterpart:
// RendererMaterials.rs::GtaGlobalLighting.
struct GtaGlobalLighting {
    fog_colour_and_start: vec4<f32>,
    ambient: vec4<f32>,
    actor_ambient: vec4<f32>,
    directional: vec4<f32>,
    light_direction_and_far_clip: vec4<f32>,
    postfx_blur_colour: vec4<f32>,
    local_light_positions: array<vec4<f32>, 32>,
    local_light_colours: array<vec4<f32>, 32>,
    local_light_directions: array<vec4<f32>, 32>,
    local_light_count: vec4<f32>,
}

// CPed/CVehicle::SetupLighting calls GenerateLightsAffectingObject before
// rendering. RenderWare uses up to four extra directionals from its 32 nearby
// registered lights. Bevy's PBR PointLight doesn't illuminate GTA WESL atomics,
// so evaluate their native fade/cone/darken rules on those materials here.
fn rw_object_local_lights(lighting: GtaGlobalLighting, position: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    var extra = vec3<f32>(0.0);
    var darken = 1.0;
    // Lights.cpp has four extra directional slots. Its strength storage is
    // integer-valued; preserve that admission/replacement order as well.
    var directions: array<vec3<f32>, 4>;
    var colours: array<vec3<f32>, 4>;
    var strengths: array<i32, 4>;
    var count = 0u;
    for (var i = 0u; i < u32(lighting.local_light_count.x); i = i + 1u) {
        let source = lighting.local_light_positions[i];
        let radius = source.w;
        let offset = source.xyz - position;
        let distance = length(offset);
        if radius <= 0.0 || distance >= radius {
            continue;
        }
        let ratio = distance / radius;
        let kind = lighting.local_light_colours[i].w;
        if kind > 1.5 && kind < 2.5 {
            darken *= ratio;
            continue;
        }
        if kind > 2.5 || distance <= 0.00001 {
            continue;
        }
        let direction = offset / distance;
        var strength = min(1.0, (1.0 - ratio) * 2.0);
        if kind > 0.5 {
            strength *= max((-dot(direction, lighting.local_light_directions[i].xyz) - 0.5) * 2.0, 0.0);
        }
        let colour = lighting.local_light_colours[i].rgb * strength;
        let score = max(max(colour.r, colour.g), colour.b);
        var slot = count;
        if count >= 4u {
            slot = 4u;
            var weakest = score;
            for (var j = 0u; j < 4u; j = j + 1u) {
                if f32(strengths[j]) < weakest {
                    weakest = f32(strengths[j]);
                    slot = j;
                }
            }
        }
        if strength > 0.0 && slot < 4u {
            directions[slot] = direction;
            colours[slot] = colour;
            strengths[slot] = i32(score);
            count = min(count + 1u, 4u);
        }
    }
    for (var j = 0u; j < count; j = j + 1u) {
        extra += colours[j] * max(dot(normal, directions[j]), 0.0);
    }
    return vec4<f32>(extra, darken);
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

// GTA's D3D8 camera raster stores and blends encoded channel values.
// The normal Bevy attachment still expects linear fragment output.
fn rw_target_rgb(encoded: vec3<f32>) -> vec3<f32> {
    @if(GTA_ENCODED_TARGET) {
        return encoded;
    }
    @if(!GTA_ENCODED_TARGET) {
        return rw_to_linear_rgb(encoded);
    }
}
