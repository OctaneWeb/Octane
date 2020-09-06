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

// constants for time.rs
pub const LEAPOCH: i64 = 946684800i64 + 86400 * (31 + 29);
pub const DAYS_PER_400Y: i64 = 365 * 400 + 97;
pub const DAYS_PER_100Y: i64 = 365 * 100 + 24;
pub const DAYS_PER_4Y: i64 = 365 * 4 + 1;
pub static DAYS_IN_MONTH: [i64; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];
pub const NOT_FOUND: &str = r#"<!DOCTYPE html><html><head><title>404 NOT FOUND - OCTANE</title></head><body style="padding: 20px;"><h2 style="text-align: center;">404 NOT FOUND</h2><hr><h5>OCTANE - 0.1</h2></body></html>"#;
// Default buffer size
pub const BUF_SIZE: usize = 512;

lazy_static! {
    pub static ref TOKEN_CHARS: HashSet<char> = HashSet::from_iter(
        "!#$%&'*+-.0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ^_`abcdefghijklmnopqrstuvwxyz|~".chars()
    );
}

/// macro to implement deref quickly on structs
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
/// macro to that spawns a tokio task
#[macro_export]
macro_rules! task {
    ($body : expr ) => {{
        tokio::spawn(async move { $body })
    }};
}
/// macro to implement default for structs
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
/// implement displa with given message
#[macro_export]
macro_rules! display {
    ($struct : tt, $message : expr) => {
        impl std::fmt::Display for $struct {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $message)
            }
        }
    };
}
