use async_trait::async_trait;

use super::Connection;

#[async_trait]
pub trait Listener<C: Connection>: Sized {
    async fn bind<'a>(interface: &'a str, port: u16) -> Result<Self, anyhow::Error>;

    async fn accept(&mut self) -> Option<C>;
}
