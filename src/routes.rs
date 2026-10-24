//! The routing engine shared by the tokio [`Router`](crate::router::Router)
//! and the WASIp3 router: routes are stored per method in a path trie and
//! matched together with the middleware in registration order. What a
//! handler is, sync closure or async function, is up to the router.

use crate::method::RequestMethod;
use crate::path::{InvalidPathError, PathBuf, PathNode};
use std::collections::HashMap;

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
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
/// The tokio [`Router`](crate::router::Router) and [`Octane`](crate::Octane)
/// take [`Closure`](crate::router::Closure)s, the WASIp3 router takes async
/// handlers: that's the [`Route::Handler`] type.
///
/// TODO: Include router example here
pub trait Route {
    /// What runs when a route matches
    type Handler;

    /// add_route() is a duplicate of add() but with a
    /// specified url on where it should run.
    /// It runs on the given path and on all types of requests
    fn add_route(&mut self, path: &str, closure: Self::Handler) -> RouterResult;
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
    fn head(&mut self, path: &str, closure: Self::Handler) -> RouterResult;
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
    fn post(&mut self, path: &str, closure: Self::Handler) -> RouterResult;
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
    fn get(&mut self, path: &str, closure: Self::Handler) -> RouterResult;
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
    fn put(&mut self, path: &str, closure: Self::Handler) -> RouterResult;
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
    fn delete(&mut self, path: &str, closure: Self::Handler) -> RouterResult;
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
    fn patch(&mut self, path: &str, closure: Self::Handler) -> RouterResult;
    /// add() is like `app.use` in express, it runs on all the
    /// paths and all types of valid methods, the request comes
    /// on
    fn add(&mut self, entity: Self::Handler) -> RouterResult;
}

/// A handler with the position it was registered at
pub(crate) struct Indexed<T> {
    pub(crate) data: T,
    pub(crate) index: usize,
}

/// A handler that matched a request, with the url variables of its path
pub(crate) struct Matched<'a, T> {
    pub(crate) data: &'a T,
    #[cfg(feature = "url_variables")]
    pub(crate) vars: HashMap<&'a str, &'a str>,
}

/// Routes per method and the middleware, numbered in registration order
pub(crate) struct Routes<T> {
    pub(crate) counter: usize,
    pub(crate) middlewares: Vec<Indexed<T>>,
    pub(crate) paths: HashMap<RequestMethod, PathNode<Indexed<T>>>,
}

impl<T> Routes<T> {
    pub(crate) fn new() -> Self {
        Self {
            counter: 0,
            middlewares: Vec::new(),
            paths: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, method: RequestMethod, path: &str, data: T) -> RouterResult {
        let path = PathBuf::parse(path)?;
        let index = self.next_index();
        self.paths
            .entry(method)
            .or_insert_with(PathNode::new)
            .insert(path, Indexed { data, index });
        Ok(())
    }

    pub(crate) fn add(&mut self, data: T) {
        let index = self.next_index();
        self.middlewares.push(Indexed { data, index });
    }

    /// Append `other`'s routes, which run after the ones already here
    pub(crate) fn append(&mut self, other: Self) {
        let offset = self.counter;
        for (method, paths) in other.paths.into_iter() {
            let updated_paths = paths.into_iter().map(|mut v| {
                v.data.index += offset;
                v
            });
            if let Some(x) = self.paths.get_mut(&method) {
                x.extend(updated_paths);
            } else {
                self.paths.insert(method, updated_paths.collect());
            }
        }
        self.middlewares
            .extend(other.middlewares.into_iter().map(|mut v| {
                v.index += offset;
                v
            }));
        self.counter += other.counter;
    }

    /// Everything that runs for `method` on `path`, in registration order:
    /// the routes for that method, the routes for every method and the
    /// middleware. Middleware have no url variables.
    pub(crate) fn matched<'a>(
        &'a self,
        method: RequestMethod,
        path: &'a PathBuf,
    ) -> Vec<Matched<'a, T>> {
        let routes = |method| {
            self.paths.get(&method).map_or_else(Vec::new, |node| {
                let mut routes = node.get(path);
                routes.sort_unstable_by_key(|route| route.index);
                routes
            })
        };
        let mut method_routes = routes(method).into_iter().peekable();
        // RequestMethod::All runs regardless of the request method
        let mut all_routes = routes(RequestMethod::All).into_iter().peekable();
        let mut middlewares = self.middlewares.iter().peekable();

        let mut matched = Vec::new();
        loop {
            let method_index = method_routes.peek().map_or(usize::MAX, |r| r.index);
            let all_index = all_routes.peek().map_or(usize::MAX, |r| r.index);
            let middleware_index = middlewares.peek().map_or(usize::MAX, |m| m.index);
            if method_index == usize::MAX
                && all_index == usize::MAX
                && middleware_index == usize::MAX
            {
                break;
            }
            // registration indexes are globally ordered, so merging the three
            // already sorted lists gives the order to run them in
            let route = if method_index <= all_index && method_index <= middleware_index {
                method_routes.next()
            } else if all_index <= middleware_index {
                all_routes.next()
            } else {
                let middleware = middlewares.next().unwrap();
                matched.push(Matched {
                    data: &middleware.data,
                    #[cfg(feature = "url_variables")]
                    vars: HashMap::new(),
                });
                continue;
            };
            let route = route.unwrap();
            matched.push(Matched {
                data: &route.data.data,
                #[cfg(feature = "url_variables")]
                vars: route.vars,
            });
        }
        matched
    }

    fn next_index(&mut self) -> usize {
        let index = self.counter;
        self.counter += 1;
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(routes: &Routes<usize>, method: RequestMethod, path: &str) -> Vec<usize> {
        let path = PathBuf::parse(path).unwrap();
        routes
            .matched(method, &path)
            .iter()
            .map(|m| *m.data)
            .collect()
    }

    #[test]
    fn merges_in_registration_order() {
        let mut routes = Routes::new();
        routes.add(0);
        routes.insert(RequestMethod::All, "/", 1).unwrap();
        routes.insert(RequestMethod::Get, "/", 2).unwrap();
        routes.insert(RequestMethod::Post, "/", 99).unwrap();
        routes.insert(RequestMethod::Get, "/other", 98).unwrap();
        routes.add(3);
        assert_eq!(order(&routes, RequestMethod::Get, "/"), [0, 1, 2, 3]);
        assert_eq!(order(&routes, RequestMethod::Put, "/"), [0, 1, 3]);
    }

    #[test]
    fn appended_routes_run_after() {
        let mut first = Routes::new();
        first.insert(RequestMethod::Get, "/", 0).unwrap();
        let mut second = Routes::new();
        second.add(1);
        second.insert(RequestMethod::Get, "/", 2).unwrap();
        first.append(second);
        first.add(3);
        assert_eq!(order(&first, RequestMethod::Get, "/"), [0, 1, 2, 3]);
    }

    #[test]
    #[cfg(feature = "url_variables")]
    fn routes_get_their_variables() {
        let mut routes = Routes::new();
        routes.add(0);
        routes.insert(RequestMethod::Get, "/users/:id", 1).unwrap();
        let path = PathBuf::parse("/users/42").unwrap();
        let matched = routes.matched(RequestMethod::Get, &path);
        assert!(matched[0].vars.is_empty());
        assert_eq!(matched[1].vars.get("id"), Some(&"42"));
    }
}
