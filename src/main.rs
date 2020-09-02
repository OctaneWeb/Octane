use octane::server::Octane;
use octane::{
    route,
    router::{Flow, Route},
};
use octane_macros as macros;
use std::error::Error;

#[macros::main]
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
