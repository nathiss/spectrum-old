use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use log::info;
use spectrum_network::{Connection, Listener, ListenerBuilder};
use spectrum_packet::{ClientMessagePacketSerializer, ServerMessagePacketSerializer};
use tokio::{select, sync::Mutex, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{util::calculate_hash, Client, ServerConfig};

pub struct Server {
    config: ServerConfig,
    new_clients: Arc<Mutex<HashMap<u64, Client>>>,
    cancellation_token: CancellationToken,

    server_join_handles: Vec<JoinHandle<()>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            new_clients: Arc::new(Mutex::new(HashMap::new())),
            cancellation_token: CancellationToken::new(),
            server_join_handles: Vec::new(),
        }
    }

    pub async fn init(&mut self) {}

    pub async fn serve(&mut self) -> Result<(), anyhow::Error> {
        let mut listener = ListenerBuilder::default()
            .configure(self.config.public_endpoint.clone())
            .build()
            .await?;

        let cancellation_token = self.cancellation_token.clone();
        let new_clients = self.new_clients.clone();

        let listener_handle = tokio::spawn(async move {
            loop {
                select! {
                    _ = cancellation_token.cancelled() => {
                        info!("Server has been cancelled. Listener will now exit.");
                        break;
                    }
                    connection = listener.accept() => {
                        match connection {
                            Some(connection) => {
                                info!("New WebSocket connection from: {}", connection.addr());

                                let client = Client::new(
                                    connection,
                                    ClientMessagePacketSerializer::default(),
                                    ServerMessagePacketSerializer::default(),
                                );

                                new_clients.lock()
                                    .await
                                    .insert(calculate_hash(&client), client);
                            },
                            None => {}
                        }
                    }
                }
            }
        });

        self.server_join_handles.push(listener_handle);

        Ok(())
    }

    pub fn get_cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub async fn join(&mut self) {
        join_all(
            self.server_join_handles
                .iter_mut()
                .map(|handle| async move { handle.await }),
        )
        .await;
    }

    #[cfg(test)]
    pub(self) fn get_config(&self) -> &ServerConfig {
        &self.config
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    use crate::ServerConfig;

    #[test]
    fn new_givenServerConfig_configSavedInServer() {
        // Arrange
        let config = ServerConfig::default();

        // Act
        let server = Server::new(config.clone());

        // Assert
        assert_eq!(&config, server.get_config());
    }
}
