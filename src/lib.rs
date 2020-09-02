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
//! octane = "0.1.1"
//! ```
//!
//! and then in your main.rs,
//!
//! ```no_run
//! use octane::server::Octane;
//! use octane::config::Config;
//! use octane::{route, router::{Flow, Route}};
//!
//! fn main() {
//!     let mut app = Octane::new();
//!     app.add(Octane::static_dir("dir_name")); // serve a static directory
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
//!     app.listen(8080).expect("Cannot establish connection");
//! }
//! ```
//! and now you can see the page at http://0.0.0.0:8080.
//!
//! ## Features
//!
//! Octane divides most of the things that one might _leave_ out for
//! any reason into features. These include,
//!
//! - `faithful`:
//! - `query_strings`:
//! - `cookies`: Basic cookie parsing and value handling.
//! - `url_variables`: To support variables in url.
//! - `raw_headers`:
//! - `rustls`: To use rustls for ssl.
//! - `openSSL`: To use openssl for ssl.
//! - 'default`: The default set includes faithful, query_strings, cookies,
//! url_variables, raw_headers.
//!
//! **Note**: If both `rustls` and `openSSL` features are enabled then
//! octane will throw a `compile_error!`
#[warn(missing_docs)]
#[macro_use]
extern crate lazy_static;

pub mod config;
mod constants;
#[cfg(feature = "cookies")]
pub mod cookies;
mod error;
mod file_handler;
mod http;
pub mod middlewares;
pub mod path;
pub mod query;
pub mod request;
pub mod responder;
pub mod router;
pub mod server;
mod server_builder;
mod time;
pub mod tls;
mod util;

// convenient aliasing for octane_json
pub use octane_json as json;

#[cfg(all(feature = "openSSL", feature = "rustls"))]
compile_error!("openSSL and rustls are both enabled, you may want to one of those");
