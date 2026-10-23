use crate::TickClock;
use std::thread::{self, JoinHandle};
use std::time::Instant;
use strata_protocol::{ClientMessage, ServerConnection, ServerMessage};

/// Server running on its own thread in the game process (single player).
pub struct IntegratedServer {
    connection: ServerConnection,
    tick: u64,
}

impl IntegratedServer {
    pub fn spawn(connection: ServerConnection) -> std::io::Result<JoinHandle<()>> {
        thread::Builder::new().name("server".into()).spawn(move || {
            Self {
                connection,
                tick: 0,
            }
            .run()
        })
    }

    fn run(mut self) {
        tracing::info!("integrated server started");
        let mut clock = TickClock::new(Instant::now());
        while self.handle_messages() {
            for _ in 0..clock.due_ticks(Instant::now()) {
                self.tick();
            }
            thread::sleep(clock.next_tick().saturating_duration_since(Instant::now()));
        }
        tracing::info!(tick = self.tick, "integrated server stopped");
    }

    /// Returns `false` when the client is gone.
    fn handle_messages(&mut self) -> bool {
        while let Ok(received) = self.connection.try_recv() {
            let Some(message) = received else {
                return true;
            };
            if !self.handle(message) {
                return false;
            }
        }
        false
    }

    fn handle(&mut self, message: ClientMessage) -> bool {
        match message {
            ClientMessage::Disconnect => false,
        }
    }

    #[tracing::instrument(skip(self), level = "trace")]
    fn tick(&mut self) {
        self.tick += 1;
        self.connection
            .send(ServerMessage::Tick { tick: self.tick });
    }
}
