//! Biome colours of a section, blended across biome borders: each vertex takes the mean
//! colour of the blocks around its corner, so colours never break at biome or column
//! borders.

use strata_core::chunk::BiomeId;
use strata_core::content::{BiomeTints, Tint, TintKind};

/// Blocks averaged on each side of a corner, at most: the blend stays within the eight
/// neighbouring columns.
pub const MAX_BLEND_RADIUS: u32 = 8;
const SIZE: usize = 16;
const CORNERS: usize = SIZE + 1;

/// Colours of the 16×16 block columns of a section, by kind: per corner (17×17) when
/// blended, per block (16×16) without blending.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnTints {
    blended: bool,
    colors: [Vec<Tint>; 3],
}

impl ColumnTints {
    /// `biome(x, z)` for local coordinates from `-radius` to `16 + radius`; `table`
    /// indexed by `BiomeId` (unknown biomes keep the texture's colour).
    pub fn new(radius: u32, table: &[BiomeTints], biome: impl Fn(i32, i32) -> BiomeId) -> Self {
        let radius = radius.min(MAX_BLEND_RADIUS) as i32;
        let tints = |id: BiomeId| table.get(usize::from(id.0)).copied().unwrap_or_default();
        if radius == 0 {
            let colors = TintKind::ALL.map(|kind| {
                (0..SIZE * SIZE)
                    .map(|i| tints(biome((i % SIZE) as i32, (i / SIZE) as i32)).get(kind))
                    .collect()
            });
            return Self {
                blended: false,
                colors,
            };
        }
        let window = Window::new(radius, |x, z| tints(biome(x, z)));
        Self {
            blended: true,
            colors: TintKind::ALL.map(|kind| window.corner_means(kind)),
        }
    }

    /// Colour at corner `(cx, cz)` (0 to 16) of a face of the block at `(x, z)` (0 to 15).
    pub fn at(&self, kind: TintKind, block: (i32, i32), corner: (i32, i32)) -> Tint {
        let colors = &self.colors[kind.index()];
        let index = if self.blended {
            let (cx, cz) = (corner.0.clamp(0, 16), corner.1.clamp(0, 16));
            cz as usize * CORNERS + cx as usize
        } else {
            let (x, z) = (block.0.clamp(0, 15), block.1.clamp(0, 15));
            z as usize * SIZE + x as usize
        };
        colors.get(index).copied().unwrap_or_default()
    }

    /// Without blending, one colour per block: every corner of a block's face agrees.
    pub fn is_blended(&self) -> bool {
        self.blended
    }
}

/// Running totals of the colours of the blocks around a section, for box means in O(1).
struct Window {
    radius: i32,
    width: usize,
    /// `[kind][z][x]` sums of channels over the blocks before `(x, z)`, row-major.
    sums: [Vec<[u32; 3]>; 3],
}

impl Window {
    fn new(radius: i32, tints: impl Fn(i32, i32) -> BiomeTints) -> Self {
        let width = SIZE + 2 * radius as usize;
        let stride = width + 1;
        let mut sums = TintKind::ALL.map(|_| vec![[0u32; 3]; stride * stride]);
        for z in 0..width {
            for x in 0..width {
                let biome = tints(x as i32 - radius, z as i32 - radius);
                for kind in TintKind::ALL {
                    let table = &mut sums[kind.index()];
                    let color = biome.get(kind).0;
                    let (left, up, diagonal) = (
                        table[(z + 1) * stride + x],
                        table[z * stride + x + 1],
                        table[z * stride + x],
                    );
                    table[(z + 1) * stride + x + 1] = std::array::from_fn(|c| {
                        u32::from(color[c]) + left[c] + up[c] - diagonal[c]
                    });
                }
            }
        }
        Self {
            radius,
            width,
            sums,
        }
    }

    /// Mean over the `2r × 2r` blocks touching each corner from every side.
    fn corner_means(&self, kind: TintKind) -> Vec<Tint> {
        let table = &self.sums[kind.index()];
        let stride = self.width + 1;
        let side = 2 * self.radius as usize;
        let count = (side * side) as u32;
        let mut colors = Vec::with_capacity(CORNERS * CORNERS);
        for cz in 0..CORNERS {
            for cx in 0..CORNERS {
                // Corner `c` sits between blocks `c - 1` and `c`; with the window offset by
                // `radius`, the box spans window cells `c .. c + 2r`.
                let (x0, z0, x1, z1) = (cx, cz, cx + side, cz + side);
                let sum = |c: usize| {
                    table[z1 * stride + x1][c] + table[z0 * stride + x0][c]
                        - table[z0 * stride + x1][c]
                        - table[z1 * stride + x0][c]
                };
                colors.push(Tint(std::array::from_fn(|c| {
                    ((sum(c) + count / 2) / count) as u8
                })));
            }
        }
        colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREEN: Tint = Tint([60, 160, 60]);
    const YELLOW: Tint = Tint([200, 160, 40]);

    fn table() -> Vec<BiomeTints> {
        [GREEN, YELLOW]
            .map(|grass| BiomeTints {
                kinds: [grass, Tint::NEUTRAL, Tint::NEUTRAL],
                fog: Tint::NEUTRAL,
            })
            .to_vec()
    }

    /// Biome 1 east of world x = 20 (inside the second of two sections side by side).
    fn border(origin_x: i32) -> impl Fn(i32, i32) -> BiomeId {
        move |x, _| BiomeId(u16::from(origin_x + x >= 20))
    }

    fn grass(tints: &ColumnTints, corner_x: i32) -> Tint {
        tints.at(TintKind::Grass, (corner_x.min(15), 3), (corner_x, 3))
    }

    #[test]
    fn colours_change_smoothly_across_a_biome_border() {
        let table = table();
        let first = ColumnTints::new(4, &table, border(0));
        let second = ColumnTints::new(4, &table, border(16));
        let row: Vec<Tint> = (0..=16)
            .map(|x| grass(&first, x))
            .chain((1..=16).map(|x| grass(&second, x)))
            .collect();
        assert_eq!(row[0], GREEN);
        assert_eq!(*row.last().unwrap(), YELLOW);
        for pair in row.windows(2) {
            let step = (0..3).map(|c| pair[0].0[c].abs_diff(pair[1].0[c])).max();
            assert!(step.unwrap() <= 18, "{pair:?}");
        }
    }

    #[test]
    fn neighbouring_columns_agree_on_their_shared_corners() {
        let table = table();
        let border_at_17 = |origin: i32| move |x: i32, _| BiomeId(u16::from(origin + x >= 17));
        let first = ColumnTints::new(3, &table, border_at_17(0));
        let second = ColumnTints::new(3, &table, border_at_17(16));
        assert_eq!(grass(&first, 16), grass(&second, 0));
        assert_ne!(
            grass(&first, 16),
            GREEN,
            "the border at x = 17 is within reach"
        );
    }

    #[test]
    fn radius_zero_gives_each_block_its_own_biome() {
        let table = table();
        let tints = ColumnTints::new(0, &table, border(10));
        assert!(!tints.is_blended());
        let block = |x: i32, corner: i32| tints.at(TintKind::Grass, (x, 0), (corner, 0));
        assert_eq!(block(9, 9), GREEN);
        assert_eq!(block(9, 10), GREEN, "every corner of the block's face");
        assert_eq!(block(10, 10), YELLOW);
        assert_eq!(block(10, 11), YELLOW);
    }

    #[test]
    fn unknown_biomes_keep_the_texture_colour() {
        let tints = ColumnTints::new(2, &[], |_, _| BiomeId(7));
        assert_eq!(tints.at(TintKind::Water, (0, 0), (5, 5)), Tint::NEUTRAL);
    }
}
