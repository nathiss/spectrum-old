use serde::Deserialize;

/// This struct holds the configuration for the game state which manages all game-level components of the system.
#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
pub struct GameStateConfig {}
