#![allow(non_snake_case)]

mod common;

use std::{sync::Once, time::Duration};

use common::*;
use log::info;
use spectrum_network::Listener;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::connect_async;

static INIT_TESTS: Once = Once::new();

fn setup_tests() {
    INIT_TESTS.call_once(|| {
        setup_logger();
    });
}

#[tokio::test]
async fn bind_passedInvalidInterface_returnsError() {
    // Arrange
    setup_tests();
    let port = get_free_port_number();

    // Act
    let result = build_websocket_listener("", port).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn bind_passedZeroAsPort_returnsError() {
    // Arrange & Act
    setup_tests();
    let result = build_websocket_listener("127.0.0.1", 0).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn bind_passedCorrectParameters_returnsOk() {
    // Arrange
    setup_tests();
    let port = get_free_port_number();

    // Act
    let result = build_websocket_listener("127.0.0.1", port).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_once_connectedWithTcp_returnsNone() -> Result<(), anyhow::Error> {
    // Arrange
    setup_tests();
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
    setup_tests();
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
    setup_tests();
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
    setup_tests();
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;
    listener.set_handshake_timeout(Duration::from_millis(50));

    let handle = tokio::spawn(async move { listener.accept_once().await });

    let client_stream = TcpStream::connect(("127.0.0.1", port)).await?;

    info!("Waiting up to 100 millis for handshake timeout.");

    // Act
    // The timeout for handshake is 50 millis. We wait up to 150 millis and then we fail.
    let result = timeout(Duration::from_millis(150), handle).await??;

    // Assert
    assert!(result.is_none());

    drop(client_stream);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_rawTcpConnectionsBeforeAValidWebSocketConnection_ableToHandleWebSocketConnection(
) -> Result<(), anyhow::Error> {
    // Arrange
    setup_tests();
    let port = get_free_port_number();

    let mut listener = build_websocket_listener("127.0.0.1", port).await?;
    listener.set_handshake_timeout(Duration::from_millis(50));

    let handle = tokio::spawn(async move { listener.accept().await });

    let _client_stream1 = TcpStream::connect(("127.0.0.1", port)).await?;
    let _client_stream2 = TcpStream::connect(("127.0.0.1", port)).await?;
    let _client_stream3 = TcpStream::connect(("127.0.0.1", port)).await?;

    let (_websocket_stream, ..) = connect_async(format!("ws://127.0.0.1:{}", port)).await?;

    info!("Waiting up to 100 millis for handshake timeout.");

    // Act
    // The timeout for handshake is 50 millis for each connection. We wait up to 250 millis and then we fail.
    let result = timeout(Duration::from_millis(250), handle).await??;

    // Assert
    assert!(result.is_some());

    Ok(())
}
