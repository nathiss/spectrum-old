use spectrum_packet::model::{ClientMessage, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub(crate) struct Player {
    _client_rx: UnboundedReceiver<ClientMessage>,
    _server_tx: UnboundedSender<ServerMessage>,
}

impl Player {
    pub fn new(
        _client_rx: UnboundedReceiver<ClientMessage>,
        _server_tx: UnboundedSender<ServerMessage>,
    ) -> Self {
        Self {
            _client_rx,
            _server_tx,
        }
    }
}
