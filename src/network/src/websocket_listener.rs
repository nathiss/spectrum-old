use std::net::SocketAddr;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async_with_config;
use tungstenite::protocol::WebSocketConfig;

use super::{Listener, WebSocketConnection};

#[derive(Debug)]
pub struct WebSocketListener {
    tcp_listener: TcpListener,
}

#[async_trait]
impl Listener<WebSocketConnection> for WebSocketListener {
    async fn bind<'a>(interface: &'a str, port: u16) -> Result<Self, anyhow::Error> {
        let tcp_listener = TcpListener::bind(format!("{}:{}", interface, port)).await?;

        info!("Listener bound to {}", format!("{}:{}", interface, port));

        Ok(Self { tcp_listener })
    }

    async fn accept(&mut self) -> Option<WebSocketConnection> {
        loop {
            match self.tcp_listener.accept().await {
                Ok((stream, addr)) => match handle_new_stream(stream, addr).await {
                    Some(ws_stream) => break Some(ws_stream),
                    None => {} /* We continue the loop waiting for the next valid connection. */
                },
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
    let mut config = WebSocketConfig::default();
    config.max_message_size = Some(16 << 20 /* 16 MiB */);
    config.max_frame_size = Some(1 << 20 /* 1 MiB */);

    Some(config)
}