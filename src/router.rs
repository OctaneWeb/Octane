use crate::default;
use crate::error::InvalidPathError;
use crate::middlewares::Closures;
use crate::path::PathNode;
use crate::request::{MatchedRequest, Request, RequestMethod};
use crate::responder::Response;
use std::collections::HashMap;
use std::result::Result;

// The type of HashMap where we will be storing the all the closures
pub(crate) type Paths = HashMap<RequestMethod, PathNode<Closures>>;
/// The Closure type is a type alias for the type
/// that the routes should return
pub type Closure = Box<dyn for<'a> Fn(&'a MatchedRequest, &'a mut Response) -> Flow + Send + Sync>;
// RouterResult is the type which the app.METHOD methods return
pub(crate) type RouterResult = Result<(), InvalidPathError>;
/// The flow enum works just like the next() callback
/// in express. The variant returns decides whether
/// the execution should go to the next similar route
/// or not
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(
///         |req, res| {
///             res.send("Hello, World");
///             Flow::Stop
///         }
///     ),
/// );
/// ```
#[derive(Copy, Clone, Debug)]
pub enum Flow {
    /// Don't execute the next closure which is similar in url with the initial one
    Stop,
    /// jump on the next closure which is similar in url with the initial one
    Next,
}

impl Flow {
    /// Returns true if the variant is `Flow::Next`
    pub fn should_continue(self) -> bool {
        matches!(self, Self::Next)
    }
}
/// The route trait adds the app.METHOD behaviour
/// to the router/Octane structures along with some
/// handful methods that can be used accordingly.
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(
///         |req, res| {
///             res.send("Hello, World");
///             Flow::Stop
///         }
///     ),
/// );
/// ```
/// While you are using these methods directly on your
/// Octane struct, you can add them on a router instance
/// which can be appended to your main server (octane)
/// struct. It's called Router
///
/// TODO: Include router example here
pub trait Route {
    /// add_route() is a duplicate of add() but with a
    /// specified url on where it should run.
    /// It runs on the given path and on all types of requests
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is HEAD
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.head(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("Hello, World");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    fn head(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is POST
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.post(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("Hello, World");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is GET
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("Hello, World");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is PUT
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.put(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("Hello, World");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is DELETE
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.delete(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("Hello, World");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    fn delete(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is PATCH
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.patch(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("Hello, World");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    fn patch(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// add() is like `app.use` in express, it runs on all the
    /// paths and all types of valid methods, the request comes
    /// on
    fn add(&mut self, entity: Closure) -> RouterResult;
}

/// The router structure defines the routes and stores them along with
/// their indexes. Router methods can be used with this struct and also
/// the main server structure.
pub struct Router {
    /// Holds a counter and increments on new additions of routes
    pub route_counter: usize,
    /// A vector of middleware closures
    pub middlewares: Vec<Closures>,
    /// The router paths which are to be executed on requests
    pub paths: Paths,
}

impl Router {
    /// Return a new and empty router instance, with this
    /// you can start using router.METHODs on it and then
    /// append the `Router` instance to the app (`Octane`)
    /// instance by doing `app.use_router(router)`.
    pub fn new() -> Self {
        Router {
            route_counter: 0,
            middlewares: Vec::new(),
            paths: HashMap::new(),
        }
    }
    // append the routes stored in a custom Router to the self Router
    pub(crate) fn append(&mut self, router: Self) {
        let self_count = self.route_counter;
        let other_count = router.route_counter;
        for (methods, paths) in router.paths.into_iter() {
            let updated_paths = paths.into_iter().map(|mut v| {
                v.data.index += self_count;
                v
            });
            if let Some(x) = self.paths.get_mut(&methods) {
                x.extend(updated_paths);
            } else {
                self.paths.insert(methods, updated_paths.collect());
            }
        }

        self.middlewares
            .extend(router.middlewares.into_iter().map(|mut v| {
                v.index += self_count;
                v
            }));
        self.route_counter += other_count;
    }

    // Fetch the closure according to the request path, run that
    // specific closure.
    pub(crate) fn run(&self, parsed_request: Request<'_>, res: &mut Response) {
        let method = parsed_request.request_line.method;
        // `MatchedPath` values borrow their input path when URL variables are
        // enabled. A path-only clone lets the router move the much larger
        // request (including all headers) into `MatchedRequest` unchanged.
        let match_path = parsed_request.request_line.path.clone();

        let mut method_routes = self
            .paths
            .get(&method)
            .map_or_else(Vec::new, |functions| {
                let mut routes = functions.get(&match_path);
                routes.sort_unstable_by_key(|route| route.index);
                routes
            })
            .into_iter()
            .peekable();
        // Run RequestMethod::All regardless of the request method.
        let mut all_routes = self
            .paths
            .get(&RequestMethod::All)
            .map_or_else(Vec::new, |functions| {
                let mut routes = functions.get(&match_path);
                routes.sort_unstable_by_key(|route| route.index);
                routes
            })
            .into_iter()
            .peekable();
        let mut middlewares = self.middlewares.iter().peekable();

        #[cfg(feature = "url_variables")]
        let mut matched = MatchedRequest {
            request: parsed_request,
            vars: HashMap::new(),
        };
        #[cfg(not(feature = "url_variables"))]
        let matched = MatchedRequest {
            request: parsed_request,
        };

        loop {
            let method_index = method_routes.peek().map_or(usize::MAX, |route| route.index);
            let all_index = all_routes.peek().map_or(usize::MAX, |route| route.index);
            let middleware_index = middlewares
                .peek()
                .map_or(usize::MAX, |middleware| middleware.index);

            if method_index == usize::MAX
                && all_index == usize::MAX
                && middleware_index == usize::MAX
            {
                break;
            }

            // Registration indexes are globally ordered. Merge the three
            // already-sorted streams directly instead of allocating a vector
            // for middleware matches plus bookkeeping vectors on every request.
            let flow = if method_index <= all_index && method_index <= middleware_index {
                let route = method_routes.next().unwrap();
                #[cfg(feature = "url_variables")]
                {
                    matched.vars = route.vars;
                }
                (&route.data.closure)(&matched, res)
            } else if all_index <= middleware_index {
                let route = all_routes.next().unwrap();
                #[cfg(feature = "url_variables")]
                {
                    matched.vars = route.vars;
                }
                (&route.data.closure)(&matched, res)
            } else {
                let middleware = middlewares.next().unwrap();
                #[cfg(feature = "url_variables")]
                matched.vars.clear();
                (&middleware.closure)(&matched, res)
            };
            if !flow.should_continue() {
                break;
            }
        }
    }
}
/// The route macro makes it easy to pass anonymous
/// functions to app.METHODs.
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(
///         |req, res| {
///             res.send("Hello, World");
///             Flow::Stop
///         }
///     ),
/// );
/// ```
///
/// The closure's environment is captured by value explictly,
/// so you are basically writing (pseudo code below)
///
/// ```ignore
/// use octane::prelude::*;
///
/// route!(move |req, res| {
///     res.send("Hello, World");
///     Flow::Stop
/// });
/// ```
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

macro_rules! inject_method {
    ( $instance: expr, $path: expr, $closure: expr, $method: expr ) => {
        use crate::middlewares::Closures;
        use crate::path::{PathBuf, PathNode};
        $instance
            .paths
            .entry($method)
            .or_insert(PathNode::new())
            .insert(
                PathBuf::parse($path)?,
                Closures {
                    closure: $closure,
                    index: $instance.route_counter,
                },
            );
        $instance.route_counter += 1;
    };
}

impl Route for Router {
    fn head(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Head);
        Ok(())
    }
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Put);
        Ok(())
    }
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Get);
        Ok(())
    }
    fn delete(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Delete);
        Ok(())
    }
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Post);
        Ok(())
    }
    fn patch(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Patch);
        Ok(())
    }
    fn add(&mut self, closure: Closure) -> RouterResult {
        self.middlewares.push(Closures {
            closure,
            index: self.route_counter,
        });
        self.route_counter += 1;
        Ok(())
    }
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::All);
        Ok(())
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
        assert_eq!(5, router.paths.len());
        // middleware without paths
        assert_eq!(1, router.middlewares.len());
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
        assert_eq!(2, first_router.middlewares.len());
        assert_eq!(
            2,
            first_router
                .paths
                .get(&RequestMethod::Get)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );

        assert_eq!(
            2,
            first_router
                .paths
                .get(&RequestMethod::Delete)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .paths
                .get(&RequestMethod::Post)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
                .paths
                .get(&RequestMethod::Head)
                .unwrap()
                .get(&PathBuf::parse("/").unwrap())
                .len()
        );
        assert_eq!(
            2,
            first_router
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
