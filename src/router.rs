use crate::default;
use crate::method::RequestMethod;
use crate::request::{MatchedRequest, Request};
use crate::responder::Response;
pub(crate) use crate::routes::RouterResult;
use crate::routes::Routes;
pub use crate::routes::{Flow, Route};
#[cfg(feature = "url_variables")]
use std::collections::HashMap;

/// The Closure type is a type alias for the type
/// that the routes should return
pub type Closure = Box<dyn for<'a> Fn(&'a MatchedRequest, &'a mut Response) -> Flow + Send + Sync>;

/// The router structure defines the routes and stores them along with
/// their indexes. Router methods can be used with this struct and also
/// the main server structure.
pub struct Router {
    pub(crate) routes: Routes<Closure>,
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
    // Fetch the closures matching the request path and run them in
    // registration order until one returns `Flow::Stop`.
    pub(crate) fn run(&self, parsed_request: Request<'_>, res: &mut Response) {
        let method = parsed_request.request_line.method;
        // `Matched` values borrow their input path when URL variables are
        // enabled. A path-only clone lets the router move the much larger
        // request (including all headers) into `MatchedRequest` unchanged.
        let match_path = parsed_request.request_line.path.clone();
        let routes = self.routes.matched(method, &match_path);

        #[cfg(feature = "url_variables")]
        let mut matched = MatchedRequest {
            request: parsed_request,
            vars: HashMap::new(),
        };
        #[cfg(not(feature = "url_variables"))]
        let matched = MatchedRequest {
            request: parsed_request,
        };

        for route in routes {
            #[cfg(feature = "url_variables")]
            {
                matched.vars = route.vars;
            }
            if !(route.data)(&matched, res).should_continue() {
                break;
            }
        }
    }
}

#[macro_export]
macro_rules! route {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        #[allow(unused_variables)]
        Box::new(move |$req, $res| $body)
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
    type Handler = Closure;

    fn head(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.routes.insert(RequestMethod::Head, path, closure)
    }
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.routes.insert(RequestMethod::Put, path, closure)
    }
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.routes.insert(RequestMethod::Get, path, closure)
    }
    fn delete(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.routes.insert(RequestMethod::Delete, path, closure)
    }
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.routes.insert(RequestMethod::Post, path, closure)
    }
    fn patch(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.routes.insert(RequestMethod::Patch, path, closure)
    }
    fn add(&mut self, closure: Closure) -> RouterResult {
        self.routes.add(closure);
        Ok(())
    }
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.routes.insert(RequestMethod::All, path, closure)
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
        assert_eq!(5, router.routes.paths.len());
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
                .paths
                .get(&RequestMethod::Get)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );

        assert_eq!(
            2,
            first_router
                .routes
                .paths
                .get(&RequestMethod::Delete)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .routes
                .paths
                .get(&RequestMethod::Post)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .routes
                .paths
                .get(&RequestMethod::Head)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .routes
                .paths
                .get(&RequestMethod::Put)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
    }

    #[test]
    fn run_merges_routes_and_middleware_in_registration_order() {
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
        router.run(request, &mut Response::new_empty());

        assert_eq!(*seen.lock().unwrap(), vec![0, 1, 2, 3]);
    }

    #[test]
    #[cfg(feature = "url_variables")]
    fn run_moves_route_variables_and_clears_them_for_middleware() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut router = Router::new();
        let route_seen = Arc::clone(&seen);
        router
            .get(
                "/thing/:id",
                Box::new(move |request, _| {
                    route_seen
                        .lock()
                        .unwrap()
                        .push(request.vars.get("id").map(|value| value.to_string()));
                    Flow::Next
                }),
            )
            .unwrap();
        let middleware_seen = Arc::clone(&seen);
        router
            .add(Box::new(move |request, _| {
                middleware_seen
                    .lock()
                    .unwrap()
                    .push(request.vars.get("id").map(|value| value.to_string()));
                Flow::Next
            }))
            .unwrap();

        let request = Request::parse(
            RequestLine::parse("GET /thing/42 HTTP/1.1").unwrap(),
            Headers::parse("").unwrap(),
            b"",
        )
        .unwrap();
        router.run(request, &mut Response::new_empty());

        assert_eq!(*seen.lock().unwrap(), vec![Some("42".to_owned()), None]);
    }
}
