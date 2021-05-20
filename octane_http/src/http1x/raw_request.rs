use tokio::io::ReadHalf;
use tokio::prelude::*;

pub struct RawRequest1x<'a, T> {
    pub headers: String,
    pub body_remainder: &'a [u8],
    pub request_line: &'a str,
    pub reader: ReadHalf<T>,
}

impl<'a, T> RawRequest1x<'a, T>
where
    T: AsyncRead + Unpin,
{
    pub fn new(reader: ReadHalf<T>) -> Self {
        Self {
            headers: Default::default(),
            body_remainder: Default::default(),
            request_line: Default::default(),
            reader,
        }
    }
}
