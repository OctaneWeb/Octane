use crate::server::Octane;
use crate::task;
use std::future::Future;
use std::io::Error;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
#[cfg(feature = "openSSL")]
use tokio_openssl::SslStream;

#[cfg(feature = "rustls")]
use tokio_rustls::server::TlsStream;

pub struct ServerBuilder {
    port: u16,
}

impl ServerBuilder {
    pub fn new() -> Self {
        ServerBuilder { port: 80 }
    }

    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    async fn get_tcp_listener(&mut self) -> Result<TcpListener, Error> {
        TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), self.port)).await
    }

    pub async fn listen<C, T>(&mut self, exec: C, server: Arc<Octane>) -> Result<(), Error>
    where
        T: Future + Send,
        C: FnOnce(TcpStream, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let mut listener = self.get_tcp_listener().await?;
        while let Some(stream) = listener.next().await {
            stream.map(|stream| {
                let server = Arc::clone(&server);
                task!({
                    exec(stream, server).await;
                })
            })?;
        }
        Ok(())
    }

    #[cfg(feature = "openSSL")]
    pub async fn listen_ssl<C, T>(&mut self, exec: C, server: Arc<Octane>) -> Result<(), Error>
    where
        T: Future + Send,
        C: FnOnce(SslStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy + Sync,
    {
        let mut ssl_listener = self.get_tcp_listener().await?;
        let acceptor = crate::tls::openssl::acceptor(&server.settings)?;
        while let Some(stream) = ssl_listener.next().await {
            let acceptor = acceptor.clone();
            stream.map(|stream| {
                let server = Arc::clone(&server);
                task!({
                    tokio_openssl::accept(&acceptor, stream)
                        .await
                        .map(move |stream_ssl| async move {
                            exec(stream_ssl, server).await;
                        })
                        .map_err(|e| panic!("{:?}", e))
                })
            })?;
        }
        Ok(())
    }

    #[cfg(feature = "rustls")]
    pub async fn listen_ssl<C, T>(&mut self, exec: C, server: Arc<Octane>) -> Result<(), Error>
    where
        T: Future + Send,
        C: FnOnce(TlsStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy + Sync,
    {
        let mut ssl_listener = self.get_tcp_listener().await?;
        let acceptor = crate::tls::rustls::acceptor(&server.settings)?;
        while let Some(stream) = ssl_listener.next().await {
            let acceptor = acceptor.clone();
            stream.map(|stream| {
                let server = Arc::clone(&server);
                tokio::spawn(async move {
                    acceptor
                        .accept(stream)
                        .await
                        .map(move |stream_ssl| async move {
                            exec(stream_ssl, server).await;
                        })
                        .map_err(|e| panic!("{:?}", e))
                })
            })?;
        }
        Ok(())
    }
}
