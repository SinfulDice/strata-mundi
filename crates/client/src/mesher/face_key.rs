use super::fluid::Surface;
use super::neighborhood::FaceTint;
use super::shading::FaceShading;
use strata_core::content::{Tint, TintKind};
use strata_core::light::PackedLight;

/// What makes two faces mergeable, packed in 16 bytes: the greedy pass compares keys a lot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceKey {
    /// One `PackedLight` byte per corner, in the order of `CORNERS`.
    light: u32,
    layer: u16,
    /// 2 bits per corner, in the order of `CORNERS`.
    ao: u8,
    /// turned:1 translucent:1 single:1 animation:2
    flags: u8,
    /// 4 bits per corner: sixteenths the corner is lowered by.
    drop: u16,
    /// 0 untinted, else r:8 g:8 b:8 masked:1 by_corner:1 kind:2 tinted:1. `by_corner`:
    /// the colour changes across the face (blend), each vertex looks its own up.
    tint: u32,
}

const FLAG_TURNED: u8 = 1;
const FLAG_TRANSLUCENT: u8 = 1 << 1;
/// Never merged with its neighbours (one quad on its own).
const FLAG_SINGLE: u8 = 1 << 2;
const FLAG_ANIMATION_SHIFT: u8 = 3;

const TINT_MASKED: u32 = 1 << 24;
const TINT_BY_CORNER: u32 = 1 << 25;
const TINTED: u32 = 1 << 28;

impl FaceKey {
    pub fn new(
        layer: u16,
        turned: bool,
        shading: FaceShading,
        translucent: bool,
        surface: Surface,
    ) -> Self {
        let pack = |values: [u8; 4], bits: u32| {
            (0..4).fold(0u16, |acc, i| {
                acc | (u16::from(values[i]) << (bits * i as u32))
            })
        };
        Self {
            light: (0..4).fold(0u32, |acc, i| {
                acc | (u32::from(shading.light[i].0) << (8 * i))
            }),
            layer,
            ao: pack(shading.ao, 2) as u8,
            flags: if turned { FLAG_TURNED } else { 0 }
                | if translucent { FLAG_TRANSLUCENT } else { 0 }
                | if surface.single { FLAG_SINGLE } else { 0 }
                | ((surface.animation & 3) << FLAG_ANIMATION_SHIFT),
            drop: pack(surface.drop, 4),
            tint: 0,
        }
    }

    /// Colours differing between corners make the face its own quad.
    pub fn with_tint(mut self, tint: FaceTint, corners: [Tint; 4]) -> Self {
        let uniform = corners.iter().all(|&c| c == corners[0]);
        let [r, g, b] = corners[0].0.map(u32::from);
        let color = if uniform { r | (g << 8) | (b << 16) } else { 0 };
        self.tint = TINTED
            | color
            | if tint.masked() { TINT_MASKED } else { 0 }
            | if uniform { 0 } else { TINT_BY_CORNER }
            | ((tint.kind().index() as u32) << 26);
        if !uniform {
            self.flags |= FLAG_SINGLE;
        }
        self
    }

    /// `None` untinted; else the kind, whether masked, and the colour of the whole face
    /// (`None`: per corner).
    pub fn tint(self) -> Option<(TintKind, bool, Option<Tint>)> {
        if self.tint & TINTED == 0 {
            return None;
        }
        let kind = TintKind::ALL[((self.tint >> 26) & 3) as usize % 3];
        let channel = |shift: u32| (self.tint >> shift) as u8;
        let color = Tint([channel(0), channel(8), channel(16)]);
        let whole = (self.tint & TINT_BY_CORNER == 0).then_some(color);
        Some((kind, self.tint & TINT_MASKED != 0, whole))
    }

    pub fn layer(self) -> u16 {
        self.layer
    }

    pub fn ao(self, corner: usize) -> u8 {
        (self.ao >> (2 * corner)) & 3
    }

    pub fn light(self, corner: usize) -> PackedLight {
        PackedLight((self.light >> (8 * corner)) as u8)
    }

    pub fn drop(self, corner: usize) -> u8 {
        ((self.drop >> (4 * corner)) & 15) as u8
    }

    pub fn turned(self) -> bool {
        self.flags & FLAG_TURNED != 0
    }

    pub fn translucent(self) -> bool {
        self.flags & FLAG_TRANSLUCENT != 0
    }

    pub fn single(self) -> bool {
        self.flags & FLAG_SINGLE != 0
    }

    pub fn animation(self) -> u8 {
        (self.flags >> FLAG_ANIMATION_SHIFT) & 3
    }
}
