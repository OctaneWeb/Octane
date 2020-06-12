#[macro_use]
extern crate lazy_static;
pub mod config;
pub mod constants;
pub mod error;
pub mod file_handler;
pub mod query;
pub mod request;
pub mod responder;
pub mod router;
pub mod server;
pub mod time;

use crate::router::Route;
use crate::server::Octane;

#[tokio::main]
async fn main() {
    let mut app = Octane::new();
    app.get(
        "/",
        route!(|_req, res| {
            res.send_file("templates/test.html")
                .await
                .expect("cannot find file");
        }),
    );
    app.listen(8080).await.expect("Cannot establish connection");
}
