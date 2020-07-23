use tokio::net::TcpStream;
#[cfg(feature = "openSSL")]
use tokio_openssl::SslStream;
#[cfg(feature = "rustls")]
use tokio_rustls::server::TlsStream;

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

#[cfg(feature = "openSSL")]
impl AsMutStream for SslStream<TcpStream> {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self.get_mut()
    }
}

#[cfg(feature = "rustls")]
impl AsMutStream for TlsStream<TcpStream> {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self.get_mut().0
    }
}
