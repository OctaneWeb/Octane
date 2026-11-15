use crate::default;
use crate::method::RequestMethod;
#[cfg(feature = "url_variables")]
use crate::path::Vars;
use crate::request::{MatchedRequest, Request};
use crate::responder::Response;
pub(crate) use crate::routes::RouterResult;
use crate::routes::Routes;
pub use crate::routes::{Flow, Route};
use std::future::Future;
use std::pin::Pin;

/// A synchronous handler, made with the [`route!`](macro.route.html),
/// [`route_next!`](macro.route_next.html) and
/// [`route_stop!`](macro.route_stop.html) macros
pub type Closure = Box<dyn for<'a> Fn(&'a MatchedRequest, &'a mut Response) -> Flow + Send + Sync>;

/// The future an async handler returns, see [`Handler`]
pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = Flow> + Send + 'a>>;

/// What runs when a route matches. The [`Route`] methods take either kind:
///
/// - a synchronous [`Closure`], made with the [`route!`](macro.route.html)
///   macros. It runs straight on the connection's task, so it should not
///   block.
/// - an async handler, made with the
///   [`route_async!`](macro.route_async.html) macros. It can `.await`
///   timers, I/O or other tasks without holding up the worker thread.
///
/// ```
/// use octane::prelude::*;
/// use std::time::Duration;
///
/// let mut app = Octane::new();
/// app.get("/sync", route_stop!(|req, res| res.send("now"))).unwrap();
/// app.get(
///     "/async",
///     route_async_stop!(|req, res| {
///         tokio::time::sleep(Duration::from_millis(10)).await;
///         res.send("later");
///     }),
/// )
/// .unwrap();
/// ```
pub struct Handler(Kind);

enum Kind {
    Sync(Closure),
    Async(Box<dyn AsyncHandler>),
}

impl Handler {
    // Run the handler, an async one until its future completes
    pub(crate) async fn call(&self, req: &MatchedRequest<'_>, res: &mut Response<'_>) -> Flow {
        match &self.0 {
            Kind::Sync(closure) => closure(req, res),
            Kind::Async(handler) => handler.call(req, res).await,
        }
    }

    /// Box an async closure for the [`Route`] methods, it's what the
    /// [`route_async!`](macro.route_async.html) macros expand to. `call`
    /// calls `handler` and boxes its future. It is passed in rather than
    /// written here so that it is checked where the closure's type is known,
    /// which is what proves that future is `Send`.
    #[doc(hidden)]
    pub fn from_async<F>(handler: F, call: AsyncCall<F>) -> Self
    where
        F: Send + Sync + 'static,
    {
        Handler(Kind::Async(Box::new(AsyncClosure { handler, call })))
    }
}

impl From<Closure> for Handler {
    fn from(closure: Closure) -> Self {
        Handler(Kind::Sync(closure))
    }
}

/// How [`Handler::from_async`] calls the async closure it was given
#[doc(hidden)]
pub type AsyncCall<F> =
    for<'a, 'r, 'c> fn(&'a F, &'a MatchedRequest<'r>, &'a mut Response<'c>) -> HandlerFuture<'a>;

// An async handler, its future may borrow from the handler itself as well as
// from the request and response, so calling it needs no reference counting
trait AsyncHandler: Send + Sync {
    fn call<'a>(
        &'a self,
        req: &'a MatchedRequest<'_>,
        res: &'a mut Response<'_>,
    ) -> HandlerFuture<'a>;
}

struct AsyncClosure<F> {
    handler: F,
    call: AsyncCall<F>,
}

impl<F: Send + Sync> AsyncHandler for AsyncClosure<F> {
    fn call<'a>(
        &'a self,
        req: &'a MatchedRequest<'_>,
        res: &'a mut Response<'_>,
    ) -> HandlerFuture<'a> {
        (self.call)(&self.handler, req, res)
    }
}

/// The router structure defines the routes and stores them along with
/// their indexes. Router methods can be used with this struct and also
/// the main server structure.
pub struct Router {
    pub(crate) routes: Routes<Handler>,
}

impl Router {
    /// Return a new and empty router instance, with this
    /// you can start using router.METHODs on it and then
    /// append the `Router` instance to the app (`Octane`)
    /// instance by doing `app.use_router(router)`.
    pub fn new() -> Self {
        Router {
            routes: Routes::new(),
        }
    }
    // append the routes stored in a custom Router to the self Router
    pub(crate) fn append(&mut self, router: Self) {
        self.routes.append(router.routes);
    }
    /// The methods with a route for `path`, for the `Allow` header of a
    /// `405 Method Not Allowed`
    pub(crate) fn allowed(&self, path: &crate::path::PathBuf) -> Vec<RequestMethod> {
        self.routes.allowed(path)
    }

    // Fetch the handlers matching the request path and run them in
    // registration order until one returns `Flow::Stop`.
    pub(crate) async fn run(&self, request: &Request<'_>, res: &mut Response<'_>) {
        let line = &request.request_line;
        let routes = self.routes.matched(line.method, &line.path);
        #[cfg(feature = "url_variables")]
        let mut matched = MatchedRequest {
            request,
            vars: Vars::new(),
        };
        #[cfg(not(feature = "url_variables"))]
        let matched = MatchedRequest { request };

        for route in routes {
            #[cfg(feature = "url_variables")]
            {
                matched.vars = route.vars;
            }
            if !route.data.call(&matched, res).await.should_continue() {
                break;
            }
        }
    }
}

/// Make a synchronous handler, a [`Closure`], whose body returns the
/// [`Flow`]
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(|req, res| {
///         res.send("Hello, World");
///         Flow::Stop
///     }),
/// );
/// ```
#[macro_export]
macro_rules! route {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        #[allow(unused_variables)]
        let closure: $crate::router::Closure = ::std::boxed::Box::new(move |$req, $res| $body);
        closure
    }};
}

/// Make an async handler, whose body can `.await` and returns the [`Flow`].
/// The body borrows what the closure captured, so captured values are not
/// cloned per request. See [`Handler`].
///
/// # Example
///
/// ```
/// use octane::prelude::*;
/// use std::time::Duration;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route_async!(|req, res| {
///         tokio::time::sleep(Duration::from_millis(5)).await;
///         res.send("Hello, World");
///         Flow::Stop
///     }),
/// );
/// ```
#[macro_export]
macro_rules! route_async {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        $crate::router::Handler::from_async(
            #[allow(unused_variables)]
            async move |$req: &$crate::request::MatchedRequest<'_>,
                        $res: &mut $crate::responder::Response<'_>|
                        -> $crate::router::Flow { $body },
            |handler, req, res| ::std::boxed::Box::pin(handler(req, res)),
        )
    }};
}

/// Just like [`route_async!`](macro.route_async.html) but returns
/// [`Flow::Next`](enum.Flow.html) implicitly
#[macro_export]
macro_rules! route_async_next {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        $crate::route_async!(|$req, $res| {
            $body;
            $crate::router::Flow::Next
        })
    }};
}

/// Just like [`route_async!`](macro.route_async.html) but returns
/// [`Flow::Stop`](enum.Flow.html) implicitly
#[macro_export]
macro_rules! route_async_stop {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        $crate::route_async!(|$req, $res| {
            $body;
            $crate::router::Flow::Stop
        })
    }};
}

/// Just like the [`route!()`](macro.route.html) macro but return [`Flow::Next`](enum.Flow.html)
/// Implicitly
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route_next!(
///         |req, res| {
///             res.send("Hello, World");
///         }
///     ),
/// );
/// ```
#[macro_export]
macro_rules! route_next {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        route!(|$req, $res| {
            $body;
            Flow::Next
        })
    }};
}

/// Just like the [`route!()`](macro.route.html) macro but return [`Flow::Stop`](enum.Flow.html)
/// Implicitly
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route_stop!(
///         |req, res| {
///             res.send("Hello, World");
///         }
///     ),
/// );
/// ```
#[macro_export]
macro_rules! route_stop {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        route!(|$req, $res| {
            $body;
            Flow::Stop
        })
    }};
}

default!(Router);

impl Route for Router {
    type Handler = Handler;

    fn head(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.routes
            .insert(RequestMethod::Head, path, handler.into())
    }
    fn put(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.routes.insert(RequestMethod::Put, path, handler.into())
    }
    fn get(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.routes.insert(RequestMethod::Get, path, handler.into())
    }
    fn delete(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.routes
            .insert(RequestMethod::Delete, path, handler.into())
    }
    fn post(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.routes
            .insert(RequestMethod::Post, path, handler.into())
    }
    fn patch(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.routes
            .insert(RequestMethod::Patch, path, handler.into())
    }
    fn add(&mut self, handler: impl Into<Handler>) -> RouterResult {
        self.routes.add(handler.into());
        Ok(())
    }
    fn add_route(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.routes.insert(RequestMethod::All, path, handler.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::path::PathBuf;
    use crate::request::{Headers, RequestLine};
    use std::sync::{Arc, Mutex};

    fn record(seen: Arc<Mutex<Vec<usize>>>, value: usize) -> Closure {
        Box::new(move |_, _| {
            seen.lock().unwrap().push(value);
            Flow::Next
        })
    }

    #[test]
    pub fn router_test() {
        let mut router = Router::new();
        router.add(route!(|req, res| { Flow::Next })).unwrap();
        router.get("/", route!(|req, res| { Flow::Next })).unwrap();
        router.post("/", route!(|req, res| { Flow::Next })).unwrap();
        router.head("/", route!(|req, res| { Flow::Next })).unwrap();
        router.put("/", route!(|req, res| { Flow::Next })).unwrap();
        router
            .delete("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        // middleware with path
        assert_eq!(5, router.routes.paths_with_routes());
        // middleware without paths
        assert_eq!(1, router.routes.middlewares.len());
    }

    #[test]
    pub fn router_append_test() {
        let mut first_router = Router::new();
        first_router.add(route!(|req, res| { Flow::Next })).unwrap();
        first_router
            .get("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        first_router
            .post("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        first_router
            .head("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        first_router
            .put("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        first_router
            .delete("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        let mut second_router = Router::new();
        second_router
            .add(route!(|req, res| { Flow::Next }))
            .unwrap();
        second_router
            .get("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        second_router
            .post("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        second_router
            .head("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        second_router
            .put("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        second_router
            .delete("/", route!(|req, res| { Flow::Next }))
            .unwrap();
        first_router.append(second_router);
        // middleware tied without path
        assert_eq!(2, first_router.routes.middlewares.len());
        assert_eq!(
            2,
            first_router
                .routes
                .node(RequestMethod::Get)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );

        assert_eq!(
            2,
            first_router
                .routes
                .node(RequestMethod::Delete)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .routes
                .node(RequestMethod::Post)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .routes
                .node(RequestMethod::Head)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .routes
                .node(RequestMethod::Put)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
    }

    #[tokio::test]
    async fn run_merges_routes_and_middleware_in_registration_order() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut router = Router::new();
        router.add(record(Arc::clone(&seen), 0)).unwrap();
        router.add_route("/", record(Arc::clone(&seen), 1)).unwrap();
        router.get("/", record(Arc::clone(&seen), 2)).unwrap();
        router.add(record(Arc::clone(&seen), 3)).unwrap();

        let request = Request::parse(
            RequestLine::parse("GET / HTTP/1.1").unwrap(),
            Headers::parse("").unwrap(),
            b"",
        )
        .unwrap();
        router.run(&request, &mut Response::new_empty()).await;

        assert_eq!(*seen.lock().unwrap(), vec![0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn async_handlers_run_in_order_with_sync_ones() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut router = Router::new();
        router.add(record(Arc::clone(&seen), 0)).unwrap();
        // the body borrows what the closure captured, across an await
        let first = Arc::clone(&seen);
        router
            .get(
                "/",
                route_async_next!(|req, res| {
                    tokio::task::yield_now().await;
                    first.lock().unwrap().push(1);
                }),
            )
            .unwrap();
        router.get("/", record(Arc::clone(&seen), 2)).unwrap();
        let last = Arc::clone(&seen);
        router
            .get(
                "/",
                route_async_stop!(|req, res| {
                    tokio::task::yield_now().await;
                    last.lock().unwrap().push(3);
                    res.send("done");
                }),
            )
            .unwrap();
        // after a Stop nothing else runs
        router.add(record(Arc::clone(&seen), 4)).unwrap();

        let request = Request::parse(
            RequestLine::parse("GET / HTTP/1.1").unwrap(),
            Headers::parse("").unwrap(),
            b"",
        )
        .unwrap();
        let mut res = Response::new_empty();
        router.run(&request, &mut res).await;

        assert_eq!(*seen.lock().unwrap(), vec![0, 1, 2, 3]);
        assert!(res.has_body());
    }

    #[tokio::test]
    #[cfg(feature = "url_variables")]
    async fn run_moves_route_variables_and_clears_them_for_middleware() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut router = Router::new();
        let route_seen = Arc::clone(&seen);
        let route: Closure = Box::new(move |request, _| {
            route_seen
                .lock()
                .unwrap()
                .push(request.vars.get("id").map(|value| value.to_string()));
            Flow::Next
        });
        router.get("/thing/:id", route).unwrap();
        let middleware_seen = Arc::clone(&seen);
        let middleware: Closure = Box::new(move |request, _| {
            middleware_seen
                .lock()
                .unwrap()
                .push(request.vars.get("id").map(|value| value.to_string()));
            Flow::Next
        });
        router.add(middleware).unwrap();

        let request = Request::parse(
            RequestLine::parse("GET /thing/42 HTTP/1.1").unwrap(),
            Headers::parse("").unwrap(),
            b"",
        )
        .unwrap();
        router.run(&request, &mut Response::new_empty()).await;

        assert_eq!(*seen.lock().unwrap(), vec![Some("42".to_owned()), None]);
    }
}
