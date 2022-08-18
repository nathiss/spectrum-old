use spectrum_packet::model::{ClientMessage, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub(crate) struct Player {
    client_rx: UnboundedReceiver<ClientMessage>,
    server_tx: UnboundedSender<ServerMessage>,
}

impl Player {
    pub fn new(
        client_rx: UnboundedReceiver<ClientMessage>,
        server_tx: UnboundedSender<ServerMessage>,
    ) -> Self {
        Self {
            client_rx,
            server_tx,
        }
    }
}
