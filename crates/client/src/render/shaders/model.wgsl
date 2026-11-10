// Entity models (boats): camera-relative vertices built on the CPU every frame, lit per
// face like blocks, by the light of their cell, and fogged the same way.

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) shade: f32,
    @location(3) relative: vec3<f32>,
    @location(4) @interpolate(flat) light: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) layer: u32,
    @location(3) shade: f32,
    @location(4) light: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.relative = position;
    out.clip = camera.view_proj * vec4<f32>(position, 1.0);
    out.uv = uv;
    out.layer = layer;
    out.shade = shade;
    out.light = light;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, input.uv, input.layer);
    if texel.a < 0.5 {
        discard;
    }
    let color = texel.rgb * input.shade * light_color(input.light.x, input.light.y);
    return vec4<f32>(fogged(color, input.relative), 1.0);
}
