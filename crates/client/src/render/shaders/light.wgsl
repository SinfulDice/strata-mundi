// Light shared by blocks and models (appended to their source). Expects `camera.light`:
// x sky light factor, y brightness (0-1), z level of the darkest places.

// Torches and lava light things a little warmer than the sky.
const WARM_LIGHT: vec3<f32> = vec3<f32>(1.0, 0.8, 0.56);

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
    return value * mix(vec3<f32>(1.0), WARM_LIGHT, warmth);
}
