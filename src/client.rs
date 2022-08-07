use std::net::SocketAddr;

use log::error;

use crate::{network::Connection, packet::PacketSerializer};

#[derive(Debug)]
pub struct Client<C: Connection, S: PacketSerializer> {
    connection: C,
    serializer: S,
}

impl<C: Connection, S: PacketSerializer> Client<C, S> {
    pub fn new(connection: C, serializer: S) -> Self {
        Self {
            connection,
            serializer,
        }
    }

    pub async fn read_packet(&mut self) -> Option<S::Packet> {
        match self.connection.read_bytes().await {
            Some(data) => match self.serializer.deserialize(data) {
                Ok(packet) => Some(packet),
                Err(e) => {
                    error!(
                        "Failed to deserialize the client's package from {}. Error: {}",
                        self.addr(),
                        e,
                    );

                    None
                }
            },
            None => None,
        }
    }

    pub async fn write_packet(&mut self, packet: &S::Packet) -> Result<(), anyhow::Error> {
        let data = self.serializer.serialize(&packet);

        self.connection.write_bytes(data).await
    }

    pub fn addr(&self) -> &SocketAddr {
        self.connection.addr()
    }
}
