#[derive(Debug, PartialEq, Clone, Default)]
pub struct ServerConfig {
    pub serve_interface: String,
    pub serve_port: u16,
}

impl ServerConfig {
    pub fn get_serve_interface(&self) -> &str {
        &self.serve_interface
    }

    pub fn get_server_port(&self) -> u16 {
        self.serve_port
    }
}
