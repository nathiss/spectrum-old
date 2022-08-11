use serde::Deserialize;
use spectrum_network::PublicEndpointConfig;

/// This struct is used to hold server configuration.
#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
pub struct ServerConfig {
    /// This field holds the configuration for the public endpoint.
    pub public_endpoint: PublicEndpointConfig,
}
