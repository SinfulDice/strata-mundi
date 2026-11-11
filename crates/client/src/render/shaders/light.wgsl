// Shared by blocks, models and particles (appended to their source): the camera, the
// block textures, light and fog.

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

fn fogged(color: vec3<f32>, relative: vec3<f32>) -> vec3<f32> {
    let distance = mix(length(relative.xz), length(relative), camera.effects.z);
    let amount = clamp((distance - camera.options.y) / max(camera.options.z - camera.options.y, 1.0), 0.0, 1.0);
    return mix(color, camera.sky.rgb, amount * amount * camera.options.w);
}

// Torches and lava light things a little warmer than the sky.
const WARM_LIGHT: vec3<f32> = vec3<f32>(1.0, 0.8, 0.56);
// Moonlight is a little cooler, so a brighter night still reads as night.
const MOON_LIGHT: vec3<f32> = vec3<f32>(0.78, 0.88, 1.15);

// Minecraft's light curve, lifted towards 1 - (1 - x)^4 by the brightness setting. The
// lift grows with the square of the setting: at the default 50 %, a quarter of it, so
// dim light (night, far from a fire) stays clearly dark.
fn light_curve(level: f32) -> f32 {
    let base = level / (4.0 - 3.0 * level);
    let dark = 1.0 - base;
    let bright = 1.0 - dark * dark * dark * dark;
    return mix(base, bright, camera.light.y * camera.light.y);
}

// Color factor for sky and block light from 0 to 1; full daylight leaves colors unchanged.
fn light_color(sky: f32, block: f32) -> vec3<f32> {
    let sky_level = sky * camera.light.x;
    let perceived = mix(camera.light.z, 1.0, light_curve(max(sky_level, block)));
    // The curve is meant for display values; render targets are linear.
    let value = pow(perceived, 2.2);
    let warmth = clamp((block - sky_level) * 2.0, 0.0, 1.0);
    let night = (1.0 - camera.light.x) * sky * (1.0 - warmth);
    let tint = mix(mix(vec3<f32>(1.0), MOON_LIGHT, night), WARM_LIGHT, warmth);
    return value * tint;
}
