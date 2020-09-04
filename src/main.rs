use octane::server::Octane;
use octane::{
    route,
    router::{Closure, Flow, Route},
};
use std::error::Error;

#[octane::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Octane::new();
    app.get(
        "/",
        route!(|req, res| {
            res.send("Hello");
            Flow::Next
        }),
    )?;
    app.add(Octane::static_dir("templates/"))?;
    let logger: Closure = route!(|req, res| {
        println!("{:#?}", req);
        Flow::Next
    });
    app.add(logger)?;
    app.listen(8000).await
}
