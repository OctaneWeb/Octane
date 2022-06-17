use crate::request::{ReqResult, RequestError};

use http::status::StatusCode;

pub struct StreamHelper<'a> {
    pub stream: &'a [u8],
}

impl<'a> StreamHelper<'a> {
    pub fn get_till(&mut self, delimeter: &u8) -> ReqResult<(&'a [u8], &'a [u8])> {
        if let Some(position) = self.stream.iter().position(|x| x == delimeter) {
            let slice = &self.stream[0..position];
            let rest = &self.stream[position + 1..];
            self.stream = rest;

            Ok((slice, rest))
        } else {
            Err(RequestError::StatusCodeErr(StatusCode::BAD_REQUEST))
        }
    }

    pub fn trim(slice: &'a [u8]) -> &'a [u8] {
        let from = match slice.iter().position(|x| !x.is_ascii_whitespace()) {
            Some(i) => i,
            None => return &slice[0..0],
        };

        let to = slice
            .iter()
            .rposition(|x| !x.is_ascii_whitespace())
            .unwrap();

        &slice[from..=to]
    }
}
