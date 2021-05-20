use crate::config::{Config, OctaneConfig, Ssl};
use crate::error::Error;
use crate::http::Http;
use crate::middlewares::Closures;
use crate::request::{Headers, Request, RequestLine};
use crate::responder::{BoxReader, Response};
use crate::router::{Closure, Route, Router, RouterResult};
use crate::server_builder::ServerBuilder;
use crate::tls::AsMutStream;
use crate::{declare_error, default};
use octane_http::http1x::raw_request::RawRequest1x;
use octane_http::http1x::Http1xReader;
use octane_http::StatusCode;
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
        let (reader, writer) = split(stream_async);
        let mut data = Vec::new();
        let parsed = Http1xReader::new(RawRequest1x::new(reader), &mut data).await;
        if let Ok((raw_headers, raw_request_line, body_remainder, reader_left)) = parsed {
            let headers = Headers::parse(raw_headers).unwrap();
            let request_line = RequestLine::parse(raw_request_line).unwrap();
            if let Some(request) = Request::from_raw(
                &headers,
                request_line,
                Default::default(),
                &mut Default::default(),
                body_remainder,
                reader_left,
            )
            .await
            {
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
        } else {
            declare_error!(writer, parsed.err().unwrap());
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
