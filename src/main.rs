use octane::config::{Config, OctaneConfig};
use octane::server::Octane;
use octane::{
    route,
    router::{Flow, Route},
};

#[tokio::main]
async fn main() {
    let mut app = Octane::new();
    let mut config = OctaneConfig::new();
    config
        .ssl
        .key("templates/key.pem")
        .cert("templates/cert.pem");
    config.add_static_dir("/", "templates");
    app.with_config(config);

    app.get(
        "/",
        route!(
            |req, res| {
                res.with_type("application/json")
                    .send(r#"{"server": "Octane"}"#);
            },
            Flow::Next
        ),
    )
    .unwrap();
    app.listen(8080).await.expect("Cannot establish connection");
}
