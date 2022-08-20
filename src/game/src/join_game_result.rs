use spectrum_packet::model::{ClientMessage, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// This enum contains all possible outcomes of adding a plyer into a game lobby.
#[derive(Debug, Default)]
pub enum JoinGameResult {
    /// This value indicates that the operation was successful.
    ///
    /// No data is returned, because the player's associated objects were moved into the other parts of the system and
    /// they are now responsible for managing player's connection.
    #[default]
    Ok,

    /// This value indicates the the operation was unsuccessful due to the fact that the targeted game is full.
    ///
    /// The associated data represents both sides of the player's connection. The owner of this value is now responsible
    /// for managing player's connection.
    GameIsFull(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),

    /// This value indicates the the operation was unsuccessful due to the fact that the targeted game does not exist
    /// or otherwise is in a invalid state (e.g. the game has started).
    ///
    /// The associated data represents both sides of the player's connection. The owner of this value is now responsible
    /// for managing player's connection.
    GameDoesNotExit(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),

    /// This value indicates the the operation was unsuccessful due to the fact that the specified nick has already been
    /// taken by another player from the same game room.
    ///
    /// The associated data represents both sides of the player's connection. The owner of this value is now responsible
    /// for managing player's connection.
    NickTaken(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),

    /// This value indicates the the operation was unsuccessful due to the fact that the player's welcome message has
    /// been ill-formed.
    ///
    /// The associated data represents both sides of the player's connection. The owner of this value is now responsible
    /// for managing player's connection.
    ///
    /// See: definition of Client's WelcomeMessage.
    BadRequest(
        UnboundedReceiver<ClientMessage>,
        UnboundedSender<ServerMessage>,
    ),
}
