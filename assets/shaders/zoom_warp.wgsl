#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct ZoomWarpUniform {
    progress: f32,
}
@group(0) @binding(2) var<uniform> settings: ZoomWarpUniform;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let offset = in.uv - center;
    let dist_factor = length(offset) * 2.0;
    let t = pow(settings.progress, 2);

    let zoom = pow(1.0 + t * 2.5, 1.0 + dist_factor * 1.5);

    let warped_uv = center + offset / zoom;
    return textureSample(screen_texture, texture_sampler, warped_uv);
}
