use octane::config::Config;
use octane::server::Octane;
use octane::{
    route,
    router::{Flow, Route},
};

fn main() {
    let mut app = Octane::new();
    app.ssl(8000)
        .key("templates/key.pem")
        .cert("templates/cert.pem");
    app.set_keepalive(std::time::Duration::new(7, 0));
    app.get(
        "/to_home",
        route!(|req, res| {
            res.redirect("/").send("redirecting");
            Flow::Stop
        }),
    )
    .unwrap();
    app.get(
        "/",
        route!(|req, res| {
            res.send_file("templates/cert.pem".parse().unwrap()).await.unwrap();
            Flow::Stop
        }),
    )
    .unwrap();
    // app.get(
    //     "/test",
    //     route!(|req, res| {
    //         //res.send_file(std::path::PathBuf::from("templates/favicon.ico"))
    //             .await
    //             .expect("oof");
    //         Flow::Next
    //     }),
    // )
    // .unwrap();
    app.listen(8080).expect("Cannot establish connection");
}
