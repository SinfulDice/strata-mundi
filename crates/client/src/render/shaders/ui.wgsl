struct Screen {
    // xy: framebuffer size in pixels, zw: atlas size in pixels.
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

// Vertex colors are written in sRGB like the theme; blending happens in linear space.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    let ndc = position / screen.size.xy * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv / screen.size.zw;
    out.color = vec4<f32>(to_linear(color.rgb), color.a);
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(atlas, atlas_sampler, input.uv) * input.color;
}
