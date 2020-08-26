use octane::config::Config;
use octane::responder::StatusCode;
use octane::server::Octane;
use octane::{
    route,
    router::{Flow, Route},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Octane::new();
    app.ssl(8001)
        .key("templates/key.pem")
        .cert("templates/cert.pem");
    //  app.set_404_file("templates/eerror.html");
    app.get(
        "/to_home",
        route!(|req, res| {
            res.redirect("/").send("redirecting");
            Flow::Stop
        }),
    )?;
    app.get(
        "/",
        route!(|req, res| {
            res.send_file("templates/test.html").await.unwrap();
            Flow::Stop
        }),
    )?;
    app.get(
        "/favicon.ico",
        route!(|req, res| {
            res.send_file("templates/favicon.ico").await.expect("oof");
            Flow::Next
        }),
    )?;
    app.add(Octane::static_dir("templates/"))?;
    app.listen(8080)
}
