use std::collections::HashMap;

use spectrum_packet::model::{ClientMessage, ClientWelcome, ServerMessage};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{game_room::GameRoom, GameState, GameStateConfig, JoinGameResult};

#[derive(Debug)]
pub struct DefaultGameState {
    _config: GameStateConfig,
    _cancellation_token: CancellationToken,
    _rooms: HashMap<Uuid, GameRoom>,
}

impl GameState for DefaultGameState {
    fn join_game(
        &mut self,
        _welcome_message: ClientWelcome,
        packet_rx: UnboundedReceiver<ClientMessage>,
        packet_tx: UnboundedSender<ServerMessage>,
    ) -> JoinGameResult {
        JoinGameResult::NickTaken(packet_rx, packet_tx)
    }
}

impl DefaultGameState {
    pub fn new(config: GameStateConfig, cancellation_token: CancellationToken) -> Self {
        Self {
            _config: config,
            _cancellation_token: cancellation_token,
            _rooms: HashMap::new(),
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
