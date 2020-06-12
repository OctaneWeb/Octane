use crate::request::Request;
use crate::responder::Response;
use futures::future::BoxFuture;

pub enum Flow {
    Stop,
    Continue,
}

#[macro_export]
macro_rules! Route {
    ( $req : ident, $res : ident => $body : expr ) => {
        Box::new(move |$req, $res| Box::pin(async move { $body }))
    };
}

pub type Closure =
    Box<dyn for<'a> Fn(&'a Request, &'a mut Response) -> BoxFuture<'a, ()> + Send + Sync>;
pub type ClosureFlow =
    Box<dyn for<'a> Fn(&'a Request, &'a mut Response) -> BoxFuture<'a, Flow> + Send + Sync>;

pub enum UseMethods<'a> {
    Static(&'a str),
}

pub trait Route {
    fn get(&mut self, path: &str, closure: Closure);
    fn post(&mut self, path: &str, closure: Closure);
    fn all(&mut self, path: &str, closure: Closure);
    fn add(&mut self, entity: ClosureFlow);
}

pub struct Router {
    pub get_paths: Vec<(String, Box<Closure>)>,
    pub post_paths: Vec<(String, Box<Closure>)>,
    pub all_paths: Vec<(String, Box<Closure>)>,
    pub add_paths: Vec<(Option<String>, Box<ClosureFlow>)>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            add_paths: Vec::new(),
            get_paths: Vec::new(),
            post_paths: Vec::new(),
            all_paths: Vec::new(),
        }
    }
    pub fn append(&mut self, mut router: Self) -> &mut Self {
        router.get_paths.append(&mut self.get_paths);
        router.post_paths.append(&mut self.post_paths);
        router.all_paths.append(&mut self.all_paths);
        router.add_paths.append(&mut self.add_paths);
        self
    }
}

impl Route for Router {
    fn get(&mut self, path: &str, closure: Closure) {
        self.get_paths.push((path.to_string(), Box::new(closure)));
    }
    fn post(&mut self, path: &str, closure: Closure) {
        self.post_paths.push((path.to_string(), Box::new(closure)));
    }
    fn all(&mut self, path: &str, closure: Closure) {
        self.all_paths.push((path.to_string(), Box::new(closure)));
    }
    fn add(&mut self, entity: ClosureFlow) {
        self.add_paths.push((None, Box::new(entity)))
    }
}
