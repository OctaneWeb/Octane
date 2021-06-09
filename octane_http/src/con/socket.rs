use socket2::{Domain, Protocol, Socket, Type};

use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener as StdTcpListener};

use tokio::net::{TcpListener, TcpStream};
#[cfg(feature = "openSSL")]
use tokio_openssl::SslStream;
#[cfg(feature = "rustls")]
use tokio_rustls::server::TlsStream;

pub struct TcpListenerBuilder {
    port: u16,
    addr: SocketAddr,
    domain: Domain,
    stream_type: Type,
    proto: Option<Protocol>,
}

impl TcpListenerBuilder {
    pub fn new(port: u16) -> Self {
        Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            port,
            domain: Domain::IPV4,
            stream_type: Type::STREAM,
            proto: None,
        }
    }

    pub fn ipv6(&mut self, addr: SocketAddr) -> &mut Self {
        if addr.is_ipv6() {
            self.addr = addr;
            self.domain = Domain::IPV6;
            self
        } else {
            // Early exit
            panic!("{:?}",);
        }
    }

    pub fn ipv4(&mut self, addr: SocketAddr) -> &mut Self {
        if addr.is_ipv4() {
            self.addr = addr;
            self.domain = Domain::IPV4;
            self
        } else {
            // Early exit
            panic!("{:?}",);
        }
    }

    pub fn bind(self) -> Result<StdTcpListener> {
        let socket = Socket::new(self.domain, self.stream_type, self.proto)?;
        socket.bind(&self.addr.into())?;
        socket.listen(128)?;
        Ok(socket.into())
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
