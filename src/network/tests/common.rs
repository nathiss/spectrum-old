use std::{
    fs::File,
    sync::atomic::{AtomicU16, Ordering},
};

use env_logger::Target;

use spectrum_network::{Listener, ListenerBuilder, PublicEndpointConfig};

static NEXT_FREE_PORT_NUMER: AtomicU16 = AtomicU16::new(14242);

pub async fn build_websocket_listener(
    interface: &str,
    port: u16,
) -> Result<impl Listener, anyhow::Error> {
    let configuration = PublicEndpointConfig {
        serve_interface: interface.to_owned(),
        serve_port: port,
    };

    ListenerBuilder::default()
        .configure(configuration)
        .build()
        .await
}

#[allow(dead_code)] // rust_analyzer for some reason does not see the usage in integration tests ¯\_(ツ)_/¯
pub fn setup_logger() {
    let log_file = File::options()
        .read(true)
        .append(true)
        .create(true)
        .open("./integration_tests.log")
        .expect("Failed to open log file for spectrum-network integration tests.");

    env_logger::builder()
        .filter(None, log::LevelFilter::Debug)
        .is_test(true)
        .target(Target::Stdout)
        .target(Target::Stderr)
        .target(Target::Pipe(Box::new(log_file)))
        .init();
}

pub fn get_free_port_number() -> u16 {
    NEXT_FREE_PORT_NUMER.fetch_add(1, Ordering::SeqCst)
}
