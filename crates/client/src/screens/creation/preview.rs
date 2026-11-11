//! Map of the generation editor, drawn on a background thread: only the newest request
//! counts, older ones are cancelled while they run and their results dropped.

use crate::generation::{GenerationError, MapRequest, MapSource};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Shown when drawing the map failed.
pub const FAILED: &str = "ui-gen-preview-failed";

/// A finished map.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewImage {
    /// Number of the request it answers; a new number means new pixels to upload.
    pub request: u64,
    pub size: (u32, u32),
    pub pixels: Vec<[u8; 3]>,
}

/// Request number, map size and pixels.
type Answer = (u64, (u32, u32), Result<Vec<[u8; 3]>, GenerationError>);

pub struct Preview {
    requests: Sender<(u64, MapRequest)>,
    answers: Receiver<Answer>,
    latest: Arc<AtomicU64>,
    asked: Option<MapRequest>,
    pub image: Option<PreviewImage>,
    pub error: Option<GenerationError>,
}

impl Preview {
    /// `None` if the thread could not start (no preview then).
    pub fn start(source: Arc<dyn MapSource>) -> Option<Self> {
        let (requests, jobs) = unbounded();
        let (replies, answers) = unbounded();
        let latest = Arc::new(AtomicU64::new(0));
        let newest = Arc::clone(&latest);
        let spawned = std::thread::Builder::new()
            .name("map-preview".into())
            .spawn(move || work(source.as_ref(), &jobs, &replies, &newest));
        if let Err(error) = spawned {
            tracing::warn!(%error, "no map preview: thread not started");
            return None;
        }
        Some(Self {
            requests,
            answers,
            latest,
            asked: None,
            image: None,
            error: None,
        })
    }

    /// Asks for `request` unless it is the one asked last.
    pub fn request(&mut self, request: MapRequest) {
        if self.asked.as_ref() == Some(&request) {
            return;
        }
        let number = self.latest.fetch_add(1, Ordering::SeqCst) + 1;
        self.asked = Some(request.clone());
        if self.requests.send((number, request)).is_err() {
            tracing::warn!("map preview thread gone");
        }
    }

    /// The map being drawn is no longer wanted (the editor was left).
    pub fn cancel(&mut self) {
        self.latest.fetch_add(1, Ordering::SeqCst);
        self.asked = None;
    }

    /// Takes the answers in; `true` if the image or the error changed.
    pub fn poll(&mut self) -> bool {
        let latest = self.latest.load(Ordering::SeqCst);
        let mut changed = false;
        for (number, size, answer) in self.answers.try_iter() {
            if number != latest {
                continue;
            }
            changed = true;
            match answer {
                Ok(pixels) => {
                    self.image = Some(PreviewImage {
                        request: number,
                        size,
                        pixels,
                    });
                    self.error = None;
                }
                Err(error) => self.error = Some(error),
            }
        }
        changed
    }

    /// A newer request than the image shown is being drawn.
    pub fn is_busy(&self) -> bool {
        let latest = self.latest.load(Ordering::SeqCst);
        self.error.is_none() && self.image.as_ref().is_none_or(|i| i.request != latest)
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Ends when the screen drops its `Preview`.
fn work(
    source: &dyn MapSource,
    jobs: &Receiver<(u64, MapRequest)>,
    replies: &Sender<Answer>,
    latest: &AtomicU64,
) {
    while let Ok(mut job) = jobs.recv() {
        while let Ok(newer) = jobs.try_recv() {
            job = newer;
        }
        let (number, request) = job;
        let cancelled = || latest.load(Ordering::SeqCst) != number;
        // A bug in the generator must not leave the editor waiting forever.
        let drawn = panic::catch_unwind(AssertUnwindSafe(|| source.render(&request, &cancelled)));
        let answer = match drawn {
            Ok(Ok(Some(pixels))) => Ok(pixels),
            Ok(Ok(None)) => continue,
            Ok(Err(error)) => Err(error),
            Err(_) => Err(GenerationError::new(FAILED)),
        };
        if replies.send((number, request.size, answer)).is_err() {
            return;
        }
    }
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
