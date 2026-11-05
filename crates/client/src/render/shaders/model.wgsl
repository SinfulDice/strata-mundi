// Entity models (boats): camera-relative vertices built on the CPU every frame, lit per
// face like blocks and fogged the same way.
struct Camera {
    view_proj: mat4x4<f32>,
    // y, z: fog start and end, in blocks from the camera; w: 1 with fog, 0 without.
    options: vec4<f32>,
    sky: vec4<f32>,
    // z: 1 when the fog counts the full distance (eyes in a liquid).
    effects: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var block_textures: texture_2d_array<f32>;
@group(1) @binding(1) var block_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) shade: f32,
    @location(3) relative: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) layer: u32,
    @location(3) shade: f32,
) -> VertexOutput {
    var out: VertexOutput;
    out.relative = position;
    out.clip = camera.view_proj * vec4<f32>(position, 1.0);
    out.uv = uv;
    out.layer = layer;
    out.shade = shade;
    return out;
}

fn fogged(color: vec3<f32>, relative: vec3<f32>) -> vec3<f32> {
    let distance = mix(length(relative.xz), length(relative), camera.effects.z);
    let amount = clamp((distance - camera.options.y) / max(camera.options.z - camera.options.y, 1.0), 0.0, 1.0);
    return mix(color, camera.sky.rgb, amount * amount * camera.options.w);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, input.uv, input.layer);
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(fogged(texel.rgb * input.shade, input.relative), 1.0);
}
