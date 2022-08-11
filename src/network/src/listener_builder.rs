use crate::{Listener, PublicEndpointConfig};

use super::websocket_listener::WebSocketListener;

#[derive(Debug, Default)]
pub struct ListenerBuilder {
    configuration: PublicEndpointConfig,
}

impl ListenerBuilder {
    pub fn configure(mut self, configuration: PublicEndpointConfig) -> Self {
        self.configuration = configuration;

        self
    }

    pub async fn build(self) -> Result<impl Listener, anyhow::Error> {
        WebSocketListener::bind(
            &self.configuration.serve_interface,
            self.configuration.serve_port,
        )
        .await
    }
}
