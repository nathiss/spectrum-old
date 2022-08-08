use std::net::SocketAddr;

use async_trait::async_trait;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt, TryStreamExt,
};
use log::{debug, error, warn};
use tokio::{
    net::TcpStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

use super::Connection;

#[derive(Debug)]
pub(crate) struct WebSocketConnection {
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

    async fn handle_incoming_messages(
        mut input_stream: SplitStream<WebSocketStream<TcpStream>>,
        tx: UnboundedSender<Vec<u8>>,
        addr: SocketAddr,
    ) {
        loop {
            match input_stream.try_next().await {
                Ok(message) => match message {
                    Some(message) => match message {
                        Message::Close(_) => {
                            debug!(
                                "The connection from {} has been closed by the other side.",
                                addr
                            );
                            break;
                        }
                        Message::Binary(message) => {
                            if let Err(e) = tx.send(message) {
                                warn!("Failed to send message to the channel. Reason: {}", e);
                            }
                        }
                        _ => {
                            error!(
                                "Received a message from {} which is not a binary message.",
                                addr
                            );
                            break;
                        }
                    },
                    None => {
                        debug!("Input stream has been exhausted");

                        break;
                    }
                },
                Err(e) => {
                    error!(
                        "An error occurred when reading message from a connection. Reason: {}",
                        e
                    );

                    break;
                }
            }
        }
    }
}
