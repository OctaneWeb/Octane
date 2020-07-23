use crate::config::OctaneConfig;
#[cfg(feature = "openSSL")]
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::io::Result;

#[cfg(feature = "openSSL")]
pub fn acceptor(settings: &OctaneConfig) -> Result<SslAcceptor> {
    let mut acceptor = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
    acceptor.set_private_key_file(&settings.ssl.key, SslFiletype::PEM)?;
    acceptor.set_certificate_chain_file(&settings.ssl.cert)?;
    let acceptor = acceptor.build();
    Ok(acceptor)
}
