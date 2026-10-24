use crate::DOUBLE_CRLF;

pub mod buffer;
pub mod headers;
pub mod raw_request;
#[cfg(feature = "std")]
mod reader;
pub mod request;

pub use buffer::{RequestBuffer, Status};
pub use headers::{Header, Headers};
#[cfg(feature = "std")]
pub use reader::{http_1x_reader, Http1xReader};
pub use request::{Method, Request};

// Upper bound on the complete request head (request-line, headers and the
// terminating CRLFCRLF). A peer that never sends the terminator must not be
// able to grow `data` without limit. [`RequestBuffer`] is bounded by the
// size of the storage it is handed instead.
pub const MAX_HEAD_SIZE: usize = 64 * 1024;

/// Incremental CRLFCRLF search state.
///
/// Once a prefix has been searched, a future terminator can only start in the
/// last three bytes of that prefix or in newly-read bytes. Remembering that
/// boundary makes fragmented, one-byte-at-a-time input linear rather than
/// repeatedly searching the whole accumulated head.
#[derive(Default)]
pub(crate) struct HeadTerminatorSearch {
    pub(crate) next_start: usize,
}

impl HeadTerminatorSearch {
    pub(crate) fn find(&mut self, data: &[u8]) -> Option<usize> {
        let start = self.next_start.min(data.len());
        let found = find_head_end(data, start);

        if found.is_none() {
            self.next_start = data.len().saturating_sub(DOUBLE_CRLF.len() - 1);
        }

        found
    }
}

/// Find the end (rather than the start) of CRLFCRLF at or after `start`.
#[inline]
fn find_head_end(data: &[u8], start: usize) -> Option<usize> {
    let last_start = data.len().checked_sub(DOUBLE_CRLF.len())?;
    let mut i = start;

    while i <= last_start {
        // This is intentionally specialized for HTTP's four-byte delimiter.
        // It avoids the inner dynamic loop in the public generic helper.
        if data[i] == b'\r' && data[i + 1] == b'\n' && data[i + 2] == b'\r' && data[i + 3] == b'\n'
        {
            return Some(i + DOUBLE_CRLF.len());
        }
        i += 1;
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_in_slice_locates_and_misses() {
        assert_eq!(find_in_slice(b"abcXYdef", b"XY"), Some(3));
        assert_eq!(find_in_slice(b"abcdef", b"XY"), None);
        assert_eq!(find_in_slice(b"a", b"abc"), None);
    }

    #[test]
    fn one_byte_fragmentation_searches_each_start_position_once() {
        let mut data = [0u8; MAX_HEAD_SIZE];
        let mut len = 0;
        let mut search = HeadTerminatorSearch::default();
        let mut candidate_starts = 0usize;

        assert_eq!(search.find(&data[..len]), None);
        for _ in 0..MAX_HEAD_SIZE {
            data[len] = b'a';
            len += 1;
            candidate_starts += (len - search.next_start).saturating_sub(DOUBLE_CRLF.len() - 1);
            assert_eq!(search.find(&data[..len]), None);
        }

        assert_eq!(candidate_starts, MAX_HEAD_SIZE - DOUBLE_CRLF.len() + 1);
    }
}
