#[macro_use]
extern crate lazy_static;
pub mod constants;
pub mod error;
pub mod file_handler;
pub mod query;
pub mod request;
pub mod responder;
pub mod server;
pub mod time;

use crate::constants::*;
use crate::server::Octane;
#[tokio::main]
async fn main() {
    let mut app = Octane::new();
    app.get("/", |_req, res| {
        Box::pin(async move {
            let _file = res.send_file("templates/test.html").await;
        })
    });
    let _res = app.listen(8080).await;
}
