#![allow(non_snake_case)]

mod common;

use common::*;
use futures_util::SinkExt;
use spectrum_network::Listener;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_incoming_data_channel_clientClosedTheConnection_noMessageReceived(
) -> Result<(), anyhow::Error> {
    // Arrange
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;
    let handle = tokio::spawn(async move { listener.accept_once().await });

    let (mut client, _) = connect_async(format!("ws://127.0.0.1:{}", port)).await?;

    let mut server_connection = handle.await?.expect("Client failed to connect correctly.");

    client.close(None).await?;

    // Act
    let result = server_connection.get_incoming_data_channel().recv().await;

    // Assert
    assert!(result.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_incoming_data_channel_clientSentNonBinaryMessage_dataChannelIsClosed(
) -> Result<(), anyhow::Error> {
    // Arrange
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;
    let handle = tokio::spawn(async move { listener.accept_once().await });

    let (mut client, _) = connect_async(format!("ws://127.0.0.1:{}", port)).await?;

    let mut data_channel = handle
        .await?
        .expect("Client failed to connect correctly.")
        .get_incoming_data_channel();

    client.send(Message::Text("Foo".to_owned())).await?;

    // Act
    let result = data_channel.recv().await;

    // Assert
    assert!(result.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_incoming_data_channel_clientSentBinaryMessage_ExactSameMessageCanBeRead(
) -> Result<(), anyhow::Error> {
    // Arrange
    let port = get_free_port_number();
    let data = vec![13u8, 37u8, 42u8];

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;
    let handle = tokio::spawn(async move { listener.accept_once().await });

    let (mut client, _) = connect_async(format!("ws://127.0.0.1:{}", port)).await?;

    let mut data_channel = handle
        .await?
        .expect("Client failed to connect correctly.")
        .get_incoming_data_channel();

    client.send(Message::Binary(data.clone())).await?;

    // Act
    let result = data_channel
        .recv()
        .await
        .expect("Binary message should be present");

    // Assert
    assert_eq!(data, result);

    Ok(())
}
