use tokio::net::TcpStream;

pub mod openssl;
pub mod rustls;

/// What TLS connections are accepted with, from the configured key and
/// certificate
#[cfg(feature = "rustls")]
pub(crate) type Acceptor = tokio_rustls::TlsAcceptor;
#[cfg(feature = "openSSL")]
pub(crate) type Acceptor = ::openssl::ssl::SslAcceptor;

/// Load the key and certificate of `settings` into an [`Acceptor`]
#[cfg(feature = "rustls")]
pub(crate) fn acceptor(
    settings: &crate::config::OctaneConfig,
) -> Result<Acceptor, Box<dyn std::error::Error>> {
    rustls::acceptor(settings)
}
#[cfg(feature = "openSSL")]
pub(crate) fn acceptor(
    settings: &crate::config::OctaneConfig,
) -> Result<Acceptor, Box<dyn std::error::Error>> {
    Ok(openssl::acceptor(settings)?)
}

pub trait AsMutStream {
    fn stream_mut(&mut self) -> &mut TcpStream;
}

impl AsMutStream for TcpStream {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self
    }
}
