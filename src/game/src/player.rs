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
        client_rx: Arc<Mutex<UnboundedReceiver<ClientMessage>>>,
        server_tx: Arc<Mutex<UnboundedSender<ServerMessage>>>,
    ) -> Self {
        Self {
            client_rx,
            server_tx,
        }
    }

    pub fn get_client_stream(&self) -> Arc<Mutex<UnboundedReceiver<ClientMessage>>> {
        self.client_rx.clone()
    }

    pub fn get_server_sink(&self) -> Arc<Mutex<UnboundedSender<ServerMessage>>> {
        self.server_tx.clone()
    }
}

#[allow(clippy::from_over_into)]
impl
    Into<(
        Arc<Mutex<UnboundedReceiver<ClientMessage>>>,
        Arc<Mutex<UnboundedSender<ServerMessage>>>,
    )> for Player
{
    fn into(
        self,
    ) -> (
        Arc<Mutex<UnboundedReceiver<ClientMessage>>>,
        Arc<Mutex<UnboundedSender<ServerMessage>>>,
    ) {
        (self.client_rx, self.server_tx)
    }
}
