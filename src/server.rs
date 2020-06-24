use crate::config::OctaneConfig;
use crate::constants::*;
use crate::error::Error;
use crate::path::PathBuf;
use crate::request::{
    parse_without_body, Headers, HttpVersion, KeepAlive, Request, RequestLine, RequestMethod,
};
use crate::responder::Response;
use crate::router::{Closure, ClosureFlow, Flow, Route, Router, RouterResult};
use crate::util::find_in_slice;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::str;
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
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult {
        if let Some(paths) = self.router.paths.get_mut(&RequestMethod::Get) {
            paths.insert(PathBuf::parse(path)?, Box::new(closure));
        }
        Ok(())
    }
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        if let Some(paths) = self.router.paths.get_mut(&RequestMethod::Post) {
            paths.insert(PathBuf::parse(path)?, Box::new(closure));
        }
        Ok(())
    }
    fn all(&mut self, path: &str, closure: Closure) -> RouterResult {
        if let Some(paths) = self.router.paths.get_mut(&RequestMethod::Post) {
            paths.insert(PathBuf::parse(path)?, Box::new(closure));
        }
        Ok(())
    }

    fn add(&mut self, _entity: ClosureFlow) -> RouterResult {
        Ok(())
    }
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult {
        Ok(())
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
        let mut listener =
            TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port)).await?;
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
    pub fn static_dir(&mut self, location: &'static str) -> () {
        // TODO: Implement static dir middleware
    }

    async fn catch_request(
        mut stream_async: TcpStream,
        server: Arc<Octane>,
    ) -> std::io::Result<()> {
        let mut data = Vec::<u8>::new();
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        let body: &[u8];
        let request_line: RequestLine;
        let headers: Headers;
        let body_remainder: &[u8];
        loop {
            let read = stream_async.read(&mut buf).await?;
            if read == 0 {
                Error::err(StatusCode::BadRequest)
                    .send(stream_async)
                    .await?;
                return Ok(());
            }
            let cur = &buf[..read];
            data.extend_from_slice(cur);
            if let Some(i) = find_in_slice(&data[..], b"\r\n\r\n") {
                let first = &data[..i];
                body_remainder = &data[i + 4..];
                if let Ok(decoded) = str::from_utf8(first) {
                    if let Some((rl, heads)) = parse_without_body(decoded) {
                        request_line = rl;
                        headers = heads;
                        break;
                    } else {
                        Error::err(StatusCode::BadRequest)
                            .send(stream_async)
                            .await?;
                        return Ok(());
                    }
                } else {
                    Error::err(StatusCode::BadRequest)
                        .send(stream_async)
                        .await?;
                    return Ok(());
                }
            }
        }
        let body_len = headers
            .get("content-length")
            .map(|s| s.parse().unwrap_or(0))
            .unwrap_or(0);
        let mut body_vec: Vec<u8>;
        if body_len > 0 {
            if body_remainder.len() < body_len {
                let mut temp: Vec<u8> = vec![0; body_len - body_remainder.len()];
                stream_async.read_exact(&mut temp[..]).await?;
                body_vec = Vec::with_capacity(body_len);
                body_vec.extend_from_slice(body_remainder);
                body_vec.extend_from_slice(&temp[..]);
                body = &body_vec[..];
            } else {
                body = body_remainder;
            }
        } else {
            body = &[];
        }
        if let Some(parsed_request) = Request::parse(request_line, headers, body) {
            if let Some(connection_type) = parsed_request.headers.get("connection") {
                if connection_type.to_lowercase() == "keep-alive" {
                    if parsed_request.request_line.version == HttpVersion::Http10 {
                        if let Some(keep_alive_header) = parsed_request.headers.get("keep-alive") {
                            let header_details = KeepAlive::parse(keep_alive_header);
                            stream_async.set_keepalive(Some(Duration::from_secs(
                                header_details.timeout.unwrap_or(0),
                            )))?;
                        }
                    } else if parsed_request.request_line.version == HttpVersion::Http11 {
                        stream_async.set_keepalive(server.settings.keep_alive)?;
                    }
                }
            }
            let mut res = Response::new(b"");
            if let Some(functions) = (*server.router).get(&parsed_request.request_line.method) {
                if let Some(closure) = functions.get(&parsed_request.request_line.path) {
                    closure(&parsed_request, &mut res).await;
                };

                Self::send_data(res.get_data(), stream_async).await?;
            } else {
                Error::err(StatusCode::NotImplemented)
                    .send(stream_async)
                    .await?;
            }
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
