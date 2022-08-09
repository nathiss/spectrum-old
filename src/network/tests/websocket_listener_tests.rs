#![allow(non_snake_case)]

mod common;

use std::time::Duration;

use common::*;
use log::info;
use spectrum_network::Listener;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn bind_passedInvalidInterface_returnsError() {
    // Arrange
    let port = get_free_port_number();

    // Act
    let result = build_websocket_listener("", port).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn bind_passedZeroAsPort_returnsError() {
    // Arrange & Act
    let result = build_websocket_listener("127.0.0.1", 0).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn bind_passedCorrectParameters_returnsOk() {
    // Arrange
    let port = get_free_port_number();

    // Act
    let result = build_websocket_listener("127.0.0.1", port).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithTcp_returnsNone() -> Result<(), anyhow::Error> {
    // Arrange
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;

    let handle = tokio::spawn(async move { listener.accept_once().await });

    drop(TcpStream::connect(("127.0.0.1", port)).await?);

    // Act
    let result = handle.await?;

    // Assert
    assert!(result.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithHttpInvalidHandshake_returnsNone() -> Result<(), anyhow::Error> {
    // Arrange
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;

    let listener_handle = tokio::spawn(async move { listener.accept_once().await });

    let client_handle =
        tokio::spawn(async move { reqwest::get(format!("http://127.0.0.1:{}", port)).await });

    // Act
    let result = listener_handle.await?;

    // Assert
    assert!(result.is_none());

    // The client will return an error, coz the server closed the connection before the whole internet message was
    // transported.
    assert!(client_handle.await?.is_err());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithWebSocket_returnsSome() -> Result<(), anyhow::Error> {
    // Arrange
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;

    let handle = tokio::spawn(async move { listener.accept_once().await });

    drop(connect_async(format!("ws://127.0.0.1:{}", port)).await?);

    // Act
    let result = handle.await?;

    // Assert
    assert!(result.is_some());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithTcpAndHang_timeoutsAndReturnsNone() -> Result<(), anyhow::Error> {
    // Arrange
    setup_logger();
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;

    let handle = tokio::spawn(async move { listener.accept_once().await });

    let client_stream = TcpStream::connect(("127.0.0.1", port)).await?;

    info!("Waiting up to 31 secs for handshake timeout.");

    // Act
    // The timeout for handshake is 30 secs. We wait up to 31 secs and then we fail.
    let result = timeout(Duration::from_secs(31), handle).await??;

    // Assert
    assert!(result.is_none());

    drop(client_stream);
    Ok(())
}
