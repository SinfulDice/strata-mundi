struct Camera {
    view_proj: mat4x4<f32>,
    // x: ambient occlusion strength (0 = off, 1 = full).
    options: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var block_textures: texture_2d_array<f32>;
@group(1) @binding(1) var block_sampler: sampler;

// Section origin relative to the camera (camera-relative rendering), set per draw.
var<immediate> section_offset: vec4<f32>;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) shade: f32,
};

// Per face (up, down, north, south, east, west), lit from above like Minecraft.
fn face_shade(face: u32) -> f32 {
    var shades = array<f32, 6>(1.0, 0.5, 0.8, 0.8, 0.6, 0.6);
    return shades[face];
}

fn ao_light(ao: u32) -> f32 {
    var levels = array<f32, 4>(0.4, 0.6, 0.8, 1.0);
    return mix(1.0, levels[ao], camera.options.x);
}

@vertex
fn vs_main(@location(0) geometry: u32, @location(1) texture: u32) -> VertexOutput {
    let position = vec3<f32>(
        f32(geometry & 31u),
        f32((geometry >> 5u) & 31u),
        f32((geometry >> 10u) & 31u),
    );
    let face = (geometry >> 15u) & 7u;
    let ao = (geometry >> 18u) & 3u;
    var out: VertexOutput;
    out.clip = camera.view_proj * vec4<f32>(position + section_offset.xyz, 1.0);
    out.uv = vec2<f32>(f32((geometry >> 20u) & 31u), f32((geometry >> 25u) & 31u));
    out.layer = texture & 0xffffu;
    out.shade = face_shade(face) * ao_light(ao);
    return out;
}

// Opaque and cutout blocks: pixels are either kept or dropped (leaves, glass frames).
@fragment
fn fs_cutout(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, input.uv, input.layer);
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(texel.rgb * input.shade, 1.0);
}

// Translucent blocks (water, ice): blended, never discarded.
@fragment
fn fs_translucent(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, input.uv, input.layer);
    return vec4<f32>(texel.rgb * input.shade, texel.a);
}
