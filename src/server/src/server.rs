use std::{collections::HashMap, net::SocketAddr, pin::Pin, sync::Arc};

use futures::Future;
use log::{debug, info};
use spectrum_network::{Listener, ListenerBuilder};
use spectrum_packet::model::ClientMessage;
use tokio::{
    select,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        RwLock,
    },
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
    clients: Arc<RwLock<HashMap<ClientMapKey, Client>>>,
    cancellation_token: CancellationToken,

    server_join_futures_tx: UnboundedSender<Pin<Box<dyn Future<Output = ()>>>>,
    server_join_futures_rx: UnboundedReceiver<Pin<Box<dyn Future<Output = ()>>>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        let (tx, rx) = unbounded_channel();

        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
            cancellation_token: CancellationToken::new(),
            server_join_futures_tx: tx,
            server_join_futures_rx: rx,
        }
    }

    pub async fn init(&mut self) {}

    pub async fn serve(&mut self) -> Result<(), anyhow::Error> {
        let mut listener = ListenerBuilder::default()
            .configure(self.config.public_endpoint.clone())
            .build()
            .await?;

        let cancellation_token = self.cancellation_token.clone();
        let clients = self.clients.clone();

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
                                    clients.clone(),
                                    cancellation_token.clone()
                                ).await;

                                clients.write().await.insert(key, client);
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

        // It is safe to drop `Result<(), Error>` here, because the receiver lives as long as `self`.
        drop(
            self.server_join_futures_tx
                .send(Box::pin(convert_to_future(listener_handle))),
        );

        Ok(())
    }

    pub fn get_cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub async fn join(mut self) {
        drop(self.server_join_futures_tx);

        while let Some(future) = self.server_join_futures_rx.recv().await {
            future.await;
        }
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
                        debug!("The sever has been cancelled. Receiving task for {} will now exit.", addr);

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
