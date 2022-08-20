/// This enum contains all possible state of a [`GameLobby`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameLobbyStatus {
    /// This value indicates that the requirements for the game to start has not yet been met.
    Waiting,

    /// This value indicates that the game lobby is ready to start the game
    Ready,
}

impl Default for GameLobbyStatus {
    fn default() -> Self {
        Self::Waiting
    }
}
