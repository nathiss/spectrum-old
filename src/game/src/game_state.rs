use async_trait::async_trait;
use spectrum_packet::model::{ClientMessage, ClientWelcome, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::JoinGameResult;

#[async_trait]
pub trait GameState: Send + Sync {
    async fn join_game(
        &self,
        welcome_message: ClientWelcome,
        packet_rx: UnboundedReceiver<ClientMessage>,
        packet_tx: UnboundedSender<ServerMessage>,
    ) -> JoinGameResult;
}
