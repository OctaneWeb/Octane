# octane

[Octane](https://github.com/OctaneWeb/Octane) on the multi-threaded Tokio
runtime, default configuration. Octane parses HTTP/1.x itself rather than
sitting on hyper.

## Stack

- **Language:** Rust 1.94
- **Framework:** Octane 0.2
- **TLS:** rustls (ring)
- **Build:** Multi-stage, `debian:bookworm-slim` runtime

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/pipeline` | GET | Returns `ok` (plain text) |
| `/baseline11` | GET | Sums query parameter values |
| `/baseline11` | POST | Sums query parameters + request body |
| `/delay/{ms}` | GET | Waits `ms` milliseconds and returns the delay |
| `/json/{count}?m=N` | GET | First `count` dataset items with `total = price * quantity * m` |
| `/echo` | POST | Returns the request body back verbatim |

The same routes are served over TLS on port 8081 (`json-tls`, `8gbit`).

## Notes

- Routing and path parameters (`/delay/:ms`, `/json/:count`) through the octane
  router, query parameters through `req.get_query()`
- Request bodies arrive whole in `req.body`, read by octane from either a
  `Content-Length` or chunked framing
- JSON through `res.json`, serialized by serde_json per request
- The dataset is leaked once at startup so responses borrow it instead of
  cloning
- Connections are persistent (octane's default 5 s idle keep-alive) and
  pipelined requests are answered in order
- `max_connections` is raised from octane's default of 1,024 to 65,536, since
  `async` holds 32,000 connections open and `baseline` 4,096
- `/delay/{ms}` is an async handler (`route_async_stop!`) waiting on
  `tokio::time::sleep`, so a pending request holds a timer, not a worker
  thread. Every other handler is synchronous and runs straight on the
  connection's task
- json-tls and 8gbit share the TLS listener on 8081, which octane runs next to
  the plaintext one and which advertises ALPN `http/1.1`. Without `/certs` it
  logs a warning and only plaintext is served
- No `json-comp`: octane has no response compression, and standard mode
  requires the framework's own
- Completeness: routes with path parameters are matched by the router;
  `app.add` / `add_route` middleware runs before or after the
  route handlers in registration order, app-wide or per path, and can stop
  the chain; the request
  carries path and query parameters, headers, cookies and the buffered body;
  responses are declared with `res.status(..).set(..).send(..)`, `res.json`
  and `res.send_file`
