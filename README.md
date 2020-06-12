<p align="center">
    <img src="https://github.com/OctaneWeb/OctaneSite/raw/master/assets/logo.png" width="500">
</p>
<h1 align="center">Octane 🚀</h1>

A no-nonsense, high-powered web server aimed at minimizing dependencies while maintaining speed. Modeled after Express, a popular Javascript web framework, Octane combines the speed of Rust with the ease-of-use and flexibility of Express to create the optimal user experience.

#  Basic Usage 

Create an octane instance, and then you can register your methods on it using `app.METHOD()`

```rust
let mut app = Octane::new();
app.get("/", Route!(_req, res {
        res.send_file("templates/test.html").await.expect("File not found");
    })
});
```

# License

OctaneWeb/Octane is licensed under the
[MIT License.](https://github.com/OctaneWeb/Octane/blob/master/LICENSE) 
