use octane::prelude::*;
use reqwest::header::USER_AGENT;

mod common;

pub fn basic_header_value_retri() {
    let mut app = Octane::new();
    let string = "My Rust Program 1.0";
    app.get(
        "/",
        route_next!(|req, res| {
            assert_eq!(string, req.headers.get("user_agent").unwrap());
        }),
    )
    .unwrap();
    common::run(app, || async {
        let client = reqwest::Client::new();
        client
            .get(&path!(""))
            .header(USER_AGENT, string)
            .send()
            .await
            .unwrap();
    });
}
