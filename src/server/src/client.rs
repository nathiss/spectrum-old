use std::{
    hash::{Hash, Hasher},
    net::SocketAddr,
};

use log::{debug, error, warn};
use spectrum_network::Connection;
use spectrum_packet::{
    model::{ClientMessage, ServerMessage},
    ClientMessagePacketSerializer, PacketSerializer, ServerMessagePacketSerializer,
};
use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};
use tokio_util::sync::CancellationToken;

/// This struct is a mid-level representation of a connected network peer.
///
/// It takes care of serializing server messages before they are send via the network layer and deserializing client's
/// messages once they are received from the incoming stream.
#[derive(Debug)]
pub struct Client {
    connection: Box<dyn Connection>,
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
        connection: Box<dyn Connection>,
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
    /// # Arguments
    ///
    /// * `cancellation_token` - A cancellation token for this Future. This method will await internally on the token
    ///                          and when it's cancelled, then it completes the future immediately.
    ///
    /// # Panics
    ///
    /// Due to the fact that this method internally uses `Connection::get_incoming_data_channel()` method, it can only
    /// be called once. The second call of this method will result in a panic.
    pub async fn open_package_stream(&mut self, cancellation_token: CancellationToken) {
        let mut raw_data_rx = self.connection.get_incoming_data_channel();

        let (packet_tx, packet_rx) = unbounded_channel();
        let addr = *self.connection.addr();
        let client_deserializer = self.client_serializer;

        tokio::spawn(async move {
            loop {
                select! {
                    biased;

                    _ = cancellation_token.cancelled() => {
                        debug!("Stream of raw packets for {} has been cancelled. Returning immediately.", addr);
                        break;
                    }

                    raw_data = raw_data_rx.recv() => {
                        match raw_data {
                            Some(raw_data) => {
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
                            None => {
                                // This means that the packet channel is exhausted.
                                break;
                            }
                        }
                    }
                }
            }
        });

        self.packet_rx = Some(packet_rx);
    }

    /// This method returns a receiving half of the incoming messages stream.
    ///
    /// # Panics
    ///
    /// The internal receiver is consumed by this method. It panics if called more than once or if it was called before
    /// the stream was properly setup by `open_package_stream`.
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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::{net::IpAddr, sync::Arc, time::Duration};

    use async_trait::async_trait;
    use tokio::sync::{mpsc::error::TryRecvError, Mutex};

    use super::*;
    use spectrum_packet::model::{
        client_message::ClientMessageData, server_leave::StatusCode,
        server_message::ServerMessageData, *,
    };

    #[derive(Debug)]
    struct MockConnection {
        pub incoming_data_channel: Option<UnboundedReceiver<Vec<u8>>>,
        pub data: Arc<Mutex<Vec<u8>>>,
        pub addr: SocketAddr,
        pub write_bytes_should_fail: bool,
    }

    #[async_trait]
    impl Connection for MockConnection {
        fn get_incoming_data_channel(&mut self) -> UnboundedReceiver<Vec<u8>> {
            self.incoming_data_channel.take().unwrap()
        }

        async fn write_bytes(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error> {
            if self.write_bytes_should_fail {
                return Err(anyhow::Error::msg("failed"));
            }

            self.data.lock().await.clone_from(&data);

            Ok(())
        }

        fn addr(&self) -> &SocketAddr {
            &self.addr
        }
    }

    #[test]
    fn addr_givenValidConnection_addrIsProperlyRetrieved() {
        // Arrange
        let addr = SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080);

        let connection = MockConnection {
            addr: addr,
            write_bytes_should_fail: false,
            data: Arc::new(Mutex::new(Vec::new())),
            incoming_data_channel: None,
        };

        // Act
        let client = Client::new(Box::new(connection), Default::default(), Default::default());

        // Assert
        assert_eq!(addr, client.addr());
    }

    #[test]
    #[should_panic]
    fn get_packet_channel_callBeforeOpenPackageStream_panics() {
        // Arrange
        let mut connection = MockConnection {
            addr: SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080),
            write_bytes_should_fail: false,
            data: Arc::new(Mutex::new(Vec::new())),
            incoming_data_channel: None,
        };

        // Act
        connection.get_incoming_data_channel();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn open_package_stream_sendPacketsBeforePackageStreamConstruction_yieldsPreviousPackets()
    {
        // Arrange
        let (tx, rx) = unbounded_channel();

        let connection = MockConnection {
            addr: SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080),
            write_bytes_should_fail: false,
            data: Arc::new(Mutex::new(Vec::new())),
            incoming_data_channel: Some(rx),
        };

        let mut client = Client::new(Box::new(connection), Default::default(), Default::default());

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: "foo".to_owned(),
                game_id: Some("bar".to_owned()),
            })),
        };

        let client_message = ClientMessagePacketSerializer::default().serialize(&client_message);

        tx.send(client_message.clone()).unwrap();
        tx.send(client_message).unwrap();

        let _ = client.open_package_stream(CancellationToken::new()).await;

        // Act
        let mut packet_rx = client.get_packet_channel();

        // Assert
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(packet_rx.try_recv().is_ok());
        assert!(packet_rx.try_recv().is_ok());
        assert_eq!(TryRecvError::Empty, packet_rx.try_recv().err().unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn open_package_stream_channelReceivedPacketsAfterCancelling_yieldsNoPreviousPackets() {
        // Arrange
        let (tx, rx) = unbounded_channel();

        let connection = MockConnection {
            addr: SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080),
            write_bytes_should_fail: false,
            data: Arc::new(Mutex::new(Vec::new())),
            incoming_data_channel: Some(rx),
        };

        let mut client = Client::new(Box::new(connection), Default::default(), Default::default());

        let client_message = ClientMessage {
            client_message_data: Some(ClientMessageData::WelcomeMessage(ClientWelcome {
                nick: "foo".to_owned(),
                game_id: Some("bar".to_owned()),
            })),
        };

        let client_message = ClientMessagePacketSerializer::default().serialize(&client_message);

        let token = CancellationToken::new();

        let _ = client.open_package_stream(token.clone()).await;

        // Act
        token.cancel();

        tx.send(client_message.clone()).unwrap();
        tx.send(client_message).unwrap();

        let mut packet_rx = client.get_packet_channel();

        // Assert
        // Without this timeout, the error was `TryRecvError::Empty`, which is still acceptable in this case.
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(
            TryRecvError::Disconnected,
            packet_rx.try_recv().err().unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn write_packet_connectionSendBytesFails_returnsError() {
        // Arrange
        let connection = MockConnection {
            addr: SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080),
            write_bytes_should_fail: true,
            data: Arc::new(Mutex::new(Vec::new())),
            incoming_data_channel: None,
        };

        let mut client = Client::new(Box::new(connection), Default::default(), Default::default());

        // Act
        let result = client
            .write_packet(&ServerMessage {
                server_message_data: None,
            })
            .await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn write_packet_sendsPackage_connectionReceivesTheSameBytes() -> Result<(), anyhow::Error>
    {
        // Arrange
        let data_container = Arc::new(Mutex::new(Vec::new()));

        let connection = MockConnection {
            addr: SocketAddr::new(IpAddr::from([127u8, 0u8, 0u8, 1u8]), 8080),
            write_bytes_should_fail: false,
            data: data_container.clone(),
            incoming_data_channel: None,
        };

        let mut client = Client::new(Box::new(connection), Default::default(), Default::default());

        let server_message = &ServerMessage {
            server_message_data: Some(ServerMessageData::LeaveMessage(ServerLeave {
                status_code: StatusCode::Success.into(),
            })),
        };

        let serialized_server_message =
            ServerMessagePacketSerializer::default().serialize(&server_message);

        // Act
        client.write_packet(&server_message).await?;

        // Assert
        assert_eq!(
            serialized_server_message,
            data_container.lock().await.clone()
        );

        Ok(())
    }
}
