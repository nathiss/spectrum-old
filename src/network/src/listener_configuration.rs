use serde::Deserialize;

/// This struct holds the configuration for the public endpoint to which the clients will try to connect to.
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize)]
pub struct PublicEndpointConfig {
    /// This field holds an interface to which the local listener will try to bind.
    ///
    /// Players will use this interface to connect to the public endpoint.
    pub serve_interface: String,

    /// This field holds a port number to which the local listener will try to bind.
    ///
    /// Players will use this port number to connect to the public endpoint.
    pub serve_port: u16,
}
