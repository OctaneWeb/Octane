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
//! #[octane::main]
//! async fn main() {
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
//!     app.listen(8080).await.expect("Cannot establish connection");
//! }
//! ```
//! and now you can see the page at http://0.0.0.0:8080.
//!
//! ## Features
//!
//! Octane divides most of the things that one might _leave_ out for
//! any reason into features. These include,
//!
//! - `faithful`: This feature, when enabled, makes octane conforms to http spec
//! with some added overhead because of it
//! - `query_strings`: To enable query string parsing, eg. `?foo=bar&bar=foo`
//! - `cookies`: Basic cookie parsing and value handling.
//! - `url_variables`: To support variables in url.
//! - `raw_headers`: To have access to original, un-normalized headers.
//! - `rustls`: To use rustls for ssl.
//! - `openSSL`: To use openssl for ssl.
//! - `default`: The default set includes faithful, query_strings, cookies,
//! url_variables, raw_headers.
//!
//! **Note**: If both `rustls` and `openSSL` features are enabled then
//! octane will throw a `compile_error!`
#[macro_use]
extern crate lazy_static;
/// Configurations for Octane web server
pub mod config;
#[doc(hidden)]
mod constants;
#[cfg(feature = "cookies")]
/// Module for cookie parsing and handling
pub mod cookies;
#[doc(hidden)]
mod error;
#[doc(hidden)]
mod file_handler;
#[doc(hidden)]
mod http;
mod middlewares;
/// Module to manipulate and work with paths
pub mod path;
/// Module to handle query string parsing
#[cfg(feature = "query_strings")]
pub mod query;
/// Request module contains the ongoing request and methods to read from it
pub mod request;
/// Responder module contains the response which will be sent
pub mod responder;
/// The router module has utils to create routes and custom routers
pub mod router;
/// Server struct that manages request/response and allows the routes to enter in
pub mod server;
#[doc(hidden)]
mod server_builder;
#[doc(hidden)]
mod time;
mod tls;
#[doc(hidden)]
mod util;

// convenient aliasing for octane_json
pub use octane_json as json;
pub use octane_macros::main;
pub use octane_macros::test;

#[cfg(all(feature = "openSSL", feature = "rustls"))]
compile_error!("openSSL and rustls are both enabled, you may want to one of those");
