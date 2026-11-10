//! Where a sound is heard from: loudness by distance, left/right balance, and the
//! direction shown in its subtitle.

use glam::DVec3;

/// Ears of the player: the camera, turned by its yaw (pitch does not move the ears).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Listener {
    pub position: DVec3,
    pub yaw: f32,
}

impl Listener {
    /// The source in the listener's frame: (right, forward) on the ground, and distance.
    fn local(&self, source: DVec3) -> (f64, f64, f64) {
        let offset = source - self.position;
        let (sin, cos) = f64::from(self.yaw).sin_cos();
        let right = offset.x * cos + offset.z * sin;
        let forward = offset.x * sin - offset.z * cos;
        (right, forward, offset.length())
    }
}

/// Closer than this, a sound is as loud as it gets and centred.
const NEAR: f32 = 1.0;
/// Behind the head, sounds are a little duller (a cue for front and back).
const BEHIND_GAIN: f32 = 0.8;

/// Inverse-distance loudness, eased to silence over the last quarter of `range`.
pub fn attenuation(distance: f32, range: f32) -> f32 {
    if distance >= range {
        return 0.0;
    }
    let inverse = NEAR / distance.max(NEAR);
    let fade_start = range * 0.75;
    let fade = if distance > fade_start {
        1.0 - (distance - fade_start) / (range - fade_start)
    } else {
        1.0
    };
    inverse * fade
}

/// Left and right gains (equal power) of a sound at `source`, distance included.
pub fn stereo_gains(listener: &Listener, source: DVec3, range: f32) -> [f32; 2] {
    let (right, forward, distance) = listener.local(source);
    let distance = distance as f32;
    let flat = (right * right + forward * forward).sqrt() as f32;
    // Pan by the side, faded to the centre close by (a sound at the feet is not "left").
    let side = if flat > 1e-3 {
        right as f32 / flat
    } else {
        0.0
    };
    let pan = side * (distance / NEAR).min(1.0);
    let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
    let behind = if forward < 0.0 && distance > NEAR {
        BEHIND_GAIN
    } else {
        1.0
    };
    let gain = attenuation(distance, range) * behind;
    [angle.cos() * gain, angle.sin() * gain]
}

/// Where a subtitle says the sound comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Front,
    Behind,
    /// Too close to tell (one's own steps, a block at hand above).
    Here,
}

impl Direction {
    pub const ALL: [Direction; 5] = [
        Self::Left,
        Self::Right,
        Self::Front,
        Self::Behind,
        Self::Here,
    ];

    pub fn of(listener: &Listener, source: DVec3) -> Self {
        let (right, forward, _) = listener.local(source);
        if right.hypot(forward) < f64::from(NEAR) * 1.5 {
            Self::Here
        } else if right.abs() > forward.abs() {
            if right > 0.0 { Self::Right } else { Self::Left }
        } else if forward > 0.0 {
            Self::Front
        } else {
            Self::Behind
        }
    }

    /// Translation key of the subtitle format (`{ $sound }` with an arrow).
    pub fn key(self) -> &'static str {
        match self {
            Self::Left => "subtitle-left",
            Self::Right => "subtitle-right",
            Self::Front => "subtitle-front",
            Self::Behind => "subtitle-behind",
            Self::Here => "subtitle-here",
        }
    }
}
