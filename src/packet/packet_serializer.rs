pub trait PacketSerializer {
    type Packet;

    fn serialize(&self, message: &Self::Packet) -> Vec<u8>;

    fn deserialize(&self, raw: Vec<u8>) -> Result<Self::Packet, anyhow::Error>;
}
