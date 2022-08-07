mod client;
mod network;
mod packet;

use log::{debug, info};

use crate::{
    client::Client,
    network::{Connection, Listener, WebSocketListener},
    packet::{model::*, ProtobufPacketSerializer},
};

static BANNER: &str = include_str!("asserts/banner.txt");

fn initialize_logger() -> Result<(), anyhow::Error> {
    let log_config_result = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}-{} ({})|{}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S%.3f]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("log/default.log")?)
        .apply();

    match log_config_result {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::Error::new(e)),
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    initialize_logger()?;

    info!("{}", BANNER);

    let mut listener = WebSocketListener::bind("0.0.0.0", 8080).await?;

    while let Some(connection) = listener.accept().await {
        info!("New WebSocket connection from: {}", connection.addr());

        let mut client = Client::new(connection, ProtobufPacketSerializer::default());

        let _ = client.write_packet(&ClientMessage::default()).await;

        let client_rx = client.get_packet_channel();
        let addr = client.addr().clone();

        tokio::spawn(async move {
            let mut client_rx = client_rx;

            while let Some(message) = client_rx.recv().await {
                debug!("Got a message: {:?} from {}", message, addr);
            }

            debug!("Connection from {} has been closed.", client.addr());
        });
    }

    Ok(())
}
