use serde::Deserialize;

/// This struct holds the configuration for the game state which manages all game-level components of the system.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize)]
pub struct GameStateConfig {
    /// This field holds the maximum number of players with which the game lobby will start.
    pub number_of_players_in_game_lobby: usize,

    /// This field holds the timeout value (in seconds) after which the player will be removed from the lobby.
    pub player_readiness_timeout: u64,
}
