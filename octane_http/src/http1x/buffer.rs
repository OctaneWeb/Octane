use super::raw_request::RawRequest1x;
use super::request::Request;
use super::{HeadTerminatorSearch, MAX_HEAD_SIZE};
use crate::StatusCode;

/// The outcome of [`RequestBuffer::poll`]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Status<T> {
    /// A whole request, head and body, is buffered
    Complete(T),
    /// More bytes are needed, read into [`RequestBuffer::unfilled`]
    Partial,
}

/// Where a buffered request ends, found once its head is complete
#[derive(Debug, Clone, Copy)]
struct Framing {
    head_len: usize,
    body_len: usize,
}

/// Accumulates requests in caller-provided storage, without allocating and
/// without doing any I/O itself.
///
/// The transport reads into [`unfilled`](Self::unfilled) and reports how
/// much it read with [`advance`](Self::advance), then [`poll`](Self::poll)
/// says whether a whole request has arrived. The storage bounds both the head
/// and the body, a request that can't fit is refused with a `431` or `413`.
/// After handling a request, [`consume`](Self::consume) drops it and keeps
/// any pipelined bytes that followed it.
///
/// ```
/// use octane_http::http1x::{RequestBuffer, Status};
///
/// let mut storage = [0u8; 1024];
/// let mut buf = RequestBuffer::new(&mut storage);
///
/// for chunk in [&b"POST / HTTP/1.1\r\nHost: a\r\nCont"[..], b"ent-Length: 2\r\n\r\nhi"] {
///     buf.unfilled()[..chunk.len()].copy_from_slice(chunk);
///     buf.advance(chunk.len());
/// }
///
/// match buf.poll().unwrap() {
///     Status::Complete(req) => assert_eq!(req.body, b"hi"),
///     Status::Partial => unreachable!(),
/// }
/// buf.consume();
/// ```
pub struct RequestBuffer<'b> {
    storage: &'b mut [u8],
    filled: usize,
    search: HeadTerminatorSearch,
    framing: Option<Framing>,
}

impl<'b> RequestBuffer<'b> {
    pub fn new(storage: &'b mut [u8]) -> Self {
        Self {
            storage,
            filled: 0,
            search: HeadTerminatorSearch::default(),
            framing: None,
        }
    }

    /// The free space the transport should read into. It is empty once the
    /// storage is full, [`poll`](Self::poll) reports that as an error.
    pub fn unfilled(&mut self) -> &mut [u8] {
        &mut self.storage[self.filled..]
    }

    /// Record that `n` bytes were written to the front of
    /// [`unfilled`](Self::unfilled)
    ///
    /// # Panics
    ///
    /// If `n` is larger than the unfilled space
    pub fn advance(&mut self, n: usize) {
        assert!(
            n <= self.storage.len() - self.filled,
            "advanced past the buffer"
        );
        self.filled += n;
    }

    /// The bytes buffered so far
    pub fn filled(&self) -> &[u8] {
        &self.storage[..self.filled]
    }

    /// Whether nothing is buffered, so the connection is between requests
    pub fn is_empty(&self) -> bool {
        self.filled == 0
    }

    /// Check for a complete request. The returned request's `body` is
    /// exactly the body declared by its `Content-Length`.
    pub fn poll(&mut self) -> Result<Status<Request<'_>>, StatusCode> {
        let framing = match self.framing {
            Some(framing) => framing,
            None => match self.find_framing()? {
                Some(framing) => framing,
                None => return Ok(Status::Partial),
            },
        };
        let end = framing.head_len + framing.body_len;
        if self.filled < end {
            return Ok(Status::Partial);
        }
        let buf = &self.storage[..end];
        let mut raw = RawRequest1x::parse_with_head_end(buf, framing.head_len)?;
        raw.body_remainder = &buf[framing.head_len..];
        Request::from_raw(raw).map(Status::Complete)
    }

    /// Drop the request the last [`poll`](Self::poll) completed, moving any
    /// bytes buffered after it to the front. Does nothing if no request is
    /// complete.
    pub fn consume(&mut self) {
        let framing = match self.framing {
            Some(framing) => framing,
            None => return,
        };
        let end = framing.head_len + framing.body_len;
        if self.filled < end {
            return;
        }
        self.storage.copy_within(end..self.filled, 0);
        self.filled -= end;
        self.search = HeadTerminatorSearch::default();
        self.framing = None;
    }

    /// Look for the end of the head and, once found, validate the head and
    /// work out the body length
    fn find_framing(&mut self) -> Result<Option<Framing>, StatusCode> {
        let head_limit = self.storage.len().min(MAX_HEAD_SIZE);
        let searchable = self.filled.min(head_limit);
        let head_len = match self.search.find(&self.storage[..searchable]) {
            Some(head_len) => head_len,
            None if searchable == head_limit => {
                return Err(StatusCode::RequestHeaderFieldsTooLarge)
            }
            None => return Ok(None),
        };
        let head = &self.storage[..head_len];
        let request = Request::from_raw(RawRequest1x::parse_with_head_end(head, head_len)?)?;
        // the body has to fit in what's left of the storage
        let body_len = request
            .headers
            .content_length(self.storage.len() - head_len)?;
        let framing = Framing { head_len, body_len };
        self.framing = Some(framing);
        Ok(Some(framing))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(buf: &mut RequestBuffer<'_>, bytes: &[u8]) {
        buf.unfilled()[..bytes.len()].copy_from_slice(bytes);
        buf.advance(bytes.len());
    }

    fn body_of(buf: &mut RequestBuffer<'_>) -> Option<[u8; 8]> {
        match buf.poll().unwrap() {
            Status::Complete(req) => {
                let mut out = [0u8; 8];
                out[..req.body.len()].copy_from_slice(req.body);
                Some(out)
            }
            Status::Partial => None,
        }
    }

    #[test]
    fn waits_for_head_and_body_one_byte_at_a_time() {
        let raw = b"POST /x HTTP/1.1\r\nHost: a\r\nContent-Length: 3\r\n\r\nabc";
        let mut storage = [0u8; 128];
        let mut buf = RequestBuffer::new(&mut storage);

        for (i, byte) in raw.iter().enumerate() {
            feed(&mut buf, &[*byte]);
            let done = body_of(&mut buf);
            assert_eq!(done.is_some(), i == raw.len() - 1, "byte {}", i);
        }
        assert_eq!(&body_of(&mut buf).unwrap()[..3], b"abc");
    }

    #[test]
    fn keeps_pipelined_requests() {
        let mut storage = [0u8; 256];
        let mut buf = RequestBuffer::new(&mut storage);
        feed(
            &mut buf,
            b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 2\r\n\r\nhiGET /2 HTTP/1.1\r\nHost: a\r\n\r\n",
        );

        match buf.poll().unwrap() {
            Status::Complete(req) => assert_eq!((req.path, req.body), ("/", &b"hi"[..])),
            Status::Partial => panic!("first request incomplete"),
        }
        buf.consume();
        match buf.poll().unwrap() {
            Status::Complete(req) => assert_eq!((req.path, req.body), ("/2", &b""[..])),
            Status::Partial => panic!("second request incomplete"),
        }
        buf.consume();
        assert!(buf.is_empty());
    }

    #[test]
    fn refuses_what_does_not_fit() {
        let mut storage = [0u8; 32];
        let mut buf = RequestBuffer::new(&mut storage);
        feed(&mut buf, &b"GET / HTTP/1.1\r\nHost: aaaaaaaaaaaaaaa"[..32]);
        assert_eq!(buf.poll(), Err(StatusCode::RequestHeaderFieldsTooLarge));

        let mut storage = [0u8; 64];
        let mut buf = RequestBuffer::new(&mut storage);
        feed(
            &mut buf,
            b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 30\r\n\r\n",
        );
        assert_eq!(buf.poll(), Err(StatusCode::PayloadTooLarge));
    }

    #[test]
    fn reports_invalid_heads() {
        let mut storage = [0u8; 64];
        let mut buf = RequestBuffer::new(&mut storage);
        feed(&mut buf, b"GET / HTTP/1.1\r\n\r\n");
        assert_eq!(buf.poll(), Err(StatusCode::BadRequest));
    }
}
