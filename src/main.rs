use log::info;
use spectrum_server::{Server, ServerConfig};

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

fn get_server_configuration() -> ServerConfig {
    ServerConfig {
        serve_interface: "127.0.0.1".to_owned(),
        serve_port: 8080,
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    initialize_logger()?;

    info!("{}", BANNER);

    let mut server = Server::new(get_server_configuration());

    let server_cancellation_token = server.get_cancellation_token();
    ctrlc::set_handler(move || server_cancellation_token.cancel())?;

    server.init().await;

    server.serve().await?;

    server.join().await;

    Ok(())
}
