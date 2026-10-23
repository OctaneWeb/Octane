use crate::server::Octane;
use crate::task;
use socket2::{Domain, SockAddr, Socket, Type};
use std::future::Future;
use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{sleep, timeout};
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

async fn accept_bounded(
    listener: &TcpListener,
    connections: &Arc<Semaphore>,
) -> (TcpStream, OwnedSemaphorePermit) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                // Only one socket per listener can wait here. Further sockets
                // stay in the kernel backlog until serving capacity returns.
                // Acquiring after accept also prevents an idle TLS listener
                // from reserving capacity needed by the plain listener (and
                // vice versa).
                let permit = reserve_connection(connections).await;
                return (stream, permit);
            }
            Err(_) => {
                // An intermittent accept error must not take down the server;
                // back off on persistent failures to avoid a hot error loop.
                sleep(ACCEPT_ERROR_BACKOFF).await;
            }
        }
    }
}

async fn run_with_timeout<F>(future: F, connection_timeout: Option<Duration>)
where
    F: Future,
{
    if let Some(duration) = connection_timeout {
        let _ = timeout(duration, future).await;
    } else {
        future.await;
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
        loop {
            let (stream, permit) = accept_bounded(&self.socket, &connections).await;
            let server = Arc::clone(&server);
            let connection_timeout = server.settings.connection_timeout;
            task!({
                let _permit = permit;
                run_with_timeout(exec(stream, server), connection_timeout).await;
            });
        }
    }

    #[cfg(feature = "openSSL")]
    pub async fn listen_ssl<C, T>(
        self,
        exec: C,
        server: Arc<Octane>,
        connections: Arc<Semaphore>,
    ) -> Result<()>
    where
        T: Future + Send + 'static,
        C: FnOnce(SslStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let acceptor = crate::tls::openssl::acceptor(&server.settings)?;
        loop {
            let (tcp_stream, permit) = accept_bounded(&self.socket, &connections).await;
            let acceptor = acceptor.clone();
            let server = Arc::clone(&server);
            let connection_timeout = server.settings.connection_timeout;

            task!({
                let _permit = permit;
                run_with_timeout(
                    async move {
                        if let Ok(stream_ssl) =
                            crate::tls::openssl::accept(&acceptor, tcp_stream).await
                        {
                            exec(stream_ssl, server).await;
                        }
                    },
                    connection_timeout,
                )
                .await;
            });
        }
    }

    #[cfg(feature = "rustls")]
    pub async fn listen_ssl<C, T>(
        self,
        exec: C,
        server: Arc<Octane>,
        connections: Arc<Semaphore>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>>
    where
        T: Future + Send + 'static,
        C: FnOnce(TlsStream<TcpStream>, Arc<Octane>) -> T + Send + 'static + Copy,
    {
        let acceptor = crate::tls::rustls::acceptor(&server.settings)?;
        loop {
            let (tcp_stream, permit) = accept_bounded(&self.socket, &connections).await;
            let acceptor = acceptor.clone();
            let server = Arc::clone(&server);
            let connection_timeout = server.settings.connection_timeout;

            task!({
                let _permit = permit;
                run_with_timeout(
                    async move {
                        if let Ok(stream_ssl) = acceptor.accept(tcp_stream).await {
                            exec(stream_ssl, server).await;
                        }
                    },
                    connection_timeout,
                )
                .await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{reserve_connection, run_with_timeout};
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

        run_with_timeout(work, Some(Duration::from_millis(10))).await;
        assert!(dropped.load(Ordering::SeqCst));
    }
}
