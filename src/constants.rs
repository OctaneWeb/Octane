use std::collections::HashSet;
use std::iter::FromIterator;

pub const SP: char = ' ';
pub const HT: char = '\t';
pub const CR: char = '\r';
pub const LF: char = '\n';
pub const CRLF: &str = "\r\n";
pub const B_CRLF: &[u8] = b"\r\n";
pub const WEEKS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sept", "Oct", "Nov", "Dec",
];
pub const LEAPOCH: i64 = 946684800i64 + 86400 * (31 + 29);
pub const DAYS_PER_400Y: i64 = 365 * 400 + 97;
pub const DAYS_PER_100Y: i64 = 365 * 100 + 24;
pub const DAYS_PER_4Y: i64 = 365 * 4 + 1;
pub static DAYS_IN_MONTH: [i64; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];
// TOKEN_CHARS needs to be defined somehow
lazy_static! {
    pub static ref TOKEN_CHARS: HashSet<char> = HashSet::from_iter(
        "!#$%&'*+-.0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ^_`abcdefghijklmnopqrstuvwxyz|~".chars()
    );
}

pub fn is_ctl(c: char) -> bool {
    c < '\x1f' || c == '\x7f'
}

pub fn from_hex(chr: char) -> Option<u8> {
    if chr > 'f' {
        return None;
    }
    let c = chr as u8;
    if c >= b'0' && c <= b'9' {
        return Some(c - b'0');
    }
    if c >= b'A' && c <= b'F' {
        return Some(c - b'A' + 10);
    }
    if c >= b'a' && c <= b'f' {
        return Some(c - b'a' + 10);
    }
    None
}
