use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use dashmap::DashMap;
use log::error;
use spectrum_packet::model::{ClientMessage, ClientWelcome, ServerMessage};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    game_lobby::{AddPlayerResult, GameLobby},
    game_lobby_status::GameLobbyStatus,
    player::Player,
    GameState, GameStateConfig, JoinGameResult,
};

/// This struct represents the game state.
///
/// It is responsible for managing players, game lobbies and game's business logic.
#[derive(Debug)]
pub struct DefaultGameState {
    config: GameStateConfig,
    cancellation_token: CancellationToken,
    lobbies: DashMap<Uuid, GameLobby>,
}

#[async_trait]
impl GameState for DefaultGameState {
    async fn join_game(
        &self,
        welcome_message: ClientWelcome,
        packet_rx: Arc<Mutex<UnboundedReceiver<ClientMessage>>>,
        packet_tx: Arc<Mutex<UnboundedSender<ServerMessage>>>,
    ) -> JoinGameResult {
        if let Some(game_id) = welcome_message.game_id {
            let uuid = Uuid::from_str(&game_id);

            if let Err(e) = uuid {
                error!(
                    "Failed to convert game_id {} into a UUID. Error: {}",
                    String::from_iter(game_id.chars().take(10)),
                    e
                );

                return JoinGameResult::BadRequest(packet_rx, packet_tx);
            }

            let uuid = uuid.unwrap();

            match self.lobbies.get_mut(&uuid) {
                Some(mut game_lobby) => match game_lobby.get_state().await {
                    GameLobbyStatus::Waiting => {
                        let player = Player::new(packet_rx, packet_tx);
                        match game_lobby.add_player(welcome_message.nick, player).await {
                            AddPlayerResult::NickTaken(player) => {
                                let (rx, tx) = player.into();
                                return JoinGameResult::NickTaken(rx, tx);
                            }
                            AddPlayerResult::GameStarted | AddPlayerResult::Success => {
                                return JoinGameResult::Ok
                            }
                        }
                    }
                    GameLobbyStatus::Ready => {
                        return JoinGameResult::GameIsFull(packet_rx, packet_tx);
                    }
                },
                None => {
                    error!("Game with ID {} does not exist.", uuid);
                    return JoinGameResult::GameDoesNotExit(packet_rx, packet_tx);
                }
            }
        }

        // If game_id is empty then we need to create a new game lobby or find a not started one.
        // TODO: add game lobby lookup logic.
        let mut game_lobby = GameLobby::new(self.config.clone(), self.cancellation_token.clone());

        let player = Player::new(packet_rx, packet_tx);

        game_lobby.add_player(welcome_message.nick, player).await;

        self.lobbies.insert(Uuid::new_v4(), game_lobby);

        JoinGameResult::Ok
    }
}

impl DefaultGameState {
    /// This method is used to construct an instance of Self.
    ///
    /// # Arguments
    ///
    /// * `config` - This is the configuration for the game state and all its internal components.
    /// * `cancellation_token` - This token is not directly used by `DefaultGameState`, but rather it is passed to all
    ///                          internal components which span new asynchronous tasks.
    pub fn new(config: GameStateConfig, cancellation_token: CancellationToken) -> Self {
        Self {
            config,
            cancellation_token,
            lobbies: DashMap::new(),
        }
    }

    #[cfg(test)]
    pub(self) fn get_config(&self) -> &GameStateConfig {
        &self.config
    }

    #[cfg(test)]
    pub(self) fn set_lobbies(&mut self, lobbies: DashMap<Uuid, GameLobby>) {
        self.lobbies = lobbies;
    }

    #[cfg(test)]
    pub(self) fn get_lobbies(&self) -> &DashMap<Uuid, GameLobby> {
        &self.lobbies
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    #[test]
    fn new_givenGameStateConfig_configSavedInGameState() {
        // Arrange
        let config = GameStateConfig::default();

        // Act
        let game_state = DefaultGameState::new(config.clone(), CancellationToken::new());

        // Assert
        assert_eq!(&config, game_state.get_config());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_game_gameIdIsNotAValidId_returnsBadRequest() {
        // Arrange
        let config = GameStateConfig {
            number_of_players_in_game_lobby: 0,
            ..Default::default()
        };
        let cancellation_token = CancellationToken::new();

        let (_, client_rx) = unbounded_channel();
        let (server_tx, _) = unbounded_channel();

        let game_state = DefaultGameState::new(config, cancellation_token);

        let welcome_message = ClientWelcome {
            nick: "nick".to_owned(),
            game_id: Some("not-a-real-uuid".to_owned()),
        };

        // Act
        let result = game_state
            .join_game(
                welcome_message,
                Arc::new(Mutex::new(client_rx)),
                Arc::new(Mutex::new(server_tx)),
            )
            .await;

        // Assert
        match result {
            JoinGameResult::BadRequest(_, _) => {}
            result => assert!(false, "result is not a BadRequest. It's {:?}", result),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_game_gameIdDoesNotExist_returnsGameDoesNotExist() {
        // Arrange
        let config = GameStateConfig {
            number_of_players_in_game_lobby: 0,
            ..Default::default()
        };
        let cancellation_token = CancellationToken::new();

        let (_, client_rx) = unbounded_channel();
        let (server_tx, _) = unbounded_channel();

        let game_state = DefaultGameState::new(config, cancellation_token);

        let welcome_message = ClientWelcome {
            nick: "nick".to_owned(),
            game_id: Some(Uuid::new_v4().to_string()),
        };

        // Act
        let result = game_state
            .join_game(
                welcome_message,
                Arc::new(Mutex::new(client_rx)),
                Arc::new(Mutex::new(server_tx)),
            )
            .await;

        // Assert
        match result {
            JoinGameResult::GameDoesNotExit(_, _) => {}
            result => assert!(false, "result is not a GameDoesNotExit. It's {:?}", result),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_game_gameIdExists_addsPlayerToThelobbyAndReturnsOk() {
        // Arrange
        let config = GameStateConfig {
            number_of_players_in_game_lobby: 5,
            ..Default::default()
        };
        let cancellation_token = CancellationToken::new();

        let (_, client_rx) = unbounded_channel();
        let (server_tx, _) = unbounded_channel();

        let mut game_state = DefaultGameState::new(config.clone(), cancellation_token.clone());

        let uuid = Uuid::new_v4();

        let lobbies = DashMap::with_capacity(1);
        lobbies.insert(uuid, GameLobby::new(config, cancellation_token));

        game_state.set_lobbies(lobbies);

        let welcome_message = ClientWelcome {
            nick: "nick".to_owned(),
            game_id: Some(uuid.to_string()),
        };

        // Act
        let result = game_state
            .join_game(
                welcome_message,
                Arc::new(Mutex::new(client_rx)),
                Arc::new(Mutex::new(server_tx)),
            )
            .await;

        // Assert
        match result {
            JoinGameResult::Ok => {}
            result => assert!(false, "result is not an Ok. It's {:?}", result),
        }

        let number_of_players = game_state
            .get_lobbies()
            .iter()
            .next()
            .unwrap()
            .get_players()
            .len();

        assert_eq!(1, number_of_players);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_game_gameIsFull_doesNotAddThePlayerReturnsGameIsFull() {
        // Arrange
        let config = GameStateConfig {
            number_of_players_in_game_lobby: 0,
            ..Default::default()
        };
        let cancellation_token = CancellationToken::new();

        let (_, client_rx) = unbounded_channel();
        let (server_tx, _) = unbounded_channel();

        let mut game_state = DefaultGameState::new(config.clone(), cancellation_token.clone());

        let uuid = Uuid::new_v4();

        let mut lobby = GameLobby::new(config, cancellation_token);
        lobby.set_game_lobby_state(GameLobbyStatus::Ready).await;

        let lobbies = DashMap::with_capacity(1);
        lobbies.insert(uuid, lobby);

        game_state.set_lobbies(lobbies);

        let welcome_message = ClientWelcome {
            nick: "nick".to_owned(),
            game_id: Some(uuid.to_string()),
        };

        // Act
        let result = game_state
            .join_game(
                welcome_message,
                Arc::new(Mutex::new(client_rx)),
                Arc::new(Mutex::new(server_tx)),
            )
            .await;

        // Assert
        match result {
            JoinGameResult::GameIsFull(_, _) => {}
            result => assert!(false, "result is not an GameIsFull. It's {:?}", result),
        }

        let number_of_players = game_state
            .get_lobbies()
            .iter()
            .next()
            .unwrap()
            .get_players()
            .len();

        assert_eq!(0, number_of_players);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn join_game_gameIdIsNone_findANewGameForThePlayerAndReturnsOk() {
        // Arrange
        let config = GameStateConfig {
            number_of_players_in_game_lobby: 2,
            ..Default::default()
        };
        let cancellation_token = CancellationToken::new();

        let (_, client_rx) = unbounded_channel();
        let (server_tx, _) = unbounded_channel();

        let game_state = DefaultGameState::new(config.clone(), cancellation_token.clone());

        let welcome_message = ClientWelcome {
            nick: "nick".to_owned(),
            game_id: None,
        };

        // Act
        let result = game_state
            .join_game(
                welcome_message,
                Arc::new(Mutex::new(client_rx)),
                Arc::new(Mutex::new(server_tx)),
            )
            .await;

        // Assert
        match result {
            JoinGameResult::Ok => {}
            result => assert!(false, "result is not an Ok. It's {:?}", result),
        }

        let lobbies = game_state.get_lobbies();
        assert_eq!(1, lobbies.len());

        let number_of_players = lobbies.iter().next().unwrap().get_players().len();
        assert_eq!(1, number_of_players);
    }
}
