use octane::prelude::*;

mod common;

#[test]
pub fn basic_router() {
    let mut app = Octane::new();
    let mut router = Router::new();
    let string = "My Rust Program 1.0";
    router
        .get("/route1", route_next!(|req, res| res.send(string)))
        .unwrap();
    router
        .post("/route1", route_next!(|req, res| res.send(string)))
        .unwrap();
    router
        .put("/route1", route_next!(|req, res| res.send(string)))
        .unwrap();
    router
        .patch("/route1", route_next!(|req, res| res.send(string)))
        .unwrap();
    router
        .delete("/route1", route_next!(|req, res| res.send(string)))
        .unwrap();
    router
        .head("/route1", route_next!(|req, res| res.send(string)))
        .unwrap();
    app.get("/route2", route_next!(|req, res| res.send(string)))
        .unwrap();
    app.post("/route2", route_next!(|req, res| res.send(string)))
        .unwrap();
    app.put("/route2", route_next!(|req, res| res.send(string)))
        .unwrap();
    app.patch("/route2", route_next!(|req, res| res.send(string)))
        .unwrap();
    app.delete("/route2", route_next!(|req, res| res.send(string)))
        .unwrap();
    app.head("/route2", route_next!(|req, res| res.send(string)))
        .unwrap();
    app.with_router(router);
    common::run(app, || async {
        let client = reqwest::Client::new();
        assert_eq!(
            string,
            client
                .get(&path!("route1"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .get(&path!("route2"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .post(&path!("route1"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .post(&path!("route2"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .patch(&path!("route1"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .patch(&path!("route2"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .put(&path!("route1"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .put(&path!("route2"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .delete(&path!("route1"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .delete(&path!("route2"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
    });
}
