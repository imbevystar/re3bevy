import bevy_pbr::render::forward_io::VertexOutput;
import super::rw_unlit_util::rw_target_rgb;

@group(3) @binding(0) var<uniform> sky_top: vec4<f32>;
@group(3) @binding(1) var<uniform> sky_bottom: vec4<f32>;
@group(3) @binding(2) var<uniform> sky_horizon: vec4<f32>;
@group(3) @binding(3) var<uniform> postfx_blur_colour: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // CClouds::RenderBackground + RenderHorizon. Mesh UV Y runs bottom->top,
    // while re3's screen Y runs top->bottom.
    let screen_y = 1.0 - mesh.uv.y;
    var background = sky_top;
    if sky_horizon.z > 0.5 {
        background = vec4<f32>(vec3<f32>(50.0 / 255.0), 1.0);
    } else {
        // RenderBackground starts from the camera clear colour (sky_top),
        // then paints a half-screen-height top->bottom gradient ending at the
        // projected horizon.
        if screen_y < sky_horizon.x {
            let top_mix = clamp((sky_horizon.x - screen_y) / 0.5, 0.0, 1.0);
            background = mix(sky_bottom, sky_top, top_mix);
        }

        // RenderHorizon then overwrites the band below the horizon with the
        // exact fog colour. Its second DrawRect has vertex alpha disabled, so
        // this region is intentionally opaque in the original PC renderer.
        if screen_y >= sky_horizon.x && screen_y < sky_horizon.y {
            let fog_rgb = (sky_top.rgb + 2.0 * sky_bottom.rgb) / 3.0;
            background = vec4<f32>(fog_rgb, 1.0);
        }
    }

    // CClouds feeds 0..255 RenderWare/D3D8 channel values directly to Im2D.
    return vec4<f32>(rw_target_rgb(background.rgb), background.a);
}
