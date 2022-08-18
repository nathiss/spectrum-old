use serde::Deserialize;

/// This struct holds the configuration for the game state which manages all game-level components of the system.
#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
pub struct GameStateConfig {
    /// This fields holds the maximum number of players with which the game room will start.
    pub number_of_players_in_game_room: usize,
}
