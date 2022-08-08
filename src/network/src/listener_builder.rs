use crate::Listener;

use super::websocket_listener::WebSocketListener;

#[derive(Debug, Default)]
pub struct ListenerBuilder<'a> {
    interface: &'a str,
    port: u16,
}

impl<'a> ListenerBuilder<'a> {
    pub fn set_interface(mut self, interface: &'a str) -> Self {
        self.interface = interface;

        self
    }

    pub fn set_port(mut self, port: u16) -> Self {
        self.port = port;

        self
    }

    pub async fn build(self) -> Result<impl Listener, anyhow::Error> {
        WebSocketListener::bind(self.interface, self.port).await
    }
}
