use crate::router::Closure;

#[derive(Clone)]
pub struct Closures {
    pub closure: Closure,
    pub index: usize,
}

impl Closures {
    // shift all the closure exection here or in a function idk
    pub fn run() {}
}

#[macro_export]
macro_rules! inject_method {
    ( $instance: expr, $path: expr, $closure: expr, $method: expr ) => {
        use crate::constants::CLOSURES;
        use crate::middlewares::Closures;
        use crate::path::PathNode;
        CLOSURES
            .lock()
            .unwrap()
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
