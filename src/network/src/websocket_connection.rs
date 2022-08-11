use std::net::SocketAddr;

use async_trait::async_trait;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt, TryStreamExt,
};
use log::{debug, error, info, warn};
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

use super::Connection;

struct ShouldCloseReceiver(bool);

impl Into<bool> for ShouldCloseReceiver {
    fn into(self) -> bool {
        self.0
    }
}

/// This struct represents a WebSocket connection with a single peer.
#[derive(Debug)]
pub struct WebSocketConnection {
    addr: SocketAddr,
    output_stream: SplitSink<WebSocketStream<TcpStream>, Message>,
    input_queue: Option<UnboundedReceiver<Vec<u8>>>,
}

#[async_trait]
impl Connection for WebSocketConnection {
    fn get_incoming_data_channel(&mut self) -> UnboundedReceiver<Vec<u8>> {
        match self.input_queue.take() {
            Some(queue) => queue,
            None => panic!("get_incoming_data_channel can only be called once."),
        }
    }

    async fn write_bytes(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error> {
        match self.output_stream.send(Message::Binary(data)).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}

impl WebSocketConnection {
    /// This method is used to construct a `WebSocketConnection` struct.
    ///
    /// # Arguments
    ///
    /// * `ws_stream` - A network stream connected to the peer. It is assumed that the connection is in a valid state.
    /// * `addr` - An internet socket address of the peer. It is assumed that this object represents an endpoint to
    ///            which `ws_stream` is connected to.
    pub(super) fn new(ws_stream: WebSocketStream<TcpStream>, addr: SocketAddr) -> Self {
        let (output_stream, input_stream) = ws_stream.split();

        let (tx, rx) = unbounded_channel();

        tokio::spawn(async move { Self::handle_incoming_messages(input_stream, tx, addr).await });

        Self {
            addr,
            output_stream,
            input_queue: Some(rx),
        }
    }

    /// This method is used to handle an incoming packet stream and process all messages from it.
    ///
    /// # Arguments
    ///
    /// * `input_stream` - a receiving half of the network stream. It is assumed that the stream is in a valid state.
    /// * `tx` - A sender for the incoming messages queue. This method ensures that only valid (i.e. Binary) messages
    ///          are sent to the queue.
    /// * `addr` - An internet socket address of the peer.
    ///
    /// # Returns
    ///
    /// The Future returned from this method completes in either of those cases:
    /// * the incoming stream is exhausted,
    /// * the client sent a Close message,
    /// * the client sent an invalid message.
    async fn handle_incoming_messages(
        mut input_stream: SplitStream<WebSocketStream<TcpStream>>,
        tx: UnboundedSender<Vec<u8>>,
        addr: SocketAddr,
    ) {
        loop {
            match input_stream.try_next().await {
                Ok(message) => {
                    match message {
                        Some(message) => {
                            let should_be_closed = Self::parse_message_type(message, &tx, &addr);

                            if should_be_closed.into() {
                                break;
                            }
                        }
                        None => {
                            debug!("Reading stream from {} has been exhausted. The reader will not exit.", addr);

                            break;
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "An error occurred while trying to read a message from {}. Error: {}",
                        addr, e
                    );

                    break;
                }
            }
        }
    }

    /// This method consumes a single message from the peer and processes it.
    ///
    /// # Arguments
    ///
    /// * `message` - A message from the peer.
    /// * `tx` - A sender half of the queue for new messages. The sender is only used to queue up binary messages from
    ///          the peer. If the channel has been closed, then a warning will be logged and the message will be
    ///          dropped.
    /// * `addr` - An internet socket address of the peer.
    ///
    /// # Returns
    ///
    /// An indication of whether the reading loop should be terminated is returned.
    fn parse_message_type(
        message: Message,
        tx: &UnboundedSender<Vec<u8>>,
        addr: &SocketAddr,
    ) -> ShouldCloseReceiver {
        match message {
            Message::Binary(data) => {
                if let Err(_e) = tx.send(data) {
                    warn!(
                        "Failed to send binary message from {} to the channel. Probably the receiving half has been closed",
                        addr
                    );
                }

                ShouldCloseReceiver(false)
            }
            Message::Ping(ping_data) => {
                debug!("Got a ping message from {}. Data: {:?}", addr, ping_data);

                ShouldCloseReceiver(false)
            }
            Message::Pong(pong_data) => {
                debug!("Got a pong message from {}. Data: {:?}", addr, pong_data);

                ShouldCloseReceiver(false)
            }
            Message::Close(_close_frame) => {
                info!("Connection from {} has been closed by the peer.", addr);

                ShouldCloseReceiver(true)
            }
            _ => {
                error!("Got a message from {} which is nighter a binary message not a control message.", addr);

                ShouldCloseReceiver(true)
            }
        }
    }
}
