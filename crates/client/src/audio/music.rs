//! When music plays: rarely, after minutes of silence (the music frequency setting), the
//! theme of the place (the title screen, or the dimension). The next piece is rendered
//! into the cache in the background while the silence lasts, then read as it plays.

use super::mixer::Command;
use super::music_threads::{RenderWorker, Source, spawn_reader};
use super::stream::MusicCommand;
use crate::settings::MusicFrequency;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use strata_core::audio::Rng;
use strata_core::audio::music::Theme;
use strata_core::dimension::DimensionId;
use strata_core::id::Identifier;

/// Silence before the first piece on the title screen.
pub const TITLE_WAIT: (f32, f32) = (2.0, 5.0);
/// The first wait in a world is this share of the usual one.
const FIRST_WAIT_SHARE: f32 = 0.1;

/// Where the listener is, for the music.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicPlace {
    Title,
    /// `None` until the server has said which dimension.
    World(Option<DimensionId>),
}

impl MusicPlace {
    fn first_wait(&self, frequency: MusicFrequency) -> (f32, f32) {
        match self {
            Self::Title => TITLE_WAIT,
            Self::World(_) => {
                let (a, b) = frequency.wait();
                (a * FIRST_WAIT_SHARE, b * FIRST_WAIT_SHARE)
            }
        }
    }

    fn same_kind(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Title, Self::Title) | (Self::World(_), Self::World(_))
        )
    }
}

/// The silence between pieces (pure: no thread, no device).
#[derive(Debug, Clone)]
pub struct MusicSchedule {
    /// Seconds left before the next piece; `None` while one plays.
    wait: Option<f32>,
    rng: Rng,
}

impl MusicSchedule {
    pub fn new(first: (f32, f32), seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        Self {
            wait: Some(rng.range(first)),
            rng,
        }
    }

    /// Counts the silence down (not while paused); true once the next piece is due.
    pub fn update(&mut self, seconds: f32, paused: bool) -> bool {
        let Some(wait) = &mut self.wait else {
            return false;
        };
        if !paused {
            *wait = (*wait - seconds.max(0.0)).max(0.0);
        }
        *wait <= 0.0
    }

    pub fn wait(&self) -> Option<f32> {
        self.wait
    }

    pub fn started(&mut self) {
        self.wait = None;
    }

    /// A piece ended: silence for a while.
    pub fn finished(&mut self, frequency: MusicFrequency) {
        self.wait = Some(self.rng.range(frequency.wait()));
    }

    pub fn restart(&mut self, first: (f32, f32)) {
        self.wait = Some(self.rng.range(first));
    }
}

/// What the music should follow this frame.
pub struct MusicMoment<'a> {
    pub seconds: f32,
    pub place: &'a MusicPlace,
    /// The pause menu: the piece goes on, the silence is not counted.
    pub paused: bool,
    /// A device and a music volume above 0.
    pub enabled: bool,
    pub frequency: MusicFrequency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Next {
    Piece(Identifier, u8),
    Recording(PathBuf),
}

pub struct MusicPlayer {
    cache: PathBuf,
    themes: Vec<Theme>,
    schedule: MusicSchedule,
    place: Option<MusicPlace>,
    worker: Option<RenderWorker>,
    /// Pieces known to be in the cache.
    ready: Vec<(Identifier, u8, PathBuf)>,
    next: Option<Next>,
    playing: Option<Arc<AtomicBool>>,
    last: Option<Next>,
    /// A render failed (disk full, folder not writable): no more tries this session.
    broken: bool,
    rng: Rng,
}

impl MusicPlayer {
    pub fn new(cache: PathBuf, themes: Vec<Theme>) -> Self {
        Self {
            cache,
            themes,
            schedule: MusicSchedule::new(TITLE_WAIT, 0x4D55_531C),
            place: None,
            worker: None,
            ready: Vec::new(),
            next: None,
            playing: None,
            last: None,
            broken: false,
            rng: Rng::new(0x7E_77A),
        }
    }

    /// New packs: the next piece is chosen again from the new themes.
    pub fn set_themes(&mut self, themes: Vec<Theme>) {
        self.themes = themes;
        self.ready.clear();
        self.next = None;
        self.worker = None;
    }

    pub fn is_playing(&self) -> bool {
        self.playing.is_some()
    }

    /// Seconds before the next piece, if one is not playing.
    pub fn wait(&self) -> Option<f32> {
        self.schedule.wait()
    }

    /// True while a render thread exists (it is only started when music is wanted).
    pub fn is_rendering(&self) -> bool {
        self.worker.is_some()
    }

    pub fn update(&mut self, moment: &MusicMoment<'_>, send: &mut dyn FnMut(Command)) {
        if !moment.enabled {
            self.stop(send);
            self.place = None;
            return;
        }
        self.follow_place(moment, send);
        self.collect_renders();
        if let Some(done) = &self.playing {
            if !done.load(Ordering::Acquire) {
                return;
            }
            self.playing = None;
            self.schedule.finished(moment.frequency);
        }
        let due = self.schedule.update(moment.seconds, moment.paused);
        let Some(theme) = self.theme_of(moment.place) else {
            return;
        };
        if self.next.is_none() && !self.broken {
            self.next = self.choose(&theme);
        }
        if due && let Some(source) = self.ready_source() {
            self.start(source, send);
        }
    }

    fn follow_place(&mut self, moment: &MusicMoment<'_>, send: &mut dyn FnMut(Command)) {
        if self.place.as_ref() == Some(moment.place) {
            return;
        }
        let kind_changed = self
            .place
            .as_ref()
            .is_none_or(|old| !old.same_kind(moment.place));
        if kind_changed {
            self.stop(send);
            self.schedule
                .restart(moment.place.first_wait(moment.frequency));
        }
        self.next = None;
        self.place = Some(moment.place.clone());
    }

    fn stop(&mut self, send: &mut dyn FnMut(Command)) {
        if self.playing.take().is_some() {
            send(Command::Music(MusicCommand::FadeOut));
        }
    }

    fn theme_of(&self, place: &MusicPlace) -> Option<Theme> {
        self.themes
            .iter()
            .find(|theme| match place {
                MusicPlace::Title => theme.def.title,
                MusicPlace::World(Some(dimension)) => theme.heard_in(&dimension.0),
                MusicPlace::World(None) => false,
            })
            .cloned()
    }

    /// A recording of the pack, or a piece (never the last one heard), asked to the
    /// render thread now so that it is ready when the silence ends.
    fn choose(&mut self, theme: &Theme) -> Option<Next> {
        let roll = self.rng.next_u64();
        if !theme.recordings.is_empty() {
            let count = theme.recordings.len();
            let mut index = roll as usize % count;
            let path = |i: usize| Next::Recording(theme.recordings[i].clone());
            if count > 1 && self.last == Some(path(index)) {
                index = (index + 1) % count;
            }
            return Some(path(index));
        }
        let pieces = theme.def.pieces.max(1);
        let mut index = (roll % u64::from(pieces)) as u8;
        if pieces > 1 && self.last == Some(Next::Piece(theme.id.clone(), index)) {
            index = (index + 1) % pieces;
        }
        if !self.is_ready(&theme.id, index) {
            if self.worker.is_none() {
                self.worker = RenderWorker::start(self.cache.clone(), self.themes.clone());
            }
            let Some(worker) = &self.worker else {
                self.broken = true;
                return None;
            };
            worker.ask(theme.clone(), index);
        }
        Some(Next::Piece(theme.id.clone(), index))
    }

    fn is_ready(&self, theme: &Identifier, index: u8) -> bool {
        self.ready.iter().any(|(t, i, _)| t == theme && *i == index)
    }

    fn collect_renders(&mut self) {
        let Some(worker) = &self.worker else {
            return;
        };
        for (theme, index, result) in worker.results() {
            match result {
                Ok(path) => self.ready.push((theme, index, path)),
                Err(error) => {
                    tracing::warn!(%theme, index, %error, "music could not be rendered: no music");
                    self.broken = true;
                    self.next = None;
                }
            }
        }
    }

    fn ready_source(&self) -> Option<Source> {
        match self.next.as_ref()? {
            Next::Recording(path) => Some(Source::Recording(path.clone())),
            Next::Piece(theme, index) => self
                .ready
                .iter()
                .find(|(t, i, _)| t == theme && i == index)
                .map(|(.., path)| Source::Cached(path.clone())),
        }
    }

    fn start(&mut self, source: Source, send: &mut dyn FnMut(Command)) {
        let Some(feed) = spawn_reader(source) else {
            self.broken = true;
            return;
        };
        self.playing = Some(Arc::clone(&feed.done));
        send(Command::Music(MusicCommand::Start(feed)));
        self.schedule.started();
        self.last = self.next.take();
    }
}
