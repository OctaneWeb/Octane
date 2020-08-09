#![cfg(feature = "openSSL")]
use crate::config::OctaneConfig;
use crate::tls::AsMutStream;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::io::Result;
use tokio::net::TcpStream;
use tokio_openssl::SslStream;

pub fn acceptor(settings: &OctaneConfig) -> Result<SslAcceptor> {
    if settings.ssl.is_good() {
        let mut acceptor = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
        acceptor.set_private_key_file(&settings.ssl.key, SslFiletype::PEM)?;
        acceptor.set_certificate_chain_file(&settings.ssl.cert)?;
        let acceptor = acceptor.build();
        Ok(acceptor)
    } else {
        panic!("{:?}", "Invald ssl cert/key");
    }
}

impl AsMutStream for SslStream<TcpStream> {
    fn stream_mut(&mut self) -> &mut TcpStream {
        self.get_mut()
    }
}
