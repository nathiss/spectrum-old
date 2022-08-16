use spectrum_packet::model::{ClientMessage, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub enum JoinGameResult {
    Ok,
    GameIsFull(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),
    GameDoesNotExit(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),
    NickTaken(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),
    BadRequest(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),
}

impl Default for JoinGameResult {
    fn default() -> Self {
        JoinGameResult::Ok
    }
}
