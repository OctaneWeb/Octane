use crate::path::{InvalidPathError, PathBuf, PathNode};
use crate::request::{Request, RequestMethod};
use crate::responder::Response;
use crate::{inject_method, middlewares::Paths};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::ops::Deref;
use std::result::Result;

pub type Closure =
    Box<dyn for<'a> Fn(&'a Request, &'a mut Response) -> BoxFuture<'a, Flow> + Send + Sync>;

pub type RouterResult = Result<(), InvalidPathError>;

/// The flow enum works just like the next() callback
/// in express. The variant returns decides wheather
/// the exection should go to the next similar route
/// or not
///
/// # Example
///
/// ```rust,no_run
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(
///         |req, res| {
///             res.send("Hello, World");
///         },
///         Flow::Next
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

pub trait Route {
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn connect(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn options(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn head(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn all(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn add(&mut self, entity: Closure) -> RouterResult;
}

pub struct Router {
    pub paths: Paths,
    pub route_counter: usize,
}

impl Deref for Router {
    type Target = Paths;

    fn deref(&self) -> &Self::Target {
        &self.paths
    }
}

impl Router {
    pub fn new() -> Self {
        let mut paths: Paths = HashMap::new();
        for variants in RequestMethod::values().iter().cloned() {
            paths.insert(variants, PathNode::new());
        }
        Router {
            paths,
            route_counter: 0,
        }
    }
    pub fn append(&mut self, _router: Self) -> &mut Self {
        // TODO: Append each of the routes with respective keys
        self
    }
}

impl Route for Router {
    fn options(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, &RequestMethod::Options);
        Ok(())
    }
    fn connect(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, &RequestMethod::Connect);
        Ok(())
    }
    fn head(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, &RequestMethod::Head);
        Ok(())
    }
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, &RequestMethod::Put);
        Ok(())
    }
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, &RequestMethod::Get);
        Ok(())
    }

    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, &RequestMethod::Post);
        Ok(())
    }
    fn all(&mut self, _path: &str, _closure: Closure) -> RouterResult {
        // TODO: Multiple putmethod! declarations here
        Ok(())
    }
    fn add(&mut self, closure: Closure) -> RouterResult {
        inject_method!(self, "/*", closure, &RequestMethod::All);
        Ok(())
    }
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self, path, closure, &RequestMethod::All);
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
/// ```rust,no_run
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(
///         |req, res| {
///             res.send("Hello, World");
///         },
///         Flow::Next
///     ),
/// );
/// ```
#[macro_export]
macro_rules! route {
    // without flow enum, by default, move to next
    ( | $req : ident, $res : ident | $body : expr ) => {
        #[allow(unused_variables)]
        Box::new(move |$req, $res| {
            Box::pin(async move {
                $body;
                Flow::Next
            })
        })
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
