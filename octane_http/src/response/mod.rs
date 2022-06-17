use std::io::Cursor;

use http::header::{AsHeaderName, HeaderMap, IntoHeaderName};
use http::response::Response;
use http::status::StatusCode;
use http::HeaderValue;
use http::Version;
use tokio::io::AsyncRead;

pub type BoxReader = Box<dyn AsyncRead + Unpin + Send>;

mod gen;

pub(crate) enum Body {
    None,
    Sized(usize, BoxReader),
    Unsized(BoxReader),
}

impl Body {
    pub fn get_reader(self) -> BoxReader {
        match self {
            Self::Sized(_, reader) => reader,
            Self::Unsized(reader) => reader,
            Self::None => Box::new(Cursor::new(Vec::new())) as BoxReader,
        }
    }
    pub fn is_some(&self) -> bool {
        !matches!(self, Body::None)
    }
}

pub struct RawResponse {
    res: Response<Body>,
}

impl RawResponse {
    pub async fn serialize(self) -> String {
        gen::serialise_res(self.res).await
    }

    pub fn new_from_slice<T: AsRef<[u8]>>(body: T) -> Self {
        let body_slice = body.as_ref();
        Self::new(
            Box::new(Cursor::new(body_slice.to_vec())) as BoxReader,
            Some(body_slice.len()),
        )
    }

    pub fn new(body: BoxReader, content_len: Option<usize>) -> Self {
        let body_res = if let Some(x) = content_len {
            Body::Sized(x, body)
        } else {
            Body::Unsized(body)
        };

        Self {
            res: Response::new(body_res),
        }
    }

    pub fn status(&mut self, code: StatusCode) {
        *self.res.status_mut() = code;
    }

    // get header value
    pub fn get<K: AsHeaderName>(&self, key: K) -> Option<&HeaderValue> {
        self.res.headers().get(key)
    }

    pub fn set<K: IntoHeaderName>(&mut self, key: K, value: HeaderValue) -> Option<HeaderValue> {
        self.res.headers_mut().insert(key, value)
    }

    pub fn remove<K: AsHeaderName, V>(&mut self, key: K) -> Option<HeaderValue> {
        self.res.headers_mut().remove(key)
    }

    pub fn send<T: AsRef<[u8]>>(&mut self, body: T) {
        let body_slice = body.as_ref();
        let len = body_slice.len();
        *self.res.body_mut() =
            Body::Sized(len, Box::new(Cursor::new(body_slice.to_vec())) as BoxReader);
    }

    pub fn set_header_map(&mut self, map: HeaderMap<HeaderValue>) {
        *self.res.headers_mut() = map;
    }
}
