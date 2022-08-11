use std::net::SocketAddr;

use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;

/// This trait represents a network connection with a single peer.
#[async_trait]
pub trait Connection: Send {
    /// This method returns a receiving half of the network connection.
    ///
    /// The receiver yields only binary packets sent by the peer. It is finished if the network peer closes its
    /// connection or an error occurred during transmission, in which case the connection should be terminated.
    ///
    /// # Panics
    ///
    /// The internal receiver is consumed by this method. It panics if called more than once.
    fn get_incoming_data_channel(&mut self) -> UnboundedReceiver<Vec<u8>>;

    /// This method allows to write packages (represented as a collection of bytes) to the network peer.
    ///
    /// # Returns
    ///
    /// * `()` - if the operation was successful,
    /// * `anyhow::Error` - if an error occurred during package transmission.
    async fn write_bytes(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error>;

    /// This method returns an internet socket address of the connected peer.
    fn addr(&self) -> &SocketAddr;
}
