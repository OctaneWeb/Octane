<p align="center">
    <img src="https://avatars.githubusercontent.com/u/65298932" width="500">
</p>
<h1 align="center">Octane 🚀</h1>
<p float="left">
    <a href="https://github.com/OctaneWeb/Octane/blob/develop/LICENSE"  ><img src="https://img.shields.io/github/license/OctaneWeb/Octane"></a>
    <a href="https://github.com/OctaneWeb/Octane/actions" title="Rust worklow"><img src="https://img.shields.io/github/workflow/status/OctaneWeb/Octane/Rust"></a>
    <a href="https://github.com/OctaneWeb/Octane/issues" title="Issues"><img src="https://img.shields.io/github/issues/OctaneWeb/Octane"></a>
    <a href="https://crates.io/crates/octane" title="Crates.io"><img src="https://img.shields.io/crates/v/octane"></a>
    <a href="https://discord.gg/j6PsmNC" title="Discord server"><img src="https://img.shields.io/discord/708306551705698446"></a>
</p>

A high-powered web server aimed at minimizing dependencies while maintaining speed. Modeled after Express, a popular Javascript web framework, Octane combines the speed of Rust with the ease-of-use and flexibility of Express to create the optimal user experience.

- Multithreaded 🚄
- Asynchronous design 🐆
- Easy to use, intuitive design 🌱
- TLS enabled, rustls/openssl ready 🔒
- Minimal dependencies (working to reduce them more!) 💕
- no_std wasip3 support using fixed stack sizes! 💕

# Road to production

The initial version of this web server was written by hand, both the parser and router 5 years ago during 2020. Since then there's various fuzzing implementations and tests to test the integrity of 
the server. The web server also had audits from OpenAI's daybreak models and Claude's capability scanning. Since the web server uses minimal dependencies, the attack vector is also reduced. 

This web server is supposed to be a deterministic set of codes that people can use in production. A web server doesn't need huge dependencies, large binary sizes or build times. When developing for WASM targets, the binary
size matters even if you are running the guest program on a server using `wasmtime`

Templating frameworks like topcoat should be supported on top of Octane, since Octane is an OS agnostic http implementation.

#  Basic Usage 

Create an octane instance, and then you can register your methods on it using `app.METHOD()`

```rust
use octane::prelude::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Octane::new();
    app.ssl(8001)
        .key("templates/key.pem")
        .cert("templates/cert.pem"); // Setup ssl

    app.get(
        "/",
        route!(|req, res| {
            res.send_file("templates/test.html").expect("File not found!");
            Flow::Next
        }),
    )?;

    app.add(Octane::static_dir("templates"))?;
    app.listen(8000, || println!("Server Started!")).await
}
```

# Docs

Documentation will be available on [docs.rs](https://docs.rs/octane/).

# Roadmap to production
- [ ] Http2
- [x] Stable SSL support
- [x] Efficient error handling (using enums instead of `Box<dyn Error>`)
- [ ] Web socket library
- [ ] Multipart/json form handling (being worked on)
- [ ] Tracing
- [ ] Topcoat/Dixous support
- Much more....

# Contribute

Checkout [CONTRIBUTING.md](https://github.com/OctaneWeb/Octane/blob/develop/CONTRIBUTING.md) for info on how to contribute to this project

# License

OctaneWeb/Octane is licensed under the
[MIT License.](https://github.com/OctaneWeb/Octane/blob/master/LICENSE) 
