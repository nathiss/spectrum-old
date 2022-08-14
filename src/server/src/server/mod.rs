mod client_map_key;
mod server_util;

use std::{pin::Pin, sync::Arc};

use futures::Future;
use log::info;
use spectrum_game::{DefaultGameState, GameState};
use spectrum_network::{Listener, ListenerBuilder};

use tokio::{
    select,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        RwLock,
    },
};
use tokio_util::sync::CancellationToken;

use crate::{util::convert_to_future, Client, ServerConfig};

pub struct Server {
    config: ServerConfig,
    cancellation_token: CancellationToken,
    game_state: Arc<RwLock<Box<dyn GameState>>>,

    server_join_futures_tx: UnboundedSender<Pin<Box<dyn Future<Output = ()>>>>,
    server_join_futures_rx: UnboundedReceiver<Pin<Box<dyn Future<Output = ()>>>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        let cancellation_token = CancellationToken::new();

        let game_state: Box<dyn GameState> = Box::new(DefaultGameState::new(
            config.game_state.clone(),
            cancellation_token.clone(),
        ));

        let (tx, rx) = unbounded_channel();

        Self {
            config,
            cancellation_token,
            game_state: Arc::new(RwLock::new(game_state)),
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
        let game_state = self.game_state.clone();

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

                                client.open_package_stream(cancellation_token.clone()).await;

                                server_util::create_receive_task(
                                    game_state.clone(),
                                    client,
                                    cancellation_token.clone()
                                ).await;
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
