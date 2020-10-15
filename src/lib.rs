#![allow(clippy::all)]
#![allow(clippy::type_complexity)]
#![forbid(unsafe_code)]
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
//! tokio = "0.2.22"
//! ```
//!
//! and then in your main.rs,
//!
//! ```no_run
//! use octane::prelude::*;
//! use std::error::Error;
//!
//! #[octane::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     let mut app = Octane::new();
//!     app.add(Octane::static_dir(path!("/templates")))?; // serve a static directory
//!     app.get("/",
//!         route_stop!(
//!             |req, res| {
//!                 res.send("Hello, World");
//!             }
//!         ),
//!     )?;
//!     let port = 8080;
//!     app.listen(port, || {
//!         println!("Server running on {}", port)
//!     }).await
//! }
//! ```
//! and now you can see the page at [http://localhost:8080](http://localhost:8080).
//!
//! ## Features
//!
//! Octane divides most of the things that one might _leave_ out for
//! any reason into features. These include,
//!
//! - `faithful`: To make octane conform to http spec
//! with some added overhead
//! - `query_strings`: To enable query string parsing, eg. `?foo=bar&bar=foo`
//! - `cookies`: To enable basic cookie parsing and value handling.
//! - `url_variables`: To support variables in url.
//! - `raw_headers`: To have access to original, un-normalized headers.
//! - `rustls`: To use rustls for ssl.
//! - `openSSL`: To use openssl for ssl.
//! - `default`: The default set includes faithful, query_strings, cookies,
//! url_variables, raw_headers.
//!
//! **Note**: If both `rustls` and `openSSL` features are enabled then
//! octane will throw a `compile_error!`
//!
//! The rust code you would use to setup openssl will be same as rustls (not including certs and keys).
//! If you want to switch between any of them, its just the matter of editing your cargo.toml
//! to enable the feature.
#[macro_use]
extern crate lazy_static;
/// Configurations for Octane web server
pub mod config;
pub(crate) mod constants;
#[cfg(feature = "cookies")]
/// Module for cookie parsing and handling
pub mod cookies;
pub(crate) mod error;
pub(crate) mod file_handler;
pub(crate) mod http;
pub(crate) mod middlewares;
pub(crate) mod path;
#[cfg(feature = "query_strings")]
pub(crate) mod query;
/// Request module contains the ongoing request and methods to read from it
pub mod request;
/// Responder module contains the response which will be sent
pub mod responder;
/// The router module has utilities to create routes and custom routers
pub mod router;
pub(crate) mod server;
/// Server struct that manages request/response and allows the routes to enter in
pub use crate::server::Octane;
pub(crate) mod server_builder;
pub(crate) mod time;
pub(crate) mod tls;
pub(crate) mod util;

// convenient aliasing for octane_json
pub use octane_json as json;
pub use octane_macros::main;
pub use octane_macros::test;

/// Prelude here brings in scope, the [`Route`](router/trait.Route.html) and
/// [`Config`](config/trait.Config.html) trait, [`Octane`](struct.Octane.html) main server
/// and [`Router`](router/struct.Router.html) struct with the [`Flow`](router/enum.Flow.html)
/// and the [`Value`](../octane_json/enum.Value.html) enum, the [`route`](macro.route.html),
/// [`route_next`](macro.route_next.html), [`path`](macro.path.html), [`route_stop`](macro.route_stop.html)
/// macros with the [`ToJSON`](../octane_json/convert/trait.ToJSON.html)
/// /[`FromJSON`](../octane_json/convert/trait.FromJSON.html) derive macros
pub mod prelude {
    pub use crate::config::Config;
    pub use crate::json::{FromJSON, ToJSON};
    pub use crate::Octane;
    pub use crate::{
        route, route_next, route_stop,
        router::{Flow, Route, Router},
    };
    pub use octane_json::Value;
    pub use octane_macros::path;
}

#[cfg(all(feature = "openSSL", feature = "rustls"))]
compile_error!("openSSL and rustls are both enabled, you may want to one of those");
