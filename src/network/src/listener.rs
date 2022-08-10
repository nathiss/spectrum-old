use std::time::Duration;

use async_trait::async_trait;

use super::Connection;

#[async_trait]
pub trait Listener: Sized {
    type C: Connection;

    async fn accept(&mut self) -> Option<Self::C>;

    async fn accept_once(&mut self) -> Option<Self::C>;

    fn set_handshake_timeout(&mut self, handshake_timeout: Duration);
}
