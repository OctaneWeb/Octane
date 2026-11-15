#![cfg(feature = "rustls")]
use crate::config::OctaneConfig;
use crate::tls::AsMutStream;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{
    rustls::{crypto::ring, ServerConfig},
    TlsAcceptor,
};

pub fn acceptor(settings: &OctaneConfig) -> Result<TlsAcceptor, Box<dyn Error>> {
    let mut config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(settings.get_cert()?, settings.get_key()?)?;
    // HTTP/1.1 is the only protocol served over TLS, so that is what ALPN
    // selects. A client offering ALPN without it gets `no_application_protocol`
    // instead of a connection speaking something it didn't ask for.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(config));
    Ok(acceptor)
}

impl AsMutStream for TlsStream<TcpStream> {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self.get_mut().0
    }
}
