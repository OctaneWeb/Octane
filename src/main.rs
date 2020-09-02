use octane::main;
use octane::server::Octane;
use octane::{
    route,
    router::{Flow, Route},
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
    app.listen(8000).await?;
    Ok(())
}
