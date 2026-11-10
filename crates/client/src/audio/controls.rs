//! What the mixer follows rather than receives: the ears, the volumes, the room and the
//! spatial setting. The game overwrites them (the last value wins, none is ever lost to a
//! full queue) and the output thread reads them before each block.

use super::mixer::{Command, Volumes};
use super::reverb::Room;
use super::spatial::Listener;
use glam::DVec3;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use strata_core::audio::SoundCategory;

#[derive(Default)]
pub struct Controls {
    position: [AtomicU64; 3],
    yaw: AtomicU32,
    volumes: [AtomicU32; 3],
    /// Feedback, damping and wet level.
    room: [AtomicU32; 3],
    spatial: AtomicBool,
}

/// The values last applied to the mixer, to apply only what changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Applied {
    listener: Listener,
    volumes: Volumes,
    room: Room,
    spatial: bool,
}

impl Controls {
    pub fn new(volumes: Volumes, spatial: bool) -> Self {
        let controls = Self::default();
        controls.set_volumes(volumes);
        controls.set_room(Room::DRY);
        controls.spatial.store(spatial, Ordering::Relaxed);
        controls
    }

    /// Keeps the value of a control command; other commands are given back.
    pub fn take(&self, command: Command) -> Option<Command> {
        match command {
            Command::Listener(listener) => self.set_listener(listener),
            Command::Volumes(volumes) => self.set_volumes(volumes),
            Command::Room(room) => self.set_room(room),
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

    fn set_room(&self, room: Room) {
        let values = [room.feedback, room.damping, room.wet];
        for (atomic, value) in self.room.iter().zip(values) {
            atomic.store(value.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn read(&self) -> Applied {
        let axis = |index: usize| f64::from_bits(self.position[index].load(Ordering::Relaxed));
        let volume = |index: usize| f32::from_bits(self.volumes[index].load(Ordering::Relaxed));
        let room = |index: usize| f32::from_bits(self.room[index].load(Ordering::Relaxed));
        Applied {
            listener: Listener {
                position: DVec3::new(axis(0), axis(1), axis(2)),
                yaw: f32::from_bits(self.yaw.load(Ordering::Relaxed)),
            },
            volumes: Volumes::from_gains([volume(0), volume(1), volume(2)]),
            room: Room {
                feedback: room(0),
                damping: room(1),
                wet: room(2),
            },
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
        if last.is_none_or(|l| l.room != self.room || l.spatial != self.spatial) {
            apply(Command::Room(self.room));
        }
    }
}
