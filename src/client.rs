use std::net::SocketAddr;

use log::error;
use spectrum_network::Connection;
use spectrum_packet::PacketSerializer;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[derive(Debug)]
pub struct Client<C: Connection, CS: PacketSerializer, SS: PacketSerializer> {
    connection: C,
    packet_rx: Option<UnboundedReceiver<CS::Packet>>,
    server_serializer: SS,
}

impl<C: Connection, CS: PacketSerializer + 'static, SS: PacketSerializer + 'static>
    Client<C, CS, SS>
{
    pub fn new(mut connection: C, client_serializer: CS, server_serializer: SS) -> Self {
        let mut raw_data_rx = connection.get_incoming_data_channel();
        let (packet_tx, packet_rx) = unbounded_channel();

        let addr = *connection.addr();

        tokio::spawn(async move {
            while let Some(raw_data) = raw_data_rx.recv().await {
                match client_serializer.deserialize(raw_data) {
                    Ok(packet) => drop(packet_tx.send(packet)),
                    Err(e) => {
                        error!("Failed to deserialize data from {}. Error: {}", addr, e);

                        break;
                    }
                }
            }

            // This means that the network client (Connection) has closed the other side of the channel.
            // In that case we simply exit and drop out tx.
        });

        Self {
            connection,
            packet_rx: Some(packet_rx),
            server_serializer,
        }
    }

    pub fn get_packet_channel(&mut self) -> UnboundedReceiver<CS::Packet> {
        match self.packet_rx.take() {
            Some(queue) => queue,
            None => panic!("get_packet_channel can only be called once."),
        }
    }

    pub async fn write_packet(&mut self, packet: &SS::Packet) -> Result<(), anyhow::Error> {
        let data = self.server_serializer.serialize(packet);

        self.connection.write_bytes(data).await
    }

    pub fn addr(&self) -> &SocketAddr {
        self.connection.addr()
    }
}
