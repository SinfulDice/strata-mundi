//! Lid closed on the DS (or any Linux machine going to sleep): logind announces it with
//! `PrepareForSleep`; the game pauses and saves, then lets the system sleep. A "delay"
//! inhibitor lock gives it the time to do so (a few seconds, `InhibitDelayMaxSec`).
//! Without logind (other systems, containers) nothing happens.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

/// Longest wait for the save before letting the system sleep anyway.
const SAVE_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepEvent {
    /// About to sleep: pause and save, then call `SleepWatcher::ready`.
    Suspending,
    Resumed,
}

pub struct SleepWatcher {
    events: Receiver<SleepEvent>,
    ready: Sender<()>,
}

impl SleepWatcher {
    /// Starts listening on a background thread; `None` if the system has no logind.
    pub fn start() -> Option<Self> {
        let (event_sender, events) = channel();
        let (ready, ready_receiver) = channel();
        let started = std::thread::Builder::new()
            .name("sleep-watch".into())
            .spawn(move || {
                if let Err(error) = watch(&event_sender, &ready_receiver) {
                    tracing::info!(%error, "sleep notifications unavailable");
                }
            });
        started.ok().map(|_| Self { events, ready })
    }

    pub fn poll(&self) -> Option<SleepEvent> {
        self.events.try_recv().ok()
    }

    /// The world is saved: the system may sleep.
    pub fn ready(&self) {
        let _ = self.ready.send(());
    }
}

#[cfg(target_os = "linux")]
fn watch(events: &Sender<SleepEvent>, ready: &Receiver<()>) -> zbus::Result<()> {
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::OwnedFd;

    let connection = Connection::system()?;
    let manager = Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )?;
    let inhibit = || -> zbus::Result<OwnedFd> {
        manager.call(
            "Inhibit",
            &("sleep", "Strata Mundi", "Saving the world", "delay"),
        )
    };
    let mut lock = Some(inhibit()?);
    for signal in manager.receive_signal("PrepareForSleep")? {
        let going_to_sleep: bool = signal.body().deserialize()?;
        if going_to_sleep {
            // A confirmation that came too late last time must not count for this one.
            while ready.try_recv().is_ok() {}
            if events.send(SleepEvent::Suspending).is_err() {
                return Ok(());
            }
            let _ = ready.recv_timeout(SAVE_TIMEOUT);
            drop(lock.take());
        } else {
            let _ = events.send(SleepEvent::Resumed);
            if lock.is_none() {
                lock = inhibit().ok();
            }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn watch(_: &Sender<SleepEvent>, _: &Receiver<()>) -> Result<(), &'static str> {
    Err("only on Linux")
}
