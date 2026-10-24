# wasip3-server

An HTTP/1.1 server built as a WASIp3 component, using octane's
allocation-free core (`octane_http` with `default-features = false`) over
`wasi:sockets@0.3.0`. Each connection is its own task (`spawn_local`), so
connections are served concurrently on one thread while they wait on
streams or timers.

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
| `GET /` | a greeting, at once |
| `GET /sleep?ms=N` | waits N ms on the host clock, then replies |
| `GET /stream?chunks=N&ms=M` | writes the body in N chunks, M ms apart |
| `POST /echo` | the request body |

10 parallel `curl localhost:8080/sleep?ms=1000` finish in about 1 s
in total, not 10, and requests made while `/stream` is writing are
answered between its chunks.

## Atomics

`-C target-feature=+atomics,+bulk-memory` builds and runs, but there's
nothing to gain: WASIp3 tasks are cooperative and share one thread, and
wasm-ld refuses `--cooperative-threading` together with `--shared-memory`,
so the atomic instructions operate on unshared memory.
