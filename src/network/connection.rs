use std::net::SocketAddr;

use async_trait::async_trait;

#[async_trait]
pub trait Connection {
    async fn read_bytes(&mut self) -> Option<Vec<u8>>;

    async fn write_bytes(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error>;

    fn addr(&self) -> &SocketAddr;
}
