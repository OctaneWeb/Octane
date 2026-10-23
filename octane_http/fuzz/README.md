# Fuzzing

A cargo-fuzz crate for Octane's HTTP/1.x parsing, from the raw head splitter
in `octane_http` to the full request path in `octane`. It has its own
`[workspace]` so it stays out of the main workspace, and it needs nightly.

```sh
cd octane_http
cargo +nightly fuzz run request                          # until it finds something
cargo +nightly fuzz run cookie -- -max_total_time=300    # for five minutes
```

| Target | Fuzzes | Checks |
|---|---|---|
| `http1x_parse` | `RawRequest1x::parse` | fields are non-empty sub-slices of the input that rebuild it |
| `http1x_read` | `Http1xReader`, reads of 1 to 17 bytes | chunking never changes how the head is split |
| `request_line` | the request-line parser | normalised path, query kept apart, method and version read exactly |
| `headers` | the header block and `Content-Length` | well formed fields, repeats merged in order, RFC 9112 section 6.3 body length |
| `path` | `.` and `..` normalisation | never above the root, idempotent |
| `query` | query strings and the component decoder | named pairs kept, the decoder inverts `%XX` and form encoding |
| `cookie` | `Cookie` header parsing and `Set-Cookie` building | token names, no control characters, round trips, attributes kept, browser rules |
| `request` | one connection through the whole server | exactly one well framed response, no header the app didn't set |
| `http2_frame`, `http2_hpack` | HTTP/2 framing and HPACK | need `--features http2` and the `http2` module, which isn't committed yet |
| `quic` | nothing yet | placeholder |

The targets for the `octane` crate go through its `fuzzing` feature
(`src/fuzzing.rs`), which exposes the internal parsers and serves a
connection from memory. They assume octane's default features, `faithful`
in particular.

`corpus/<target>/` holds the inputs each target starts from, and crashes are
saved to `artifacts/<target>/`. Replay one with
`cargo +nightly fuzz run <target> artifacts/<target>/<file>`.
