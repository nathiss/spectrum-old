use std::io::Cursor;

use prost::Message;

use super::{model::ClientMessage, PacketSerializer};

#[derive(Debug, Default, Copy, Clone)]
pub struct ProtobufPacketSerializer;

impl PacketSerializer for ProtobufPacketSerializer {
    type Packet = ClientMessage;

    fn serialize(&self, message: &Self::Packet) -> Vec<u8> {
        message.encode_to_vec()
    }

    fn deserialize(&self, raw: Vec<u8>) -> Result<Self::Packet, anyhow::Error> {
        Ok(ClientMessage::decode(&mut Cursor::new(raw))?)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::{PacketSerializer, ProtobufPacketSerializer};

    #[test]
    fn deserialize_givenVectorIsEmpty_returnsError() {
        let serializer = ProtobufPacketSerializer::default();
        let incorrect_data = vec![13u8, 37u8, 42u8];

        // Act
        let result = serializer.deserialize(incorrect_data);

        // Assert
        assert!(result.is_err());
    }
}
