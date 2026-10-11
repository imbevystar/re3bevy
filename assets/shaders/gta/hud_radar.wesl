// CRadar::DrawRadarSection / DrawRadarMask. Original streamed TXD raster;
// no CPU map composite or invented map imagery.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
struct RadarFrame { origins: array<vec4<f32>, 2>, headings: array<vec4<f32>, 2> }
@group(1) @binding(0) var<uniform> tile: vec4<f32>;
@group(1) @binding(1) var<uniform> sample_origin: vec4<f32>;
@group(1) @binding(2) var tile_texture: texture_2d<f32>;
@group(1) @binding(3) var tile_sampler: sampler;
@group(1) @binding(4) var<storage, read> frame: RadarFrame;

@fragment
fn fragment(input: UiVertexOutput) -> @location(0) vec4<f32> {
    let r = vec2<f32>(input.uv.x * 2.0 - 1.0, 1.0 - input.uv.y * 2.0);
    // Native mask: six segments per quadrant, rather than a smooth circle.
    let step = 0.2617993877991494;
    let angle = atan2(r.y, r.x);
    let middle = (floor(angle / step) + 0.5) * step;
    if dot(r, vec2<f32>(cos(middle), sin(middle))) > cos(step * 0.5) { discard; }
    let player = u32(tile.w);
    let origin = frame.origins[player];
    let h = frame.headings[player];
    let world = origin.xy + vec2<f32>(h.x * r.x - h.y * r.y, h.y * r.x + h.x * r.y) * origin.z;
    let local = (world - tile.xy) / tile.z;
    // Non-overlapping cells reproduce ClipRadarPoly without per-frame meshes.
    if any(local < vec2<f32>(0.0)) || any(local >= vec2<f32>(1.0)) { discard; }
    let uv = (world - sample_origin.xy) / tile.z;
    return textureSampleLevel(tile_texture, tile_sampler, vec2<f32>(uv.x, 1.0 - uv.y), 0.0);
}
