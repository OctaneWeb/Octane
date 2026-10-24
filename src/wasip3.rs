//! Serving from a WASIp3 component with octane's router.
//!
//! Routes are added through the same [`Route`] trait as on the tokio server
//! (`get`, `post`, `add` for middleware, url variables like `/users/:id`
//! with the `url_variables` feature) and run in registration order until
//! one returns [`Flow::Stop`]. Handlers are async, so a handler that waits
//! on a timer or streams a body doesn't hold up other connections. They
//! answer through a [`Responder`]; a request nothing answered gets a `404`.
//!
//! ```ignore
//! use octane::portable::config::Config;
//! use octane::wasip3::{handler, serve, Flow, Route, Router};
//! use octane::StatusCode;
//!
//! const CONFIG: Config<{ 16 * 1024 }, 1024> = Config::new();
//!
//! let mut router = Router::new();
//! router.get("/users/:id", handler(async |req, res| {
//!     let id = req.vars.get("id").copied().unwrap_or("");
//!     let _ = res.send(StatusCode::Ok, &[], id.as_bytes()).await;
//!     Flow::Stop
//! }))?;
//!
//! // inside the component's async `run` export
//! serve(CONFIG, "127.0.0.1:8080".parse().unwrap(), |_| {}, router).await
//! ```

use crate::default;
pub use crate::method::RequestMethod;
/// The error the [`Route`] methods return for a path that climbs above the root
pub use crate::path::InvalidPathError;
use crate::path::PathBuf;
pub use crate::routes::{Flow, Route};
use crate::routes::{RouterResult, Routes};
use octane_http::config::Config;
use octane_http::http1x::Request;
pub use octane_http::wasip3::{BodyWriter, ErrorCode, Responder, SendError};
#[cfg(feature = "url_variables")]
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;

/// The request a handler sees: the parsed request and, with the
/// `url_variables` feature, the variables of the route that matched. It
/// derefs to [`Request`].
pub struct MatchedRequest<'r> {
    /// The request coming from the client
    pub request: &'r Request<'r>,
    /// The url variables of the matched route, empty for middleware
    #[cfg(feature = "url_variables")]
    pub vars: HashMap<&'r str, &'r str>,
}

impl<'r> Deref for MatchedRequest<'r> {
    type Target = Request<'r>;

    fn deref(&self) -> &Self::Target {
        self.request
    }
}

/// A boxed async handler, made with [`handler`]
pub type Handler = Box<
    dyn for<'a, 'r, 'c> Fn(
        &'a MatchedRequest<'r>,
        &'a mut Responder<'c>,
    ) -> Pin<Box<dyn Future<Output = Flow> + 'a>>,
>;

/// Box an async closure or `async fn` into a [`Handler`] for the
/// [`Route`] methods
pub fn handler<F>(f: F) -> Handler
where
    F: AsyncFn(&MatchedRequest<'_>, &mut Responder<'_>) -> Flow + 'static,
{
    let f = Rc::new(f);
    Box::new(higher_ranked(move |req, res| {
        let f = Rc::clone(&f);
        Box::pin(async move { (*f)(req, res).await })
    }))
}

/// Pins the closure's signature to the higher-ranked one of [`Handler`],
/// which closure inference doesn't do on its own
fn higher_ranked<C>(closure: C) -> C
where
    C: for<'a, 'r, 'c> Fn(
        &'a MatchedRequest<'r>,
        &'a mut Responder<'c>,
    ) -> Pin<Box<dyn Future<Output = Flow> + 'a>>,
{
    closure
}

/// The WASIp3 router, filled through the [`Route`] trait
pub struct Router {
    routes: Routes<Handler>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Routes::new(),
        }
    }

    /// Append the routes of `router`, which run after the ones already here
    pub fn append(&mut self, router: Router) {
        self.routes.append(router.routes);
    }

    /// Run the handlers matching `request` in registration order until one
    /// returns [`Flow::Stop`]
    pub async fn run(&self, request: &Request<'_>, res: &mut Responder<'_>) {
        let method = RequestMethod::from(request.method);
        // the parser already refused `..` segments, so this can't fail by
        // climbing above the root
        let path = match PathBuf::parse(request.path) {
            Ok(path) => path,
            Err(_) => return,
        };
        for route in self.routes.matched(method, &path) {
            let matched = MatchedRequest {
                request,
                #[cfg(feature = "url_variables")]
                vars: route.vars,
            };
            if !(route.data)(&matched, res).await.should_continue() {
                break;
            }
        }
    }
}

default!(Router);

impl Route for Router {
    type Handler = Handler;

    fn add_route(&mut self, path: &str, handler: Handler) -> RouterResult {
        self.routes.insert(RequestMethod::All, path, handler)
    }
    fn head(&mut self, path: &str, handler: Handler) -> RouterResult {
        self.routes.insert(RequestMethod::Head, path, handler)
    }
    fn post(&mut self, path: &str, handler: Handler) -> RouterResult {
        self.routes.insert(RequestMethod::Post, path, handler)
    }
    fn get(&mut self, path: &str, handler: Handler) -> RouterResult {
        self.routes.insert(RequestMethod::Get, path, handler)
    }
    fn put(&mut self, path: &str, handler: Handler) -> RouterResult {
        self.routes.insert(RequestMethod::Put, path, handler)
    }
    fn delete(&mut self, path: &str, handler: Handler) -> RouterResult {
        self.routes.insert(RequestMethod::Delete, path, handler)
    }
    fn patch(&mut self, path: &str, handler: Handler) -> RouterResult {
        self.routes.insert(RequestMethod::Patch, path, handler)
    }
    fn add(&mut self, handler: Handler) -> RouterResult {
        self.routes.add(handler);
        Ok(())
    }
}

/// Listen on `addr` and serve every connection with `router`, each as its
/// own task. `on_listen` gets the bound address once the socket listens.
/// Errors creating, binding or listening on the socket are returned.
pub async fn serve<const INPUT: usize, const OUTPUT: usize, L>(
    config: Config<INPUT, OUTPUT>,
    addr: SocketAddr,
    on_listen: L,
    router: Router,
) -> Result<(), ErrorCode>
where
    L: FnOnce(SocketAddr),
{
    octane_http::wasip3::serve(
        config,
        addr,
        on_listen,
        async move |req: &Request<'_>, res: &mut Responder<'_>| router.run(req, res).await,
    )
    .await
}
