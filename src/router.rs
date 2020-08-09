use crate::inject_method;
use crate::middlewares::{Closures, Paths};
use crate::path::{InvalidPathError, PathBuf};
use crate::request::{MatchedRequest, RequestMethod};
use crate::responder::Response;
use crate::{default, deref};
use core::future::Future;
use core::pin::Pin;
use std::collections::HashMap;
use std::result::Result;

/// The Closure type is a type alias for the type
/// that the routes should return
pub type Closure = Box<
    dyn for<'a> Fn(
            &'a MatchedRequest,
            &'a mut Response,
        ) -> Pin<Box<dyn Future<Output = Flow> + 'a + Send>>
        + Send
        + Sync,
>;
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
    pub paths: Paths,
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
            paths: HashMap::new(),
            route_counter: 0,
            middlewares: Vec::new(),
        }
    }

    /// Append a router instance to `self`, allows users
    /// to use a custom router independently from the octane
    /// (main server) structure
    pub fn append(&mut self, router: Self) {
        let self_count = self.route_counter;
        let other_count = router.route_counter;
        for (methods, paths) in router.paths.into_iter() {
            if let Some(x) = self.paths.get_mut(&methods) {
                x.extend(paths.into_iter().map(|mut v| {
                    v.data.index += self_count;
                    v
                }));
            } else {
                self.paths.insert(methods, paths);
            }
        }

        self.middlewares
            .extend(router.middlewares.into_iter().map(|mut v| {
                v.index += self_count;
                v
            }));
        self.route_counter += other_count;
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
    ( | $req : ident, $res : ident | $body : expr ) => {
        #[allow(unused_variables)]
        Box::new(move |$req, $res| Box::pin(async move { $body }))
    };
}

deref!(Router, Paths, paths);
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
