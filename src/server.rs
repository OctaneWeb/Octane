use crate::config::{Config, OctaneConfig, Ssl};
use crate::constants::with_lock;
use crate::constants::*;
use crate::default;
use crate::error::Error;
use crate::inject_method;
use crate::middlewares::Closures;
use crate::path::OwnedMatchedPath;
use crate::path::PathBuf;

use crate::request::{
    parse_without_body, Headers, HttpVersion, KeepAlive, MatchedRequest, Request, RequestLine,
    RequestMethod,
};
use crate::responder::{BoxReader, Response};
use crate::router::{Closure, Route, Router, RouterResult};
use crate::tls::AsMutStream;
use crate::util::find_in_slice;
#[cfg(feature = "url_variables")]
use std::collections::HashMap;
use std::io::Result;
use std::marker::Unpin;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::PathBuf as StdPathBuf;
use std::str;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{copy, AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::runtime::Builder;
use tokio::stream::StreamExt;

#[macro_use]
macro_rules! declare_error {
    ($stream : expr, $error_type : expr, $settings : expr) => {
        Error::err($error_type, $settings).send($stream).await?;
        return Ok(());
    };
}
/// The octane server
///
/// # Example
///
/// ```no_run
/// use octane::server::Octane;
/// use octane::{route, router::{Flow, Route}};
///
/// fn main() {
///     let mut app = Octane::new();
///     app.get(
///         "/",
///         route!(
///             |req, res| {
///                 res.send("Hello, World");
///                 Flow::Stop
///             }
///         ),
///     );
///
///     app.listen(8080).expect("Cannot establish connection");
/// }
/// ```
pub struct Octane {
    pub settings: OctaneConfig,
    pub router: Router,
}

impl Octane {
    /// Creates a new server instance
    pub fn new() -> Self {
        Octane {
            settings: OctaneConfig::new(),
            router: Router::new(),
        }
    }
    /// **Appends** the router routes to the routes that
    /// the server instance holds, this allows you to
    /// independently add routes to a route Router structure
    /// and then use it with the server struct
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::{route, router::{Flow, Route, Router}};
    ///
    /// let mut app = Octane::new();
    /// let mut router = Router::new();
    /// router.get("/", route!(|req, res| { res.send("It's a get request!!"); Flow::Stop }));
    /// router.post("/", route!(|req, res| { res.send("It's a post request!!"); Flow::Stop }));
    /// app.with_router(router);
    /// ```
    ///
    /// Note that it appends, meaning if you have 3 routes in
    /// Router struct and 3 routes in the Octane struct,
    /// you'll have total 3 + 3 routes in the Octane struct.
    pub fn with_router(&mut self, router: Router) {
        self.router.append(router);
    }
    /// Appends the config of the Octane struct with a custom
    /// generated one. The Octane struct contains an OctaneConfig
    /// instance by default
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::config::{OctaneConfig, Config};
    /// use octane::{route, router::{Flow, Route}};
    ///
    /// let mut app = Octane::new();
    /// let mut config = OctaneConfig::new();
    /// config.ssl.key("key.pem").cert("cert.pem"); // we supply some ssl certs and key in the config
    /// app.add_static_dir("/", "templates");
    /// app.with_config(config);
    /// ```
    ///
    /// **Note**: While it replaces properties that must be unique
    /// i.e which can only have one value at a time, so for
    /// static_dirs, it appends the locations defined in config
    /// with the settings that Octane struct already has
    pub fn with_config(&mut self, config: OctaneConfig) {
        self.settings.append(config);
    }
    /// Start listening on the port specified, the listen
    /// function also starts the Ssl server if the features
    /// are enabled and the key/certs are provided
    ///
    /// # Example
    /// ```no_run
    /// use octane::server::Octane;
    ///
    /// fn main() {
    ///     let mut app = Octane::new();
    ///     app.listen(80).expect("Cannot establish connection");
    /// }
    /// ```
    pub fn listen(self, port: u16) -> Result<()> {
        let ssl_enable = false;
        let mut builder = Builder::new();
        builder.threaded_scheduler().enable_io();
        if let Some(threads) = &self.settings.worker_threads {
            builder.core_threads(*threads);
        }
        let mut runtime = builder.build()?;
        runtime.block_on(async {
            let mut listener =
                TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port)).await?;
            let server = Arc::new(self);
            #[cfg(all(feature = "openSSL", feature = "rustls"))]
            compile_error!("openSSL and rustls are both enabled, you may want to one of those");
            #[cfg(any(feature = "openSSL", feature = "rustls"))]
            {
                let mut ssl_listener = TcpListener::bind(SocketAddrV4::new(
                    Ipv4Addr::new(0, 0, 0, 0),
                    server.settings.ssl.port,
                ))
                .await?;
                server.settings.ssl.validate();
                ssl_enable = true;
                #[cfg(feature = "openSSL")]
                let acceptor = tls::openssl::acceptor(&server.settings)?;
                #[cfg(feature = "rustls")]
                let acceptor = tls::rustls::acceptor(&server.settings)?;
                let server_clone = Arc::clone(&server);
                tokio::spawn(async move {
                    while let Some(stream) = StreamExt::next(&mut ssl_listener).await {
                        let server_clone = Arc::clone(&server_clone);
                        let acceptor = acceptor.clone();
                        tokio::spawn(async move {
                            match stream {
                                Ok(value) => {
                                    #[cfg(feature = "rustls")]
                                    let stream = acceptor.accept(value).await;
                                    #[cfg(feature = "openSSL")]
                                    let stream = tokio_openssl::accept(&acceptor, value).await;
                                    match stream {
                                        Ok(stream_ssl) => {
                                            let _res =
                                                Self::catch_request(stream_ssl, server_clone, true)
                                                    .await;
                                        }
                                        Err(e) => println!("{:#?}", e),
                                    }
                                }
                                Err(e) => println!("{:#?}", e),
                            };
                        });
                    }
                });
            }

            // http
            while let Some(stream) = StreamExt::next(&mut listener).await {
                let server_clone = Arc::clone(&server);
                tokio::spawn(async move {
                    match stream {
                        Ok(value) => {
                            let _res = Self::catch_request(value, server_clone, ssl_enable).await;
                        }
                        Err(_e) => (),
                    };
                });
            }
            Ok(())
        })
    }
    async fn catch_request<S>(mut stream_async: S, server: Arc<Octane>, has_ssl: bool) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin + AsMutStream,
    {
        let settings = &server.settings;
        let mut data = Vec::<u8>::new();
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        let body: &[u8];
        let request_line: RequestLine;
        let headers: Headers;
        let body_remainder: &[u8];
        loop {
            let read = stream_async.read(&mut buf).await?;
            if read == 0 {
                declare_error!(stream_async, StatusCode::BadRequest, settings);
            }
            let cur = &buf[..read];
            data.extend_from_slice(cur);
            if let Some(i) = find_in_slice(&data[..], b"\r\n\r\n") {
                let first = &data[..i];
                body_remainder = &data[i + 4..];
                if let Ok(Some((rl, heads))) = str::from_utf8(first).map(parse_without_body) {
                    request_line = rl;
                    headers = heads;
                    break;
                } else {
                    declare_error!(stream_async, StatusCode::BadRequest, settings);
                }
            }
        }
        let body_len = headers
            .get("content-length")
            .and_then(|s| s.parse().ok())
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

                            stream_async
                                .stream_mut()
                                .set_keepalive(Some(Duration::from_secs(
                                    header_details.timeout.unwrap_or(0),
                                )))?;
                        }
                    } else if parsed_request.request_line.version == HttpVersion::Http11 {
                        stream_async
                            .stream_mut()
                            .set_keepalive(server.settings.keep_alive)?;
                    }
                }
            }

            let mut res = Response::new_from_slice(b"");
            let req = &parsed_request.request_line;
            if req.method.is_some() {
                let mut matches: Vec<Vec<OwnedMatchedPath<Closures>>> = Vec::new();

                with_lock(|map| {
                    if let Some(functions) = map.get(&req.method) {
                        let mut routes = functions.get(&req.path);
                        routes.sort_by_key(|v| v.index);
                        let owned_routes = routes
                            .clone()
                            .iter()
                            .map(|e| crate::path::matched_path_to_owned(e))
                            .collect();
                        matches.push(owned_routes);
                    };
                    // run RequestMethod::All regardless of the request method
                    if let Some(functions) = map.get(&RequestMethod::All) {
                        let mut routes = functions.get(&req.path);
                        routes.sort_by_key(|v| v.index);
                        let owned_routes = routes
                            .clone()
                            .iter()
                            .map(|e| crate::path::matched_path_to_owned(e))
                            .collect();

                        matches.push(owned_routes);
                    }
                });
                matches.push(
                    server
                        .router
                        .middlewares
                        .clone()
                        .into_iter()
                        .map(|c| OwnedMatchedPath {
                            data: c,
                            #[cfg(feature = "url_variables")]
                            vars: HashMap::new(),
                        })
                        .collect(),
                );

                let mut indices = vec![0_usize; matches.len()];
                let total: usize = matches.iter().map(Vec::len).sum();
                #[cfg(feature = "url_variables")]
                let mut matched = MatchedRequest {
                    request: parsed_request.clone(),
                    vars: HashMap::new(),
                };
                #[cfg(not(feature = "url_variables"))]
                let matched = MatchedRequest {
                    request: &parsed_request,
                };
                for _ in 0..total {
                    let mut minind = 0;
                    let mut minval = usize::MAX;
                    for (n, (v, i)) in matches.iter().zip(indices.iter()).enumerate() {
                        if *i < v.len() && v[*i].index < minval {
                            minval = v[*i].index;
                            minind = n;
                        }
                    }
                    #[cfg(feature = "url_variables")]
                    {
                        matched.vars = matches[minind][indices[minind]].vars.clone();
                    }
                    let flow = (matches[minind][indices[minind]].closure)(&matched, &mut res).await;
                    indices[minind] += 1;
                    if !flow.should_continue() {
                        break;
                    }
                }

                // Run static file middleware
                if res.content_len.unwrap_or(0) == 0 {
                    let mut parent_path = req.path.clone();
                    parent_path.chunks.pop();
                    for loc in server.settings.static_dir.iter() {
                        for dirs in loc.1.into_iter() {
                            if req.method == RequestMethod::Get {
                                let mut dir_final = dirs.clone();
                                dir_final.extend(parent_path.clone().chunks);
                                res.send_file(dir_final).await.ok(); // ignore result
                            }
                        }
                    }
                }
                if res.content_len.unwrap_or(0) == 0 {
                    declare_error!(stream_async, StatusCode::NotFound, settings);
                }

                if let Some(x) = parsed_request.headers.get("upgrade-insecure-requests") {
                    match x.as_str() {
                        "1" => {
                            if has_ssl {
                                res.redirect(
                                    format!(
                                        "https://{:?}/{:?}",
                                        parsed_request.headers.get("Host"),
                                        parsed_request.request_line.path
                                    )
                                    .as_str(),
                                );
                            } else {
                                res.set("Vary", "Upgrade-Insecure-Requests");
                            }
                        }
                        _ => (),
                    }
                }
                Self::send_data(res.get_data(), stream_async).await?;
            } else {
                declare_error!(stream_async, StatusCode::NotImplemented, settings);
            }
        } else {
            declare_error!(stream_async, StatusCode::BadRequest, settings);
        }
        Ok(())
    }
    pub async fn send_data<S>(mut response: (String, BoxReader), mut stream_async: S) -> Result<()>
    where
        S: AsyncWrite + Unpin,
    {
        stream_async.write_all(response.0.as_bytes()).await?;
        copy(&mut response.1, &mut stream_async).await?;
        Ok(())
    }
}
default!(Octane);

impl Route for Octane {
    fn option(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.option(path, closure)
    }
    fn head(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.head(path, closure)
    }
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.put(path, closure)
    }
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.get(path, closure)
    }
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.post(path, closure)
    }
    fn add(&mut self, closure: Closure) -> RouterResult {
        self.router.add(closure)
    }
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult {
        inject_method!(self.router, path, closure, RequestMethod::All);
        Ok(())
    }
}

impl Config for Octane {
    fn set_keepalive(&mut self, duration: Duration) {
        self.settings.keep_alive = Some(duration);
    }
    fn add_static_dir(&mut self, loc: &'static str, dir_name: &'static str) {
        let loc_buf = StdPathBuf::from(loc);
        let dir_buf = StdPathBuf::from(dir_name);
        if let Some(paths) = self.settings.static_dir.get_mut(&loc_buf) {
            paths.push(dir_buf)
        } else {
            self.settings.static_dir.insert(loc_buf, vec![dir_buf]);
        }
    }
    fn set_404_file(&mut self, dir_name: &'static str) {
        self.settings.file_404 = StdPathBuf::from(dir_name);
    }
    fn with_ssl_config(&mut self, ssl_conf: Ssl) {
        self.settings.ssl.key = ssl_conf.key;
        self.settings.ssl.cert = ssl_conf.cert;
    }
    fn ssl(&mut self, port: u16) -> &mut Ssl {
        self.settings.ssl.port = port;
        &mut self.settings.ssl
    }
}
