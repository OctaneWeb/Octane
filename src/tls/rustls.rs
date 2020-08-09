#![cfg(feature = "rustls")]
use crate::config::OctaneConfig;
use crate::tls::AsMutStream;
use std::io;
use std::io::Result;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{
    rustls::{NoClientAuth, ServerConfig},
    TlsAcceptor,
};

pub fn acceptor(settings: &OctaneConfig) -> Result<TlsAcceptor> {
    let mut config = ServerConfig::new(NoClientAuth::new());
    // FIXME: panic on get_key()?.remove(0)
    config
        .set_single_cert(settings.get_cert()?, settings.get_key()?.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    Ok(acceptor)
}

impl AsMutStream for TlsStream<TcpStream> {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self.get_mut().0
    }
}
