use octane::prelude::*;
use std::error::Error;

#[octane::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Octane::new();
    app.ssl(8000)
        .cert(path!("/templates/cert.pem"))
        .key(path!("/templates/key.pem"));
    app.get("/", route_next!(|req, res| res.send("helllo there")))?;
    app.listen(8080, || {}).await?;
    Ok(())
}
