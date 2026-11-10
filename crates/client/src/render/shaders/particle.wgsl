// Particles: one camera-facing square per instance, a piece of a block texture, lit by
// the light of its cell and fogged like blocks.
struct Camera {
    view_proj: mat4x4<f32>,
    // y, z: fog start and end, in blocks from the camera; w: 1 with fog, 0 without.
    options: vec4<f32>,
    sky: vec4<f32>,
    // z: 1 when the fog counts the full distance (eyes in a liquid).
    effects: vec4<f32>,
    // x: sky light factor (1 by day); y: brightness (0-1); z: light of the darkest places.
    light: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var block_textures: texture_2d_array<f32>;
@group(1) @binding(1) var block_sampler: sampler;

// Camera right and up, in the world.
struct Billboard {
    right: vec4<f32>,
    up: vec4<f32>,
};

var<immediate> billboard: Billboard;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) @interpolate(flat) color: vec3<f32>,
    @location(3) relative: vec3<f32>,
    @location(4) @interpolate(flat) light: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) corner: u32,
    @location(0) centre: vec3<f32>,
    @location(1) size: f32,
    @location(2) color: vec3<f32>,
    @location(3) layer: u32,
    @location(4) piece: vec3<f32>,
    @location(5) light: vec2<f32>,
) -> VertexOutput {
    let x = f32(corner & 1u);
    let y = f32(corner >> 1u);
    let spread = billboard.right.xyz * (x - 0.5) + billboard.up.xyz * (y - 0.5);
    let position = centre + spread * size;
    var out: VertexOutput;
    out.clip = camera.view_proj * vec4<f32>(position, 1.0);
    out.uv = piece.xy + vec2<f32>(x, 1.0 - y) * piece.z;
    out.layer = layer;
    out.color = color;
    out.relative = position;
    out.light = light;
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
    let color = texel.rgb * input.color * light_color(input.light.x, input.light.y);
    return vec4<f32>(fogged(color, input.relative), 1.0);
}
