use crate::config::OctaneConfig;
use crate::constants::*;
use crate::error::Error;
use crate::request::{HttpVersion, KeepAlive, Request};
use crate::responder::Response;
use crate::router::{Closure, ClosureFlow, Flow, Route, Router};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::copy;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::stream::StreamExt;

pub struct Octane {
    pub settings: OctaneConfig,
    router: Router,
}

impl Route for Octane {
    fn get(&mut self, path: &str, closure: Closure) {
        self.router
            .get_paths
            .push((path.to_string(), Box::new(closure)));
    }
    fn post(&mut self, path: &str, closure: Closure) {
        self.router
            .post_paths
            .push((path.to_string(), Box::new(closure)));
    }
    fn all(&mut self, path: &str, closure: Closure) {
        self.router
            .all_paths
            .push((path.to_string(), Box::new(closure)));
    }
    fn add(&mut self, entity: ClosureFlow) {
        self.router.add_paths.push((None, Box::new(entity)))
    }
}

impl Octane {
    pub fn new() -> Self {
        Octane {
            settings: OctaneConfig::new(),
            router: Router::new(),
        }
    }

    pub fn use_router(&mut self, _router: Router) {
        // FIXME: this function
        // self.router = router.append(self.router);
    }

    pub fn with_config(&mut self, config: OctaneConfig) -> &mut Self {
        self.settings = config;
        self
    }
    pub async fn listen(self, port: u16) -> std::io::Result<()> {
        let mut listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
            .await
            .unwrap();
        let server = Arc::new(self);
        while let Some(stream) = StreamExt::next(&mut listener).await {
            let server_clone = Arc::clone(&server);
            tokio::spawn(async move {
                match stream {
                    Ok(value) => {
                        let _res = Self::catch_request(value, server_clone).await;
                    }
                    Err(_e) => (),
                };
            });
        }
        Ok(())
    }
    pub fn static_dir(location: &'static str) -> ClosureFlow {
        Box::new(move |_req, res| {
            res.set_static_dir(location.to_owned());
            Box::pin(async move { Flow::Continue })
        })
    }

    async fn catch_request(
        mut stream_async: TcpStream,
        server: Arc<Octane>,
    ) -> std::io::Result<()> {
        let mut data = Vec::<u8>::new();
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        loop {
            let read = stream_async.read(&mut buf).await?;
            data.extend_from_slice(&buf);
            if read < BUF_SIZE {
                break;
            }
        }
        if let Some(parsed_request) = Request::parse(&data[..]) {
            if let Some(connection_type) = parsed_request.headers.get("connection") {
                if connection_type.to_lowercase() == "keep-alive" {
                    if parsed_request.version == HttpVersion::Http10 {
                        if let Some(keep_alive_header) = parsed_request.headers.get("keep-alive") {
                            let header_details = KeepAlive::parse(keep_alive_header);
                            stream_async.set_keepalive(Some(Duration::from_secs(
                                header_details.timeout.unwrap_or(0),
                            )))?;
                        }
                    } else if parsed_request.version == HttpVersion::Http11 {
                        stream_async.set_keepalive(server.settings.keep_alive)?;
                    }
                }
            }
            let mut res = Response::new(b"");
            // server get paths
            for get_paths in &server.router.get_paths {
                get_paths.1(&parsed_request, &mut res).await;
            }

            Self::send_data(res.get_data(), stream_async).await?;
        } else {
            Error::err(StatusCode::BadRequest)
                .send(stream_async)
                .await?;
        }
        Ok(())
    }
    async fn send_data(response: Vec<u8>, mut stream_async: TcpStream) -> std::io::Result<()> {
        copy(&mut &response[..], &mut stream_async).await?;
        Ok(())
    }
}

impl Default for Octane {
    fn default() -> Self {
        Self::new()
    }
}
