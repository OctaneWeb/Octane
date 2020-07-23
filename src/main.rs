#[macro_use]
extern crate lazy_static;
pub mod config;
pub mod constants;
pub mod error;
pub mod file_handler;
pub mod middlewares;
pub mod path;
pub mod query;
pub mod request;
pub mod responder;
pub mod router;
pub mod server;
pub mod time;
pub mod tls;
pub mod util;

use crate::config::{Config, OctaneConfig};
use crate::router::{Flow, Route};
use crate::server::Octane;

#[tokio::main]
async fn main() {
    let mut app = Octane::new();
    let mut config = OctaneConfig::new();
    config
        .ssl
        .key("templates/key.pem")
        .cert("templates/cert.pem");
    config.add_static_dir("/", "templates");
    app.with_config(config);

    app.get(
        "/",
        route!(
            |req, res| {
                res.send("HELLO");
            },
            Flow::Next
        ),
    )
    .unwrap();
    app.listen(8080).await.expect("Cannot establish connection");
}
