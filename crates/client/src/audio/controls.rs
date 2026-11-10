//! What the mixer follows rather than receives: the ears, the volumes, the space and the
//! spatial setting. The game overwrites them (the last value wins, none is ever lost to a
//! full queue) and the output thread reads them before each block.

use super::mixer::{Command, Volumes};
use super::space::Space;
use super::spatial::Listener;
use glam::DVec3;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use strata_core::audio::SoundCategory;

const SPACES: [Space; 4] = [
    Space::Outdoors,
    Space::Interior,
    Space::SmallCave,
    Space::LargeCavern,
];

#[derive(Default)]
pub struct Controls {
    position: [AtomicU64; 3],
    yaw: AtomicU32,
    volumes: [AtomicU32; 3],
    space: AtomicU8,
    spatial: AtomicBool,
}

/// The values last applied to the mixer, to apply only what changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Applied {
    listener: Listener,
    volumes: Volumes,
    space: Space,
    spatial: bool,
}

impl Controls {
    pub fn new(volumes: Volumes, spatial: bool) -> Self {
        let controls = Self::default();
        controls.set_volumes(volumes);
        controls.spatial.store(spatial, Ordering::Relaxed);
        controls
    }

    /// Keeps the value of a control command; other commands are given back.
    pub fn take(&self, command: Command) -> Option<Command> {
        match command {
            Command::Listener(listener) => self.set_listener(listener),
            Command::Volumes(volumes) => self.set_volumes(volumes),
            Command::Space(space) => self.set_space(space),
            Command::Spatial(spatial) => self.spatial.store(spatial, Ordering::Relaxed),
            other => return Some(other),
        }
        None
    }

    fn set_listener(&self, listener: Listener) {
        let axes = listener.position.to_array();
        for (atomic, value) in self.position.iter().zip(axes) {
            atomic.store(value.to_bits(), Ordering::Relaxed);
        }
        self.yaw.store(listener.yaw.to_bits(), Ordering::Relaxed);
    }

    fn set_volumes(&self, volumes: Volumes) {
        for (atomic, category) in self.volumes.iter().zip(SoundCategory::ALL) {
            atomic.store(volumes.get(category).to_bits(), Ordering::Relaxed);
        }
    }

    fn set_space(&self, space: Space) {
        let index = SPACES.iter().position(|s| *s == space).unwrap_or(0);
        self.space.store(index as u8, Ordering::Relaxed);
    }

    pub fn read(&self) -> Applied {
        let axis = |index: usize| f64::from_bits(self.position[index].load(Ordering::Relaxed));
        let volume = |index: usize| f32::from_bits(self.volumes[index].load(Ordering::Relaxed));
        let space = usize::from(self.space.load(Ordering::Relaxed));
        Applied {
            listener: Listener {
                position: DVec3::new(axis(0), axis(1), axis(2)),
                yaw: f32::from_bits(self.yaw.load(Ordering::Relaxed)),
            },
            volumes: Volumes::from_gains([volume(0), volume(1), volume(2)]),
            space: SPACES.get(space).copied().unwrap_or_default(),
            spatial: self.spatial.load(Ordering::Relaxed),
        }
    }
}

impl Applied {
    /// The commands that bring the mixer from `last` (nothing yet: all) to `self`.
    pub fn changes(&self, last: Option<&Applied>, apply: &mut impl FnMut(Command)) {
        if last.is_none_or(|l| l.spatial != self.spatial) {
            apply(Command::Spatial(self.spatial));
        }
        if last.is_none_or(|l| l.listener != self.listener) {
            apply(Command::Listener(self.listener));
        }
        if last.is_none_or(|l| l.volumes != self.volumes) {
            apply(Command::Volumes(self.volumes));
        }
        if last.is_none_or(|l| l.space != self.space || l.spatial != self.spatial) {
            apply(Command::Space(self.space));
        }
    }
}
