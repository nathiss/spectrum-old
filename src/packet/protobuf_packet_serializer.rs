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
