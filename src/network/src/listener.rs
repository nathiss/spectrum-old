use async_trait::async_trait;

use super::Connection;

#[async_trait]
pub trait Listener: Sized {
    type C: Connection;

    async fn accept(&mut self) -> Option<Self::C>;

    async fn accept_once(&mut self) -> Option<Self::C>;
}
