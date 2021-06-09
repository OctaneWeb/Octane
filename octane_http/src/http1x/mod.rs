use crate::http1x::raw_request::RawRequest1x;
use crate::StatusCode;
use tokio::io::{AsyncRead, AsyncReadExt, ReadHalf};

pub mod raw_request;

pub struct Http1xReader<'a, T> {
    pub raw_request: RawRequest1x<'a, T>,
}

impl<'a, T> Http1xReader<'a, T>
where
    T: AsyncRead + Unpin,
{
    pub async fn new(
        request: RawRequest1x<'a, T>,
        data: &'a mut Vec<u8>,
    ) -> Result<(String, &'a str, &'a [u8], ReadHalf<T>), StatusCode> {
        let mut instance = Self {
            raw_request: request,
        };
        let mut buf: [u8; 512] = [0; 512];

        loop {
            let read = instance.raw_request.reader.read(&mut buf).await.unwrap();
            if read == 0 {
                return Err(StatusCode::BadRequest);
            }
            data.extend_from_slice(&buf[..read]);
            if let Some(i) = find_in_slice(&data[..], b"\r\n\r\n") {
                instance.raw_request.body_remainder = &data[i + 4..];
                if let Ok(Some((rl, heads))) =
                    std::str::from_utf8(&data[..i]).map(parse_without_body)
                {
                    instance.raw_request.request_line = rl;
                    instance.raw_request.headers = heads;
                    break;
                } else {
                    return Err(StatusCode::BadRequest);
                }
            }
        }
        Ok((
            instance.raw_request.headers,
            instance.raw_request.request_line,
            instance.raw_request.body_remainder,
            instance.raw_request.reader,
        ))
    }
}

pub fn find_in_slice<T: Eq>(haystack: &[T], needle: &[T]) -> Option<usize> {
    // naive algorithm only meant for small needles
    if needle.len() > haystack.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        let mut matching = true;
        for j in 0..needle.len() {
            if haystack[i + j] != needle[j] {
                matching = false;
                break;
            }
        }
        if matching {
            return Some(i);
        }
    }
    None
}

// Helper function for extracting some headers
pub(crate) fn parse_without_body(data: &str) -> Option<(&str, String)> {
    let n = data.find("\r\n")?;
    let (line, rest) = data.split_at(n);
    let request_line = line;
    let headers = (&rest[2..]).to_owned();
    Some((request_line, headers))
}
