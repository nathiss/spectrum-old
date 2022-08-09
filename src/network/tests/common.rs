use std::sync::atomic::{AtomicU16, Ordering};

use env_logger::Target;

use spectrum_network::{Listener, ListenerBuilder};

static NEXT_FREE_PORT_NUMER: AtomicU16 = AtomicU16::new(14242);

pub async fn build_websocket_listener(
    interface: &str,
    port: u16,
) -> Result<impl Listener, anyhow::Error> {
    ListenerBuilder::default()
        .set_interface(interface)
        .set_port(port)
        .build()
        .await
}

#[allow(dead_code)] // rust_analyzer for some reason does not see the usage in integration tests ¯\_(ツ)_/¯
pub fn setup_logger() {
    env_logger::builder()
        .is_test(true)
        .target(Target::Stdout)
        .target(Target::Stderr)
        .init();
}

pub fn get_free_port_number() -> u16 {
    NEXT_FREE_PORT_NUMER.fetch_add(1, Ordering::SeqCst)
}
