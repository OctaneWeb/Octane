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

A no-nonsense, high-powered web server aimed at minimizing dependencies while maintaining speed. Modeled after Express, a popular Javascript web framework, Octane combines the speed of Rust with the ease-of-use and flexibility of Express to create the optimal user experience.

#  Basic Usage 

Create an octane instance, and then you can register your methods on it using `app.METHOD()`

```rust
let mut app = Octane::new();
app.get("/", route!(|_req, res| {
    res.send_file("templates/test.html").await.expect("File not found");
});
```

# License

OctaneWeb/Octane is licensed under the
[MIT License.](https://github.com/OctaneWeb/Octane/blob/master/LICENSE) 
