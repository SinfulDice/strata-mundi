//! Boats: float on liquids, steered forward/backward and turned, slide on slippery
//! blocks. Shared by the server (authoritative) and anything that wants to predict them.

use super::Terrain;
use super::aabb::Aabb;
use super::body::{GRAVITY, TICK};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// How fast a boat at most moves (m/s), whatever the ice.
const MAX_SPEED: f64 = 40.0;
const MAX_SUBSTEP: f64 = 0.4;
const MAX_FALL: f64 = 40.0;
/// Fastest rise towards the surface, and how quickly the vertical speed eases (per tick).
const MAX_RISE: f64 = 4.0;
const FLOAT_EASE: f64 = 0.5;
/// Gap to the floating height closed per second.
const FLOAT_RATE: f64 = 8.0;
/// Liquid cells searched upwards for a surface.
const SURFACE_SEARCH: i32 = 32;
/// Default `slipperiness` of blocks: like Minecraft, the fraction of speed kept per tick.
pub const DEFAULT_SLIPPERINESS: f64 = 0.6;

/// Handling of a boat type, from its entity definition in the pack.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename = "Boat", deny_unknown_fields)]
pub struct BoatHandling {
    /// Speed gained each tick paddling forward (m/s).
    pub thrust: f64,
    /// Speed gained each tick paddling backward (m/s).
    pub reverse_thrust: f64,
    /// Turning speed with the stick or key fully pressed (radians per second).
    pub turn_speed: f64,
    /// Fraction of the speed kept each tick afloat and in the air.
    pub water_momentum: f64,
    /// How deep the bottom sits under the surface (blocks).
    pub draft: f64,
    /// Height of the rider's feet above the bottom of the boat (blocks, may be negative).
    pub seat: f64,
}

/// What the pilot asks this tick: `forward` and `turn` between -1 and 1 (turn > 0 = right).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoatControls {
    pub forward: f32,
    pub turn: f32,
}

/// Position (bottom centre), motion and heading of a boat.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoatBody {
    pub position: DVec3,
    pub velocity: DVec3,
    /// Radians, 0 = north (-Z), like the player's yaw.
    pub yaw: f32,
    pub on_ground: bool,
}

/// Collision box: a square footprint `width` wide, `height` tall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxSize {
    pub width: f64,
    pub height: f64,
}

impl BoatBody {
    pub fn at(position: DVec3, yaw: f32) -> Self {
        Self {
            position,
            velocity: DVec3::ZERO,
            yaw,
            on_ground: false,
        }
    }

    pub fn aabb(&self, size: BoxSize) -> Aabb {
        Aabb::standing(self.position, size.width, size.height)
    }

    /// Unit vector of the heading, on the horizontal plane (x, z).
    pub fn forward(&self) -> DVec2 {
        let (sin, cos) = f64::from(self.yaw).sin_cos();
        DVec2::new(sin, -cos)
    }
}

/// Advances `boat` by one tick.
pub fn step_boat(
    boat: &mut BoatBody,
    controls: BoatControls,
    handling: &BoatHandling,
    size: BoxSize,
    terrain: &impl Terrain,
) {
    let turn = f64::from(controls.turn.clamp(-1.0, 1.0));
    let yaw = f64::from(boat.yaw) + turn * handling.turn_speed * TICK;
    boat.yaw = yaw.rem_euclid(std::f64::consts::TAU) as f32;
    let surface = liquid_surface(boat, size, terrain).filter(|&s| s >= boat.position.y);
    let momentum = match (surface, boat.on_ground) {
        (None, true) => ground_slipperiness(boat, size, terrain),
        _ => handling.water_momentum,
    };
    let forward = f64::from(controls.forward.clamp(-1.0, 1.0));
    let thrust = if forward >= 0.0 {
        handling.thrust
    } else {
        handling.reverse_thrust
    };
    let push = boat.forward() * forward * thrust;
    let horizontal = (DVec2::new(boat.velocity.x, boat.velocity.z) * momentum + push)
        .clamp_length_max(MAX_SPEED);
    boat.velocity.x = horizontal.x;
    boat.velocity.z = horizontal.y;
    boat.velocity.y = match surface {
        Some(level) => {
            let wanted = ((level - handling.draft - boat.position.y) * FLOAT_RATE)
                .clamp(-MAX_RISE, MAX_RISE);
            boat.velocity.y + (wanted - boat.velocity.y) * FLOAT_EASE
        }
        None => (boat.velocity.y - GRAVITY * TICK).max(-MAX_FALL),
    };
    move_boat(boat, size, terrain);
}

/// Highest liquid surface under or around the boat's footprint, if any.
pub fn liquid_surface(boat: &BoatBody, size: BoxSize, terrain: &impl Terrain) -> Option<f64> {
    let aabb = boat.aabb(size);
    let start = (boat.position.y - 1.0).floor() as i32;
    let mut highest: Option<f64> = None;
    for x in aabb.cells(0) {
        for z in aabb.cells(2) {
            let mut top = None;
            for y in start..start + SURFACE_SEARCH {
                match terrain.liquid(x, y, z) {
                    Some(cell) => top = Some(f64::from(y) + cell.surface),
                    None if top.is_some() || y > start + 1 => break,
                    None => {}
                }
            }
            if let Some(top) = top {
                highest = Some(highest.map_or(top, |h| h.max(top)));
            }
        }
    }
    highest
}

/// The slipperiness of the block under the centre, or the most slippery block under the
/// footprint when the centre hangs over an edge.
fn ground_slipperiness(boat: &BoatBody, size: BoxSize, terrain: &impl Terrain) -> f64 {
    let below = (boat.position.y - 0.01).floor() as i32;
    let centre = (
        boat.position.x.floor() as i32,
        boat.position.z.floor() as i32,
    );
    if terrain.solid(centre.0, below, centre.1) {
        return terrain.slipperiness(centre.0, below, centre.1);
    }
    let aabb = boat.aabb(size);
    aabb.cells(0)
        .flat_map(|x| aabb.cells(2).map(move |z| (x, z)))
        .filter(|&(x, z)| terrain.solid(x, below, z))
        .map(|(x, z)| terrain.slipperiness(x, below, z))
        .fold(None, |best: Option<f64>, s| {
            Some(best.map_or(s, |b| b.max(s)))
        })
        .unwrap_or(DEFAULT_SLIPPERINESS)
}

/// Sub-steps, axis by axis; a blocked axis loses its speed.
fn move_boat(boat: &mut BoatBody, size: BoxSize, terrain: &impl Terrain) {
    let solid = |x, y, z| terrain.solid(x, y, z);
    let delta = boat.velocity * TICK;
    let steps = (delta.length() / MAX_SUBSTEP).ceil().max(1.0);
    let part = delta / steps;
    boat.on_ground = false;
    for _ in 0..steps as u32 {
        for axis in [1, 0, 2] {
            let moved = boat.aabb(size).sweep(axis, part[axis], &solid);
            boat.position[axis] += moved;
            if moved != part[axis] {
                boat.on_ground |= axis == 1 && part.y < 0.0;
                boat.velocity[axis] = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::LiquidMotion;
    use crate::physics::LiquidCell;

    const SIZE: BoxSize = BoxSize {
        width: 1.375,
        height: 0.5625,
    };
    const HANDLING: BoatHandling = BoatHandling {
        thrust: 0.8,
        reverse_thrust: 0.2,
        turn_speed: 2.5,
        water_momentum: 0.9,
        draft: 0.125,
        seat: -0.25,
    };
    const EAST: f32 = std::f32::consts::FRAC_PI_2;

    /// A floor at y < 0; water up to y = 4 (surface 14/16 into the top cell), or a dry
    /// floor of stone (0.6) or ice (0.98).
    struct Flat {
        water: bool,
        slipperiness: f64,
    }

    impl Terrain for Flat {
        fn solid(&self, _x: i32, y: i32, _z: i32) -> bool {
            y < 0
        }

        fn liquid(&self, _x: i32, y: i32, _z: i32) -> Option<LiquidCell> {
            (self.water && (0..=4).contains(&y)).then_some(LiquidCell {
                motion: LiquidMotion {
                    speed: 0.5,
                    drag: 0.8,
                    gravity: 0.15,
                },
                surface: if y == 4 { 0.875 } else { 1.0 },
            })
        }

        fn slipperiness(&self, _x: i32, _y: i32, _z: i32) -> f64 {
            self.slipperiness
        }
    }

    const WATER: Flat = Flat {
        water: true,
        slipperiness: DEFAULT_SLIPPERINESS,
    };

    fn run(boat: &mut BoatBody, controls: BoatControls, terrain: &Flat, ticks: u32) {
        for _ in 0..ticks {
            step_boat(boat, controls, &HANDLING, SIZE, terrain);
        }
    }

    #[test]
    fn a_boat_dropped_on_water_settles_at_the_surface() {
        let surface = 4.875;
        for start in [surface + 3.0, surface - 2.5] {
            let mut boat = BoatBody::at(DVec3::new(0.5, start, 0.5), 0.0);
            run(&mut boat, BoatControls::default(), &WATER, 100);
            let floating = surface - HANDLING.draft;
            assert!(
                (boat.position.y - floating).abs() < 0.01,
                "from {start}: {}",
                boat.position.y
            );
            assert!(boat.velocity.y.abs() < 0.05);
        }
    }

    #[test]
    fn paddling_goes_where_the_boat_points_and_turning_turns() {
        let mut boat = BoatBody::at(DVec3::new(0.5, 4.75, 0.5), EAST);
        let ahead = BoatControls {
            forward: 1.0,
            turn: 0.0,
        };
        run(&mut boat, ahead, &WATER, 40);
        assert!(boat.position.x > 10.0, "{:?}", boat.position);
        assert!((boat.position.z - 0.5).abs() < 1e-4);
        let right = BoatControls {
            forward: 0.0,
            turn: 1.0,
        };
        run(&mut boat, right, &WATER, 10);
        let expected = EAST + (2.5 * 0.5) as f32;
        assert!((boat.yaw - expected).abs() < 1e-4, "{}", boat.yaw);
    }

    #[test]
    fn ice_is_much_faster_than_land_and_land_is_slow() {
        let distance = |slipperiness| {
            let floor = Flat {
                water: false,
                slipperiness,
            };
            let mut boat = BoatBody::at(DVec3::new(0.5, 0.0, 0.5), EAST);
            boat.on_ground = true;
            let ahead = BoatControls {
                forward: 1.0,
                turn: 0.0,
            };
            run(&mut boat, ahead, &floor, 60);
            assert!(boat.position.y.abs() < 1e-6, "stays on the floor");
            boat.position.x - 0.5
        };
        let (land, ice) = (distance(DEFAULT_SLIPPERINESS), distance(0.98));
        assert!(land < 7.0, "slow on land: {land}");
        assert!(ice > land * 4.0, "ice {ice} vs land {land}");
    }

    #[test]
    fn a_boat_never_goes_through_a_wall() {
        let walled = |x: i32, y: i32, _z: i32| y < 0 || x >= 5;
        let mut boat = BoatBody::at(DVec3::new(3.0, 0.0, 0.5), EAST);
        boat.velocity.x = 39.0;
        step_boat(&mut boat, BoatControls::default(), &HANDLING, SIZE, &walled);
        assert!(boat.position.x <= 5.0 - SIZE.width / 2.0 + 1e-6);
        assert_eq!(boat.velocity.x, 0.0);
    }
}
