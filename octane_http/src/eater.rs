use crate::StatusCode;

pub fn eat_till(buf: &[u8], byte: char) -> Result<usize, StatusCode> {
    let mut i = 0;

    for needle in buf {
        if *needle == byte as u8 {
            return Ok(i);
        }

        i += 1;
    }

    Err(StatusCode::BadRequest)
}

pub fn eat_till_two_bytes(buf: &[u8], bytes: &[u8; 2]) -> Result<usize, StatusCode> {
    let mut i = 0;

    for needle in buf {
        if *needle == bytes[0] {
            if i + 1 < buf.len() && buf[i + 1] == bytes[1] {
                return Ok(i);
            }
        }

        i += 1;
    }

    Err(StatusCode::BadRequest)
}

pub fn eat_till_four_bytes(buf: &[u8], bytes: &[u8; 4]) -> Result<usize, StatusCode> {
    let mut i = 0;

    for needle in buf {
        if *needle == bytes[0] {
            if i + 3 < buf.len()
                && buf[i + 1] == bytes[1]
                && buf[i + 2] == bytes[2]
                && buf[i + 3] == bytes[3]
            {
                return Ok(i);
            }
        }

        i += 1;
    }

    Err(StatusCode::BadRequest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CRLF, DOUBLE_CRLF, SP};

    #[test]
    fn eat_till_finds_first_byte() {
        assert_eq!(eat_till(b"abc def", SP), Ok(3));
        assert_eq!(eat_till(b"GET", SP), Err(StatusCode::BadRequest));
        // index of the *first* occurrence is returned
        assert_eq!(eat_till(b"a b c", SP), Ok(1));
    }

    #[test]
    fn eat_till_two_bytes_finds_pair() {
        assert_eq!(eat_till_two_bytes(b"abc\r\nxyz", CRLF), Ok(3));
        assert_eq!(eat_till_two_bytes(b"\r\nrest", CRLF), Ok(0));
        // a lone CR with no following LF is not a match
        assert_eq!(eat_till_two_bytes(b"abc\rxyz", CRLF), Err(StatusCode::BadRequest));
        // a trailing CR at the very end has no room for the LF
        assert_eq!(eat_till_two_bytes(b"abc\r", CRLF), Err(StatusCode::BadRequest));
    }

    #[test]
    fn eat_till_four_bytes_finds_terminator() {
        assert_eq!(eat_till_four_bytes(b"head\r\n\r\nbody", DOUBLE_CRLF), Ok(4));
        assert_eq!(eat_till_four_bytes(b"\r\n\r\n", DOUBLE_CRLF), Ok(0));
        // only three of the four bytes present
        assert_eq!(
            eat_till_four_bytes(b"abc\r\n\rx", DOUBLE_CRLF),
            Err(StatusCode::BadRequest)
        );
    }
}
