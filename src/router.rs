use crate::path::{InvalidPathError, PathBuf};
use crate::request::{Request, RequestMethod};
use crate::responder::Response;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::ops::Deref;
use std::result::Result;

pub enum Flow {
    Stop,
    Continue,
}

#[macro_export]
macro_rules! route {
    ( | $req : ident, $res : ident | $body : expr ) => {
        Box::new(move |$req, $res| Box::pin(async move { $body }))
    };
}

pub type Closure =
    Box<dyn for<'a> Fn(&'a Request, &'a mut Response) -> BoxFuture<'a, ()> + Send + Sync>;
pub type ClosureFlow =
    Box<dyn for<'a> Fn(&'a Request, &'a mut Response) -> BoxFuture<'a, Flow> + Send + Sync>;

pub type RouterResult = Result<(), InvalidPathError>;

pub type Paths = HashMap<RequestMethod, HashMap<PathBuf, Closure>>;

pub enum UseMethods<'a> {
    Static(&'a str),
}

pub trait Route {
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn all(&mut self, path: &str, closure: Closure) -> RouterResult;
    fn add(&mut self, entity: ClosureFlow);
}

pub struct Router {
    pub paths: Paths,
}

impl Deref for Router {
    type Target = Paths;

    fn deref(&self) -> &Self::Target {
        &self.paths
    }
}

impl Router {
    pub fn new() -> Self {
        let mut hashmap: Paths = HashMap::new();
        for variants in RequestMethod::values().iter().cloned() {
            hashmap.insert(variants, HashMap::new());
        }
        Router { paths: hashmap }
    }
    pub fn append(&mut self, _router: Self) -> &mut Self {
        // TODO: Append each of the routes with respective keys
        self
    }
}

impl Route for Router {
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult {
        if let Some(paths) = self.paths.get_mut(&RequestMethod::Get) {
            paths.insert(PathBuf::parse(path)?, Box::new(closure));
        }
        Ok(())
    }
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        if let Some(paths) = self.paths.get_mut(&RequestMethod::Post) {
            paths.insert(PathBuf::parse(path)?, Box::new(closure));
        }
        Ok(())
    }
    fn all(&mut self, path: &str, closure: Closure) -> RouterResult {
        if let Some(paths) = self.paths.get_mut(&RequestMethod::Post) {
            paths.insert(PathBuf::parse(path)?, Box::new(closure));
        }
        Ok(())
    }
    fn add(&mut self, _entity: ClosureFlow) {}
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
