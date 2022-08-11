use std::time::Duration;

use async_trait::async_trait;

use crate::WebSocketConnection;

/// This trait represents an abstraction over a local network listener.
#[async_trait]
pub trait Listener: Sized {
    /// This method accepts a single *successful* incoming connection.
    ///
    /// This method does not complete if a new connection failed to complete the WebSocket handshake. The connection is
    /// dropped and the listener continues to await the new connection unless an internal listener error occurs.
    ///
    /// # Returns
    ///
    /// A `WebSocketConnection` instance is returned in case of success or `None` if any error occurs.
    async fn accept(&mut self) -> Option<WebSocketConnection>;

    /// This method accepts a single incoming connection.
    ///
    /// # Returns
    ///
    /// A `WebSocketConnection` instance is returned in case of success or `None` if any error occurs.
    async fn accept_once(&mut self) -> Option<WebSocketConnection>;

    /// This method is used to set the timeout for the WebSocket handshake.
    ///
    /// # Arguments
    ///
    /// * `handshake_timeout` - A maximum span of time after which the network connection will be timed-out due to
    ///                         unsuccessful WebSocket handshake completion.
    fn set_handshake_timeout(&mut self, handshake_timeout: Duration);
}
