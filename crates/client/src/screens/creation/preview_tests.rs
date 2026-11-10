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
}

impl MapSource for Fake {
    fn render(
        &self,
        request: &MapRequest,
        _: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Vec<[u8; 3]>>, MapError> {
        if request.seed == "slow" {
            self.started.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
        }
        if request.seed == "broken" {
            return Err(MapError {
                message: "world-settings-no-layers",
                id: None,
            });
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

fn preview() -> (Preview, Receiver<()>, Sender<()>) {
    let (started, on_start) = bounded(1);
    let (release, released) = bounded(1);
    let fake = Fake {
        started,
        release: Mutex::new(released),
    };
    (Preview::start(Arc::new(fake)).unwrap(), on_start, release)
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
