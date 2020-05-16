#[macro_use]
extern crate lazy_static;
pub mod constants;
pub mod error;
pub mod file_handler;
pub mod http;
pub mod query;
pub mod responder;
pub mod server;
pub mod time;

use crate::server::Server;
fn main() {
    let mut app = Server::new();
    app.static_dir("templates");
    app.listen(8080);
}
