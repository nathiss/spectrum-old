use super::model::server_message::*;
use super::model::*;

pub fn make_server_welcome(status_code: server_welcome::StatusCode) -> ServerMessage {
    ServerMessage {
        server_message_data: Some(ServerMessageData::WelcomeMessage(ServerWelcome {
            status_code: status_code.into(),
        })),
    }
}

pub fn make_lobby_update(
    status_code: lobby_update::StatusCode,
    players: Vec<String>,
) -> ServerMessage {
    ServerMessage {
        server_message_data: Some(ServerMessageData::LobbyUpdate(LobbyUpdate {
            status_code: status_code.into(),
            players,
        })),
    }
}
