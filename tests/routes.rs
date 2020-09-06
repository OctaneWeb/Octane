use octane::{
    path::PathBuf,
    request::RequestMethod,
    route,
    router::{Flow, Route, Router},
};

#[test]
pub fn router_test() {
    let mut router = Router::new();
    router.add(route!(|req, res| { Flow::Next })).unwrap();
    router.get("/", route!(|req, res| { Flow::Next })).unwrap();
    router
        .option("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    router.post("/", route!(|req, res| { Flow::Next })).unwrap();
    router.head("/", route!(|req, res| { Flow::Next })).unwrap();
    router.put("/", route!(|req, res| { Flow::Next })).unwrap();
    // middleware with path
    assert_eq!(5, router.paths.len());
    // middleware without paths
    assert_eq!(1, router.middlewares.len());
}

#[test]
pub fn router_append_test() {
    let mut first_router = Router::new();
    first_router.add(route!(|req, res| { Flow::Next })).unwrap();
    first_router
        .get("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    first_router
        .option("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    first_router
        .post("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    first_router
        .head("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    first_router
        .put("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    let mut second_router = Router::new();
    second_router
        .add(route!(|req, res| { Flow::Next }))
        .unwrap();
    second_router
        .get("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    second_router
        .option("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    second_router
        .post("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    second_router
        .head("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    second_router
        .put("/", route!(|req, res| { Flow::Next }))
        .unwrap();
    first_router.append(second_router);
    // middleware tied without path
    assert_eq!(2, first_router.middlewares.len());
    assert_eq!(
        2,
        first_router
            .paths
            .get(&RequestMethod::Get)
            .unwrap()
            .get(&PathBuf::parse("/").unwrap())
            .len()
    );

    assert_eq!(
        2,
        first_router
            .paths
            .get(&RequestMethod::Options)
            .unwrap()
            .get(&PathBuf::parse("/").unwrap())
            .len()
    );
    assert_eq!(
        2,
        first_router
            .paths
            .get(&RequestMethod::Post)
            .unwrap()
            .get(&PathBuf::parse("/").unwrap())
            .len()
    );
    assert_eq!(
        2,
        first_router
            .paths
            .get(&RequestMethod::Head)
            .unwrap()
            .get(&PathBuf::parse("/").unwrap())
            .len()
    );
    assert_eq!(
        2,
        first_router
            .paths
            .get(&RequestMethod::Put)
            .unwrap()
            .get(&PathBuf::parse("/").unwrap())
            .len()
    );
}
