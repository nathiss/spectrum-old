use std::net::SocketAddr;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async_with_config;
use tungstenite::protocol::WebSocketConfig;

use super::{websocket_connection::WebSocketConnection, Listener};

#[derive(Debug)]
pub(super) struct WebSocketListener {
    tcp_listener: TcpListener,
}

#[async_trait]
impl Listener for WebSocketListener {
    type C = WebSocketConnection;

    async fn accept(&mut self) -> Option<Self::C> {
        loop {
            match self.tcp_listener.accept().await {
                Ok((stream, addr)) => {
                    if let Some(ws_stream) = handle_new_stream(stream, addr).await {
                        break Some(ws_stream);
                    }

                    /* We continue the loop waiting for the next valid connection. */
                }
                Err(e) => {
                    error!(
                        "Received an error while listening for incoming connections: {}",
                        e
                    );

                    break None;
                }
            }
        }
    }
}

impl WebSocketListener {
    pub(super) async fn bind<'a>(interface: &'a str, port: u16) -> Result<Self, anyhow::Error> {
        let tcp_listener = TcpListener::bind(format!("{}:{}", interface, port)).await?;

        info!("Listener bound to {}", format!("{}:{}", interface, port));

        Ok(Self { tcp_listener })
    }
}

async fn handle_new_stream(stream: TcpStream, addr: SocketAddr) -> Option<WebSocketConnection> {
    debug!("New connection from: {}", addr);

    match accept_async_with_config(stream, get_websocket_config()).await {
        Ok(ws_stream) => {
            debug!("Handshake with {} completed successfully", addr);

            Some(WebSocketConnection::new(ws_stream, addr))
        }
        Err(e) => {
            warn!(
                "Failed to complete WebSocket handshake with {}; reason: {}",
                addr, e
            );

            None
        }
    }
}

#[inline]
fn get_websocket_config() -> Option<WebSocketConfig> {
    Some(WebSocketConfig {
        max_message_size: Some(16 << 20 /* 16 MiB */),
        max_frame_size: Some(1 << 20 /* 1 MiB */),
        ..Default::default()
    })
}
