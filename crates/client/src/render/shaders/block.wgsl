struct Camera {
    view_proj: mat4x4<f32>,
    // x: ambient occlusion strength (0 = off, 1 = full); y, z: fog start and end, in
    // blocks from the camera; w: 1 with fog, 0 without.
    options: vec4<f32>,
    // Sky color (linear), which the fog fades to (the liquid's color with the eyes in it).
    sky: vec4<f32>,
    // x: seconds, wrapped; y: 1 with animated textures, 0 without; z: 1 when the fog
    // counts the full distance (eyes in a liquid), 0 for the horizontal distance only.
    effects: vec4<f32>,
    // x: sky light factor (1 by day); y: brightness (0-1); z: light of the darkest places.
    light: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var block_textures: texture_2d_array<f32>;
@group(1) @binding(1) var block_sampler: sampler;
// Per layer: first_extra:16 frames:8 frame_time:8 (ticks); 0 for still layers.
@group(1) @binding(2) var<storage, read> animations: array<u32>;

// Section origin relative to the camera (camera-relative rendering), set per draw.
var<immediate> section_offset: vec4<f32>;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) layer: u32,
    @location(2) shade: f32,
    @location(3) relative: vec3<f32>,
    // Ripple strength of liquid surfaces, 0 elsewhere.
    @location(4) @interpolate(flat) wave: f32,
    // Sky and block light, 0 to 1, blended between corners.
    @location(5) light: vec2<f32>,
};

// Per face (up, down, north, south, east, west), lit from above like Minecraft; 6 is the
// flat, even light of plant planes.
fn face_shade(face: u32) -> f32 {
    var shades = array<f32, 7>(1.0, 0.5, 0.8, 0.8, 0.6, 0.6, 0.9);
    return shades[face];
}

fn ao_light(ao: u32) -> f32 {
    var levels = array<f32, 4>(0.4, 0.6, 0.8, 1.0);
    return mix(1.0, levels[ao], camera.options.x);
}

// Animation speed of each liquid surface code: 0 still, 1 water, 2 lava.
fn surface_speed(animation: u32) -> f32 {
    var speeds = array<f32, 4>(0.0, 1.0, 0.25, 0.0);
    return speeds[animation] * camera.effects.y;
}

// Layer showing now: image `(ticks / frame_time) % frames`, image 0 being the texture's
// own layer and the others following `first_extra` (as `TextureLayout::frame_at`).
fn animated_layer(layer: u32) -> u32 {
    let entry = animations[min(layer, arrayLength(&animations) - 1u)];
    let frames = (entry >> 16u) & 0xffu;
    if frames < 2u || camera.effects.y == 0.0 {
        return layer;
    }
    let ticks = u32(camera.effects.x * 20.0);
    let frame = (ticks / max(entry >> 24u, 1u)) % frames;
    if frame == 0u {
        return layer;
    }
    return (entry & 0xffffu) + frame - 1u;
}

@vertex
fn vs_main(@location(0) geometry: u32, @location(1) texture: u32) -> VertexOutput {
    // Liquid surfaces are lowered by `drop` sixteenths; sides keep their texture scale.
    let drop = f32((texture >> 16u) & 15u) / 16.0;
    let position = vec3<f32>(
        f32(geometry & 31u),
        f32((geometry >> 5u) & 31u) - drop,
        f32((geometry >> 10u) & 31u),
    );
    let face = (geometry >> 15u) & 7u;
    let ao = (geometry >> 18u) & 3u;
    let speed = surface_speed((texture >> 20u) & 3u);
    let time = camera.effects.x;
    var uv = vec2<f32>(f32((geometry >> 20u) & 31u), f32((geometry >> 25u) & 31u));
    if face == 0u {
        uv += vec2<f32>(0.07, 0.11) * time * speed;
    } else if face >= 2u && face <= 5u {
        uv.y += drop - 0.8 * time * speed;
    }
    var out: VertexOutput;
    out.relative = position + section_offset.xyz;
    out.clip = camera.view_proj * vec4<f32>(out.relative, 1.0);
    out.uv = uv;
    out.layer = animated_layer(texture & 0xffffu);
    out.shade = face_shade(face) * ao_light(ao);
    out.wave = speed;
    out.light = vec2<f32>(f32(texture >> 28u), f32((texture >> 24u) & 15u)) / 15.0;
    return out;
}

fn rippled(uv: vec2<f32>, wave: f32) -> vec2<f32> {
    if wave == 0.0 {
        return uv;
    }
    let phase = camera.effects.x * 1.7 * wave;
    return uv + vec2<f32>(sin(uv.y * 12.566 + phase), cos(uv.x * 12.566 + phase)) * 0.02;
}

// Far terrain fades into the sky, hiding where the loaded world stops.
fn fogged(color: vec3<f32>, relative: vec3<f32>) -> vec3<f32> {
    let distance = mix(length(relative.xz), length(relative), camera.effects.z);
    let amount = clamp((distance - camera.options.y) / max(camera.options.z - camera.options.y, 1.0), 0.0, 1.0);
    return mix(color, camera.sky.rgb, amount * amount * camera.options.w);
}

fn lit(input: VertexOutput) -> vec3<f32> {
    return input.shade * light_color(input.light.x, input.light.y);
}

// Opaque and cutout blocks: pixels are either kept or dropped (leaves, glass frames).
@fragment
fn fs_cutout(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, rippled(input.uv, input.wave), input.layer);
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(fogged(texel.rgb * lit(input), input.relative), 1.0);
}

// Translucent blocks (water, ice): blended, never discarded.
@fragment
fn fs_translucent(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, rippled(input.uv, input.wave), input.layer);
    return vec4<f32>(fogged(texel.rgb * lit(input), input.relative), texel.a);
}
