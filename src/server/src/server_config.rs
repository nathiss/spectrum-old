use serde::Deserialize;
use spectrum_game::GameStateConfig;
use spectrum_network::PublicEndpointConfig;

/// This struct is used to hold server configuration.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize)]
pub struct ServerConfig {
    /// This field holds the configuration for the public endpoint.
    pub public_endpoint: PublicEndpointConfig,

    /// This field holds the configuration for the game state.
    pub game_state: GameStateConfig,
}
