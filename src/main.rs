#[macro_use]
extern crate lazy_static;
pub mod config;
pub mod constants;
pub mod error;
pub mod file_handler;
pub mod path;
pub mod query;
pub mod request;
pub mod responder;
pub mod router;
pub mod server;
pub mod time;
pub mod util;

use crate::router::Route;
use crate::server::Octane;

#[tokio::main]
async fn main() {
    let mut app = Octane::new();
    let static_dir = app.static_dir("templates").ok().unwrap();
    app.add(static_dir);
    app.listen(8080).await.expect("Cannot establish connection");
}
