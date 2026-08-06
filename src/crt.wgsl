#import bevy_pbr::forward_io::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var frame_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var frame_sampler: sampler;

const TAU: f32 = 6.28318530718;
const SCANLINES: f32 = 200.0;
const SCAN_BASE: f32 = 0.82;
const SCAN_DEPTH: f32 = 0.18;
const VIGNETTE_STRENGTH: f32 = 0.6;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(frame_texture, frame_sampler, in.uv);
    let scan = SCAN_BASE + SCAN_DEPTH * cos(in.uv.y * SCANLINES * TAU);
    let centered = in.uv - vec2<f32>(0.5, 0.5);
    let vignette = 1.0 - VIGNETTE_STRENGTH * dot(centered, centered);
    return vec4<f32>(color.rgb * scan * vignette, 1.0);
}
