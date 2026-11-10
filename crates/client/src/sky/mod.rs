//! The sky at one moment: colors, sun, moon and stars worked out from the dimension's
//! sky and day cycle, ready for the renderer. Also the world time the client shows.

mod biome_fog;
mod clock;

pub use biome_fog::BiomeFog;
pub use clock::DayClock;

use crate::render::linear_rgb;
use glam::{DVec2, Vec3};
use strata_core::sky::{Clouds, DayCycle, Moment, Sky};
use strata_core::texture::Rgba;

/// Share of the twilight color taken by the whole sky (the glow itself is on the
/// sun's side of the horizon).
const TWILIGHT_ZENITH: f32 = 0.6;
/// How much the fog takes of the glow, since it lies all around the horizon.
const TWILIGHT_FOG: f32 = 0.35;

/// How much the clouds take of the twilight color at sunrise and sunset.
const TWILIGHT_CLOUDS: f32 = 0.8;

/// The cloud layer at one moment. Colors are linear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudFrame {
    pub color: Vec3,
    pub opacity: f32,
    /// World height of the underside.
    pub height: f32,
    pub thickness: f32,
    pub cell_size: f32,
    pub coverage: f32,
    /// How far the layer has drifted since time 0, in blocks.
    pub drift: DVec2,
}

impl CloudFrame {
    fn at(clouds: &Clouds, time: f64, daylight: f32, glow: f32) -> Self {
        let color = linear(clouds.night)
            .lerp(linear(clouds.day), daylight)
            .lerp(linear(clouds.twilight), glow * TWILIGHT_CLOUDS);
        Self {
            color,
            opacity: clouds.opacity,
            height: clouds.height,
            thickness: clouds.thickness,
            cell_size: clouds.cell_size,
            coverage: clouds.coverage,
            drift: DVec2::from_array(clouds.drift(time)),
        }
    }
}

/// Everything the renderer needs to draw the sky and light the world. Colors are linear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyFrame {
    pub zenith: Vec3,
    pub horizon: Vec3,
    /// Color of the horizon on the sun's side at sunrise and sunset.
    pub glow: Vec3,
    /// 0 to 1, strongest right at sunrise and sunset.
    pub glow_strength: f32,
    /// What the fog fades into: the horizon.
    pub fog: Vec3,
    /// Sky light reaching the world (uniform factor of the block shader).
    pub light: f32,
    /// Unit vector toward the sun; the moon is opposite.
    pub sun: Vec3,
    /// Unit vector across the sun's path (the sprites' vertical).
    pub across: Vec3,
    /// Half-widths, as tangents of their angle (0 = not drawn).
    pub sun_size: f32,
    pub moon_size: f32,
    pub sun_layer: u32,
    pub moon_layer: u32,
    pub moon_phase: u32,
    /// Visibility of the stars, 0 by day.
    pub stars: f32,
    pub star_density: f32,
    pub star_twinkle: f32,
    /// `None` in a sky without clouds.
    pub clouds: Option<CloudFrame>,
}

impl Default for SkyFrame {
    /// The plain blue sky of the title screen before any world, without sun.
    fn default() -> Self {
        let cycle = DayCycle::default();
        Self::at(
            &Sky::fallback(),
            &cycle,
            cycle.moment(Moment::Noon) as f64,
            false,
        )
    }
}

impl SkyFrame {
    /// The sky at world time `time` (ticks, fractional); `stars` is the player's setting.
    pub fn at(sky: &Sky, cycle: &DayCycle, time: f64, stars: bool) -> Self {
        let daylight = cycle.daylight(time);
        let glow_strength = cycle.twilight_glow(time);
        let gradient = |index: usize| {
            let base = linear(sky.night[index]).lerp(linear(sky.day[index]), daylight);
            (base, linear(sky.twilight[index]))
        };
        let (zenith, twilight_zenith) = gradient(0);
        let (horizon, glow) = gradient(1);
        let zenith = zenith.lerp(twilight_zenith, glow_strength * TWILIGHT_ZENITH);
        let angle = cycle.sun_angle(time) as f32;
        let (sun, across) = sun_basis(angle, sky.path_tilt);
        let star_light = if stars { (1.0 - daylight).powi(2) } else { 0.0 };
        Self {
            zenith,
            horizon,
            glow,
            glow_strength,
            fog: horizon.lerp(glow, glow_strength * TWILIGHT_FOG),
            light: cycle.sky_factor(time),
            sun,
            across,
            sun_size: sky.sun_size.tan(),
            moon_size: sky.moon_size.tan(),
            sun_layer: u32::from(sky.sun.0),
            moon_layer: u32::from(sky.moon.0),
            moon_phase: u32::from(cycle.moon_phase(time.max(0.0) as u64)),
            stars: star_light * sky.stars.brightness,
            star_density: sky.stars.density,
            star_twinkle: sky.stars.twinkle,
            clouds: sky
                .clouds
                .map(|clouds| CloudFrame::at(&clouds, time, daylight, glow_strength)),
        }
    }
}

impl SkyFrame {
    /// The horizon and the fog multiplied by the biome's colour (linear factors), so the
    /// fogged terrain and the sky behind it stay one colour.
    pub fn with_fog_tint(self, tint: Vec3) -> Self {
        Self {
            horizon: self.horizon * tint,
            fog: self.fog * tint,
            ..self
        }
    }
}

/// The sun rises in the east (+X), passes high in the south (+Z leaning by `tilt`) and
/// sets in the west.
fn sun_basis(angle: f32, tilt: f32) -> (Vec3, Vec3) {
    let up = Vec3::new(0.0, tilt.cos(), tilt.sin());
    let sun = Vec3::X * angle.cos() + up * angle.sin();
    let across = Vec3::X.cross(up);
    (sun.normalize_or(Vec3::Y), across.normalize_or(Vec3::Z))
}

fn linear(Rgba([r, g, b, _]): Rgba) -> Vec3 {
    linear_rgb([r, g, b])
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn terra() -> (Sky, DayCycle) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = strata_core::pack::PackSet::discover(&dir, &mut Vec::new());
        let (content, _) = strata_core::content::Content::load(&packs, &Default::default());
        let terra = strata_core::dimension::DimensionId("strata:terra".parse().unwrap());
        (content.sky(&terra), content.day_cycle(&terra))
    }

    #[test]
    fn noon_is_bright_and_midnight_starry() {
        let (sky, cycle) = terra();
        let noon = SkyFrame::at(&sky, &cycle, cycle.moment(Moment::Noon) as f64, true);
        let midnight = SkyFrame::at(&sky, &cycle, cycle.moment(Moment::Midnight) as f64, true);
        assert_eq!((noon.light, noon.stars), (1.0, 0.0));
        assert!(noon.sun.y > 0.9 && midnight.sun.y < -0.9);
        assert!(midnight.stars > 0.5 && midnight.light < 0.3);
        assert!(midnight.zenith.length() < noon.zenith.length());
        assert!(noon.sun_size > 0.0 && noon.sun_layer != 0 && noon.moon_layer != 0);
        let no_stars = SkyFrame::at(&sky, &cycle, cycle.moment(Moment::Midnight) as f64, false);
        assert_eq!(no_stars.stars, 0.0);
    }

    #[test]
    fn the_sun_rises_in_the_east_with_a_glowing_horizon() {
        let (sky, cycle) = terra();
        let sunrise = SkyFrame::at(&sky, &cycle, 0.0, true);
        assert!(sunrise.sun.x > 0.99 && sunrise.glow_strength > 0.99);
        assert!(sunrise.fog.x > sunrise.horizon.x, "the fog warms up");
    }

    #[test]
    fn clouds_are_white_by_day_rosy_at_sunset_and_dark_at_night() {
        let (sky, cycle) = terra();
        let clouds = |time| SkyFrame::at(&sky, &cycle, time, true).clouds.unwrap();
        let (noon, sunrise, midnight) = (
            clouds(cycle.moment(Moment::Noon) as f64),
            clouds(0.0),
            clouds(cycle.moment(Moment::Midnight) as f64),
        );
        assert!(noon.color.min_element() > 0.9);
        assert!(sunrise.color.x > sunrise.color.z, "rosy");
        assert!(midnight.color.max_element() < 0.1);
        assert!(noon.height > 200.0 && noon.coverage > 0.0 && noon.coverage < 1.0);
        assert_ne!(noon.drift, midnight.drift, "they drift with time");
    }
}
