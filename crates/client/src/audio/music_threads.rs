//! The two background jobs of the music: rendering pieces into the cache (one thread,
//! one piece at a time, started only when music is wanted) and reading the playing piece
//! a chunk ahead of the mixer.

use super::decode::{Resampler, downmix_into};
use super::stream::MusicFeed;
use crossbeam_channel::{Receiver, Sender};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use strata_core::audio::SAMPLE_RATE;
use strata_core::audio::music::{Mood, Piece, Theme, WavStream, cached_piece, prune};
use strata_core::id::Identifier;

/// Samples sent to the mixer at once (half a second).
const CHUNK: usize = SAMPLE_RATE as usize / 2;
/// Chunks read ahead of the mixer.
const AHEAD: usize = 4;
/// Played chunks waiting to be filled again (more than can be in flight).
const SPARE: usize = AHEAD + 2;

pub type RenderResult = (Identifier, u8, io::Result<PathBuf>);

enum Job {
    /// New packs: the cache is cleaned of what they no longer use.
    Themes(Vec<Theme>),
    Render(Box<Theme>, u8, u64),
}

/// The one render thread of the session (kept over reloads of the packs, so two never
/// write or clean the cache at once).
pub struct RenderWorker {
    jobs: Sender<Job>,
    results: Receiver<(u64, RenderResult)>,
    /// Bumped by each reload: older jobs are skipped and their results ignored.
    generation: Arc<AtomicU64>,
}

impl RenderWorker {
    /// The thread, which first removes the cached files no current piece uses.
    pub fn start(cache: PathBuf, themes: Vec<Theme>) -> Option<Self> {
        let (jobs, job_receiver) = crossbeam_channel::unbounded::<Job>();
        let (result_sender, results) = crossbeam_channel::unbounded();
        let generation = Arc::new(AtomicU64::new(0));
        let current = Arc::clone(&generation);
        let spawned = std::thread::Builder::new()
            .name("music-render".to_owned())
            .spawn(move || {
                for job in job_receiver {
                    match job {
                        Job::Themes(themes) => clean(&cache, &themes),
                        Job::Render(theme, index, asked) => {
                            if asked != current.load(Ordering::Acquire) {
                                continue;
                            }
                            let result = render(&cache, &theme, index);
                            if result_sender
                                .send((asked, (theme.id, index, result)))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            });
        match spawned {
            Ok(_) => {
                let worker = Self {
                    jobs,
                    results,
                    generation,
                };
                worker.set_themes(themes);
                Some(worker)
            }
            Err(error) => {
                tracing::warn!(%error, "no music render thread: no music");
                None
            }
        }
    }

    /// New packs: what was asked before is dropped.
    pub fn renew(&self, themes: Vec<Theme>) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.set_themes(themes);
    }

    fn set_themes(&self, themes: Vec<Theme>) {
        let _ = self.jobs.send(Job::Themes(themes));
    }

    pub fn ask(&self, theme: Theme, index: u8) {
        let generation = self.generation.load(Ordering::Acquire);
        let _ = self
            .jobs
            .send(Job::Render(Box::new(theme), index, generation));
    }

    /// The next piece rendered for the current packs.
    pub fn result(&self) -> Option<RenderResult> {
        let generation = self.generation.load(Ordering::Acquire);
        self.results
            .try_iter()
            .find_map(|(asked, result)| (asked == generation).then_some(result))
    }
}

fn clean(cache: &Path, themes: &[Theme]) {
    match prune(cache, themes) {
        Ok(0) => {}
        Ok(removed) => tracing::info!(removed, "old music removed from the cache"),
        Err(error) => tracing::warn!(%error, "music cache could not be cleaned"),
    }
}

fn render(cache: &Path, theme: &Theme, index: u8) -> io::Result<PathBuf> {
    let started = Instant::now();
    let piece = Piece {
        theme,
        index,
        mood: Mood::Calm,
    };
    let cached = cached_piece(cache, piece)?;
    if cached.rendered {
        let seconds = started.elapsed().as_secs_f32();
        tracing::info!(theme = %theme.id, index, seconds, "music rendered into the cache");
    }
    Ok(cached.path)
}

/// Where a piece comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Cached(PathBuf),
    /// A pack's `.ogg`.
    Recording(PathBuf),
}

/// A thread reading `source` a few chunks ahead; it stops once the mixer lets go.
pub fn spawn_reader(source: Source) -> Option<MusicFeed> {
    let (sender, chunks) = crossbeam_channel::bounded(AHEAD);
    let (recycle, spares) = crossbeam_channel::bounded(SPARE);
    let spawned = std::thread::Builder::new()
        .name("music-read".to_owned())
        .spawn(move || {
            let chunks = Chunks { sender, spares };
            let result = match &source {
                Source::Cached(path) => read_cached(path, &chunks),
                Source::Recording(path) => read_recording(path, &chunks),
            };
            if let Err(error) = result {
                tracing::warn!(?source, %error, "music could not be read");
            }
        });
    match spawned {
        Ok(_) => Some(MusicFeed {
            chunks,
            recycle,
            done: Arc::new(AtomicBool::new(false)),
        }),
        Err(error) => {
            tracing::warn!(%error, "no music reader thread");
            None
        }
    }
}

/// Where chunks go, and the played ones come back from.
struct Chunks {
    sender: Sender<Vec<f32>>,
    spares: Receiver<Vec<f32>>,
}

impl Chunks {
    fn spare(&self) -> Vec<f32> {
        self.spares
            .try_recv()
            .unwrap_or_else(|_| Vec::with_capacity(CHUNK))
    }

    /// False once the mixer let the piece go.
    fn send(&self, chunk: Vec<f32>) -> bool {
        self.sender.send(chunk).is_ok()
    }
}

fn read_cached(path: &Path, chunks: &Chunks) -> io::Result<()> {
    let mut stream = WavStream::open(path)?;
    loop {
        let mut chunk = chunks.spare();
        stream.read_into(&mut chunk, CHUNK)?;
        if chunk.is_empty() || !chunks.send(chunk) {
            return Ok(());
        }
    }
}

fn read_recording(path: &Path, chunks: &Chunks) -> io::Result<()> {
    let file = std::fs::File::open(path)?;
    let decoder = rodio::Decoder::try_from(file).map_err(io::Error::other)?;
    let channels = usize::from(rodio::Source::channels(&decoder).get());
    let mut resampler = Resampler::new(rodio::Source::sample_rate(&decoder).get());
    let mut samples = decoder.into_iter();
    let (mut interleaved, mut mono) = (Vec::new(), Vec::new());
    loop {
        interleaved.clear();
        interleaved.extend(samples.by_ref().take(CHUNK * channels));
        if interleaved.is_empty() {
            return Ok(());
        }
        downmix_into(&interleaved, channels, &mut mono);
        let mut chunk = chunks.spare();
        chunk.clear();
        resampler.push(&mono, &mut chunk);
        if !chunk.is_empty() && !chunks.send(chunk) {
            return Ok(());
        }
    }
}
