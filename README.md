<p align="center">
    <img src="https://github.com/OctaneWeb/OctaneSite/raw/master/assets/logo.png" width="500">
</p>
<h1 align="center">Octane :rocket:</h1>

A no-nonsense, high-powered web server aimed at minimizing dependencies while maintaining speed. Modeled after Express, a popular Javascript web framework, Octane combines the speed of Rust with the ease-of-use and flexibility of Express to create the optimal user experience.

#  Basic Usage 

Create an octane instance, and then you can register your methods on it using `app.METOHD()`

```rust
let mut app = Octane::new();
app.get("/", |_req, res| {
    Box::pin(async move {
        if let Ok(result) = res.send_file("templates/test.html").await {
            // yay!
        }
    })
});
```

# License

OctaneWeb/Octane is licensed under the
[MIT License.](https://github.com/OctaneWeb/Octane/blob/master/LICENSE) 
