use crate::inject_method;
use crate::middlewares::{Closures, Paths};
use crate::path::{InvalidPathError, PathBuf};
use crate::request::{MatchedRequest, RequestMethod};
use crate::responder::Response;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::ops::Deref;
use std::result::Result;

/// The Closure type is a type alias for the type
/// that the routes should return
pub type Closure =
    Box<dyn for<'a> Fn(&'a MatchedRequest, &'a mut Response) -> BoxFuture<'a, Flow> + Send + Sync>;

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
/// use octane::route::Route;
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
    /// It runs on the given path and all types of requests
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is OPTION
    fn options(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is HEAD
    fn head(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is POST
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is GET
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// Part of app.METHOD, runs on when the request is on the
    /// path given and the request method is PUT
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult;
    /// add() is like `app.use` in express, it runs on all the
    /// paths and all types of valid methods the request comes
    /// on
    fn add(&mut self, entity: Closure) -> RouterResult;
}

/// The router structure defines the routes and
/// stores them along with their indexes. Router
/// methods can be used with this struct also with
/// the main server structure
pub struct Router {
    pub paths: Paths,
    pub route_counter: usize,
    pub middlewares: Vec<Closures>,
}

impl Deref for Router {
    type Target = Paths;

    fn deref(&self) -> &Self::Target {
        &self.paths
    }
}

impl Router {
    pub fn new() -> Self {
        Router {
            paths: HashMap::new(),
            route_counter: 0,
            middlewares: Vec::new(),
        }
    }
    /// Append a router instance to itself, allows users
    /// to use a custom router independently from th   pe octane
    /// (main server) structure
    pub fn append(&mut self, router: Self) {
        for all_methods in router.iter() {}

        self.route_counter = router.route_counter;
        self.middlewares.extend(router.middlewares);
    }
}

impl Route for Router {
    fn options(&mut self, path: &str, closure: Closure) -> RouterResult {
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

impl Default for Router {
    fn default() -> Self {
        Self::new()
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
    // without flow enum, by default, move to next
    ( | $req : ident, $res : ident | $body : expr ) => {
        #[allow(unused_variables)]
        Box::new(move |$req, $res| Box::pin(async move { $body }))
    };
    // with flow enum
    ( | $req : ident, $res : ident | $body : expr, $ret : expr ) => {
        #[allow(unused_variables)]
        Box::new(move |$req, $res| {
            Box::pin(async move {
                $body;
                $ret
            })
        })
    };
}
