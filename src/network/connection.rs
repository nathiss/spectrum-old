use std::net::SocketAddr;

use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;

#[async_trait]
pub trait Connection {
    /// Returns a receiver of peer's incoming data.
    ///
    /// This method can only be called once.
    fn get_incoming_data_channel(&mut self) -> UnboundedReceiver<Vec<u8>>;

    async fn write_bytes(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error>;

    fn addr(&self) -> &SocketAddr;
}
