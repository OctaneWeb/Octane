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

use crate::path::InvalidPathError;
use crate::router::Route;
use crate::server::Octane;

#[tokio::main]
async fn main() {
    let mut app = Octane::new();
    app.get(
        "hello",
        route!(|req, res| {
            res.send(b"Hello");
        }),
    );
    app.add(Octane::static_dir(app, "lo"));
    app.listen(8080).await.expect("Cannot establish connection");
}
