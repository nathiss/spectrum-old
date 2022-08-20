use std::sync::Arc;

use spectrum_packet::model::{ClientMessage, ServerMessage};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

#[derive(Debug)]
pub(crate) struct Player {
    client_rx: Arc<Mutex<UnboundedReceiver<ClientMessage>>>,
    server_tx: Arc<Mutex<UnboundedSender<ServerMessage>>>,
}

impl Player {
    pub fn new(
        client_rx: UnboundedReceiver<ClientMessage>,
        server_tx: UnboundedSender<ServerMessage>,
    ) -> Self {
        Self {
            client_rx: Arc::new(Mutex::new(client_rx)),
            server_tx: Arc::new(Mutex::new(server_tx)),
        }
    }

    pub fn get_client_stream(&self) -> Arc<Mutex<UnboundedReceiver<ClientMessage>>> {
        self.client_rx.clone()
    }

    pub fn get_server_sink(&self) -> Arc<Mutex<UnboundedSender<ServerMessage>>> {
        self.server_tx.clone()
    }
}
