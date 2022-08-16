use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::stream::StreamExt;
use log::{error, info, warn};
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook_tokio::Signals;
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

async fn handle_signals(mut signals: Signals, server_cancellation_token: CancellationToken) {
    while let Some(signal) = signals.next().await {
        match signal {
            SIGTERM | SIGINT | SIGQUIT => {
                if RECEIVED_SIGNAL.fetch_or(true, Ordering::SeqCst) {
                    // Got signal the second time.
                    error!("Received a second signal. Exiting the program forcefully.");
                    std::process::exit(TERMINATED_BY_CTRL_C);
                }

                warn!("Received a signal. Cancelling all server operations...");
                server_cancellation_token.cancel();
            }
            _ => unreachable!(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    initialize_logger()?;

    info!("{}", BANNER);

    let mut server = Server::new(get_server_configuration()?);

    // Set up signals handler
    let signals = Signals::new(&[SIGTERM, SIGINT, SIGQUIT])?;
    let handle = signals.handle();

    let server_cancellation_token = server.get_cancellation_token();

    let signals_task = tokio::spawn(handle_signals(signals, server_cancellation_token));

    server.init().await;

    server.serve().await?;

    server.join().await;

    handle.close();
    signals_task.await?;

    Ok(())
}
