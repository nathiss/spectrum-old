use env_logger::Target;

use spectrum_network::{Listener, ListenerBuilder};

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

pub fn setup_logger() {
    env_logger::builder()
        .is_test(true)
        .target(Target::Stdout)
        .target(Target::Stderr)
        .init();
}
