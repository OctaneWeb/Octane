use crate::default;
use crate::error::InvalidPathError;
use crate::middlewares::Closures;
use crate::path::MatchedPath;
use crate::path::PathNode;
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

/// RouterResult is the type which the app.METHOD methods
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
    /// use octane::prelude::*;
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
    /// # Example
    ///
    /// ```no_run
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
    /// # Example
    ///
    /// ```no_run
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
    /// # Example
    ///
    /// ```no_run
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
    /// add() is like `app.use` in express, it runs on all the
    /// paths and all types of valid methods, the request comes
    /// on
    fn add(&mut self, entity: Closure) -> RouterResult;
}

/// The router structure defines the routes and stores them along with
/// their indexes. Router methods can be used with this struct also with
/// the main server structure.
pub struct Router {
    /// Holds a counter and increments on new additions of routes
    pub route_counter: usize,
    /// A vector of middleware closures
    pub middlewares: Vec<Closures>,
    /// The router paths which are to be exectued on requests
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
    /// append the routes stored in a custom Router to the self Router
    pub fn append(&mut self, router: Self) {
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

    /// Runs all the closures
    pub fn run(&self, parsed_request: Request<'_>, mut res: &mut Response) {
        let req = &parsed_request.request_line;

        let mut matches: Vec<Vec<MatchedPath<Closures>>> = Vec::new();
        if let Some(functions) = self.paths.get(&req.method) {
            let mut routes = functions.get(&req.path);
            routes.sort_by_key(|v| v.index);
            matches.push(routes);
        };
        // run RequestMethod::All regardless of the request method
        if let Some(functions) = self.paths.get(&RequestMethod::All) {
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
    }
}
/// The route macro makes it easy to pass anonymous
/// functions to app.METHODs.
///
/// # Example
///
/// ```no_run
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
#[macro_export]
macro_rules! route {
    ( | $req : ident, $res : ident | $body : expr ) => {{
        #[allow(unused_variables)]
        Box::new(move |$req, $res| $body)
    }};
}

/// Just like the `route!()` macro but return Flow::Next
/// Implicitly
///
/// # Example
///
/// ```no_run
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
        #[allow(unused_variables)]
        Box::new(move |$req, $res| {
            $body;
            Flow::Next
        })
    }};
}

/// Just like the `route!()` macro but return Flow::Stop
/// Implicitly
///
/// # Example
///
/// ```no_run
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
        #[allow(unused_variables)]
        Box::new(move |$req, $res| {
            $body;
            Flow::Stop
        })
    }};
}

default!(Router);

#[macro_use]
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
