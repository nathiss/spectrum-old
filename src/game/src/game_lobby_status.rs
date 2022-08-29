/// This enum contains all possible state of a [`GameLobby`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GameLobbyStatus {
    /// This value indicates that the requirements for the game to start has not yet been met.
    #[default]
    Waiting,

    /// This value indicates that the requirements for the game to start has been met.
    ///
    /// Now the lobby will wait for all players to confirm their readiness and after that the game will start. If at
    /// least one of the players fail to confirm, then the start will roll back to `Waiting`.
    Starting,

    /// This value indicates that the game lobby has started the game.
    Started,
}
