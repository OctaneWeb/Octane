use crate::middlewares::Closures;
use crate::path::PathNode;
use crate::request::RequestMethod;
use crate::router::Paths;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::sync::Mutex;

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

// constants for time.rs
pub const LEAPOCH: i64 = 946684800i64 + 86400 * (31 + 29);
pub const DAYS_PER_400Y: i64 = 365 * 400 + 97;
pub const DAYS_PER_100Y: i64 = 365 * 100 + 24;
pub const DAYS_PER_4Y: i64 = 365 * 4 + 1;
pub static DAYS_IN_MONTH: [i64; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

// Default buffer size
pub const BUF_SIZE: usize = 512;

lazy_static! {
    pub static ref TOKEN_CHARS: HashSet<char> = HashSet::from_iter(
        "!#$%&'*+-.0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ^_`abcdefghijklmnopqrstuvwxyz|~".chars()
    );
    // HashMap for storing closures
    pub static ref CLOSURES: Mutex<Paths> = Mutex::new(HashMap::new());
}

pub fn closures_lock<F, T>(f: F) -> T
where
    F: FnOnce(&mut HashMap<RequestMethod, PathNode<Closures>>) -> T,
{
    let mut lock = CLOSURES.lock().unwrap();
    f(&mut *lock)
}

pub fn is_ctl(c: char) -> bool {
    c < '\x1f' || c == '\x7f'
}

#[macro_export]
macro_rules! deref {
    ( $struct : ident<$($gen: tt),+>, $target : ty, $body : ident ) => {
        impl<$($gen),+> std::ops::Deref for $struct <$($gen),+> {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$body
            }
        }
    };
    ( $struct : ty, $target : ty, $body : ident ) => {
        impl std::ops::Deref for $struct {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$body
            }
        }
    };

}

#[macro_export]
macro_rules! default {
    ( $struct : ident<$($gen: tt),+> ) => {
        impl<$($gen),+> Default for $struct <$($gen),+> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
    ( $struct : ty ) => {
        impl Default for $struct {
            fn default() -> Self {
                Self::new()
            }
        }
    };
    ( $struct : ty, $args : expr ) => {
        impl Default for $struct {
            fn default() -> Self {
                Self::new($args)
            }
        }
    };
}
