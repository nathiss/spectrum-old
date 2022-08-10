use std::time::Duration;

use async_trait::async_trait;

use crate::WebSocketConnection;

#[async_trait]
pub trait Listener: Sized {
    async fn accept(&mut self) -> Option<WebSocketConnection>;

    async fn accept_once(&mut self) -> Option<WebSocketConnection>;

    fn set_handshake_timeout(&mut self, handshake_timeout: Duration);
}
