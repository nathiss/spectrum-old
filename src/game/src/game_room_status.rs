#[derive(Debug, Clone, Copy)]
pub enum GameRoomStatus {
    Waiting,
    Running,
}

impl Default for GameRoomStatus {
    fn default() -> Self {
        Self::Waiting
    }
}
