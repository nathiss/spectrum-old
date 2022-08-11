use std::{
    hash::{Hash, Hasher},
    net::SocketAddr,
};

use futures::Future;
use log::{debug, error, warn};
use spectrum_network::{Connection, WebSocketConnection};
use spectrum_packet::{
    model::{ClientMessage, ServerMessage},
    ClientMessagePacketSerializer, PacketSerializer, ServerMessagePacketSerializer,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::util::convert_to_future;

/// This struct is a mid-level representation of a connected network peer.
///
/// It takes care of serializing server messages before they are send via the network layer and deserializing client's
/// messages once they are received from the incoming stream.
#[derive(Debug)]
pub struct Client {
    connection: WebSocketConnection,
    packet_rx: Option<UnboundedReceiver<ClientMessage>>,
    client_serializer: ClientMessagePacketSerializer,
    server_serializer: ServerMessagePacketSerializer,
}

impl Client {
    /// This method is used to construct an instance of `Client`.
    ///
    /// # Arguments
    ///
    /// * `connection` - A WebSocket connection to the network peer.
    /// * `client_serializer` - A serializer/deserializer for the messages received from the network peer.
    /// * `server_serializer` - A serializer/deserializer for the messages sent from the server to the network peer.
    pub fn new(
        connection: WebSocketConnection,
        client_serializer: ClientMessagePacketSerializer,
        server_serializer: ServerMessagePacketSerializer,
    ) -> Self {
        Self {
            connection,
            packet_rx: None,
            client_serializer,
            server_serializer,
        }
    }

    /// This method creates a converter for raw network packages and turns them into `ClientMessage` objects.
    ///
    /// The conversion is run in a loop which exists if the underlying stream is exhausted. The loop is wrapped into an
    /// asynchronous task and the handle of that task is returned.
    ///
    /// # Panics
    ///
    /// Due to the fact that this method internally uses `Connection::get_incoming_data_channel()` method, it can only
    /// be called once. The second call of this method will result in a panic.
    ///
    /// # Returns
    ///
    /// A handle into the asynchronous conversion task is returned.
    pub async fn open_package_stream(&mut self) -> impl Future<Output = ()> {
        let mut raw_data_rx = self.connection.get_incoming_data_channel();

        let (packet_tx, packet_rx) = unbounded_channel();
        let addr = *self.connection.addr();
        let client_deserializer = self.client_serializer.clone();

        let handle = tokio::spawn(async move {
            while let Some(raw_data) = raw_data_rx.recv().await {
                match client_deserializer.deserialize(raw_data) {
                    Ok(message) => {
                        if let Err(_e) = packet_tx.send(message) {
                            warn!(
                                "Failed to send message from {} to the channel. \
                                Probably the receiving half has been closed",
                                addr
                            );
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize data from {}", addr);
                        debug!("Failed to deserialize data from {}. Error: {}", addr, e);

                        break;
                    }
                }
            }

            // This means that either we broke the loop due to the deserializer error or the packet channel is
            // exhausted. Either way we finish.
        });

        self.packet_rx = Some(packet_rx);

        convert_to_future(handle)
    }

    /// This method returns a receiving half of the incoming messages stream.
    ///
    /// # Panics
    ///
    /// The internal receiver is consumed by this method. It panics if called more than once.
    pub fn get_packet_channel(&mut self) -> UnboundedReceiver<ClientMessage> {
        match self.packet_rx.take() {
            Some(queue) => queue,
            None => panic!("get_packet_channel can only be called once."),
        }
    }

    /// This method allows to write messages to the network peer.
    ///
    /// # Returns
    ///
    /// * `()` - if the operation was successful,
    /// * `anyhow::Error` - if an error occurred during package transmission.
    pub async fn write_packet(&mut self, packet: &ServerMessage) -> Result<(), anyhow::Error> {
        let data = self.server_serializer.serialize(packet);

        self.connection.write_bytes(data).await?;

        Ok(())
    }

    /// This method returns an internet socket address of the connected peer.
    pub fn addr(&self) -> SocketAddr {
        *self.connection.addr()
    }
}

impl Hash for Client {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.connection.addr().hash(state);
    }
}
