use crate::config::{Config, OctaneConfig, Ssl};
use crate::connection::Connection;
use crate::constants::B_CRLF;
use crate::default;
use crate::error::Error;
use crate::http::Http;
use crate::request::{BodyFraming, Headers, Request, RequestMethod};
use crate::responder::{ResBody, Response};
use crate::router::{Handler, Route, Router, RouterResult};
use crate::server_builder::ServerBuilder;
use crate::shutdown::{Shutdown, Watch};
use crate::tls::AsMutStream;
use crate::util::{read_some, Deadline};
use octane_http::http1x::Http1xReader;
use octane_http::{HttpVersion, StatusCode};
use std::error::Error as StdError;
use std::future::{pending, poll_fn, Future};
use std::io;
use std::marker::Unpin;
use std::pin::pin;
use std::str;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tokio::io::{copy, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Semaphore;

// The interim response a client sending `Expect: 100-continue` waits for
const CONTINUE: &[u8] = b"HTTP/1.1 100 Continue\r\n\r\n";

/// How long a closing connection keeps reading what the client still sends,
/// see [`Octane::close`]
const LINGER: Duration = Duration::from_secs(2);

// The most a connection's read buffer keeps between requests. Past it, the
// buffer is cut down to what the next request has already sent.
const READ_BUFFER_KEPT: usize = 64 * 1024;

// Whether a client is waiting for a `100 Continue` before sending a body
// that will be read (RFC 9110 section 10.1.1). Not when the body is going to
// be refused, or for HTTP/1.0 which can't ask for it.
fn expects_continue(headers: &Headers, version: HttpVersion, max_body_size: usize) -> bool {
    version == HttpVersion::Http11
        && headers
            .get("expect")
            .map_or(false, |value| value.eq_ignore_ascii_case("100-continue"))
        && match headers.body_framing(version, max_body_size) {
            Ok(BodyFraming::Length(len)) => len > 0,
            Ok(BodyFraming::Chunked) => true,
            Err(_) => false,
        }
}

fn checked_connection_limit(configured: usize) -> io::Result<usize> {
    let configured = configured.max(1);
    if configured > Semaphore::MAX_PERMITS {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "max_connections exceeds the supported semaphore limit",
        ))
    } else {
        Ok(configured)
    }
}

/// The Octane server
///
/// # Example
///
/// ```no_run
/// use octane::prelude::*;
///
/// #[tokio::main]
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
    shutdown: Arc<Shutdown>,
}

impl Octane {
    /// Creates a new server instance with empty config and empty router
    pub fn new() -> Self {
        Octane {
            settings: OctaneConfig::new(),
            router: Router::new(),
            shutdown: Arc::new(Shutdown::new()),
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
    /// are enabled and the key/certs are provided. A key or
    /// certificate that can't be loaded, or a port that can't be
    /// bound, is returned as an error before anything is served.
    ///
    /// The default connection deadline uses Tokio's time driver. A custom
    /// runtime must call `enable_time` or `enable_all`; runtimes created by
    /// `#[tokio::main]` already enable it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::Octane;
    ///
    /// #[tokio::main]
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
        self.listen_with_shutdown(port, exec, pending()).await
    }

    /// Like [`listen`](#method.listen), until `signal` completes. Then the
    /// server shuts down gracefully: it stops accepting connections, closes
    /// the ones waiting for a request, and lets the ones answering a request
    /// finish it, with `Connection: close` on the response. It returns once
    /// every connection is closed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::Octane;
    /// use tokio::sync::oneshot;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let app = Octane::new();
    ///     // send on `stop` from anywhere to shut the server down
    ///     let (stop, stopped) = oneshot::channel::<()>();
    ///     # drop(stop);
    ///     app.listen_with_shutdown(8080, || {}, async {
    ///         let _ = stopped.await;
    ///     })
    ///     .await
    ///     .expect("Cannot establish connection");
    /// }
    /// ```
    pub async fn listen_with_shutdown<F, S>(
        self,
        port: u16,
        exec: F,
        signal: S,
    ) -> Result<(), Box<dyn StdError>>
    where
        F: FnOnce(),
        S: Future<Output = ()>,
    {
        let max_connections = checked_connection_limit(self.settings.max_connections)?;
        // Everything that can fail happens before anything is served: the TLS
        // key and certificate are loaded and both ports bound, so `listen`
        // returns the error instead of running without one of them.
        #[cfg(any(feature = "openSSL", feature = "rustls"))]
        let tls = Octane::bind_tls(&self.settings)?;
        let listener = ServerBuilder::new(port)?;
        let server = Arc::new(self);
        let connections = Arc::new(Semaphore::new(max_connections));

        #[cfg(any(feature = "openSSL", feature = "rustls"))]
        if let Some((tls_listener, acceptor)) = tls {
            use crate::task;
            let server = Arc::clone(&server);
            let connections = Arc::clone(&connections);
            task!({
                tls_listener
                    .listen_ssl(
                        acceptor,
                        |stream, server| async { Octane::serve(stream, server).await },
                        server,
                        connections,
                    )
                    .await
            });
        }
        exec();
        let shutdown = Arc::clone(&server.shutdown);
        let accepting = listener.listen(
            move |stream, server| async move { Octane::serve(stream, server).await },
            server,
            Arc::clone(&connections),
        );
        // accepting ends once the signal has started the shutdown
        let mut accepting = pin!(accepting);
        let mut signal = pin!(async move {
            signal.await;
            shutdown.start();
        });
        let mut signalled = false;
        poll_fn(|cx| {
            if !signalled && signal.as_mut().poll(cx).is_ready() {
                signalled = true;
            }
            accepting.as_mut().poll(cx)
        })
        .await?;
        // every connection holds a permit until it's closed
        let mut open = max_connections;
        while open > 0 {
            let batch = open.min(u32::MAX as usize);
            let _ = connections.acquire_many(batch as u32).await;
            open -= batch;
        }
        Ok(())
    }

    /// The TLS listener and the acceptor for its connections, `None` when
    /// TLS isn't set up: neither a key nor a certificate was given. Giving
    /// only one of them is an error, as is a key or certificate that can't
    /// be loaded.
    #[cfg(any(feature = "openSSL", feature = "rustls"))]
    fn bind_tls(
        settings: &OctaneConfig,
    ) -> Result<Option<(ServerBuilder, crate::tls::Acceptor)>, Box<dyn StdError>> {
        let ssl = &settings.ssl;
        match (
            ssl.key.as_os_str().is_empty(),
            ssl.cert.as_os_str().is_empty(),
        ) {
            (true, true) => return Ok(None),
            (false, false) => {}
            _ => {
                return Err("TLS needs both a key and a certificate, \
                    set them with `app.ssl(port).key(..).cert(..)`"
                    .into())
            }
        }
        let acceptor = crate::tls::acceptor(settings).map_err(|error| {
            format!(
                "can't load the TLS certificate {} and key {}: {}",
                ssl.cert.display(),
                ssl.key.display(),
                error
            )
        })?;
        let listener = ServerBuilder::new(ssl.port)
            .map_err(|error| format!("can't listen on the TLS port {}: {}", ssl.port, error))?;
        Ok(Some((listener, acceptor)))
    }

    /// Register a task that stops when the server shuts down
    pub(crate) fn watch_shutdown(&self) -> Watch {
        self.shutdown.watch()
    }

    async fn serve<S>(stream_async: S, server: Arc<Octane>) -> Result<(), Box<dyn StdError>>
    where
        S: AsyncRead + AsyncWrite + Unpin + AsMutStream,
    {
        Octane::serve_http1x(stream_async, server).await
    }
    /// Serve HTTP/1.x requests off `stream` until the client closes it or
    /// asks for it to be closed, sends something that can't be served, or
    /// leaves it idle for longer than
    /// [`keep_alive`](config/struct.OctaneConfig.html#structfield.keep_alive).
    /// Requests sent without waiting for the previous response (pipelining)
    /// are answered in order. It only needs a byte stream, so the fuzz
    /// targets can run it in memory.
    pub(crate) async fn serve_http1x<S>(
        stream: S,
        server: Arc<Octane>,
    ) -> Result<(), Box<dyn StdError>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let settings = &server.settings;
        // responses are held until the server would wait on the client
        let mut stream = Connection::new(stream);
        let shutdown = server.shutdown.watch();
        // bytes read off the stream that no request has used yet, kept
        // between requests so its allocation is reused
        let mut data = Vec::new();
        // Waiting for a request to start, and receiving and answering one.
        // Nothing has been asked of the connection yet, so the first request
        // may take as long to start as answering any request may.
        let mut wait = Deadline::new(settings.connection_timeout);
        let mut work = Deadline::new(settings.connection_timeout);
        loop {
            // A server should ignore empty lines before a request line (RFC
            // 9112 section 2.2), some clients send one after a body
            let blank = data
                .chunks_exact(2)
                .take_while(|pair| *pair == B_CRLF)
                .count();
            data.drain(..blank * 2);
            // wait for the next request, or for enough of it to tell it from
            // an empty line
            if data.is_empty() || data[..] == B_CRLF[..1] {
                // the answers so far go out before the wait, which the idle
                // time shouldn't count
                if stream.has_pending() {
                    match work.run(stream.flush()).await {
                        Some(Ok(())) => {}
                        _ => return Ok(()),
                    }
                }
                let read = {
                    let mut read = pin!(wait.run(read_some(&mut stream, &mut data)));
                    poll_fn(|cx| match read.as_mut().poll(cx) {
                        Poll::Ready(read) => Poll::Ready(Some(read)),
                        // a request that already came in is still answered
                        Poll::Pending => shutdown.poll_started(cx).map(|()| None),
                    })
                    .await
                };
                match read {
                    Some(Some(Ok(read))) if read > 0 => continue,
                    // nothing asked of it when the server shuts down
                    None => {
                        let _ = stream.shutdown().await;
                        return Ok(());
                    }
                    // idle for too long, closed or broken: nothing to answer
                    _ => return Ok(()),
                }
            }
            let keep_alive = match work
                .run(Octane::serve_one(&mut stream, &server, &mut data))
                .await
            {
                Some(keep_alive) => keep_alive?,
                // the request took too long to arrive or to answer
                None => return Ok(()),
            };
            if !keep_alive {
                Octane::close(&mut stream, &mut data).await;
                return Ok(());
            }
            wait.limit = settings.keep_alive;
            // a large request leaves a large buffer behind, which an idle
            // connection shouldn't hold on to
            if data.capacity() > READ_BUFFER_KEPT {
                data.shrink_to(0);
            }
        }
    }

    /// End a connection the server is done with, once the client has seen
    /// the end of the last response.
    ///
    /// Our side is shut down first, which ends the stream with close_notify
    /// over TLS. Then whatever the client is still sending is read and
    /// thrown away until it closes too, for at most [`LINGER`]: closing a
    /// socket with input left unread makes the kernel answer with a reset,
    /// which can destroy the response before the client reads it. The usual
    /// case is a `413` sent before an upload's body was read.
    async fn close<S>(stream: &mut Connection<S>, data: &mut Vec<u8>)
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        if stream.shutdown().await.is_err() {
            return;
        }
        let deadline = tokio::time::Instant::now() + LINGER;
        loop {
            data.clear();
            match tokio::time::timeout_at(deadline, read_some(stream, data)).await {
                Ok(Ok(read)) if read > 0 => {}
                _ => return,
            }
        }
    }

    /// Read the request at the start of `data`, answer it and remove it from
    /// `data`, which then starts with whatever the client sent after it.
    /// Returns whether the connection can carry another request.
    async fn serve_one<S>(
        stream: &mut Connection<S>,
        server: &Octane,
        data: &mut Vec<u8>,
    ) -> Result<bool, Box<dyn StdError>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let settings = &server.settings;
        // The head is a view of `data` until the body is read, which may
        // move the buffer: what's needed of it is taken out first, and the
        // headers are made again from where they are in `data` afterwards.
        let (request_line, head, framing, buffered_body_len, expects) = {
            let raw = match Http1xReader::new(&mut *stream, data).await {
                Ok(raw) => raw,
                Err(code) => return Octane::fail(stream, code).await,
            };
            // everything buffered past the head, the body and any request
            // after it
            let buffered_body_len = raw.body_remainder.len();
            let request_line = crate::request::request_line_parse(
                raw.request_method,
                raw.request_url,
                raw.request_version,
            );
            let headers = str::from_utf8(raw.headers).ok().and_then(Headers::parse);
            let (request_line, headers) = match (request_line, headers) {
                (Some(request_line), Some(headers)) => (request_line, headers),
                _ => return Octane::fail(stream, StatusCode::BadRequest).await,
            };
            // where the header block is in `data`: right after the request
            // line, `METHOD SP target SP version CRLF`, at its start
            let start =
                raw.request_method.len() + raw.request_url.len() + raw.request_version.len() + 4;
            let head = start..start + raw.headers.len();
            let version = request_line.version;
            let framing = headers.body_framing(version, settings.max_body_size);
            let expects = buffered_body_len == 0
                && expects_continue(&headers, version, settings.max_body_size);
            (request_line, head, framing, buffered_body_len, expects)
        };
        // The raw view has ended, so `data` can be borrowed mutably again.
        let body_start = data.len() - buffered_body_len;
        let framing = match framing {
            Ok(framing) => framing,
            Err(code) => return Octane::fail(stream, code).await,
        };
        if expects {
            stream.write_all(CONTINUE).await?;
        }
        let (request, body_end) = match Request::from_raw(
            head,
            framing,
            request_line,
            data,
            body_start,
            &mut *stream,
            settings.max_body_size,
        )
        .await
        {
            Ok(parsed) => parsed,
            Err(code) => return Octane::fail(stream, code).await,
        };
        // Detect http version and validate
        let checker = Http::validate(&request);
        if checker.is_malformed() {
            let code = checker.err_code.unwrap_or(StatusCode::BadRequest);
            return Octane::fail(stream, code).await;
        }
        if !request.request_line.method.is_some() {
            return Octane::fail(stream, StatusCode::NotImplemented).await;
        }

        let head_only = request.request_line.method == RequestMethod::Head;
        let version = request.request_line.version;
        let keep_alive = settings.keep_alive.is_some() && request.keep_alive();
        let mut res = Response::new_empty();
        server.route(request, &mut res).await;
        // a shutdown that started while the handler ran ends the connection
        let keep_alive = keep_alive && !server.shutdown.started();
        if !keep_alive {
            res.headers.set("Connection", "close");
        } else if version == HttpVersion::Http10 {
            // HTTP/1.0 connections only persist when both sides say so
            res.headers.set("Connection", "keep-alive");
        }
        res.frame();
        Octane::send(res, stream, head_only).await?;
        // the request is answered, what follows it is the next one
        data.drain(..body_end);
        Ok(keep_alive)
    }

    // Answer a request that can't be served with `code` and give up on the
    // connection, whatever the client sent after it can't be trusted to
    // start a request.
    async fn fail<S>(
        stream: &mut Connection<S>,
        code: StatusCode,
    ) -> Result<bool, Box<dyn StdError>>
    where
        S: AsyncWrite + Unpin,
    {
        let mut res = Error::response(code);
        res.headers.set("Connection", "close");
        res.frame();
        Octane::send(res, stream, false).await?;
        Ok(false)
    }

    /// Run the router for a parsed request, filling `res`. A request that
    /// nothing answered (see [`Flow`](crate::router::Flow)) gets a 404, or
    /// a 405 naming the methods its path has. Also used by the HTTP/2
    /// driver (a separate module) which cannot reach the private router.
    pub(crate) async fn route(&self, request: Request<'_>, res: &mut Response<'_>) {
        if self.router.run(&request, res).await || res.has_body() {
            return;
        }
        // the path doesn't exist, or not for this method, which gets a 405
        // naming the ones it has (RFC 9110 section 15.5.6)
        let allowed = self.router.allowed(&request.request_line.path);
        let method = request.request_line.method;
        if allowed.is_empty() || allowed.contains(&method) {
            *res = Error::response(StatusCode::NotFound);
        } else {
            let names: Vec<&str> = allowed.iter().map(RequestMethod::as_str).collect();
            *res = Error::response(StatusCode::MethodNotAllowed);
            res.headers.set("Allow", names.join(", "));
        }
    }

    /// Write a response, without its body when `head_only` (the answer to a
    /// `HEAD` request) or when its status has none. It is held with the responses before it until the
    /// server waits on the client, and goes out with them in one write.
    async fn send<S>(
        res: Response<'_>,
        stream: &mut Connection<S>,
        head_only: bool,
    ) -> Result<(), Box<dyn StdError>>
    where
        S: AsyncWrite + Unpin,
    {
        let head_only = head_only || !res.status_has_body();
        res.write_head(stream.buffer());
        match res.into_body() {
            _ if head_only => {}
            ResBody::None => {}
            ResBody::Bytes(body) => stream.write_all(&body).await?,
            ResBody::Static(body) => stream.write_all(body).await?,
            ResBody::Sized(len, reader) => {
                let mut reader = reader.take(len as u64);
                let sent = copy(&mut reader, &mut *stream).await?;
                // the client is still waiting for the rest of the body, the
                // connection can't carry anything after it
                if sent < len as u64 {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the response body ended before its Content-Length",
                    )));
                }
            }
        }
        Ok(())
    }
}

default!(Octane);

impl Route for Octane {
    type Handler = Handler;

    fn head(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.router.head(path, handler)
    }
    fn put(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.router.put(path, handler)
    }
    fn get(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.router.get(path, handler)
    }
    fn delete(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.router.delete(path, handler)
    }
    fn post(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.router.post(path, handler)
    }
    fn patch(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.router.patch(path, handler)
    }
    fn add(&mut self, handler: impl Into<Handler>) -> RouterResult {
        self.router.add(handler)
    }
    fn add_route(&mut self, path: &str, handler: impl Into<Handler>) -> RouterResult {
        self.router.add_route(path, handler)
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

#[cfg(test)]
mod tests {
    use super::checked_connection_limit;
    use tokio::sync::Semaphore;

    #[test]
    fn connection_limit_is_validated_before_building_the_semaphore() {
        assert_eq!(checked_connection_limit(0).unwrap(), 1);
        assert_eq!(
            checked_connection_limit(Semaphore::MAX_PERMITS).unwrap(),
            Semaphore::MAX_PERMITS
        );
        assert!(checked_connection_limit(usize::MAX).is_err());
    }
}
