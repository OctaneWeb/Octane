use octane::prelude::*;
use std::error::Error;
#[octane::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Octane::new();
    let mut router = Router::new();
    app.ssl(8080)
        .key(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/key.pem"))
        .cert(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/cert.pem"));
    router.get(
        "/test",
        route!(|req, res| {
            res.charset("utf-8").send("Hello");
            Flow::Next
        }),
    )?;
    router.get("/", route_next!(|req, res| res.send("Hello")))?;
    app.with_router(router);
    app.add(Octane::static_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/"
    )))?;
    app.listen(8000).await?;
    Ok(())
}
