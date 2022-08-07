use std::net::SocketAddr;

use log::error;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::{network::Connection, packet::PacketSerializer};

#[derive(Debug)]
pub struct Client<C: Connection, S: PacketSerializer> {
    connection: C,
    packet_rx: Option<UnboundedReceiver<S::Packet>>,
    serializer: S,
}

impl<C: Connection, S: PacketSerializer + 'static> Client<C, S> {
    pub fn new(mut connection: C, serializer: S) -> Self {
        let mut raw_data_rx = connection.get_incoming_data_channel();
        let (packet_tx, packet_rx) = unbounded_channel();

        let addr = connection.addr().clone();
        let deserializer = serializer.clone();

        tokio::spawn(async move {
            while let Some(raw_data) = raw_data_rx.recv().await {
                match deserializer.deserialize(raw_data) {
                    Ok(packet) => drop(packet_tx.send(packet)),
                    Err(e) => {
                        error!("Failed to deserialize data from {}. Error: {}", addr, e);

                        break;
                    }
                }
            }

            // This means tPhat the network client (Connection) has closed the other side of the channel.
            // In that case we simply exit and drop out tx.
        });

        Self {
            connection,
            packet_rx: Some(packet_rx),
            serializer,
        }
    }

    pub fn get_packet_channel(&mut self) -> UnboundedReceiver<S::Packet> {
        match self.packet_rx.take() {
            Some(queue) => queue,
            None => panic!("get_packet_channel can only be called once."),
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
