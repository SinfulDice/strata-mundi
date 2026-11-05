use crate::chunk::SECTION_VOLUME;
use crate::coords::SECTIONS_PER_COLUMN;
use std::sync::Arc;

pub const MAX_LIGHT: u8 = 15;

/// The two kinds of light, stored side by side in one byte per block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Comes down from above the world; the night dims it without recomputing (v0.2 j7).
    Sky,
    /// Given off by blocks (torches, lava).
    Block,
}

impl Channel {
    pub const ALL: [Channel; 2] = [Self::Sky, Self::Block];
}

/// Sky light in the high four bits, block light in the low four.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackedLight(pub u8);

impl PackedLight {
    pub const DARK: Self = Self(0);
    pub const OPEN_SKY: Self = Self(MAX_LIGHT << 4);

    pub const fn new(sky: u8, block: u8) -> Self {
        Self((sky << 4) | (block & 0x0f))
    }

    pub const fn sky(self) -> u8 {
        self.0 >> 4
    }

    pub const fn block(self) -> u8 {
        self.0 & 0x0f
    }

    pub const fn get(self, channel: Channel) -> u8 {
        match channel {
            Channel::Sky => self.sky(),
            Channel::Block => self.block(),
        }
    }

    pub const fn with(self, channel: Channel, level: u8) -> Self {
        match channel {
            Channel::Sky => Self::new(level, self.block()),
            Channel::Block => Self::new(self.sky(), level),
        }
    }
}

/// Light of a 16³ section. Sections where every block has the same light (open sky
/// above the terrain, solid rock below it) take no memory; the others take 4 KiB, shared
/// with the copies sent to the renderer until one of them changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionLight {
    Uniform(PackedLight),
    Mixed(Arc<[u8; SECTION_VOLUME]>),
}

impl SectionLight {
    /// `index` as in `section_index`.
    #[inline]
    pub fn get(&self, index: usize) -> PackedLight {
        match self {
            Self::Uniform(light) => *light,
            Self::Mixed(cells) => PackedLight(cells[index]),
        }
    }

    /// Whether the value changed.
    #[inline]
    pub fn set(&mut self, index: usize, light: PackedLight) -> bool {
        match self {
            Self::Uniform(current) if *current == light => false,
            Self::Uniform(current) => {
                let mut cells = Arc::new([current.0; SECTION_VOLUME]);
                Arc::make_mut(&mut cells)[index] = light.0;
                *self = Self::Mixed(cells);
                true
            }
            Self::Mixed(cells) => {
                let changed = cells[index] != light.0;
                if changed {
                    Arc::make_mut(cells)[index] = light.0;
                }
                changed
            }
        }
    }

    /// Back to the memory-free form when every block ended up with the same light.
    pub fn compact(&mut self) {
        if let Self::Mixed(cells) = self
            && cells.iter().all(|&cell| cell == cells[0])
        {
            *self = Self::Uniform(PackedLight(cells[0]));
        }
    }

    pub fn heap_bytes(&self) -> usize {
        match self {
            Self::Uniform(_) => 0,
            Self::Mixed(_) => SECTION_VOLUME,
        }
    }
}

/// Light of a whole column, section by section from the bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnLight {
    sections: Vec<SectionLight>,
}

impl ColumnLight {
    /// Dark up to section `top` included, open sky above.
    pub fn below_open_sky(top: Option<usize>) -> Self {
        let sections = (0..SECTIONS_PER_COLUMN as usize)
            .map(|index| match top {
                Some(top) if index <= top => SectionLight::Uniform(PackedLight::DARK),
                _ => SectionLight::Uniform(PackedLight::OPEN_SKY),
            })
            .collect();
        Self { sections }
    }

    pub fn section(&self, index: usize) -> Option<&SectionLight> {
        self.sections.get(index)
    }

    pub fn section_mut(&mut self, index: usize) -> Option<&mut SectionLight> {
        self.sections.get_mut(index)
    }

    pub fn sections(&self) -> &[SectionLight] {
        &self.sections
    }

    pub fn compact(&mut self) {
        self.sections.iter_mut().for_each(SectionLight::compact);
    }

    pub fn heap_bytes(&self) -> usize {
        self.sections
            .iter()
            .map(SectionLight::heap_bytes)
            .sum::<usize>()
            + self.sections.capacity() * std::mem::size_of::<SectionLight>()
    }
}

/// Same layout as block storage: x fastest, then z, then y.
#[inline]
pub const fn section_index(x: u32, y: u32, z: u32) -> usize {
    ((y * 16 + z) * 16 + x) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_light_keeps_both_channels() {
        let light = PackedLight::new(12, 7).with(Channel::Block, 3);
        assert_eq!((light.sky(), light.block()), (12, 3));
        assert_eq!(light.with(Channel::Sky, 0), PackedLight::new(0, 3));
    }

    #[test]
    fn uniform_sections_allocate_only_when_they_differ() {
        let mut section = SectionLight::Uniform(PackedLight::OPEN_SKY);
        assert!(!section.set(5, PackedLight::OPEN_SKY));
        assert_eq!(section.heap_bytes(), 0);
        assert!(section.set(5, PackedLight::new(14, 0)));
        assert_eq!(section.get(5), PackedLight::new(14, 0));
        assert_eq!(section.get(6), PackedLight::OPEN_SKY);
        section.set(5, PackedLight::OPEN_SKY);
        section.compact();
        assert_eq!(section, SectionLight::Uniform(PackedLight::OPEN_SKY));
    }
}
