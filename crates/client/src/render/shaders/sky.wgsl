// The sky behind the world: gradient, sunrise and sunset glow, stars, sun and moon.

struct Camera {
    view_proj: mat4x4<f32>,
    options: vec4<f32>,
    sky: vec4<f32>,
    // x: seconds, wrapped.
    effects: vec4<f32>,
    light: vec4<f32>,
};

struct Sky {
    inverse_view_proj: mat4x4<f32>,
    // w: glow strength (1 at sunrise and sunset).
    zenith: vec4<f32>,
    // w: star visibility.
    horizon: vec4<f32>,
    // w: share of the sky's cells holding a star.
    glow: vec4<f32>,
    // Toward the sun; w: its half-width (tangent, 0 = not drawn).
    sun: vec4<f32>,
    // Across the path of the sun and moon; w: the moon's half-width.
    across: vec4<f32>,
    // Sun texture layer, moon texture layer, moon phase (0 full, 4 new).
    bodies: vec4<u32>,
    // x: how much stars twinkle; y: seconds wrapped on a full turn (2π).
    stars: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var block_textures: texture_2d_array<f32>;
@group(2) @binding(0) var<uniform> sky: Sky;

// Star cells along each side of a cube around the camera: about two pixels each.
const STAR_GRID: f32 = 360.0;
const PI: f32 = 3.14159265;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOutput;
    out.ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    out.clip = vec4<f32>(out.ndc, 0.0, 1.0);
    return out;
}

fn hash3(x: u32, y: u32, z: u32) -> vec3<f32> {
    var h = x * 1664525u + y * 22695477u + z * 2891336453u + 1013904223u;
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = (h ^ (h >> 13u)) * 3266489917u;
    h = h ^ (h >> 16u);
    let a = h * 747796405u + 2891336453u;
    let b = a * 747796405u + 2891336453u;
    return vec3<f32>(f32(h >> 8u), f32(a >> 8u), f32(b >> 8u)) / 16777216.0;
}

fn gradient(dir: vec3<f32>) -> vec3<f32> {
    let up = clamp(dir.y, 0.0, 1.0);
    var color = mix(sky.zenith.rgb, sky.horizon.rgb, pow(1.0 - up, 2.5));
    let flat_sun = sky.sun.xz;
    var toward = 0.0;
    if dot(flat_sun, flat_sun) > 1e-6 && dot(dir.xz, dir.xz) > 1e-6 {
        toward = max(dot(normalize(dir.xz), normalize(flat_sun)), 0.0);
    }
    let near_horizon = pow(1.0 - abs(dir.y), 4.0);
    let glow = sky.zenith.w * near_horizon * (0.25 + 0.75 * pow(toward, 3.0));
    return mix(color, sky.glow.rgb, clamp(glow, 0.0, 1.0));
}

// Stars are fixed in the frame of the sun's path, so they turn with it through the night.
fn star(dir: vec3<f32>) -> f32 {
    let visibility = sky.horizon.w * smoothstep(-0.02, 0.2, dir.y);
    if visibility <= 0.0 {
        return 0.0;
    }
    let s = sky.sun.xyz;
    let n = sky.across.xyz;
    let q = vec3<f32>(dot(dir, s), dot(dir, cross(n, s)), dot(dir, n));
    let a = abs(q);
    var face = 0u;
    var uv = vec2<f32>(0.0);
    if a.x >= a.y && a.x >= a.z {
        face = select(0u, 1u, q.x < 0.0);
        uv = q.yz / a.x;
    } else if a.y >= a.z {
        face = select(2u, 3u, q.y < 0.0);
        uv = q.xz / a.y;
    } else {
        face = select(4u, 5u, q.z < 0.0);
        uv = q.xy / a.z;
    }
    let cell = vec2<u32>(clamp((uv * 0.5 + 0.5) * STAR_GRID, vec2<f32>(0.0), vec2<f32>(STAR_GRID - 1.0)));
    let h = hash3(cell.x, cell.y, face);
    if h.x >= sky.glow.w {
        return 0.0;
    }
    // Whole speeds, so the time wrapping on a turn never makes a star jump.
    let speed = 1.0 + floor(h.z * 3.0);
    let flicker = 1.0 - sky.stars.x * 0.5 * (1.0 + sin(sky.stars.y * speed + h.y * 2.0 * PI));
    return visibility * (0.35 + 0.65 * h.y) * flicker;
}

// Shadow of the moon's phase, per texel so it stays pixel art.
fn moon_lit(texel: vec2<u32>, size: vec2<u32>, phase: u32) -> bool {
    let p = (vec2<f32>(texel) + 0.5) / vec2<f32>(size) * 2.0 - 1.0;
    let edge = sqrt(max(1.0 - p.y * p.y, 0.0)) * cos(f32(phase) * PI / 4.0);
    if phase <= 4u {
        return p.x < edge;
    }
    return p.x > -edge;
}

// A square sprite of the sky centred on `center`; alpha 0 outside it.
fn body(dir: vec3<f32>, center: vec3<f32>, size: f32, layer: u32, phase: u32, is_moon: bool) -> vec4<f32> {
    let facing = dot(dir, center);
    if facing <= 0.0 || size <= 0.0 {
        return vec4<f32>(0.0);
    }
    // Upright like the world, whatever the tilt of the path: south (+Z) to the right
    // when facing the rising sun.
    let right = normalize(vec3<f32>(0.0, 0.0, 1.0) - center * center.z);
    let up = cross(right, center);
    let p = vec2<f32>(dot(dir, right), dot(dir, up)) / (facing * size);
    if any(abs(p) > vec2<f32>(1.0)) {
        return vec4<f32>(0.0);
    }
    let dims = textureDimensions(block_textures).xy;
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    let texel = min(vec2<u32>(uv * vec2<f32>(dims)), dims - 1u);
    var color = textureLoad(block_textures, texel, layer, 0);
    if is_moon && !moon_lit(texel, dims, phase) {
        color = vec4<f32>(color.rgb * 0.03, color.a);
    }
    let above = smoothstep(-0.12, 0.0, center.y);
    return vec4<f32>(color.rgb, color.a * above);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let far = sky.inverse_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(far.xyz / far.w);
    var color = gradient(dir);
    color += vec3<f32>(star(dir));
    let sun = body(dir, sky.sun.xyz, sky.sun.w, sky.bodies.x, 0u, false);
    color = mix(color, sun.rgb, sun.a);
    let moon = body(dir, -sky.sun.xyz, sky.across.w, sky.bodies.y, sky.bodies.z, true);
    color = mix(color, moon.rgb, moon.a);
    return vec4<f32>(color, 1.0);
}
