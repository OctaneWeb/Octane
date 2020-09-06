use octane::prelude::*;
use std::error::Error;

#[octane::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Octane::new();
    let mut router = Router::new();
    router.get(
        "/",
        route!(|req, res| {
            res.send("Hello");
            Flow::Next
        }),
    )?;
    router.get(
        "/test",
        route!(|req, res| {
            res.send("Hello");
            Flow::Next
        }),
    )?;
    app.with_router(router);
    app.add(Octane::static_dir("templates/"))?;
    app.listen(8000).await
}
