use std::{collections::HashMap, net::SocketAddr, pin::Pin, sync::Arc};

use futures::{future::join_all, Future};
use log::{debug, info};
use spectrum_network::{Listener, ListenerBuilder};
use spectrum_packet::model::ClientMessage;
use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, RwLock},
};
use tokio_util::sync::CancellationToken;

use crate::{
    util::{calculate_hash, convert_to_future},
    Client, ServerConfig,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
struct ClientMapKey(u64);

impl From<u64> for ClientMapKey {
    fn from(key: u64) -> Self {
        Self(key)
    }
}

pub struct Server {
    config: ServerConfig,
    new_clients: Arc<RwLock<HashMap<ClientMapKey, Client>>>,
    cancellation_token: CancellationToken,

    server_join_futures: Vec<Pin<Box<dyn Future<Output = ()>>>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            new_clients: Arc::new(RwLock::new(HashMap::new())),
            cancellation_token: CancellationToken::new(),
            server_join_futures: Vec::new(),
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

                                let mut client = Client::new(
                                    connection,
                                    Default::default(),
                                    Default::default(),
                                );

                                let key = ClientMapKey::from(calculate_hash(&client));

                                let _raw_packets_future = client.open_package_stream(cancellation_token.clone()).await;
                                let packet_rx = client.get_packet_channel();

                                Self::create_receive_task(
                                    key,
                                    packet_rx, client.addr(),
                                    new_clients.clone(),
                                    cancellation_token.clone()
                                ).await;

                                new_clients.write().await.insert(key, client);
                            },
                            None => {
                                // This means that an error occurred internally inside the listener.
                                // We cannot accept new connections, so we exit.
                                break;
                            }
                        }
                    }
                }
            }
        });

        self.server_join_futures
            .push(Box::pin(convert_to_future(listener_handle)));

        Ok(())
    }

    pub fn get_cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub async fn join(self) {
        // FIXME: When calling it consumes the collection and future elements are not added to join_all.
        join_all(self.server_join_futures.into_iter()).await;
    }

    async fn create_receive_task(
        key: ClientMapKey,
        mut packet_rx: UnboundedReceiver<ClientMessage>,
        addr: SocketAddr,
        new_clients: Arc<RwLock<HashMap<ClientMapKey, Client>>>,
        cancellation_token: CancellationToken,
    ) {
        tokio::spawn(async move {
            loop {
                select! {
                    biased;

                    _ = cancellation_token.cancelled() => {
                        info!("The sever has been cancelled. Receiving task for {} will now exit.", addr);

                        new_clients.write().await.remove(&key);
                        break;
                    }

                    message = packet_rx.recv() => {
                        if let None = message {
                            debug!("Underlying Connection was closed. Receiving task for {} will now exit.", addr);
                            break;
                        }

                        // TODO: handle message
                    }
                }
            }
        });
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
