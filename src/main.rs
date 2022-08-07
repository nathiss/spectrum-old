mod network;

use log::{debug, error, info};
use network::WebSocketConnection;

use crate::network::{Connection, Listener, WebSocketListener};

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

    while let Some(mut connection) = listener.accept().await {
        info!("New WebSocket connection from: {}", connection.addr());

        if let Err(e) = connection.write_bytes(vec![1u8, 2u8, 3u8]).await {
            error!("Failed to send data: {}", e);
        }

        tokio::spawn(async move { handle_new_connection(connection).await });
    }

    Ok(())
}

async fn handle_new_connection(mut connection: WebSocketConnection) {
    while let Some(message) = connection.read_bytes().await {
        debug!(
            "Got a message with length: {} from {}",
            message.len(),
            connection.addr()
        );
    }

    debug!("Connection from {} has been closed.", connection.addr());
}
