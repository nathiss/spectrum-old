use std::marker::Send;

pub trait PacketSerializer: Copy + Send {
    type Packet: Send;

    fn serialize(&self, message: &Self::Packet) -> Vec<u8>;

    fn deserialize(&self, raw: Vec<u8>) -> Result<Self::Packet, anyhow::Error>;
}
