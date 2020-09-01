<p align="center">
    <img src="https://raw.githubusercontent.com/OctaneWeb/OctaneSite/develop/docs/assets/logo.png" width="500">
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
- Minimal dependencies (working to reduce them more!) 💕

#  Basic Usage 

Create an octane instance, and then you can register your methods on it using `app.METHOD()`

```rust
use octane::config::Config;
use octane::responder::StatusCode;
use octane::server::Octane;
use octane::{
    route,
    router::{Flow, Route},
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Octane::new();
    app.ssl(8001)
        .key("templates/key.pem")
        .cert("templates/cert.pem");
    app.get(
        "/to_home",
        route!(|req, res| {
            res.redirect("/").send("redirecting");
            Flow::Stop
        }),
    )?;

    app.get(
        "/",
        route!(|req, res| {
            res.send_file("templates/test.html").expect("File not found!");
            Flow::Next
        }),
    )?;

    app.add(Octane::static_dir("templates/"))?;
    app.listen(8000)
}
```

# Docs

Documentation will be available on [docs.rs](https://docs.rs/octane/0.1.1/octane/) and on the offical [Octane Site]().

# Contribute

Checkout [CONTRIBUTING.md](https://github.com/OctaneWeb/Octane/CONTRIBUTING.md) for info on how to contribute to this project

# License

OctaneWeb/Octane is licensed under the
[MIT License.](https://github.com/OctaneWeb/Octane/blob/master/LICENSE) 
