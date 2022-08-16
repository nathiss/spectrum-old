use std::{net::SocketAddr, sync::Arc};

use log::{debug, error};
use spectrum_game::GameState;
use spectrum_packet::{
    make_server_welcome,
    model::{client_message::ClientMessageData, server_welcome, ClientMessage, ServerMessage},
};
use tokio::{
    select,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        RwLock,
    },
};
use tokio_util::sync::CancellationToken;

use crate::Client;

pub(super) async fn create_receive_task(
    game_state: Arc<RwLock<Box<dyn GameState>>>,
    mut client: Client,
    cancellation_token: CancellationToken,
) {
    let addr = client.addr();
    let packet_rx = client.get_packet_channel();
    let packet_tx = create_send_task(client);

    spawn_receiving_task(addr, game_state, packet_rx, packet_tx, cancellation_token);
}

fn spawn_receiving_task(
    addr: SocketAddr,
    game_state: Arc<RwLock<Box<dyn GameState>>>,
    mut packet_rx: UnboundedReceiver<ClientMessage>,
    mut packet_tx: UnboundedSender<ServerMessage>,
    cancellation_token: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    debug!("The sever has been cancelled. Receiving task for {} will now exit.", addr);

                    break;
                }

                message = packet_rx.recv() => {
                    match message {
                        Some(message) => {
                            match message.client_message_data {
                                Some(ClientMessageData::WelcomeMessage(welcome_message)) => {
                                    let nick_len = welcome_message.nick.len();

                                    if nick_len == 0 || nick_len > 32 {
                                        error!("Nick from {} is not of reasonable length ({})", addr, nick_len);

                                        let _ = packet_tx.send(make_server_welcome(server_welcome::StatusCode::BadRequest));
                                        break;
                                    }

                                    match game_state.read().await.join_game(welcome_message, packet_rx, packet_tx) {
                                        spectrum_game::JoinGameResult::Ok => break,
                                        spectrum_game::JoinGameResult::GameIsFull(rx, tx) => {
                                            packet_rx = rx;
                                            packet_tx = tx;

                                            let _ = packet_tx.send(make_server_welcome(server_welcome::StatusCode::GameCouldNotBeFound));
                                        },
                                        spectrum_game::JoinGameResult::GameDoesNotExit(rx, tx) => {
                                            packet_rx = rx;
                                            packet_tx = tx;

                                            let _ = packet_tx.send(make_server_welcome(server_welcome::StatusCode::GameCouldNotBeFound));
                                        },
                                        spectrum_game::JoinGameResult::NickTaken(rx, tx) => {
                                            packet_rx = rx;
                                            packet_tx = tx;

                                            let _ = packet_tx.send(make_server_welcome(server_welcome::StatusCode::NickTaken));
                                        },
                                        spectrum_game::JoinGameResult::BadRequest(_rx, tx) => {
                                            packet_tx = tx;

                                            let _ = packet_tx.send(make_server_welcome(server_welcome::StatusCode::BadRequest));
                                            break;
                                        }
                                    }
                                }
                                _ => {
                                    // The first message the client needs to send is the welcome message. In every other
                                    // case we exit and terminate the connection.
                                    error!("Expected to receive a \"ClientWelcome\" from {}.", addr);
                                    break;
                                }
                            }
                        }
                        None => {
                            debug!("Underlying Connection was closed. Receiving task for {} will now exit.", addr);
                            break;
                        }
                    }
                }
            }
        }
    });
}

fn create_send_task(mut client: Client) -> UnboundedSender<ServerMessage> {
    let (tx, mut rx) = unbounded_channel();

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = client.write_packet(&message).await {
                error!("Failed to send message to {}. Error: {}", client.addr(), e);
            }
        }
    });

    tx
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::{cell::RefCell, net::IpAddr, time::Duration};

    use spectrum_game::JoinGameResult;
    use spectrum_packet::model::{server_message::ServerMessageData, ClientLeave, ClientWelcome};
    use tokio::time::timeout;

    use super::*;

    #[derive(Debug)]
    enum JoinGameResultAction {
        DoOk,
        DoGameIsFull,
        DoGameDoesNotExit,
        DoNickTaken,
    }

    impl Default for JoinGameResultAction {
        fn default() -> Self {
            JoinGameResultAction::DoOk
        }
    }

    #[derive(Debug, Default)]
    struct MockGameState {
        pub(self) welcome_message: RefCell<Option<ClientWelcome>>,
        pub(self) packet_rx: RefCell<Option<UnboundedReceiver<ClientMessage>>>,
        pub(self) packet_tx: RefCell<Option<UnboundedSender<ServerMessage>>>,
        pub(self) join_game_result: RefCell<Option<JoinGameResult>>,
        pub(self) action: JoinGameResultAction,
    }

    unsafe impl Sync for MockGameState {}

    impl GameState for MockGameState {
        fn join_game(
            &self,
            welcome_message: ClientWelcome,
            packet_rx: UnboundedReceiver<ClientMessage>,
            packet_tx: UnboundedSender<ServerMessage>,
        ) -> JoinGameResult {
            self.welcome_message.replace(Some(welcome_message));

            match self.action {
                JoinGameResultAction::DoOk => {
                    self.packet_rx.replace(Some(packet_rx));
                    self.packet_tx.replace(Some(packet_tx));

                    JoinGameResult::Ok
                }
                JoinGameResultAction::DoGameIsFull => {
                    JoinGameResult::GameIsFull(packet_rx, packet_tx)
                }
                JoinGameResultAction::DoGameDoesNotExit => {
                    JoinGameResult::GameDoesNotExit(packet_rx, packet_tx)
                }
                JoinGameResultAction::DoNickTaken => {
                    JoinGameResult::NickTaken(packet_rx, packet_tx)
                }
            }
        }
    }

    impl MockGameState {
        pub(self) fn new() -> Arc<RwLock<Box<dyn GameState>>> {
            Arc::new(RwLock::new(Box::new(Self::default())))
        }

        pub(self) fn into(self) -> Arc<RwLock<Box<dyn GameState>>> {
            Arc::new(RwLock::new(Box::new(self)))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_operationCancelled_futureCanComplete() -> Result<(), anyhow::Error>
    {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (_client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let game_state = MockGameState::new();
        let cancellation_token = CancellationToken::new();

        spawn_receiving_task(
            addr,
            game_state,
            client_rx,
            server_tx,
            cancellation_token.clone(),
        );

        // Act
        cancellation_token.cancel();

        // Assert
        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await?;

        assert!(timeout.is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_sendInvalidMessage_futureCanComplete() -> Result<(), anyhow::Error>
    {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let game_state = MockGameState::new();
        let cancellation_token = CancellationToken::new();

        spawn_receiving_task(addr, game_state, client_rx, server_tx, cancellation_token);

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::LeaveMessage(ClientLeave {})),
        };

        // Act
        client_tx.send(client_message)?;

        // Assert
        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await?;

        assert!(timeout.is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_connectionClosed_futureCanComplete() -> Result<(), anyhow::Error>
    {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let game_state = MockGameState::new();
        let cancellation_token = CancellationToken::new();

        spawn_receiving_task(addr, game_state, client_rx, server_tx, cancellation_token);

        // Act
        drop(client_tx);

        // Assert
        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await?;

        assert!(timeout.is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_sendEmptyNick_sendsBadRequest() -> Result<(), anyhow::Error> {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let game_state = MockGameState::new();
        let cancellation_token = CancellationToken::new();

        spawn_receiving_task(addr, game_state, client_rx, server_tx, cancellation_token);

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: String::new(),
                game_id: None,
            })),
        };

        // Act
        client_tx.send(client_message)?;

        // Assert
        let bad_request = timeout(Duration::from_millis(50), server_rx.recv()).await?;
        let bad_request = bad_request.unwrap().server_message_data.unwrap();

        match bad_request {
            ServerMessageData::WelcomeMessage(message) => {
                let bad_request: i32 = server_welcome::StatusCode::BadRequest.into();
                assert_eq!(bad_request, message.status_code);
            }
            _ => {
                panic!("ServerMessage type is not a BadRequest");
            }
        }

        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await?;

        assert!(timeout.is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_sendNickWithMaxLength_doesNotComplete(
    ) -> Result<(), anyhow::Error> {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let cancellation_token = CancellationToken::new();

        let game_state = MockGameState::default();
        game_state
            .join_game_result
            .replace(Some(JoinGameResult::Ok));

        let game_state = game_state.into();

        spawn_receiving_task(
            addr,
            game_state.clone(),
            client_rx,
            server_tx,
            cancellation_token,
        );

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: "A".repeat(32),
                game_id: None,
            })),
        };

        // Act
        client_tx.send(client_message)?;

        // Assert
        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await;

        // We expect to get timeout, since the receiving task has not been completed.
        assert!(timeout.is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_gameIsFull_acceptsNextPackets() -> Result<(), anyhow::Error> {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let cancellation_token = CancellationToken::new();

        let mut game_state = MockGameState::default();
        game_state
            .join_game_result
            .replace(Some(JoinGameResult::Ok));
        game_state.action = JoinGameResultAction::DoGameIsFull;

        let game_state = game_state.into();

        spawn_receiving_task(
            addr,
            game_state.clone(),
            client_rx,
            server_tx,
            cancellation_token,
        );

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: "A".repeat(32),
                game_id: None,
            })),
        };

        // Act
        client_tx.send(client_message)?;

        // Assert
        let game_is_full = timeout(Duration::from_millis(50), server_rx.recv()).await?;
        let game_is_full = game_is_full.unwrap().server_message_data.unwrap();

        match game_is_full {
            ServerMessageData::WelcomeMessage(message) => {
                let game_is_full: i32 = server_welcome::StatusCode::GameCouldNotBeFound.into();
                assert_eq!(game_is_full, message.status_code);
            }
            _ => {
                panic!("ServerMessage type is not a GameCouldNotBeFound");
            }
        }

        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await;

        // We expect to get timeout, since the receiving task has not been completed.
        assert!(timeout.is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_gameDoesNotExist_acceptsNextPackets() -> Result<(), anyhow::Error>
    {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let cancellation_token = CancellationToken::new();

        let mut game_state = MockGameState::default();
        game_state
            .join_game_result
            .replace(Some(JoinGameResult::Ok));
        game_state.action = JoinGameResultAction::DoGameDoesNotExit;

        let game_state = game_state.into();

        spawn_receiving_task(
            addr,
            game_state.clone(),
            client_rx,
            server_tx,
            cancellation_token,
        );

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: "A".repeat(32),
                game_id: None,
            })),
        };

        // Act
        client_tx.send(client_message)?;

        // Assert
        let game_does_not_exist = timeout(Duration::from_millis(50), server_rx.recv()).await?;
        let game_does_not_exist = game_does_not_exist.unwrap().server_message_data.unwrap();

        match game_does_not_exist {
            ServerMessageData::WelcomeMessage(message) => {
                let game_does_not_exist: i32 =
                    server_welcome::StatusCode::GameCouldNotBeFound.into();
                assert_eq!(game_does_not_exist, message.status_code);
            }
            _ => {
                panic!("ServerMessage type is not a GameCouldNotBeFound");
            }
        }

        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await;

        // We expect to get timeout, since the receiving task has not been completed.
        assert!(timeout.is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_receiving_task_nickTaken_acceptsNextPackets() -> Result<(), anyhow::Error> {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();
        let cancellation_token = CancellationToken::new();

        let mut game_state = MockGameState::default();
        game_state
            .join_game_result
            .replace(Some(JoinGameResult::Ok));
        game_state.action = JoinGameResultAction::DoNickTaken;

        let game_state = game_state.into();

        spawn_receiving_task(
            addr,
            game_state.clone(),
            client_rx,
            server_tx,
            cancellation_token,
        );

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: "A".repeat(32),
                game_id: None,
            })),
        };

        // Act
        client_tx.send(client_message)?;

        // Assert
        let nick_taken = timeout(Duration::from_millis(50), server_rx.recv()).await?;
        let nick_taken = nick_taken.unwrap().server_message_data.unwrap();

        match nick_taken {
            ServerMessageData::WelcomeMessage(message) => {
                let nick_taken: i32 = server_welcome::StatusCode::NickTaken.into();
                assert_eq!(nick_taken, message.status_code);
            }
            _ => {
                panic!("ServerMessage type is not a NickTaken");
            }
        }

        let timeout = timeout(Duration::from_millis(50), server_rx.recv()).await;

        // We expect to get timeout, since the receiving task has not been completed.
        assert!(timeout.is_err());

        Ok(())
    }
}
