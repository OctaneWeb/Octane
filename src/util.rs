use std::future::{poll_fn, Future};
use std::io;
use std::mem::MaybeUninit;
use std::pin::{pin, Pin};
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::time::{sleep_until, Instant, Sleep};

// Bytes read at once while waiting for a request or its body
const READ_SIZE: usize = 8192;

/// Append one read's worth of bytes from `reader` to `data`, returning how
/// many arrived, 0 at the end of the stream.
///
/// The read buffer only lives on the stack while the read is polled, so a
/// connection waiting here for its next request doesn't hold 8 KiB of it.
pub(crate) async fn read_some<R: AsyncRead + Unpin>(
    reader: &mut R,
    data: &mut Vec<u8>,
) -> io::Result<usize> {
    poll_fn(|cx| {
        let mut storage = [MaybeUninit::<u8>::uninit(); READ_SIZE];
        let mut buf = ReadBuf::uninit(&mut storage);
        match Pin::new(&mut *reader).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(())) => {
                data.extend_from_slice(buf.filled());
                Poll::Ready(Ok(buf.filled().len()))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}

/// Run `future` for at most `limit`, `None` if it ran out of time. Without a
/// limit it runs to completion.
#[cfg(any(feature = "rustls", feature = "openSSL", test))]
pub(crate) async fn within<F: Future>(limit: Option<Duration>, future: F) -> Option<F::Output> {
    match limit {
        Some(limit) => tokio::time::timeout(limit, future).await.ok(),
        None => Some(future.await),
    }
}

/// [`within`] for a connection's successive waits, reusing one timer.
///
/// A new timeout clones the runtime's handle, a reference count every worker
/// thread shares, and registers a timer of its own. Moving this one's
/// deadline later, as each wait does when the limit stays the same, only
/// updates it, so a connection pays for its timer once rather than per
/// request.
pub(crate) struct Deadline {
    /// How long each wait may take, `None` for no limit
    pub(crate) limit: Option<Duration>,
    sleep: Option<Pin<Box<Sleep>>>,
}

impl Deadline {
    pub(crate) fn new(limit: Option<Duration>) -> Self {
        Self { limit, sleep: None }
    }

    /// Run `future` for at most `limit` from now, `None` if it ran out of time
    pub(crate) async fn run<F: Future>(&mut self, future: F) -> Option<F::Output> {
        let limit = match self.limit {
            Some(limit) => limit,
            None => return Some(future.await),
        };
        let deadline = Instant::now() + limit;
        let sleep = match &mut self.sleep {
            Some(sleep) => {
                sleep.as_mut().reset(deadline);
                sleep
            }
            None => self.sleep.insert(Box::pin(sleep_until(deadline))),
        };
        let mut future = pin!(future);
        poll_fn(|cx| {
            if let Poll::Ready(output) = future.as_mut().poll(cx) {
                return Poll::Ready(Some(output));
            }
            sleep.as_mut().poll(cx).map(|()| None)
        })
        .await
    }
}
