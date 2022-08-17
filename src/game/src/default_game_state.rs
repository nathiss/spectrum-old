use std::str::FromStr;

use dashmap::DashMap;
use log::error;
use spectrum_packet::model::{ClientMessage, ClientWelcome, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    game_room::GameRoom, game_room_status::GameRoomStatus, GameState, GameStateConfig,
    JoinGameResult,
};

/// This struct represents the game state.
///
/// It is responsible for managing players, game rooms and game's business logic.
#[derive(Debug)]
pub struct DefaultGameState {
    _config: GameStateConfig,
    _cancellation_token: CancellationToken,
    rooms: DashMap<Uuid, GameRoom>,
}

impl GameState for DefaultGameState {
    /// This method *tries* to add a new player to a game room (either existing or a new one).
    ///
    /// # Arguments
    ///
    /// * `welcome_message` - This message contains the unique identifier of the game and player's information.
    /// * `packet_rx` - This stream is used to receive messages from the player.
    /// * `packet_tx_ - This sink is used to send messages to the player.
    ///
    /// # Returns
    ///
    /// A result of this operation is returned. For more details see: [`JoinGameResult`].
    fn join_game(
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
                Some(game_room) => {
                    match game_room.get_state() {
                        GameRoomStatus::Waiting => {
                            // TODO: add player to the room
                            return JoinGameResult::Ok;
                        }
                        GameRoomStatus::Running => {
                            return JoinGameResult::GameIsFull(packet_rx, packet_tx);
                        }
                    }
                }
                None => {
                    error!("Game with ID {} does not exist.", uuid);
                    return JoinGameResult::GameDoesNotExit(packet_rx, packet_tx);
                }
            }
        }

        // This mean that the game needs to be created.
        todo!()
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
            _config: config,
            _cancellation_token: cancellation_token,
            rooms: DashMap::new(),
        }
    }

    #[cfg(test)]
    pub(self) fn get_config(&self) -> &GameStateConfig {
        &self._config
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
