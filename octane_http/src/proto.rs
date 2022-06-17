use std::io::Result;
use std::time::Duration;

use socket2::{Domain, Protocol, Socket, TcpKeepalive, Type};

pub enum Proto {
    Udp(Socket),
    Tcp(Socket),
}

impl Proto {
    pub fn tcp() -> Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        socket.set_nonblocking(true);
        Ok(Self::Tcp(socket))
    }

    pub fn udp() -> Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::UDP))?;
        socket.set_nonblocking(true);
        Ok(Self::Udp(socket))
    }

    pub fn set_tcp_keepalive(&mut self, duration: Duration) -> Result<()> {
        let ka = TcpKeepalive::new().with_time(duration);
        if let Self::Tcp(socket) = self {
            socket.set_tcp_keepalive(&ka)?;
        }

        Ok(())
    }
}
