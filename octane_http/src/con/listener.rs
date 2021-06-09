use crate::con::socket::TcpListenerBuilder;
use std::io::Result;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub struct Listener {
    pub stream: TcpListener,
}

impl Listener {
    pub fn new(stream: TcpListener) -> Self {
        Self { stream }
    }
    pub fn listen(&mut self, port: u16, addr: SocketAddr) -> Result<Self> {
        let listener_builder = TcpListenerBuilder::new(port);
        let listener = TcpListener::from_std(listener_builder.bind()?.into())?;
        self.stream = listener;
        Ok()
    }
}
