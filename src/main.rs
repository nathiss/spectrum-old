use std::sync::atomic::{AtomicBool, Ordering};

use log::{error, info};
use spectrum_server::{Server, ServerConfig};
use tokio_util::sync::CancellationToken;

const BANNER: &str = include_str!("asserts/banner.txt");
const TERMINATED_BY_CTRL_C: i32 = 130;

static RECEIVED_SIGNAL: AtomicBool = AtomicBool::new(false);

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

fn get_server_configuration() -> Result<ServerConfig, anyhow::Error> {
    let config_file = std::fs::File::options()
        .read(true)
        .open("server_config.json")?;

    Ok(serde_json::from_reader(config_file)?)
}

fn signal_handler(server_cancellation_token: &CancellationToken) {
    if RECEIVED_SIGNAL.fetch_or(true, Ordering::SeqCst) {
        // Got signal the second time.
        error!("Received a second signal. Exiting the program forcefully.");
        std::process::exit(TERMINATED_BY_CTRL_C);
    }

    info!("Received a signal. Cancelling all server operations...");
    server_cancellation_token.cancel();
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    initialize_logger()?;

    info!("{}", BANNER);

    let mut server = Server::new(get_server_configuration()?);

    let server_cancellation_token = server.get_cancellation_token();
    ctrlc::set_handler(move || signal_handler(&server_cancellation_token))?;

    server.init().await;

    server.serve().await?;

    server.join().await;

    Ok(())
}
