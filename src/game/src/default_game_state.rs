use std::str::FromStr;

use async_trait::async_trait;
use dashmap::DashMap;
use log::error;
use spectrum_packet::model::{ClientMessage, ClientWelcome, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    game_room::GameRoom, game_room_status::GameRoomStatus, player::Player, GameState,
    GameStateConfig, JoinGameResult,
};

/// This struct represents the game state.
///
/// It is responsible for managing players, game rooms and game's business logic.
#[derive(Debug)]
pub struct DefaultGameState {
    config: GameStateConfig,
    cancellation_token: CancellationToken,
    rooms: DashMap<Uuid, GameRoom>,
}

#[async_trait]
impl GameState for DefaultGameState {
    async fn join_game(
        &self,
        welcome_message: ClientWelcome,
        packet_rx: UnboundedReceiver<ClientMessage>,
        packet_tx: UnboundedSender<ServerMessage>,
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

            match self.rooms.get_mut(&uuid) {
                Some(game_room) => match game_room.get_state().await {
                    GameRoomStatus::Waiting => {
                        let player = Player::new(packet_rx, packet_tx);
                        game_room.add_player(welcome_message.nick, player).await;

                        return JoinGameResult::Ok;
                    }
                    GameRoomStatus::Running => {
                        return JoinGameResult::GameIsFull(packet_rx, packet_tx);
                    }
                },
                None => {
                    error!("Game with ID {} does not exist.", uuid);
                    return JoinGameResult::GameDoesNotExit(packet_rx, packet_tx);
                }
            }
        }

        // If game_id is empty then we need to create a new game room.
        let game_room = GameRoom::new(self.config.clone(), self.cancellation_token.clone());

        let player = Player::new(packet_rx, packet_tx);

        game_room.add_player(welcome_message.nick, player).await;

        self.rooms.insert(Uuid::new_v4(), game_room);

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
            rooms: DashMap::new(),
        }
    }

    #[cfg(test)]
    pub(self) fn get_config(&self) -> &GameStateConfig {
        &self.config
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
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
}
