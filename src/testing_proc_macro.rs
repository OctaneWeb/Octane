#[macro_use]
extern crate lazy_static;
pub mod config;
pub mod constants;
pub mod error;
pub mod file_handler;
pub mod json;
pub mod path;
pub mod query;
pub mod request;
pub mod responder;
pub mod router;
pub mod server;
pub mod time;
pub mod util;

use crate::json::{FromJSON, Value};

#[derive(FromJSON, Debug, Clone)]
pub struct UhWhat {
    a: i32,
    b: String,
}

fn main() {
    let hmm = Value::parse(r#"{"a": "abc", "b": "asf"}"#).unwrap();

    println!("{:?}", UhWhat::from_json(hmm));
}
