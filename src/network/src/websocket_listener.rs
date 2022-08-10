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

static HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(super) struct WebSocketListener {
    tcp_listener: TcpListener,
}

#[async_trait]
impl Listener for WebSocketListener {
    type C = WebSocketConnection;

    async fn accept(&mut self) -> Option<Self::C> {
        loop {
            match self.inner_accept().await {
                Ok(Some(ws_stream)) => break Some(ws_stream),
                Ok(None) => { /* We continue and wait for the next valid connection. */ }
                Err(_) => break None,
            }
        }
    }

    async fn accept_once(&mut self) -> Option<Self::C> {
        match self.inner_accept().await {
            Ok(Some(ws_stream)) => Some(ws_stream),
            Ok(None) => None,
            Err(_) => None,
        }
    }
}

impl WebSocketListener {
    pub(super) async fn bind(interface: &str, port: u16) -> Result<Self, anyhow::Error> {
        if interface.is_empty() {
            return Err(anyhow::Error::msg("Interface cannot be empty"));
        }

        match port {
            0 => Err(anyhow::Error::msg("Port number must be a positive integer")),
            _ => {
                let tcp_listener = TcpListener::bind((interface, port)).await?;

                info!("Listener bound to {}:{}", interface, port);

                Ok(Self { tcp_listener })
            }
        }
    }

    async fn inner_accept(&self) -> Result<Option<WebSocketConnection>, anyhow::Error> {
        match self.tcp_listener.accept().await {
            Ok((stream, addr)) => match handle_new_stream(stream, addr).await {
                Some(ws_stream) => Ok(Some(ws_stream)),
                None => Ok(None),
            },
            Err(e) => {
                error!(
                    "Received an error while listening for incoming connections: {}",
                    e
                );

                Err(anyhow::Error::from(e))
            }
        }
    }
}

async fn handle_new_stream(stream: TcpStream, addr: SocketAddr) -> Option<WebSocketConnection> {
    debug!("New connection from: {}", addr);

    match timeout(
        HANDSHAKE_TIMEOUT,
        accept_async_with_config(stream, get_websocket_config()),
    )
    .await
    {
        Ok(Ok(ws_stream)) => {
            debug!("Handshake with {} completed successfully", addr);

            Some(WebSocketConnection::new(ws_stream, addr))
        }
        Ok(Err(e)) => {
            error!(
                "Failed to complete WebSocket handshake with {}; reason: {}",
                addr, e
            );

            None
        }
        Err(e) => {
            error!(
                "The handshake from {} failed to complete in {}. Error: {}",
                addr,
                HANDSHAKE_TIMEOUT.as_secs(),
                e
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
