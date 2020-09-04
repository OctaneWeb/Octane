use crate::constants::closures_lock;
use crate::default;
use crate::error::InvalidPathError;
use crate::inject_method;
use crate::middlewares::Closures;
use crate::path::PathNode;
use crate::path::{MatchedPath, PathBuf};
use crate::request::{MatchedRequest, Request, RequestMethod};
use crate::responder::Response;
use std::collections::HashMap;
use std::result::Result;

/// The type of HashMap where we will be storing the all the closures
pub type Paths = HashMap<RequestMethod, PathNode<Closures>>;
/// The Closure type is a type alias for the type
/// that the routes should return
pub type Closure =
    Box<dyn for<'a> Fn(&'a MatchedRequest, &'a mut Response) -> Flow + Send + Send + Sync>;
/// RouterResult is the type with the app.METHOD methods
/// return
pub type RouterResult = Result<(), InvalidPathError>;
/// The flow enum works just like the next() callback
/// in express. The variant returns decides wheather
/// the exection should go to the next similar route
/// or not
///
/// # Example
///
/// ```no_run
/// use octane::server::Octane;
/// use octane::{route, router::{Flow, Route}};
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
    Stop,
    Next,
}

impl Flow {
    /// Returns true if the variant is `Flow::Next`
    pub fn should_continue(self) -> bool {
        if let Self::Next = self {
            true
        } else {
            false
        }
    }
}
/// The route trait adds the app.METHOD behaviour
/// to the router/Octane structures along with some
/// handful methods that can be used accordingly.
///
/// # Example
///
/// ```no_run
/// use octane::{route, router::{Flow, Route}};
/// use octane::server::Octane;
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
/// octane struct, you can add them on a router instance
/// which can be appended to your main server (octane)
/// struct. It's called Router
/// TODO: Include router example here
pub trait Route {
    /// add_route() is a dupliate of add() but with a
    /// specified url on where it should run.
    /// It runs on the given path and on all types of requests
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is OPTION
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
    ///
    /// let mut app = Octane::new();
    /// app.option(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("Hello, World");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    fn option(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is HEAD
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// add() is like `app.use` in express, it runs on all the
    /// paths and all types of valid methods, the request comes
    /// on
    fn add(&mut self, entity: Closure) -> RouterResult;
}

/// The router structure defines the routes and
/// stores them along with their indexes. Router
/// methods can be used with this struct also with
/// the main server structure
pub struct Router {
    pub route_counter: usize,
    pub middlewares: Vec<Closures>,
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
        }
    }

    /// Runs all the closures
    pub async fn run(&self, parsed_request: Request<'_>, mut res: &mut Response) {
        let req = &parsed_request.request_line;

        closures_lock(|map| {
            let mut matches: Vec<Vec<MatchedPath<Closures>>> = Vec::new();
            if let Some(functions) = map.get(&req.method) {
                let mut routes = functions.get(&req.path);
                routes.sort_by_key(|v| v.index);
                matches.push(routes);
            };
            // run RequestMethod::All regardless of the request method
            if let Some(functions) = map.get(&RequestMethod::All) {
                let mut routes = functions.get(&req.path);
                routes.sort_by_key(|v| v.index);
                matches.push(routes);
            }

            matches.push(
                self.middlewares
                    .iter()
                    .map(|c| MatchedPath {
                        data: c,
                        #[cfg(feature = "url_variables")]
                        vars: HashMap::new(),
                    })
                    .collect(),
            );

            let mut indices = vec![0_usize; matches.len()];
            let total: usize = matches.iter().map(Vec::len).sum();
            #[cfg(feature = "url_variables")]
            let mut matched = MatchedRequest {
                request: parsed_request.clone(),
                vars: HashMap::new(),
            };
            #[cfg(not(feature = "url_variables"))]
            let matched = MatchedRequest {
                request: parsed_request,
            };
            for _ in 0..total {
                let mut minind = 0;
                let mut minval = usize::MAX;
                for (n, (v, i)) in matches.iter().zip(indices.iter()).enumerate() {
                    if *i < v.len() && v[*i].index < minval {
                        minval = v[*i].index;
                        minind = n;
                    }
                }
                #[cfg(feature = "url_variables")]
                {
                    matched.vars = matches[minind][indices[minind]].vars.clone();
                }
                let flow = (matches[minind][indices[minind]].closure)(&matched, &mut res);
                indices[minind] += 1;
                if !flow.should_continue() {
                    break;
                }
            }
        });
    }
}
/// The route macro makes it easy to pass anonymous
/// functions to app.METHODs.
///
/// # Example
///
/// ```no_run
/// use octane::{route, router::{Flow, Route}};
/// use octane::server::Octane;
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
#[macro_export]
macro_rules! route {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        #[allow(unused_variables)]
        Box::new(move |$req, $res| $body)
    }};
}

default!(Router);

impl Route for Router {
    fn option(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Options);
        Ok(())
    }
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

    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, RequestMethod::Post);
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
