use tokio::net::TcpStream;

pub mod openssl;
pub mod rustls;

pub trait AsMutStream {
    fn stream_mut(&mut self) -> &mut TcpStream;
}

impl AsMutStream for TcpStream {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self
    }
}
