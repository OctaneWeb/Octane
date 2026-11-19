use crate::server::Octane;
use crate::shutdown::Watch;
use crate::task;
#[cfg(any(feature = "openSSL", feature = "rustls"))]
use crate::util::within;
use socket2::{Domain, SockAddr, Socket, Type};
use std::future::{poll_fn, Future};
use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::sleep;
#[cfg(feature = "openSSL")]
use tokio_openssl::SslStream;
#[cfg(feature = "rustls")]
use tokio_rustls::server::TlsStream;

pub struct ServerBuilder {
    socket: TcpListener,
}

const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(50);

async fn reserve_connection(connections: &Arc<Semaphore>) -> OwnedSemaphorePermit {
    Arc::clone(connections)
        .acquire_owned()
        .await
        .expect("connection semaphore is never closed")
}

// The next connection and the capacity to serve it, `None` once the server
// is shutting down
async fn accept_bounded(
    listener: &TcpListener,
    connections: &Arc<Semaphore>,
    shutdown: &Watch,
) -> Option<(TcpStream, OwnedSemaphorePermit)> {
    loop {
        let accepted = poll_fn(|cx| match shutdown.poll_started(cx) {
            Poll::Ready(()) => Poll::Ready(None),
            Poll::Pending => listener.poll_accept(cx).map(Some),
        })
        .await?;
        match accepted {
            Ok((stream, _)) => {
                // Responses are written whole, so there is nothing for
                // Nagle's algorithm to coalesce, only a delayed ACK to wait
                // on before the end of a response leaves.
                let _ = stream.set_nodelay(true);
                // Only one socket per listener can wait here. Further sockets
                // stay in the kernel backlog until serving capacity returns.
                // Acquiring after accept also prevents an idle TLS listener
                // from reserving capacity needed by the plain listener (and
                // vice versa).
                let permit = reserve_connection(connections).await;
                return Some((stream, permit));
            }
            Err(_) => {
                // An intermittent accept error must not take down the server;
                // back off on persistent failures to avoid a hot error loop.
                sleep(ACCEPT_ERROR_BACKOFF).await;
            }
        }
    }
}

impl ServerBuilder {
    pub fn new(port: u16) -> Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        socket.set_nonblocking(true)?;
        // must come before bind, or a restart fails while old connections
        // are still in TIME_WAIT
        socket.set_reuse_address(true)?;
        let bind_add = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
        socket.bind(&SockAddr::from(bind_add))?;
        socket.listen(2048)?;
        let listener: std::net::TcpListener = socket.into();
        Ok(ServerBuilder {
            socket: TcpListener::from_std(listener)?,
        })
    }

    pub async fn listen<C, T>(
        self,
        exec: C,
        server: Arc<Octane>,
        connections: Arc<Semaphore>,
    ) -> Result<()>
    where
        T: Future + Send + 'static,
        C: FnOnce(TcpStream, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let shutdown = server.watch_shutdown();
        // until the server shuts down, which closes the listening socket
        while let Some((stream, permit)) =
            accept_bounded(&self.socket, &connections, &shutdown).await
        {
            let server = Arc::clone(&server);
            // the connection keeps its own time, see `connection_timeout`
            task!({
                let _permit = permit;
                exec(stream, server).await;
            });
        }
        Ok(())
    }

    #[cfg(feature = "openSSL")]
    pub async fn listen_ssl<C, T>(
        self,
        acceptor: crate::tls::Acceptor,
        exec: C,
        server: Arc<Octane>,
        connections: Arc<Semaphore>,
    ) where
        T: Future + Send + 'static,
        C: FnOnce(SslStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let shutdown = server.watch_shutdown();
        while let Some((tcp_stream, permit)) =
            accept_bounded(&self.socket, &connections, &shutdown).await
        {
            let acceptor = acceptor.clone();
            let server = Arc::clone(&server);
            let connection_timeout = server.settings.connection_timeout;

            task!({
                let _permit = permit;
                let handshake = crate::tls::openssl::accept(&acceptor, tcp_stream);
                if let Some(Ok(stream_ssl)) = within(connection_timeout, handshake).await {
                    exec(stream_ssl, server).await;
                }
            });
        }
    }

    #[cfg(feature = "rustls")]
    pub async fn listen_ssl<C, T>(
        self,
        acceptor: crate::tls::Acceptor,
        exec: C,
        server: Arc<Octane>,
        connections: Arc<Semaphore>,
    ) where
        T: Future + Send + 'static,
        C: FnOnce(TlsStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let shutdown = server.watch_shutdown();
        while let Some((tcp_stream, permit)) =
            accept_bounded(&self.socket, &connections, &shutdown).await
        {
            let acceptor = acceptor.clone();
            let server = Arc::clone(&server);
            let connection_timeout = server.settings.connection_timeout;

            task!({
                let _permit = permit;
                let handshake = acceptor.accept(tcp_stream);
                if let Some(Ok(stream_ssl)) = within(connection_timeout, handshake).await {
                    exec(stream_ssl, server).await;
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reserve_connection;
    use crate::util::within;
    use std::future::pending;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Semaphore;
    use tokio::time::timeout;

    #[tokio::test]
    async fn connection_capacity_blocks_until_released() {
        let connections = Arc::new(Semaphore::new(1));
        let held_permit = reserve_connection(&connections).await;
        let task_connections = Arc::clone(&connections);

        let mut waiting = tokio::spawn(async move { reserve_connection(&task_connections).await });

        assert!(timeout(Duration::from_millis(25), &mut waiting)
            .await
            .is_err());
        drop(held_permit);

        let permit = timeout(Duration::from_secs(1), waiting).await.unwrap();
        assert!(permit.is_ok());
    }

    #[tokio::test]
    async fn timeout_cancels_connection_work() {
        struct DropFlag(Arc<AtomicBool>);

        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let task_flag = Arc::clone(&dropped);
        let work = async move {
            let _drop_flag = DropFlag(task_flag);
            pending::<()>().await;
        };

        assert!(within(Some(Duration::from_millis(10)), work)
            .await
            .is_none());
        assert!(dropped.load(Ordering::SeqCst));
    }
}
