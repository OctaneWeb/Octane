# wasip3-server

An HTTP/1.1 server built as a WASIp3 component with octane's `wasm`
feature:

```toml
octane = { path = "../..", default-features = false, features = ["wasm", "url_variables"] }
```

Routes are added to an `octane::wasip3::Router` through octane's `Route`
trait (`get`, `post`, `add` for middleware, `:name` url variables), the
same as on the tokio server, with async handlers made by `handler(...)`.
`octane::wasip3::serve` creates the listening socket over
`wasi:sockets@0.3.0` and runs each connection as its own task, so
`src/lib.rs` only defines routes. Connections are served concurrently on
one thread while they wait on sockets, timers or streamed bodies.

## Build and run

See [the guests README](../README.md) for installing nightly, wasi-sdk and
wasmtime. Then:

```sh
export WASI_SDK_PATH=~/wasi-sdk/wasi-sdk-34.0-arm64-macos
cargo build --release
wasmtime run -S p3 -S inherit-network -W component-model-async \
    target/wasm32-wasip3/release/wasip3_server.wasm
```

## How it builds

- `rust-toolchain.toml` selects nightly, and `.cargo/config.toml` builds
  `std` for `wasm32-wasip3` with `-Zbuild-std`.
- `build.rs` links wasi-sdk's WASIp3 libc and `crt1-reactor.o`. rustc has
  no copies of these for a tier 3 target.
- `tools/wasm-component-ld` is the linker. It runs wasi-sdk's
  `wasm-component-ld` and `wasm-ld`, because the ones bundled with rustc
  are too old for cooperative threading.

## Routes

| Route | Response |
| --- | --- |
| every request | logged by a middleware (`router.add`), then routing continues |
| `GET /` | a greeting, at once |
| `GET /hello/:name` | greets `name`, a url variable |
| `GET /sleep?ms=N` | waits N ms on the host clock, then replies |
| `GET /stream?chunks=N&ms=M` | writes the body in N chunks, M ms apart |
| `POST /echo` | the request body |
| anything else | `404`, sent by the server when no route answered |

Like on the tokio server, a route only answers the method it was added
with: `HEAD /` is a `404` unless it's added with `router.head`.

10 parallel `curl localhost:8080/sleep?ms=1000` finish in about 1 s
in total, not 10, and requests made while `/stream` is writing are
answered between its chunks.

## Atomics

`-C target-feature=+atomics,+bulk-memory` builds and runs, but there's
nothing to gain: WASIp3 tasks are cooperative and share one thread, and
wasm-ld refuses `--cooperative-threading` together with `--shared-memory`,
so the atomic instructions operate on unshared memory.
