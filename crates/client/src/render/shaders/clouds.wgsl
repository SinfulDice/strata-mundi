// Pixel-art clouds: one instance per cell of a grid around the camera. Empty cells, and
// the sides shared by two cloudy cells, collapse to nothing in the vertex shader.

struct Camera {
    view_proj: mat4x4<f32>,
    options: vec4<f32>,
    sky: vec4<f32>,
    effects: vec4<f32>,
    light: vec4<f32>,
};

struct Clouds {
    // xy: the camera's place in the pattern (x, z); z: underside height relative to the
    // camera; w: thickness.
    origin: vec4<f32>,
    // rgb: linear color; a: opacity.
    color: vec4<f32>,
    // rgb: fog color; w: reach, in blocks.
    fog: vec4<f32>,
    // x: cell size; y: coverage; z: cells along a side of the grid; w: 1 for volumes.
    grid: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(2) @binding(0) var<uniform> clouds: Clouds;

// The pattern repeats every PERIOD cells (keep in step with `cloud_pipeline.rs`).
const PERIOD: u32 = 256u;
// Cells between the knots of the large and small noise; both divide PERIOD.
const LARGE: u32 = 16u;
const SMALL: u32 = 4u;

struct VertexOutput {
    // Invariant: the depth pass and the color pass must land on the same depths.
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) shade: f32,
};

fn hash(x: u32, z: u32, salt: u32) -> f32 {
    var h = x * 1664525u + z * 22695477u + salt * 2891336453u + 1013904223u;
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = (h ^ (h >> 13u)) * 3266489917u;
    h = h ^ (h >> 16u);
    return f32(h >> 8u) / 16777216.0;
}

// Value noise on knots `step` cells apart, wrapping with the pattern.
fn noise(cell: vec2<u32>, step: u32, salt: u32) -> f32 {
    let knots = PERIOD / step;
    let k = cell / step;
    let f = vec2<f32>(cell % step) / f32(step);
    let t = f * f * (3.0 - 2.0 * f);
    let k1 = (k + 1u) % knots;
    let a = hash(k.x, k.y, salt);
    let b = hash(k1.x, k.y, salt);
    let c = hash(k.x, k1.y, salt);
    let d = hash(k1.x, k1.y, salt);
    return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

fn cloudy(cell: vec2<i32>) -> bool {
    let c = bitcast<vec2<u32>>(cell) % PERIOD;
    let value = 0.65 * noise(c, LARGE, 1u) + 0.35 * noise(c, SMALL, 2u);
    // Mostly between 0.25 and 0.75: stretch it so the coverage reads as a share.
    let spread = clamp((value - 0.5) * 2.0 + 0.5, 0.0, 1.0);
    return spread > 1.0 - clouds.grid.y;
}

// Corner `corner` (0-5, two triangles) of face `face`: 0 under, 1 top, 2 west, 3 east,
// 4 north, 5 south. Unit cube coordinates.
fn corner_of(face: u32, corner: u32) -> vec3<f32> {
    let quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let q = quad[corner];
    switch face {
        case 0u: { return vec3<f32>(q.x, 0.0, q.y); }
        case 1u: { return vec3<f32>(q.x, 1.0, q.y); }
        case 2u: { return vec3<f32>(0.0, q.x, q.y); }
        case 3u: { return vec3<f32>(1.0, q.x, q.y); }
        case 4u: { return vec3<f32>(q.x, q.y, 0.0); }
        default: { return vec3<f32>(q.x, q.y, 1.0); }
    }
}

fn neighbor(face: u32) -> vec2<i32> {
    switch face {
        case 2u: { return vec2<i32>(-1, 0); }
        case 3u: { return vec2<i32>(1, 0); }
        case 4u: { return vec2<i32>(0, -1); }
        case 5u: { return vec2<i32>(0, 1); }
        default: { return vec2<i32>(0, 0); }
    }
}

fn shade_of(face: u32) -> f32 {
    if clouds.grid.w < 0.5 {
        return 1.0;
    }
    let shades = array<f32, 6>(0.72, 1.0, 0.88, 0.88, 0.8, 0.8);
    return shades[face];
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOutput {
    var out: VertexOutput;
    // Off screen and with no area: the triangle is dropped.
    out.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    out.relative = vec3<f32>(0.0);
    out.shade = 0.0;
    let side = u32(clouds.grid.z);
    let size = clouds.grid.x;
    let base = vec2<i32>(floor(clouds.origin.xy / size));
    let cell = base + vec2<i32>(i32(instance % side), i32(instance / side)) - i32(side / 2u);
    let face = vertex / 6u;
    if !cloudy(cell) || (face >= 2u && cloudy(cell + neighbor(face))) {
        return out;
    }
    let unit = corner_of(face, vertex % 6u);
    let start = vec2<f32>(cell) * size - clouds.origin.xy;
    out.relative = vec3<f32>(
        start.x + unit.x * size,
        clouds.origin.z + unit.y * clouds.origin.w,
        start.y + unit.z * size,
    );
    out.clip = camera.view_proj * vec4<f32>(out.relative, 1.0);
    out.shade = shade_of(face);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let reach = clouds.fog.w;
    let distance = length(in.relative.xz);
    let fog = smoothstep(reach * 0.45, reach, distance);
    let color = mix(clouds.color.rgb * in.shade, clouds.fog.rgb, fog);
    let alpha = clouds.color.a * (1.0 - smoothstep(reach * 0.85, reach, distance));
    return vec4<f32>(color, alpha);
}
