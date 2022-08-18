use async_trait::async_trait;
use spectrum_packet::model::{ClientMessage, ClientWelcome, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::JoinGameResult;

/// This trait provides an interface through which its clients might interact with the global game state.
#[async_trait]
pub trait GameState: Send + Sync {
    /// This method allows to add a new player into a game room.
    ///
    /// The game room is identified based on the information passed inside `welcome_message`.
    ///
    /// # Arguments
    ///
    /// * `welcome_message` - This message contains information with which its passible to uniquely identify a game room
    ///                       and uniquely identify the new players inside the scope of the game room.
    /// * `packet_rx` - This is a stream of player's incoming messages.
    /// * `packet_tx` - This is a sink of server outgoing messages.
    ///
    /// # Returns
    ///
    /// An indication of whether or not the operation was successful is returned. There are several reasons by which the
    /// operation might fail. For more details see [`JoinGameResult`].
    async fn join_game(
        &self,
        welcome_message: ClientWelcome,
        packet_rx: UnboundedReceiver<ClientMessage>,
        packet_tx: UnboundedSender<ServerMessage>,
    ) -> JoinGameResult;
}
