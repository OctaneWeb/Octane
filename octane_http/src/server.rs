use crate::config::Config;
use crate::proto::Proto;

use std::io::Result;
use std::net::ToSocketAddrs;

use socket2::SockAddr;
use tokio::net::TcpListener;
use tracing::{debug, error, info, span, warn, Level};

pub struct Server {
    pub proto: Proto,
    pub server_config: Config,
}

impl Server {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            proto: Proto::tcp()?,
            server_config: Config::default(),
        })
    }

    pub async fn connect<IP: ToSocketAddrs + tracing::Value>(self, url: IP) -> Result<Connection> {
        span!(Level::TRACE, "Connecting").entered();

        let mut socket_addr = url.to_socket_addrs()?;

        if let Proto::Tcp(socket) = self.proto {
            info!("Registering Tcp Socket");

            if let Some(addr) = socket_addr.next() {
                info!("Binding address to Socket");

                socket.bind(&SockAddr::from(addr))?;
                let listener = TcpListener::from_std(socket.into())?;

                info!("Starting listening for connections");
                Connection::handle(listener)
            } else {
                error!("Invalid binding address")
            }
        } else {
            error!("Cannot register tcp socket")
        }

        Ok(())
    }
}
