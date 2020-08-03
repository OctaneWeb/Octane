//! Octane is a web server that's modelled after express (a very
//! popular and easy to use web framework) for rust.
//!
//! While minimising dependencies, Octane thrives to be a high performance
//! web server while being easy to use at the same time.
//!
//! You can find other docs at the [OctaneSite]().
//!
//! # Example
//!
//! Get started by adding the lib entry in your cargo.toml file
//!
//! ```toml
//! octane = "0.2"
//! ```
//! and then in your main file,
//! ```no_run
//! use octane::server::Octane;
//! use octane::config::Config;
//! use octane::{route, router::{Flow, Route}};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut app = Octane::new();
//!     app.add_static_dir("/", "dir_name"); // server a static directory
//!     app.get(
//!         "/",
//!         route!(
//!             |req, res| {
//!                 res.send("Hello, World");
//!                 Flow::Stop
//!             }
//!         ),
//!     );
//!
//!     app.listen(8080).await.expect("Cannot establish connection");
//! }
//! ```
//! and now you can see the page at http://0.0.0.0:8080.
//!
//! ## Features
//! Octane divides most of the things that one might _leave_ out for
//! any reason into features. These include
//! - `faithful`
//! - `query_strings`
//! - `cookies`
//! - `url_variables`
//! - `raw_headers`
//! - `rustls`
//! - `openSSL`
//! - 'default`
//!

#[macro_use]
extern crate lazy_static;
pub mod config;
pub mod constants;
#[cfg(feature = "cookies")]
pub mod cookies;
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

// convenient aliasing for octane_json
pub use octane_json as json;
