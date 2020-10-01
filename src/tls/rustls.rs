#![cfg(feature = "rustls")]
use crate::config::OctaneConfig;
use crate::tls::AsMutStream;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{
    rustls::{NoClientAuth, ServerConfig},
    TlsAcceptor,
};

pub fn acceptor(settings: &OctaneConfig) -> Result<TlsAcceptor, Box<dyn Error>> {
    let mut config = ServerConfig::new(NoClientAuth::new());
    let mut key = settings.get_key()?;
    if key.get(0).is_none() {
        panic!(
            "{:?}",
            "rustls expects a RSA_PRIVATE_KEY, invalid key provided"
        );
    }
    config.set_single_cert(settings.get_cert()?, key.remove(0))?;
    config.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);
    let acceptor = TlsAcceptor::from(Arc::new(config));
    Ok(acceptor)
}

impl AsMutStream for TlsStream<TcpStream> {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self.get_mut().0
    }
}
