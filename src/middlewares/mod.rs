use crate::path::PathNode;
use crate::request::RequestMethod;
use crate::router::Closure;
use std::collections::HashMap;

pub type Paths = HashMap<RequestMethod, PathNode<Closures>>;

pub struct Closures {
    pub closure: Closure,
    pub index: usize,
}

#[macro_export]
macro_rules! inject_method {
    ( $instance: expr, $path: expr, $closure: expr, $method: expr ) => {
        use crate::middlewares::Closures;
        use crate::path::PathNode;
        $instance
            .paths
            .entry($method)
            .or_insert(PathNode::new())
            .insert(
                PathBuf::parse($path)?,
                Closures {
                    closure: $closure,
                    index: $instance.route_counter + 1,
                },
            );
        $instance.route_counter += 1;
    };
}
