# WASI guests

Programs that run octane as WebAssembly components inside wasmtime, using
`octane` with `default-features = false` (no tokio) and the `wasm` feature,
which adds `octane::wasip3` on `wasm32-wasip3`.

| Guest | What it is |
| --- | --- |
| [`wasip3-server`](wasip3-server) | HTTP/1.1 server as a WASIp3 component: `octane::wasip3::serve` with an async handler |

Each guest is its own Cargo workspace, so its target and nightly settings
don't affect the main octane build.

## Setup

You need three things: Rust nightly with `rust-src`, wasi-sdk 34 or newer
and wasmtime with WASIp3 support. The steps below work on macOS and Linux
(arm64 and x86_64).

### 1. Rust nightly

`wasm32-wasip3` is a tier 3 target with no prebuilt standard library, so it
is built from source with `-Zbuild-std`. That needs nightly and `rust-src`:

```sh
rustup toolchain install nightly --component rust-src
```

Each guest has a `rust-toolchain.toml` that selects nightly, so nothing has
to be set as the default.

### 2. wasi-sdk

wasi-sdk provides the WASIp3 libc, `crt1-reactor.o`, `wasm-ld` and
`wasm-component-ld`. The `wasm-component-ld` that ships with rustc is too
old for WASIp3, so the guests use wasi-sdk's.

Download the latest release from
<https://github.com/WebAssembly/wasi-sdk/releases> into `~/wasi-sdk`:

```sh
# latest release number, e.g. 34
WASI_SDK_VERSION=$(curl -fsSL https://api.github.com/repos/WebAssembly/wasi-sdk/releases/latest \
    | sed -n 's/.*"tag_name": *"wasi-sdk-\([0-9.]*\)".*/\1/p')

case "$(uname -m)" in arm64|aarch64) ARCH=arm64;; x86_64|amd64) ARCH=x86_64;; esac
case "$(uname -s)" in Darwin) OS=macos;; Linux) OS=linux;; esac
NAME=wasi-sdk-$WASI_SDK_VERSION.0-$ARCH-$OS

mkdir -p ~/wasi-sdk
curl -fL "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-$WASI_SDK_VERSION/$NAME.tar.gz" \
    | tar -xz -C ~/wasi-sdk
echo "installed ~/wasi-sdk/$NAME"
```

Then point `WASI_SDK_PATH` at it. Add this to `~/.zshrc` (or `~/.bashrc`)
so new shells have it:

```sh
export WASI_SDK_PATH=~/wasi-sdk/wasi-sdk-34.0-arm64-macos   # the folder printed above
```

Check it works:

```sh
"$WASI_SDK_PATH/bin/wasm-ld" --version              # LLD 23 or newer
ls "$WASI_SDK_PATH/share/wasi-sysroot/lib/wasm32-wasip3"   # the WASIp3 libc
```

**macOS:** a wasi-sdk downloaded with a browser is quarantined by
Gatekeeper. Its tools then fail, often silently or with an
"Apple could not verify" dialog. `curl` doesn't quarantine files, so the
steps above aren't affected. If you downloaded it some other way, clear the
quarantine once:

```sh
xattr -dr com.apple.quarantine "$WASI_SDK_PATH"
find "$WASI_SDK_PATH" -type l -exec xattr -s -d com.apple.quarantine {} \;
```

### 3. wasmtime

Install it with the official script, or update an existing install to the
latest version:

```sh
curl https://wasmtime.dev/install.sh -sSf | bash
wasmtime --version
```

WASIp3 needs a release that has the `-S p3` option (tested with 47.0.3):

```sh
wasmtime run -S help | grep p3
```

## Build and run

```sh
cd guests/wasip3-server
cargo build --release
wasmtime run -S p3 -S inherit-network -W component-model-async \
    target/wasm32-wasip3/release/wasip3_server.wasm
```

The flags:

- `-S p3` enables the WASIp3 interfaces
- `-S inherit-network` lets the guest bind and accept on the host's network
- `-W component-model-async` enables async lifting, `stream<T>` and
  `future<T>`

The first build takes a minute while the standard library is compiled.
Later builds only rebuild the guest.

In another terminal:

```sh
curl localhost:8080/
curl -X POST -d 'hello' localhost:8080/echo
curl -N 'localhost:8080/stream?chunks=5&ms=500'    # body arrives in chunks

# 10 requests that each wait 1 s finish in about 1 s, not 10
time (for i in $(seq 10); do curl -s 'localhost:8080/sleep?ms=1000' & done; wait)
```

See the [wasip3-server README](wasip3-server/README.md) for its routes and
how it is put together.

## Troubleshooting

| Error | Cause |
| --- | --- |
| `set WASI_SDK_PATH to the wasi-sdk install` | `WASI_SDK_PATH` isn't exported in this shell |
| `.../lib/wasm32-wasip3 is missing` | wasi-sdk older than 34, it has no WASIp3 libc |
| `can't find crate for core` | `rust-src` isn't installed for nightly, see step 1 |
| `undefined symbol: __wasm_get_stack_pointer` | `crt1-reactor.o` isn't linked, check `build.rs` ran with `WASI_SDK_PATH` set |
| `failed to resolve import env::__wasm_set_tls_base` | rustc's bundled `wasm-component-ld` ran instead of wasi-sdk's, check `.cargo/config.toml` uses `tools/wasm-component-ld` |
| wasmtime: `unknown option p3` | wasmtime too old, update it (step 3) |
