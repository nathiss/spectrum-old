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
    let result = build_websocket_listener("", 8080).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn bind_passedZeroAsPort_returnsError() {
    let result = build_websocket_listener("127.0.0.1", 0).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn bind_passedCorrectParameters_returnsOk() {
    let result = build_websocket_listener("127.0.0.1", 14242).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithTcp_returnsNone() -> Result<(), anyhow::Error> {
    // Arrange
    let mut listener = build_websocket_listener("127.0.0.1", 14243).await?;

    let handle = tokio::spawn(async move { listener.accept_once().await });

    drop(TcpStream::connect("127.0.0.1:14243").await?);

    // Act
    let result = handle.await?;

    // Assert
    assert!(result.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithHttpInvalidHandshake_returnsNone() -> Result<(), anyhow::Error> {
    // Arrange
    let mut listener = build_websocket_listener("127.0.0.1", 14244).await?;

    let handle = tokio::spawn(async move { listener.accept_once().await });

    drop(reqwest::get("http://127.0.0.1:14244").await);

    // Act
    let result = handle.await?;

    // Assert
    assert!(result.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithWebSocket_returnsSome() -> Result<(), anyhow::Error> {
    // Arrange
    let mut listener = build_websocket_listener("127.0.0.1", 14245).await?;

    let handle = tokio::spawn(async move { listener.accept_once().await });

    drop(connect_async("ws://127.0.0.1:14245").await?);

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

    let mut listener = build_websocket_listener("127.0.0.1", 14246).await?;

    let handle = tokio::spawn(async move { listener.accept_once().await });

    let client_stream = TcpStream::connect("127.0.0.1:14246").await?;

    info!("Waiting up to 31 secs for handshake timeout.");

    // Act
    // The timeout for handshake is 30 secs. We wait up to 31 secs and then we fail.
    let result = timeout(Duration::from_secs(31), handle).await??;

    // Assert
    assert!(result.is_none());

    drop(client_stream);
    Ok(())
}
