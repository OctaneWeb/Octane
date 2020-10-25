use crate::server::Octane;
use crate::task;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::future::Future;
use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
#[cfg(feature = "openSSL")]
use tokio_openssl::SslStream;
#[cfg(feature = "rustls")]
use tokio_rustls::server::TlsStream;

pub struct ServerBuilder {
    socket: TcpListener,
}

impl ServerBuilder {
    pub fn new(port: u16) -> Result<Self> {
        let stream = Type::stream();
        let socket = Socket::new(Domain::ipv4(), stream.non_blocking(), Some(Protocol::tcp()))?;
        let bind_add = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
        socket.bind(&SockAddr::from(bind_add))?;
        socket.listen(2048)?;
        socket.set_reuse_address(true)?;
        Ok(ServerBuilder {
            socket: TcpListener::from_std(socket.into_tcp_listener())?,
        })
    }

    pub async fn listen<C, T>(mut self, exec: C, server: Arc<Octane>) -> Result<()>
    where
        T: Future + Send,
        C: FnOnce(TcpStream, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        while let Some(stream) = self.socket.next().await {
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
    pub async fn listen_ssl<C, T>(self, exec: C, server: Arc<Octane>) -> Result<()>
    where
        T: Future + Send,
        C: FnOnce(SslStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let mut ssl_listener = self.socket;
        let acceptor = crate::tls::openssl::acceptor(&server.settings)?;
        while let Some(stream) = ssl_listener.next().await {
            let acceptor = acceptor.clone();
            let tcp_stream = stream?;
            let server = Arc::clone(&server);

            task!({
                let stream = tokio_openssl::accept(&acceptor, tcp_stream).await;
                if let Ok(stream_ssl) = stream {
                    exec(stream_ssl, server).await;
                } else {
                    stream.map_err(|e| println!("{:?}", e)).err();
                }
            });
        }
        Ok(())
    }

    #[cfg(feature = "rustls")]
    pub async fn listen_ssl<C, T>(
        self,
        exec: C,
        server: Arc<Octane>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>>
    where
        T: Future + Send,
        C: FnOnce(TlsStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let mut ssl_listener = self.socket;
        let acceptor = crate::tls::rustls::acceptor(&server.settings)?;

        while let Some(stream) = ssl_listener.next().await {
            let acceptor = acceptor.clone();

            let server = Arc::clone(&server);
            let tcp_stream = stream?;
            task!({
                let stream = acceptor.accept(tcp_stream).await;
                if let Ok(stream_ssl) = stream {
                    exec(stream_ssl, server).await;
                } else {
                    stream
                        .map_err(|e| println!("WARNING: {:?}", e.kind()))
                        .err();
                }
            });
        }
        Ok(())
    }
}
