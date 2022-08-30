use std::time::Duration;

use dashmap::DashMap;
use futures::{stream::FuturesUnordered, Future, StreamExt};
use log::{debug, error, warn};
use spectrum_packet::{
    is_player_ready, make_lobby_update,
    model::{lobby_update::StatusCode, ClientMessage},
};
use tokio::{select, sync::RwLock, task::JoinError, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::{game_lobby_status::GameLobbyStatus, player::Player, GameStateConfig};

#[derive(Debug)]
pub(crate) enum AddPlayerResult {
    GameRunning(Player),
    NickTaken(Player),
    Success,
    GameStarted,
}

/// This struct represents a single game lobby and all its associated content.
///
/// NOTE: Once the player has been added into the lobby we do not check whether the peer has terminated the connection.
/// This incident will be detected when we await for readiness signal from all players. We should probably find a better
/// way to do this. Perhaps a periodic ping?
#[derive(Debug)]
pub(crate) struct GameLobby {
    config: GameStateConfig,
    players: DashMap<String, Player>,
    game_lobby_status: RwLock<GameLobbyStatus>,
    cancellation_token: CancellationToken,
}

impl GameLobby {
    /// This method is used to construct a new game lobby.
    ///
    /// # Arguments
    ///
    /// * `config` - A configuration for the game lobby and all its internal components.
    /// * `cancellation_token` - A server's cancellation token. If canceled it will cause all internal asynchronous
    ///                          tasks to complete immediately.
    pub fn new(config: GameStateConfig, cancellation_token: CancellationToken) -> Self {
        let maximum_number_of_players = config.number_of_players_in_game_lobby;

        Self {
            config,
            players: DashMap::with_capacity(maximum_number_of_players),
            game_lobby_status: Default::default(),
            cancellation_token,
        }
    }

    /// This method is used to add a new player into the game lobby.
    ///
    /// This method can cause the game to start if the maximum number of players has been reached. Before the player is
    /// actually added into the lobby, this method first checks the lobby's state. If it's not `GameLobbyStatus::Waiting`
    /// then the method returns immediately with `HasGameLobbyStarted(true)` as the return value.
    ///
    /// It takes a mutable reference to self to ensure that only one thread can access this method at any given moment.
    ///
    /// # Arguments
    ///
    /// * `nick` - A nick for the new player.
    /// * `player` - A context object contains all player's associated data.
    ///
    /// # Returns
    ///
    /// An indication of whether the operation caused the game to start is returned.
    pub async fn add_player(&self, nick: String, player: Player) -> AddPlayerResult {
        // This lock guard ensures that only one thread can access this method at any given moment.
        let game_status = self.game_lobby_status.write().await;

        if *game_status != GameLobbyStatus::Waiting {
            warn!(
                "Tried to add player {} to a game that's already started.",
                nick
            );

            return AddPlayerResult::GameRunning(player);
        }

        if self.players.contains_key(&nick) {
            warn!("Player with nick {} already exists in the lobby.", nick);
            return AddPlayerResult::NickTaken(player);
        }

        self.players.insert(nick, player);

        if self.players.len() < self.config.number_of_players_in_game_lobby {
            self.broadcast_players(StatusCode::Waiting).await;
            AddPlayerResult::Success
        } else {
            // Mutable reference needs to be dropped to prevent dead-locking on `game_lobby_status`.
            drop(game_status);

            self.start().await;
            AddPlayerResult::GameStarted
        }
    }

    /// This method is used to start the game omitting all constrains.
    ///
    /// It is assumed that a LobbyUpdate message with `GameReady` status has been broadcasted to all players. Now
    /// players need to send their confirmation messages that they are ready to start the game. This method awaits on
    /// all players' stream for the message. If the message has not been received from any player within the specified
    /// timeout window, then [`GameLobbyStatus`] is again switched to `Waiting` and the confirmation messages from other
    /// players are discarded. Unresponsive players are removed from the game lobby.
    /// If the procedure was successful, then the game starts.
    pub async fn start(&self) {
        let mut game_lobby_state = self.game_lobby_status.write().await;
        if *game_lobby_state != GameLobbyStatus::Waiting {
            error!("GameLobby::start() has been called while the game is already running.");
            return;
        }

        *game_lobby_state = GameLobbyStatus::Starting;
        drop(game_lobby_state);

        let players_to_remove = self.wait_for_readiness().await;

        match players_to_remove {
            Ok(nicks) if nicks.is_empty() => {
                // This means that all players responded with 'Ready'. The game can now start.
                let mut game_lobby_state = self.game_lobby_status.write().await;
                *game_lobby_state = GameLobbyStatus::Started;
                drop(game_lobby_state);

                self.broadcast_players(StatusCode::GameReady).await;

                // TODO: start the game
            }
            Ok(nicks) => {
                // At least one player failed to confirm readiness. The lobby needs to wait for more players.
                for nick in &nicks {
                    self.players.remove(nick);
                }

                self.broadcast_players(StatusCode::Waiting).await;

                let mut game_lobby_state = self.game_lobby_status.write().await;
                *game_lobby_state = GameLobbyStatus::Waiting;
                drop(game_lobby_state);
            }
            Err(e) => {
                error!(
                    "[GameLobby] Failed to join async task handle correctly. Error: {}",
                    e
                );
            }
        }
    }

    pub async fn get_state(&self) -> GameLobbyStatus {
        *self.game_lobby_status.read().await
    }

    async fn wait_for_readiness(&self) -> Result<Vec<String>, JoinError> {
        let mut read_futures = self.get_readiness_futures_collection();

        let players_readiness_handle = tokio::spawn(async move {
            let mut players_to_remove = Vec::new();

            while let Some((nick, message)) = read_futures.next().await {
                if message.is_none() {
                    warn!(
                        "Player {} failed to confirm readiness within timeout window",
                        nick
                    );

                    players_to_remove.push(nick);
                    continue;
                }

                let message = message.unwrap();

                if !is_player_ready(&message) {
                    error!(
                        "Player {} sent incorrect type of message. Got: {:?}",
                        nick, message
                    );

                    players_to_remove.push(nick);
                    continue;
                }

                // This means the player correctly sent us `PlayerReady` message. We can continue to process the rest of
                // the players.
            }

            players_to_remove
        });

        players_readiness_handle.await
    }

    fn get_readiness_futures_collection(
        &self,
    ) -> FuturesUnordered<impl Future<Output = (String, Option<ClientMessage>)>> {
        let cancellation_token = self.cancellation_token.clone();

        let read_futures = FuturesUnordered::new();
        let player_readiness_timeout = self.config.player_readiness_timeout;

        for player in &self.players {
            let cancellation_token = cancellation_token.clone();
            let client_rx = player.get_client_stream();

            let nick = player.key().clone();

            let read_or_timeout_future = async move {
                select! {
                    biased;

                    _ = cancellation_token.cancelled() => {
                        debug!("listening for PlayerReady message from {} has been cancelled.", nick);
                        (nick, None)
                    }

                    result = timeout(Duration::from_millis(player_readiness_timeout), async move {
                        let mut client_stream = client_rx.lock().await;
                        client_stream.recv().await
                    }) => {
                        match result {
                            Ok(message @ Some(_)) => (nick, message),
                            Ok(None) => {
                                error!("Client stream returned None while waiting for PlayerReady message from {}", nick);
                                (nick, None)
                            },
                            Err(_timeout_error) => {
                                error!("Player {} failed to sent a PlayerReady message within the timeout window.", nick);
                                (nick, None)
                            }
                        }
                    }
                }
            };

            read_futures.push(read_or_timeout_future);
        }

        read_futures
    }

    async fn broadcast_players(&self, status_code: StatusCode) {
        let mut nicks = Vec::with_capacity(self.players.len());
        for player in &self.players {
            nicks.push(player.key().clone());
        }

        let lobby_update = make_lobby_update(status_code, nicks);

        for player in &self.players {
            let player_sink = player.get_server_sink();
            let player_stream = player_sink.lock().await;
            if let Err(e) = player_stream.send(lobby_update.clone()) {
                error!(
                    "Failed to send lobby update to player {}. Error: {}",
                    player.key(),
                    e
                );
            }
        }
    }

    #[cfg(test)]
    pub(self) fn set_players(&mut self, players: DashMap<String, Player>) {
        self.players = players;
    }

    #[cfg(test)]
    pub(crate) fn get_players(&self) -> &DashMap<String, Player> {
        &self.players
    }

    #[cfg(test)]
    pub(crate) async fn set_game_lobby_state(&mut self, state: GameLobbyStatus) {
        *self.game_lobby_status.write().await = state;
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use spectrum_packet::model::{
        client_message::ClientMessageData, server_message::ServerMessageData, ClientWelcome,
        LobbyUpdate, PlayerReady, ServerMessage,
    };
    use tokio::sync::{
        mpsc::{error::TryRecvError, unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    };
    use tokio_util::sync::CancellationToken;

    use crate::player::Player;

    fn make_player_welcome() -> ClientMessage {
        ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: "nick".to_owned(),
                game_id: None,
            })),
        }
    }

    fn make_player_ready() -> ClientMessage {
        ClientMessage {
            client_message_data: Some(ClientMessageData::PlayerReady(PlayerReady {})),
        }
    }

    fn create_player() -> (
        Player,
        UnboundedReceiver<ServerMessage>,
        UnboundedSender<ClientMessage>,
    ) {
        let (client_tx, client_rx) = unbounded_channel();
        let (server_tx, server_rx) = unbounded_channel();

        let player = Player::new(
            Arc::new(Mutex::new(client_rx)),
            Arc::new(Mutex::new(server_tx)),
        );

        (player, server_rx, client_tx)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn broadcast_players_singlePlayerPresent_broadcastsSent() {
        // Arrange
        let mut lobby = GameLobby::new(Default::default(), CancellationToken::new());

        let (_client_tx, client_rx) = unbounded_channel();
        let (server_tx, mut server_rx) = unbounded_channel();

        let player = Player::new(
            Arc::new(Mutex::new(client_rx)),
            Arc::new(Mutex::new(server_tx)),
        );

        let map = DashMap::with_capacity(1);
        map.insert("nick".to_owned(), player);

        lobby.set_players(map);

        // Act
        lobby.broadcast_players(StatusCode::Waiting).await;

        // Assert
        let message = server_rx.try_recv().unwrap();
        match message.server_message_data.unwrap() {
            ServerMessageData::LobbyUpdate(lobby_update) => {
                assert_eq!(
                    StatusCode::Waiting,
                    StatusCode::from_i32(lobby_update.status_code).unwrap()
                );
                assert_eq!(1, lobby_update.players.len());
                assert_eq!("nick", lobby_update.players.get(0).unwrap());
            }
            message_data => assert!(
                false,
                "ServerMessageData is not a LobbyUpdate. It's {:?}",
                message_data
            ),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn broadcast_players_playerClosedItsChannel_broadcastedToOtherPlayers() {
        // Arrange
        let mut lobby = GameLobby::new(Default::default(), CancellationToken::new());

        let map = DashMap::with_capacity(3);

        let (player1, server_rx1, _client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, server_rx2, _client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        // Drop server's receiving stream for player2.
        drop(server_rx2);

        let (player3, server_rx3, _client_tx3) = create_player();
        map.insert("nick3".to_owned(), player3);

        lobby.set_players(map);

        // Act
        lobby.broadcast_players(StatusCode::GameReady).await;

        // Assert
        for mut server_rx in [server_rx1, server_rx3] {
            let message = server_rx.try_recv().unwrap();
            match message.server_message_data.unwrap() {
                ServerMessageData::LobbyUpdate(lobby_update) => {
                    assert_eq!(
                        StatusCode::GameReady,
                        StatusCode::from_i32(lobby_update.status_code).unwrap()
                    );
                    assert_eq!(3, lobby_update.players.len());
                    assert!(lobby_update.players.contains(&"nick1".to_owned()));
                    assert!(lobby_update.players.contains(&"nick2".to_owned()));
                    assert!(lobby_update.players.contains(&"nick3".to_owned()));
                }
                message_data => assert!(
                    false,
                    "ServerMessageData is not a LobbyUpdate. It's {:?}",
                    message_data
                ),
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_readiness_futures_collection_timeoutIsDefined_futuresCancelledInADefinedTimeout() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 50,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, _client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, _client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        // Act
        let mut futures = lobby.get_readiness_futures_collection();

        // Assert
        let result = timeout(Duration::from_millis(50 + 20), futures.next()).await;
        assert!(result.is_ok());
        let result = timeout(Duration::from_millis(50 + 20), futures.next()).await;
        assert!(result.is_ok());
        let result = futures.next().await;
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_readiness_futures_collection_tokenHasBeenCancelled_futuresCompleteImmediately() {
        // Arrange
        let cancellation_token = CancellationToken::new();

        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, cancellation_token.clone());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, _client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, _client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        let mut futures = lobby.get_readiness_futures_collection();

        // Act
        cancellation_token.cancel();

        // Assert
        let result = futures.next().await.unwrap();
        assert!(result.1.is_none());
        let result = futures.next().await.unwrap();
        assert!(result.1.is_none());
        let result = futures.next().await;
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_readiness_futures_collection_clientCloseConnection_futuresCompleteImmediately() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        let mut futures = lobby.get_readiness_futures_collection();

        // Act
        drop(client_tx1);
        drop(client_tx2);

        // Assert
        let result = futures.next().await.unwrap();
        assert!(result.1.is_none());
        let result = futures.next().await.unwrap();
        assert!(result.1.is_none());
        let result = futures.next().await;
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_readiness_futures_collection_clientSentMessage_futuresCompleteImmediatelyWithMessage(
    ) {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        let mut futures = lobby.get_readiness_futures_collection();

        let client_message = make_player_ready();

        // Act
        client_tx1.send(client_message.clone()).unwrap();
        client_tx2.send(client_message.clone()).unwrap();

        // Assert
        let result = futures.next().await.unwrap();
        assert_eq!(client_message, result.1.unwrap());
        let result = futures.next().await.unwrap();
        assert_eq!(client_message, result.1.unwrap());
        let result = futures.next().await;
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn wait_for_readiness_getsNone_playerIsAddedToRemoveList() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        drop(client_tx1);
        drop(client_tx2);

        // Act
        let result = lobby.wait_for_readiness().await;

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(2, result.len());
        assert!(result.contains(&"nick1".to_string()));
        assert!(result.contains(&"nick2".to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn wait_for_readiness_playerSendsIncorrectMessage_playerIsAddedToRemoveList() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        let message = make_player_welcome();

        client_tx1.send(message.clone()).unwrap();
        client_tx2.send(message.clone()).unwrap();

        // Act
        let result = lobby.wait_for_readiness().await;

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(2, result.len());
        assert!(result.contains(&"nick1".to_string()));
        assert!(result.contains(&"nick2".to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn wait_for_readiness_playerSendsCorrectMessage_playerIsNotAddedToRemoveList() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        let message = make_player_ready();

        client_tx1.send(message.clone()).unwrap();
        client_tx2.send(message.clone()).unwrap();

        // Act
        let result = lobby.wait_for_readiness().await;

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(0, result.len());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn wait_for_readiness_onePlayerSendsIncorrectMessage_hasNoEffectOnOtherPlayers() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        let map = DashMap::with_capacity(2);

        let (player1, _server_rx1, client_tx1) = create_player();
        map.insert("nick1".to_owned(), player1);

        let (player2, _server_rx2, client_tx2) = create_player();
        map.insert("nick2".to_owned(), player2);

        lobby.set_players(map);

        let message1 = make_player_ready();
        let message2 = make_player_welcome();

        client_tx1.send(message1.clone()).unwrap();
        client_tx2.send(message2.clone()).unwrap();

        // Act
        let result = lobby.wait_for_readiness().await;

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(1, result.len());
        assert!(result.contains(&"nick2".to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_player_gameAlreadyStarted_ReturnsGameStarted() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let mut lobby = GameLobby::new(config, CancellationToken::new());

        lobby.set_game_lobby_state(GameLobbyStatus::Starting).await;

        let (player, _server_rx, _client_tx) = create_player();

        // Act
        let result = lobby.add_player(String::new(), player).await;

        // Assert
        match result {
            AddPlayerResult::GameRunning(_player) => {}
            result => {
                assert!(false, "Result is not a GameRunning. It's {:?}", result);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_player_nickTaken_ReturnsNickTaken() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 10,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player1, _server_rx1, _client_tx1) = create_player();
        let (player2, _server_rx2, _client_tx2) = create_player();

        let result = lobby.add_player("nick".to_owned(), player1).await;
        assert!(matches!(result, AddPlayerResult::Success));

        // Act
        let result = lobby.add_player("nick".to_owned(), player2).await;

        // Assert
        match result {
            AddPlayerResult::NickTaken(_player) => {}
            result => {
                assert!(false, "Result is not a NickTaken. It's {:?}", result);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_player_numberOfPlayersHasNotBeenMet_ReturnsSuccess() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 3,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player1, _server_rx1, _client_tx1) = create_player();
        let (player2, _server_rx2, _client_tx2) = create_player();

        // Act
        let result = lobby.add_player("nick1".to_owned(), player1).await;
        assert!(matches!(result, AddPlayerResult::Success));
        let result = lobby.add_player("nick2".to_owned(), player2).await;
        assert!(matches!(result, AddPlayerResult::Success));

        // Assert
        match lobby.get_state().await {
            GameLobbyStatus::Waiting => {}
            status => {
                assert!(false, "GameLobbyStatus is not Waiting. It's {:?}", status);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_player_numberOfPlayersHasNotBeenMet_broadCastsWaitingLobbyUpdate() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 3,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player1, mut server_rx1, _client_tx1) = create_player();
        let (player2, mut server_rx2, _client_tx2) = create_player();

        // Act
        let result = lobby.add_player("nick1".to_owned(), player1).await;
        assert!(matches!(result, AddPlayerResult::Success));
        let result = lobby.add_player("nick2".to_owned(), player2).await;
        assert!(matches!(result, AddPlayerResult::Success));

        // Assert

        // Player 1
        let message = server_rx1.try_recv().unwrap();
        assert!(matches!(
            message.server_message_data.unwrap(),
            ServerMessageData::LobbyUpdate(_)
        ));

        let message = server_rx1.try_recv().unwrap();
        assert!(matches!(
            message.server_message_data.unwrap(),
            ServerMessageData::LobbyUpdate(_)
        ));

        let message = server_rx1.try_recv();
        assert!(matches!(message, Err(TryRecvError::Empty)));

        // Player 2
        let message = server_rx2.try_recv().unwrap();
        assert!(matches!(
            message.server_message_data.unwrap(),
            ServerMessageData::LobbyUpdate(_)
        ));

        let message = server_rx2.try_recv();
        assert!(matches!(message, Err(TryRecvError::Empty)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_player_numberOfPlayersHasBeenMet_startsTheGame() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 2,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player1, _server_rx1, client_tx1) = create_player();
        let (player2, _server_rx2, client_tx2) = create_player();

        drop(client_tx1);
        drop(client_tx2);

        let result = lobby.add_player("nick1".to_owned(), player1).await;
        assert!(matches!(result, AddPlayerResult::Success));

        // Act
        let result = lobby.add_player("nick2".to_owned(), player2).await;

        // Assert
        assert!(matches!(result, AddPlayerResult::GameStarted));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_statusIsNotWaiting_returnsImmediately() {
        // Arrange
        let mut lobby = GameLobby::new(Default::default(), CancellationToken::new());

        lobby.set_game_lobby_state(GameLobbyStatus::Started).await;

        // Act
        lobby.start().await;

        // Assert
        assert_eq!(GameLobbyStatus::Started, lobby.get_state().await);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_playerFailsToConfirmReadiness_changesStatusToWaiting() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 3,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player1, _server_rx1, client_tx1) = create_player();
        let (player2, _server_rx2, client_tx2) = create_player();

        let result = lobby.add_player("nick1".to_owned(), player1).await;
        assert!(matches!(result, AddPlayerResult::Success));
        let result = lobby.add_player("nick2".to_owned(), player2).await;
        assert!(matches!(result, AddPlayerResult::Success));

        drop(client_tx1);
        client_tx2.send(make_player_ready()).unwrap();

        // Act
        lobby.start().await;

        // Assert
        assert_eq!(GameLobbyStatus::Waiting, lobby.get_state().await);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_playerFailsToConfirmReadiness_removesPlayerFromLobby() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 3,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player1, _server_rx1, client_tx1) = create_player();
        let (player2, _server_rx2, client_tx2) = create_player();

        let result = lobby.add_player("nick1".to_owned(), player1).await;
        assert!(matches!(result, AddPlayerResult::Success));
        let result = lobby.add_player("nick2".to_owned(), player2).await;
        assert!(matches!(result, AddPlayerResult::Success));

        drop(client_tx1);
        client_tx2.send(make_player_ready()).unwrap();

        // Act
        lobby.start().await;

        // Assert
        let players = lobby.get_players();
        assert_eq!(1, players.len());
        assert!(players.contains_key("nick2"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_gameStarts_broadcastsGameReady() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 2,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player, mut server_rx, client_tx) = create_player();

        let result = lobby.add_player("nick1".to_owned(), player).await;
        assert!(matches!(result, AddPlayerResult::Success));

        client_tx.send(make_player_ready()).unwrap();

        // Act
        lobby.start().await;

        // Assert
        let _waiting_message = server_rx.try_recv().unwrap();

        let game_ready_message = server_rx.try_recv().unwrap();
        let _game_ready_value = StatusCode::GameReady as i32;
        assert!(
            matches!(
                game_ready_message.clone().server_message_data.unwrap(),
                ServerMessageData::LobbyUpdate(LobbyUpdate {
                    status_code: _game_ready_value,
                    ..
                })
            ),
            "game_ready_message is not a GameStarted. It's {:?}",
            game_ready_message
        );

        let message = server_rx.try_recv();
        assert!(matches!(message, Err(TryRecvError::Empty)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn start_playerFailsToConfirmReadiness_broadcastsWaiting() {
        // Arrange
        let config = GameStateConfig {
            player_readiness_timeout: 2000,
            number_of_players_in_game_lobby: 3,
        };

        let lobby = GameLobby::new(config, CancellationToken::new());

        let (player1, _server_rx1, client_tx1) = create_player();
        let (player2, mut server_rx2, client_tx2) = create_player();

        let result = lobby.add_player("nick1".to_owned(), player1).await;
        assert!(matches!(result, AddPlayerResult::Success));
        let result = lobby.add_player("nick2".to_owned(), player2).await;
        assert!(matches!(result, AddPlayerResult::Success));

        drop(client_tx1);
        client_tx2.send(make_player_ready()).unwrap();

        // Act
        lobby.start().await;

        // Assert
        let _waiting_message = server_rx2.try_recv().unwrap();

        let waiting_message = server_rx2.try_recv().unwrap();
        let _waiting_value = StatusCode::Waiting as i32;
        assert!(
            matches!(
                waiting_message.clone().server_message_data.unwrap(),
                ServerMessageData::LobbyUpdate(LobbyUpdate {
                    status_code: _waiting_value,
                    ..
                })
            ),
            "game_ready_message is not a Waiting. It's {:?}",
            waiting_message
        );

        let message = server_rx2.try_recv();
        assert!(matches!(message.clone(), Err(TryRecvError::Empty)));
    }
}
