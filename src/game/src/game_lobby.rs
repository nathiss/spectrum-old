use std::{collections::HashMap, time::Duration};

use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, error, warn};
use spectrum_packet::{is_player_ready, make_lobby_update, model::lobby_update};
use tokio::{select, sync::RwLock, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::{game_lobby_status::GameLobbyStatus, player::Player, GameStateConfig};

pub struct HasGameLobbyStarted(bool);

impl Into<bool> for HasGameLobbyStarted {
    fn into(self) -> bool {
        self.0
    }
}

/// This struct represents a single game lobby and all its associated content.
#[derive(Debug)]
pub(crate) struct GameLobby {
    config: GameStateConfig,
    players: HashMap<String, Player>,
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
            players: HashMap::with_capacity(maximum_number_of_players),
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
    pub async fn add_player(&mut self, nick: String, player: Player) -> HasGameLobbyStarted {
        // This lock guard ensures that only one thread can access this method at any given moment.
        let mut game_status = self.game_lobby_status.write().await;

        if *game_status != GameLobbyStatus::Waiting {
            warn!(
                "Tried to add player {} to a game that's already started.",
                nick
            );

            return HasGameLobbyStarted(true);
        }

        self.players.insert(nick, player);

        if self.players.len() < self.config.number_of_players_in_game_lobby {
            self.broadcast_players(lobby_update::StatusCode::Waiting)
                .await;
            HasGameLobbyStarted(false)
        } else {
            *game_status = GameLobbyStatus::Ready;

            // Mutable reference needs to be dropped to prevent dead-locking on `game_lobby_status`.
            drop(game_status);

            self.broadcast_players(lobby_update::StatusCode::GameReady)
                .await;
            self.start().await;
            HasGameLobbyStarted(true)
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
            panic!("GameLobby::start() can only be called once.");
        }

        *game_lobby_state = GameLobbyStatus::Ready;
        drop(game_lobby_state);

        let cancellation_token = self.cancellation_token.clone();

        let mut read_futures = FuturesUnordered::new();

        for (nick, player) in &self.players {
            let cancellation_token = cancellation_token.clone();
            let client_rx = player.get_client_stream();

            let nick = nick.clone();

            let read_or_timeout_future = async move {
                select! {
                    biased;

                    _ = cancellation_token.cancelled() => {
                        debug!("listening for PlayerReady message from {} has been cancelled.", nick);
                        (nick, None)
                    }

                    result = timeout(Duration::from_secs(30), async move {
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

        tokio::spawn(async move {
            while let Some((nick, message)) = read_futures.next().await {
                if let None = message {
                    warn!(
                        "Player {} failed to confirm readiness within timeout window",
                        nick
                    );
                    // TODO: remove player from the lobby
                    continue;
                }

                let message = message.unwrap();

                if !is_player_ready(&message) {
                    error!(
                        "Player {} sent incorrect type of message. Got: {:?}",
                        nick, message
                    );
                    // TODO: remove player from the lobby
                    continue;
                }

                // This means the player correctly sent us `PlayerReady` message. We can continue to process the rest of
                // the players.
            }
        });
    }

    pub async fn get_state(&self) -> GameLobbyStatus {
        self.game_lobby_status.read().await.clone()
    }

    async fn broadcast_players(&self, status_code: lobby_update::StatusCode) {
        let nicks: Vec<_> = self.players.keys().cloned().collect();

        let lobby_update = make_lobby_update(status_code, nicks);

        for (nick, player) in &self.players {
            let player_sink = player.get_server_sink();
            let player_stream = player_sink.lock().await;
            if let Err(e) = player_stream.send(lobby_update.clone()) {
                error!(
                    "Failed to send lobby update to player {}. Error: {}",
                    nick, e
                );
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn get_players(&self) -> &HashMap<String, Player> {
        &self.players
    }

    #[cfg(test)]
    pub(crate) async fn set_game_lobby_state(&mut self, state: GameLobbyStatus) {
        *self.game_lobby_status.write().await = state;
    }
}
