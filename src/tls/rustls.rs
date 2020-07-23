use crate::config::OctaneConfig;
use std::io;
use std::io::Result;
use std::sync::Arc;
#[cfg(feature = "rustls")]
use tokio_rustls::{
    rustls::{NoClientAuth, ServerConfig},
    TlsAcceptor,
};

#[cfg(feature = "rustls")]
pub fn acceptor(settings: &OctaneConfig) -> Result<TlsAcceptor> {
    let mut config = ServerConfig::new(NoClientAuth::new());
    // FIXME: panic on get_key()?.remove(0)
    config
        .set_single_cert(settings.get_cert()?, settings.get_key()?.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    Ok(acceptor)
}
