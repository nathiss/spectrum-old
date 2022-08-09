use std::io::Cursor;

use prost::Message;

use super::{model::ServerMessage, PacketSerializer};

#[derive(Debug, Default, Copy, Clone)]
pub struct ServerMessagePacketSerializer;

impl PacketSerializer for ServerMessagePacketSerializer {
    type Packet = ServerMessage;

    fn serialize(&self, message: &Self::Packet) -> Vec<u8> {
        message.encode_to_vec()
    }

    fn deserialize(&self, raw: Vec<u8>) -> Result<Self::Packet, anyhow::Error> {
        Ok(ServerMessage::decode(&mut Cursor::new(raw))?)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_givenVectorWithInvalidData_returnsError() {
        let serializer = ServerMessagePacketSerializer::default();
        let incorrect_data = vec![13u8, 37u8, 42u8];

        // Act
        let result = serializer.deserialize(incorrect_data);

        // Assert
        assert!(result.is_err());
    }
}
