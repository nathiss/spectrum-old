/// This enum contains all possible state of a [`GameRoom`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameRoomStatus {
    /// This value indicates that the requirements for the game to start has not yet been met.
    Waiting,

    /// This value indicates that the game room is currently
    Running,
}

impl Default for GameRoomStatus {
    fn default() -> Self {
        Self::Waiting
    }
}
