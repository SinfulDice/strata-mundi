use super::*;
use crate::world::{ClientWorld, requested_view_distance};
use strata_core::coords::ColumnPos;
use strata_protocol::in_view_disc;

/// The columns the client meshes once the server's disc is loaded, by the same rules
/// as the game: lit when the 8 neighbours are loaded, meshed when they are all lit.
fn meshed(render_distance: u32) -> impl Fn(ColumnPos) -> bool {
    let loaded = requested_view_distance(render_distance);
    let r = loaded as i32;
    let mut world = ClientWorld::default();
    let disc = || {
        (-r..=r)
            .flat_map(move |z| (-r..=r).map(move |x| ColumnPos { x, z }))
            .filter(move |pos| in_view_disc(pos.x, pos.z, loaded))
    };
    for pos in disc() {
        world.set_block(
            pos.x * SECTION_SIZE,
            0,
            pos.z * SECTION_SIZE,
            strata_core::content::BlockId::AIR,
        );
    }
    let lit: Vec<_> = disc().filter(|&pos| world.is_complete(pos)).collect();
    for pos in lit {
        world.mark_lit(pos);
    }
    move |pos| world.is_light_complete(pos)
}

/// Horizontal distance from `(x, z)` to the nearest point of a column.
fn gap((x, z): (f32, f32), pos: ColumnPos) -> f32 {
    let size = SECTION_SIZE as f32;
    let axis = |c: f32, column: i32| {
        let (low, high) = (column as f32 * size, (column + 1) as f32 * size);
        (low - c).max(c - high).max(0.0)
    };
    axis(x, pos.x).hypot(axis(z, pos.z))
}

#[test]
fn the_fog_is_complete_before_the_first_undrawn_column() {
    for distance in [2, 6, 12, 32] {
        let is_meshed = meshed(distance);
        let end = fog_end(distance);
        let reach = distance as i32 + 4;
        let size = SECTION_SIZE as f32;
        for camera in [
            (0.0, 0.0),
            (size, size),
            (size / 2.0, size / 2.0),
            (0.0, size),
        ] {
            let nearest = (-reach..=reach)
                .flat_map(|z| (-reach..=reach).map(move |x| ColumnPos { x, z }))
                .filter(|&pos| !is_meshed(pos))
                .map(|pos| gap(camera, pos))
                .fold(f32::MAX, f32::min);
            assert!(end <= nearest, "distance {distance}: fog {end} > {nearest}");
        }
        assert!(end >= (distance as f32 - 2.0) * size, "{distance}: {end}");
    }
}
