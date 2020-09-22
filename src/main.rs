use octane::prelude::*;
use std::error::Error;

#[octane::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Octane::new();

    let port = 8000;
    app.listen(8000, || {
        println!("{:?}", "Server has started running!");
    })
    .await?;
    Ok(())
}
