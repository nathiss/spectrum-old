use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;
use log::{debug, error, info};
use tokio::{
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_tungstenite::accept_async_with_config;
use tungstenite::protocol::WebSocketConfig;

use super::{websocket_connection::WebSocketConnection, Listener};

static DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
static DEFAULT_WEBSOCKET_CONFIG: Option<WebSocketConfig> = Some(WebSocketConfig {
    max_message_size: Some(16 << 20 /* 16 MiB */),
    max_frame_size: Some(1 << 20 /* 1 MiB */),
    max_send_queue: None, /* unlimited */
    accept_unmasked_frames: false,
});

enum NewConnectionResult {
    Ok(WebSocketConnection),
    HandshakeFailed(SocketAddr),
    Timeout(SocketAddr),
    ListenerError,
}

/// This struct represents a local listener able to accept incoming WebSocket connection.
#[derive(Debug)]
pub(super) struct WebSocketListener {
    tcp_listener: TcpListener,
    handshake_timeout: Duration,
}

#[async_trait]
impl Listener for WebSocketListener {
    async fn accept(&mut self) -> Option<WebSocketConnection> {
        loop {
            match self.inner_accept().await {
                NewConnectionResult::Ok(ws_stream) => break Some(ws_stream),
                NewConnectionResult::HandshakeFailed(_addr)
                | NewConnectionResult::Timeout(_addr) => {
                    // The error was already logged internally by `inner_accept`. We await the next successful
                    // WebSocket connection.
                }
                NewConnectionResult::ListenerError => {
                    // The error was already logged internally by `inner_accept.` In case of internal listener error we
                    // cannot accept new connection.
                    break None;
                }
            }
        }
    }

    async fn accept_once(&mut self) -> Option<WebSocketConnection> {
        match self.inner_accept().await {
            NewConnectionResult::Ok(ws_stream) => Some(ws_stream),
            _ => {
                // Certain errors were already logged internally by `inner_accept`. No need to log them again.

                None
            }
        }
    }

    fn set_handshake_timeout(&mut self, handshake_timeout: Duration) {
        self.handshake_timeout = handshake_timeout;
    }
}

impl WebSocketListener {
    /// This method is used to construct `WebSocketListener` and to bind the internal TCP listener to the interface and
    /// the port.
    ///
    /// # Arguments
    ///
    /// * `interface` - A local interface to which the internal listener will try to bind to. It cannot be empty.
    /// * `port` - A local port number to which the internal listener will try to bind to. It cannot be zero.
    ///
    /// # Returns
    ///
    /// An instance of `Self` is returned in case of success or an error describing a failed constraint or an internal
    /// error if the operation fails.
    pub(super) async fn bind(interface: &str, port: u16) -> Result<Self, anyhow::Error> {
        if interface.is_empty() {
            error!("Given listener interface is empty.");
            return Err(anyhow::Error::msg("Interface cannot be empty"));
        }

        match port {
            0 => {
                error!("Given listener port number is zero.");
                Err(anyhow::Error::msg("Port number must be a positive integer"))
            }
            _ => {
                let tcp_listener = TcpListener::bind((interface, port)).await?;

                info!("Listener bound to {}:{}", interface, port);

                Ok(Self {
                    tcp_listener,
                    handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                })
            }
        }
    }

    /// This method is used to do a single accept of the local listener and then convert the incoming TCP connection
    /// into a `WebSocketConnection`.
    ///
    /// # Returns
    ///
    /// A new `WebSocketConnection` is returned in case of success. Otherwise the error can be returned for a number of
    /// reasons:
    /// * `NewConnectionResult::HandshakeFailed` - An error occurred during the WebSocket handshake with the peer.
    /// * `NewConnectionResult::Timeout` - Failed to complete the WebSocket handshake within the given timeout.
    /// * `NewConnectionResult::ListenerError` - An internal error of the local listener rendering it unable to accept
    ///                                          new connections.
    async fn inner_accept(&self) -> NewConnectionResult {
        match self.tcp_listener.accept().await {
            Ok((stream, addr)) => self.handle_new_stream(stream, addr).await,
            Err(e) => {
                error!(
                    "Received an error while listening for incoming connections: {}",
                    e
                );

                NewConnectionResult::ListenerError
            }
        }
    }

    /// This method is used to process a single TCP connection and convert it into a valid `WebSocketConnection`.
    ///
    /// # Arguments
    ///
    /// * `stream` - A TCP stream connected to the network peer.
    /// * `addr` - An internet socket address of the connected peer.
    ///
    /// # Returns
    ///
    /// A new `WebSocketConnection` is returned in case of success or `addr` if any error occurred.
    async fn handle_new_stream(&self, stream: TcpStream, addr: SocketAddr) -> NewConnectionResult {
        debug!("New TCP connection from: {}", addr);

        match timeout(
            self.handshake_timeout,
            accept_async_with_config(stream, DEFAULT_WEBSOCKET_CONFIG),
        )
        .await
        {
            Ok(Ok(ws_stream)) => {
                debug!("Handshake with {} completed successfully", addr);

                NewConnectionResult::Ok(WebSocketConnection::new(ws_stream, addr))
            }
            Ok(Err(e)) => {
                error!("Failed to complete WebSocket handshake with {}", addr);
                debug!(
                    "Failed to complete WebSocket handshake with {}. Error: {}",
                    addr, e
                );

                NewConnectionResult::HandshakeFailed(addr)
            }
            Err(_e) => {
                error!(
                    "The handshake from {} failed to complete in {}.",
                    addr,
                    self.handshake_timeout.as_secs()
                );

                NewConnectionResult::Timeout(addr)
            }
        }
    }
}
