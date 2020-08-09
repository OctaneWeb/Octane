use octane::config::{Config, OctaneConfig};
use octane::server::Octane;
use octane::{
    route,
    router::{Flow, Route, Router},
};

fn main() -> std::io::Result<()> {
    let mut app = Octane::new();
    let mut config = OctaneConfig::new();
    config
        .ssl
        .cert("templates/cert.pem")
        .key("templates/key.pem");
    let mut router = Router::new();
    let port = 80;
    config.add_static_dir("/", "templates");
    config.add_static_dir("/", "target");
    app.with_config(config);
    router
        .get(
            "/",
            route!(|req, res| {
                res.with_type("application/json")
                    .send(r#"{"server": "Octane"}"#);
                Flow::Stop
            }),
        )
        .unwrap();
    router
        .add(route!(|req, res| {
            println!("This is a middleware!");
            Flow::Next
        }))
        .unwrap();
    router
        .get(
            "/testing",
            route!(|req, res| {
                res.cookies.set("Name", "Value");
                Flow::Stop
            }),
        )
        .unwrap();
    app.with_router(router);
    app.get(
        "/to_home",
        route!(|req, res| {
            res.redirect("/").send("redirecting");
            Flow::Stop
        }),
    )
    .unwrap();
    app.listen(port)
}
