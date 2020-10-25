use crate::config::{Config, OctaneConfig, Ssl};
use crate::constants::*;
use crate::error::Error;
use crate::http::Http;
use crate::middlewares::Closures;
use crate::request::{parse_without_body, Headers, Request, RequestLine};
use crate::responder::{BoxReader, Response, StatusCode};
use crate::route;
use crate::router::{Closure, Flow, Route, Router, RouterResult};
use crate::server_builder::ServerBuilder;
use crate::tls::AsMutStream;
use crate::util::find_in_slice;
use crate::{declare_error, default, route_next};
use std::error::Error as StdError;
use std::marker::Unpin;
use std::str;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{copy, split, AsyncWriteExt};
use tokio::prelude::*;

/// The Octane server
///
/// # Example
///
/// ```no_run
/// use octane::prelude::*;
///
/// #[octane::main]
/// async fn main() {
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
///     let port = 8080;
///     app.listen(port, || {
///         println!("Server started on port {}", port);
///     }).await.expect("Cannot establish connection");
/// }
/// ```
pub struct Octane {
    /// Some preferences which decides upon how the web server runs
    pub settings: OctaneConfig,
    router: Router,
}

impl Octane {
    /// Creates a new server instance with empty config and empty router
    pub fn new() -> Self {
        Octane {
            settings: OctaneConfig::new(),
            router: Router::new(),
        }
    }
    /// Appends the config of the Octane struct with a custom
    /// generated one. The Octane struct contains an OctaneConfig
    /// instance by default
    ///
    /// # Example
    ///
    /// ```
    /// use octane::config::OctaneConfig;
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// let mut config = OctaneConfig::new();
    /// config.ssl.key("key.pem").cert("cert.pem"); // we supply some ssl certs and key in the config
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
    /// **Appends** the router routes to the routes that
    /// the server instance holds, this allows you to
    /// independently add routes to a route Router structure
    /// and then use it with the server struct
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
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
    /// Returns a closure which can be used with the add or add_route method
    /// to serve a static directory.
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    ///
    /// app.add(Octane::static_dir(path!(
    ///    "/templates"
    /// )));
    /// ```
    pub fn static_dir(dir: &'static str) -> Closure {
        route_next!(|req, res| {
            let static_dir_name = std::path::PathBuf::from(dir);
            let final_url = static_dir_name.join(req.request_line.path.to_std_pathbuf());
            let final_string = final_url.to_str().unwrap();
            if &final_string[final_string.len() - 1..] == "/" {
                let stripped = &final_string[..final_string.len() - 1];
                res.send_file(stripped).ok();
            } else {
                res.send_file(final_string).ok();
            };
        })
    }
    /// Start listening on the port specified, the listen
    /// function also starts the Ssl server if the features
    /// are enabled and the key/certs are provided
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::Octane;
    ///
    /// #[octane::main]
    /// async fn main() {
    ///     let mut app = Octane::new();
    ///     let port = 80;
    ///     app.listen(port, || println!("Server started on port {}", port))
    ///         .await
    ///         .expect("Cannot establish connection");
    /// }
    /// ```
    pub async fn listen<F>(self, port: u16, exec: F) -> Result<(), Box<dyn StdError>>
    where
        F: FnOnce(),
    {
        let server = Arc::new(self);
        let mut _ssl = false;

        #[cfg(any(feature = "openSSL", feature = "rustls"))]
        {
            use crate::task;
            let clone = Arc::clone(&server);
            _ssl = true;

            async fn listen_ssl(server: Arc<Octane>) -> Result<(), Box<dyn StdError>> {
                let server_builder = ServerBuilder::new(server.settings.ssl.port);
                server_builder?
                    .listen_ssl(
                        |stream, server| async { Octane::serve(stream, server).await },
                        server,
                    )
                    .await?;
                Ok(())
            }

            task!({
                if let Err(x) = listen_ssl(clone).await {
                    println!("WARNING: {}", x);
                }
            });
        }
        exec();
        let server_builder = ServerBuilder::new(port);
        server_builder?
            .listen(
                move |stream, server| async move { Octane::serve(stream, server).await },
                server,
            )
            .await?;

        Ok(())
    }

    async fn serve<S>(stream_async: S, server: Arc<Octane>) -> Result<(), Box<dyn StdError>>
    where
        S: AsyncRead + AsyncWrite + Unpin + AsMutStream,
    {
        let (mut reader, writer) = split(stream_async);
        let mut data = Vec::<u8>::new();
        let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
        let body: &[u8];
        let request_line: RequestLine;
        let headers: Headers;
        let body_remainder: &[u8];

        loop {
            let read = reader.read(&mut buf).await?;
            if read == 0 {
                declare_error!(writer, StatusCode::BadRequest);
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
                    declare_error!(writer, StatusCode::BadRequest);
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
                reader.read_exact(&mut temp[..]).await?;
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
        if let Some(request) = Request::parse(request_line, headers, body) {
            let request_line = &request.request_line;
            let mut res = Response::new_empty();
            // Detect http version and validate
            let checker = Http::validate(&request);
            if checker.is_malformed() {
                declare_error!(writer, checker.err_code.unwrap());
            }
            if request_line.method.is_some() {
                // run closures
                server.router.run(request.clone(), &mut res);
                if !res.has_body() {
                    declare_error!(writer, StatusCode::NotFound);
                }

                Octane::send(res.get_data(), writer).await?;
            } else {
                declare_error!(writer, StatusCode::NotImplemented);
            }
        } else {
            declare_error!(writer, StatusCode::BadRequest);
        }
        Ok(())
    }
    pub(crate) async fn send<S>(
        mut response: (String, BoxReader),
        mut stream_async: S,
    ) -> Result<(), Box<dyn StdError>>
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
    fn head(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.head(path, closure)
    }
    fn put(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.put(path, closure)
    }
    fn get(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.get(path, closure)
    }
    fn delete(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.delete(path, closure)
    }
    fn post(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.post(path, closure)
    }
    fn patch(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.patch(path, closure)
    }
    fn add(&mut self, closure: Closure) -> RouterResult {
        self.router.middlewares.push(Closures {
            closure,
            index: self.router.route_counter,
        });
        self.router.route_counter += 1;
        Ok(())
    }
    fn add_route(&mut self, path: &str, closure: Closure) -> RouterResult {
        self.router.add_route(path, closure)
    }
}

impl Config for Octane {
    fn set_keepalive(&mut self, duration: Duration) {
        self.settings.keep_alive = Some(duration);
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
