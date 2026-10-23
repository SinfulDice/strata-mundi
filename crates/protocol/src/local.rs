use crate::{ClientMessage, ServerMessage};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

/// In-memory transport used in single player: the same messages a network would carry.
pub fn local_pair() -> (ClientConnection, ServerConnection) {
    let (to_server, from_client) = channel();
    let (to_client, from_server) = channel();
    (
        ClientConnection {
            outgoing: to_server,
            incoming: from_server,
        },
        ServerConnection {
            outgoing: to_client,
            incoming: from_client,
        },
    )
}

pub struct ClientConnection {
    outgoing: Sender<ClientMessage>,
    incoming: Receiver<ServerMessage>,
}

pub struct ServerConnection {
    outgoing: Sender<ServerMessage>,
    incoming: Receiver<ClientMessage>,
}

impl ClientConnection {
    /// Returns `false` once the server is gone.
    pub fn send(&self, message: ClientMessage) -> bool {
        self.outgoing.send(message).is_ok()
    }

    pub fn try_recv(&self) -> Result<Option<ServerMessage>, Disconnected> {
        receive(&self.incoming)
    }
}

impl ServerConnection {
    /// Returns `false` once the client is gone.
    pub fn send(&self, message: ServerMessage) -> bool {
        self.outgoing.send(message).is_ok()
    }

    pub fn try_recv(&self) -> Result<Option<ClientMessage>, Disconnected> {
        receive(&self.incoming)
    }
}

/// The other side of the connection is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disconnected;

fn receive<T>(receiver: &Receiver<T>) -> Result<Option<T>, Disconnected> {
    match receiver.try_recv() {
        Ok(message) => Ok(Some(message)),
        Err(TryRecvError::Empty) => Ok(None),
        Err(TryRecvError::Disconnected) => Err(Disconnected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_cross_the_local_pair() {
        let (client, server) = local_pair();
        assert!(client.send(ClientMessage::Disconnect));
        assert_eq!(server.try_recv(), Ok(Some(ClientMessage::Disconnect)));
        assert!(server.send(ServerMessage::Tick { tick: 7 }));
        assert_eq!(client.try_recv(), Ok(Some(ServerMessage::Tick { tick: 7 })));
    }

    #[test]
    fn dropped_client_is_detected() {
        let (client, server) = local_pair();
        drop(client);
        assert_eq!(server.try_recv(), Err(Disconnected));
    }
}
