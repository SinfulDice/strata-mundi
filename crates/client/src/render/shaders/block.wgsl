struct Camera {
    view_proj: mat4x4<f32>,
    // x: ambient occlusion strength (0 = off, 1 = full); y, z: fog start and end, in
    // blocks from the camera; w: 1 with fog, 0 without.
    options: vec4<f32>,
    // Sky color (linear), which the fog fades to (the liquid's color with the eyes in it).
    sky: vec4<f32>,
    // x: seconds, wrapped; y: 1 with animated textures, 0 without; z: 1 when the fog
    // counts the full distance (eyes in a liquid), 0 for the horizontal distance only;
    // w: 1 with random texture variants, 0 without.
    effects: vec4<f32>,
    // x: sky light factor (1 by day); y: brightness (0-1); z: light of the darkest places.
    light: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var block_textures: texture_2d_array<f32>;
@group(1) @binding(1) var block_sampler: sampler;
// Per layer: first_extra:16 frames:8 frame_time:8 (ticks); 0 for still layers.
@group(1) @binding(2) var<storage, read> animations: array<u32>;
// Per layer: second image, then start:24 images:8 into the running totals of the weights
// that follow (see `variant_table`).
@group(1) @binding(3) var<storage, read> variants: array<u32>;

struct Section {
    // Origin relative to the camera (camera-relative rendering).
    offset: vec4<f32>,
    // Origin in the world, for the random variants.
    origin: vec4<i32>,
};

var<immediate> section: Section;

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
    // Position in the section, and the face (6 for plant planes): the fragment finds its
    // block even on merged faces.
    @location(6) local: vec3<f32>,
    @location(7) @interpolate(flat) face: u32,
    // Biome colour factor (linear, 1 = as painted), and 1 when pixels with alpha 254
    // keep their colour (grass fringe on the sides).
    @location(8) tint: vec3<f32>,
    @location(9) @interpolate(flat) masked: u32,
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
    if camera.effects.y == 0.0 {
        return layer;
    }
    let entry = animations[min(layer, arrayLength(&animations) - 1u)];
    let frames = (entry >> 16u) & 0xffu;
    if frames < 2u {
        return layer;
    }
    let ticks = u32(camera.effects.x * 20.0);
    let frame = (ticks / max(entry >> 24u, 1u)) % frames;
    if frame == 0u {
        return layer;
    }
    return (entry & 0xffffu) + frame - 1u;
}

// Same as `strata_core::content::position_hash` (u32 products wrap in WGSL too).
fn position_hash(block: vec3<i32>) -> u32 {
    var h = (bitcast<u32>(block.x) * 0x8DA6B343u) ^ (bitcast<u32>(block.y) * 0xD8163841u)
        ^ (bitcast<u32>(block.z) * 0xCB1AB31Fu);
    h ^= h >> 16u;
    h *= 0x7FEB352Du;
    h ^= h >> 15u;
    h *= 0x846CA68Bu;
    return h ^ (h >> 16u);
}

fn face_normal(face: u32) -> vec3<f32> {
    var normals = array<vec3<f32>, 7>(
        vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, -1.0),
        vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0),
        vec3<f32>(0.0),
    );
    return normals[min(face, 6u)];
}

// Random variant of the block under this pixel: the image whose running total of
// weights first exceeds `position_hash % total` (as `TextureLayout::random_index`).
// Plant planes keep their image, as before.
fn variant_layer(input: VertexOutput) -> u32 {
    let slot = input.layer * 2u + 1u;
    if camera.effects.w == 0.0 || input.face >= 6u || slot >= arrayLength(&variants) {
        return input.layer;
    }
    let entry = variants[slot];
    let images = entry & 0xffu;
    if images < 2u {
        return input.layer;
    }
    // Faces lie on block boundaries: step back half a block to stand inside the block.
    let inside = input.local - face_normal(input.face) * 0.5;
    let block = vec3<i32>(floor(inside)) + section.origin.xyz;
    let start = entry >> 8u;
    let roll = position_hash(block) % max(variants[start + images - 1u], 1u);
    var index = 0u;
    while index + 1u < images && roll >= variants[start + index] {
        index += 1u;
    }
    if index == 0u {
        return input.layer;
    }
    return variants[slot - 1u] + index - 1u;
}

@vertex
fn vs_main(
    @location(0) geometry: u32,
    @location(1) texture: u32,
    @location(2) tint: u32,
) -> VertexOutput {
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
    out.relative = position + section.offset.xyz;
    out.clip = camera.view_proj * vec4<f32>(out.relative, 1.0);
    out.uv = uv;
    out.layer = animated_layer(texture & 0xffffu);
    out.shade = face_shade(face) * ao_light(ao);
    out.wave = speed;
    out.light = vec2<f32>(f32(texture >> 28u), f32((texture >> 24u) & 15u)) / 15.0;
    out.local = position + vec3<f32>(0.0, drop, 0.0);
    out.face = face;
    // Factors are given in display space (128 = ×1); textures are sampled in linear.
    let factor = vec3<f32>(f32(tint & 0xffu), f32((tint >> 8u) & 0xffu), f32((tint >> 16u) & 0xffu)) / 128.0;
    out.tint = pow(factor, vec3<f32>(2.2));
    out.masked = (tint >> 24u) & 1u;
    return out;
}

// The marked pixels of a masked face (alpha 254) keep their colour; mipmaps blend both.
fn tinted(texel: vec4<f32>, input: VertexOutput) -> vec3<f32> {
    var amount = 1.0;
    if input.masked == 1u {
        amount = clamp(texel.a * 255.0 - 254.0, 0.0, 1.0);
    }
    return texel.rgb * mix(vec3<f32>(1.0), input.tint, amount);
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
    let texel = textureSample(block_textures, block_sampler, rippled(input.uv, input.wave), variant_layer(input));
    if texel.a < 0.5 {
        discard;
    }
    return vec4<f32>(fogged(tinted(texel, input) * lit(input), input.relative), 1.0);
}

// Translucent blocks (water, ice): blended, never discarded.
@fragment
fn fs_translucent(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(block_textures, block_sampler, rippled(input.uv, input.wave), variant_layer(input));
    return vec4<f32>(fogged(texel.rgb * input.tint * lit(input), input.relative), texel.a);
}
