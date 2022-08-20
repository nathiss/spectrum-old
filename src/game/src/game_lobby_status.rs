/// This enum contains all possible state of a [`GameLobby`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GameLobbyStatus {
    /// This value indicates that the requirements for the game to start has not yet been met.
    #[default]
    Waiting,

    /// This value indicates that the game lobby is ready to start the game
    Ready,
}
