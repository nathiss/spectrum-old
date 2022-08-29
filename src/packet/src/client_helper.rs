use super::model::client_message::ClientMessageData;
use super::model::ClientMessage;

pub fn is_player_ready(client_message: &ClientMessage) -> bool {
    matches!(
        &client_message.client_message_data,
        Some(ClientMessageData::PlayerReady(_))
    )
}
