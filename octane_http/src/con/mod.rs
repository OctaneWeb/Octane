use crate::con::listener::Listener;
use std::future::Future;
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;

mod listener;
mod socket;

struct Connection {
    has_http2: bool,
    http2_only: bool,
    listener: Listener,
}

impl Connection {
    pub fn new(has_http2: bool, http2_only: bool, listener: Listener) -> Self {
        Self {
            has_http2,
            http2_only,
            listener,
        }
    }

    pub fn handle(stream: TcpStream, addr: SocketAddr) {}
}

impl Future for Connection {
    type Output = Error;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        loop {
            match self.listener.stream.poll_accept(ctx) {
                Poll::Ready(Ok(x)) => Self::handle(x.0, x.1),
                Poll::Pending => continue,
                Poll::Ready(Err(x)) => return x.into(),
            };
        }
    }
}
