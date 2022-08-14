use super::model::server_message::*;
use super::model::*;

pub fn make_server_welcome(status_code: server_welcome::StatusCode) -> ServerMessage {
    ServerMessage {
        server_message_data: Some(ServerMessageData::WelcomeMessage(ServerWelcome {
            status_code: status_code.into(),
        })),
    }
}
