// re3 extras/postfx.cpp, shaders/colourfilterIII_PS.hlsl and contrastPS.hlsl.
// The source is the completed camera colour buffer, never an individual material.
import super::rw_unlit_util::{linear_to_rw_rgb, rw_to_linear_rgb};

struct PostFxSettings {
    colour: vec4<f32>,
    contrast_mult: vec4<f32>,
    contrast_add_and_mode: vec4<f32>,
    view_info: vec4<f32>,
}

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: PostFxSettings;

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(scene));
    let uv = position.xy / size;
    let source = textureSample(scene, scene_sampler, uv);
    // All postprocess writes cover the complete render target to preserve the
    // other camera(s) while ViewTarget ping-pongs between colour attachments.
    // Only the current camera's physical viewport receives a colour filter.
    let viewport = settings.view_info;
    if (position.x < viewport.x || position.y < viewport.y ||
        position.x >= viewport.x + viewport.z || position.y >= viewport.y + viewport.w) {
        return source;
    }
    var rgb = source.rgb;
    @if(!GTA_ENCODED_TARGET) {
        rgb = linear_to_rw_rgb(rgb);
    }
    let mode = settings.contrast_add_and_mode.w;

    // POSTFX_NORMAL: colourfilterIII_PS. Saturation is per iteration, matching
    // the original encoded RenderWare camera raster arithmetic.
    if mode > 0.5 && mode < 1.5 {
        let a = clamp(settings.colour.a, 0.0, 1.0);
        var prev = vec4<f32>(rgb, source.a);
        let dst = prev;
        for (var i = 0; i < 5; i = i + 1) {
            prev = clamp(dst * (1.0 - a) + prev * settings.colour * a,
                         vec4<f32>(0.0), vec4<f32>(1.0));
        }
        rgb = prev.rgb;
    }
    // POSTFX_MOBILE: contrastPS. The multipliers/addends originate in postfx.cpp.
    if mode > 1.5 && mode < 2.5 {
        rgb = rgb * settings.contrast_mult.rgb + settings.contrast_add_and_mode.rgb;
    }
    // POSTFX_SIMPLE: RenderOverlaySimple uses SRCALPHA/INVSRCALPHA.
    if mode > 2.5 && mode < 3.5 {
        let a = settings.colour.a * 0.6;
        rgb = mix(rgb, settings.colour.rgb * 0.6, a);
    }
    @if(!GTA_ENCODED_TARGET) {
        rgb = rw_to_linear_rgb(rgb);
    }
    return vec4<f32>(rgb, 1.0);
}
