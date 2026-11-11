#![allow(clippy::unwrap_used)]

use super::*;
use crossbeam_channel::bounded;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use strata_core::generation::WorldgenSettings;

/// Fills the map with one gray per seed; seed "slow" waits for `release`, ignoring
/// cancellation, to answer after a newer request.
struct Fake {
    started: Sender<()>,
    release: Mutex<Receiver<()>>,
    /// Told when the "endless" map gives up.
    stopped: Sender<()>,
}

impl MapSource for Fake {
    fn render(
        &self,
        request: &MapRequest,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Vec<[u8; 3]>>, GenerationError> {
        if request.seed == "slow" {
            self.started.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
        }
        if request.seed == "endless" {
            self.started.send(()).unwrap();
            while !cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
            self.stopped.send(()).unwrap();
            return Ok(None);
        }
        if request.seed == "panic" {
            panic!("generator bug");
        }
        if request.seed == "broken" {
            return Err(GenerationError::new("world-settings-no-layers"));
        }
        let gray = request.seed.len() as u8;
        let (w, h) = request.size;
        Ok(Some(vec![[gray; 3]; (w * h) as usize]))
    }
}

fn request(seed: &str, size: (u32, u32)) -> MapRequest {
    MapRequest {
        seed: seed.to_owned(),
        settings: WorldgenSettings::default(),
        size,
        scale: 4,
    }
}

struct Harness {
    preview: Preview,
    started: Receiver<()>,
    release: Sender<()>,
    stopped: Receiver<()>,
}

fn harness() -> Harness {
    let (started, on_start) = bounded(1);
    let (release, released) = bounded(1);
    let (stopped, on_stop) = bounded(1);
    let fake = Fake {
        started,
        release: Mutex::new(released),
        stopped,
    };
    Harness {
        preview: Preview::start(Arc::new(fake)).unwrap(),
        started: on_start,
        release,
        stopped: on_stop,
    }
}

fn preview() -> (Preview, Receiver<()>, Sender<()>) {
    let h = harness();
    (h.preview, h.started, h.release)
}

fn wait_for_change(preview: &mut Preview) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !preview.poll() {
        assert!(
            Instant::now() < deadline,
            "no answer from the preview thread"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn an_outdated_map_never_replaces_a_newer_one() {
    let (mut preview, started, release) = preview();
    preview.request(request("slow", (8, 8)));
    started.recv().unwrap();
    preview.request(request("fast!", (6, 4)));
    assert!(preview.is_busy());
    release.send(()).unwrap();
    wait_for_change(&mut preview);
    let image = preview.image.clone().unwrap();
    assert_eq!(image.request, 2);
    assert_eq!(image.size, (6, 4));
    assert_eq!(image.pixels, vec![[5; 3]; 24]);
    assert!(!preview.is_busy());
    std::thread::sleep(Duration::from_millis(20));
    assert!(!preview.poll(), "the slow map arrived late and was dropped");
    assert_eq!(preview.image, Some(image));
}

#[test]
fn the_same_request_is_not_drawn_twice() {
    let (mut preview, _, _) = preview();
    preview.request(request("same", (3, 3)));
    wait_for_change(&mut preview);
    preview.request(request("same", (3, 3)));
    assert!(!preview.is_busy());
    assert_eq!(preview.image.as_ref().map(|i| i.request), Some(1));
}

#[test]
fn errors_replace_the_waiting() {
    let (mut preview, _, _) = preview();
    preview.request(request("broken", (3, 3)));
    wait_for_change(&mut preview);
    assert_eq!(
        preview.error.as_ref().map(|e| e.message),
        Some("world-settings-no-layers")
    );
    assert!(!preview.is_busy());
}

#[test]
fn leaving_the_editor_cancels_the_map() {
    let mut h = harness();
    h.preview.request(request("endless", (3, 3)));
    h.started.recv().unwrap();
    h.preview.cancel();
    h.stopped.recv_timeout(Duration::from_secs(10)).unwrap();
    let mut h = harness();
    h.preview.request(request("endless", (3, 3)));
    h.started.recv().unwrap();
    let stopped = h.stopped.clone();
    drop(h);
    stopped.recv_timeout(Duration::from_secs(10)).unwrap();
}

#[test]
fn a_crash_while_drawing_is_reported() {
    let (mut preview, _, _) = preview();
    preview.request(request("panic", (3, 3)));
    wait_for_change(&mut preview);
    assert_eq!(preview.error.as_ref().map(|e| e.message), Some(FAILED));
    assert!(!preview.is_busy());
}
