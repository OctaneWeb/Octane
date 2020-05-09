use std::collections::HashSet;
use std::iter::FromIterator;

pub const SP: char = ' ';
pub const HT: char = '\t';
pub const CR: char = '\r';
pub const LF: char = '\n';
pub const CRLF: &str = "\r\n";
// TOKEN_CHARS needs to be defined somehow
lazy_static! {
    pub static ref TOKEN_CHARS: HashSet<char> = HashSet::from_iter(
        "!#$%&'*+-.0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ^_`abcdefghijklmnopqrstuvwxyz|~".chars()
    );
}

pub fn is_ctl(c: char) -> bool {
    c < '\x1f' || c == '\x7f'
}
