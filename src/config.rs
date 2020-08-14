use crate::default;
use core::time::Duration;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

#[cfg(feature = "rustls")]
use tokio_rustls::rustls::{
    internal::pemfile::{certs, pkcs8_private_keys},
    Certificate, PrivateKey,
};

/// Ssl struct, contains the key and cert
/// required to setup SSL with the selected
/// feature (rustls or openssl)
///
/// By default, the Config struct has an ssl field so
/// you don't have to use it directly but if you want
/// to then you can and then append it to the config by
/// [`with_ssl_config()`]()
///
/// ```no_run
/// use octane::server::Octane;
/// use std::time::Duration;
/// use octane::config::{Config, Ssl};
///
/// let mut app = Octane::new();
/// let mut ssl_config = Ssl::new();
/// ssl_config.key("templates/key.pem");
/// app.with_ssl_config(ssl_config);
/// ```
/// The ssl struct has three fields
///
/// - `key`: Location of the private key file, should have the
/// extension as .pem
/// - `cert`: Location of the certificate file, should have the
/// extension as .pem
/// - `port`: The port where TLS should listen, is 443 by defaults
#[derive(Clone)]
pub struct Ssl {
    pub key: PathBuf,
    pub cert: PathBuf,
    pub port: u16,
}

impl Ssl {
    /// Returns a new Ssl struct instance with default port
    /// 443
    pub fn new() -> Self {
        Ssl {
            key: PathBuf::new(),
            cert: PathBuf::new(),
            port: 443,
        }
    }
    /// Mutates the Ssl struct and sets the private key path
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::config::OctaneConfig;
    ///
    /// let mut config = OctaneConfig::new();
    /// config
    ///    .ssl
    ///    .key("templates/key.pem");
    /// ```
    pub fn key(&mut self, path: &str) -> &mut Self {
        self.key = PathBuf::from(path);
        self
    }
    /// Mutates the Ssl struct and sets the SSL certificate path
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::config::OctaneConfig;
    ///
    /// let mut config = OctaneConfig::new();
    /// config
    ///    .ssl
    ///    .cert("templates/cert.pem");
    /// ```
    pub fn cert(&mut self, path: &str) -> &mut Self {
        self.cert = PathBuf::from(path);
        self
    }
    /// Validates the certs and keys by checking their extensions
    pub fn validate(&self) {
        let key_ext = self
            .key
            .as_path()
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("");
        let cert_ext = self
            .cert
            .as_path()
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("");
        if key_ext != "pem" && cert_ext != "pem" {
            panic!("Invalid key/cert file, {:?}", "bad extension")
        }
    }
}

/// An independent OctaneConfig struct that can be used
/// seperately from the app structure and then be appended
/// to it.
///
/// **Note**: If you won't push the independently made config
/// then the configurations won't take place, make sure
/// to push them to the main server struct like the following
///
/// ```no_run
/// use octane::server::Octane;
/// use octane::config::OctaneConfig;
///
/// let mut app = Octane::new();
/// let mut config = OctaneConfig::new();
/// app.with_config(config);
/// ```
///
/// The config holds the values for various configurable
/// item. If no config is specfied then defaults are used.
///
/// # Config parameters
///
/// - `keep_alive`: The duration for keep alive requests.
/// - `static_dir`: Holds the static directory names and
/// locations which are to be served.
/// - `ssl`: An instance of the `Ssl` struct to store the
/// values of key and certificates.
/// - `worker_threads`: The number of worker threads to use
/// while handling requests, by default this value is equal
/// to the number of cores available to the system, this is
/// later on used for setting number for the
/// [`core_threads`](https://docs.rs/tokio/0.2.13/tokio/runtime/struct.Builder.html#method.core_threads)
/// method
pub struct OctaneConfig {
    pub keep_alive: Option<Duration>,
    pub static_dir: HashMap<PathBuf, Vec<PathBuf>>,
    pub ssl: Ssl,
    pub has_ssl: bool,
    pub file_404: PathBuf,
    pub worker_threads: Option<usize>,
}

/// Shared config trait which allows us to use the config
/// methods on the Octane server struct too as it has a
/// config field by default
pub trait Config {
    /// Add a directory to server as a static folder
    /// to server files from
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::config::{Config, OctaneConfig};
    ///
    /// let mut config = OctaneConfig::new();
    /// config.add_static_dir("/", "templates");
    /// ```
    ///
    /// Or with Octane struct
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::config::Config;
    ///
    /// let mut app = Octane::new();
    /// app.add_static_dir("/", "templates");
    /// ```
    fn add_static_dir(&mut self, loc: &'static str, dir_name: &'static str);
    /// Sets the keepalive duration for a keepalive request
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::config::{OctaneConfig, Config};
    /// use std::time::Duration;
    ///
    /// let mut config = OctaneConfig::new();
    /// config.set_keepalive(Duration::new(5, 0));
    /// ```
    ///
    /// Or with Octane struct
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use std::time::Duration;
    /// use octane::config::Config;
    ///
    /// let mut app = Octane::new();
    /// app.set_keepalive(Duration::new(5, 0));
    /// ```
    fn set_keepalive(&mut self, duration: Duration);
    /// Sets the path of the file which is to be served
    /// when the server sends a 404 to the client
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::config::{OctaneConfig, Config};
    /// use std::time::Duration;
    ///
    /// let mut config = OctaneConfig::new();
    /// config.set_404_file("templates/error.html");
    /// ```
    ///
    /// Or with Octane struct
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use std::time::Duration;
    /// use octane::config::Config;
    ///
    /// let mut app = Octane::new();
    /// app.set_404_file("templates/error.html");
    /// ```
    fn set_404_file(&mut self, dir_name: &'static str);
    /// Replaces the current ssl config with the one
    /// specified in the arguments
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::config::{OctaneConfig, Config, Ssl};
    /// use std::time::Duration;
    ///
    /// let mut config = OctaneConfig::new();
    /// let mut ssl_config = Ssl::new();
    /// ssl_config.key("templates/key.pem");
    /// config.set_404_file("templates/error.html");
    /// ```
    ///
    /// Or with Octane struct
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use std::time::Duration;
    /// use octane::config::{Config, Ssl};
    ///
    /// let mut app = Octane::new();
    /// let mut ssl_config = Ssl::new();
    /// ssl_config.key("templates/key.pem");
    /// app.with_ssl_config(ssl_config);
    /// ```
    fn with_ssl_config(&mut self, ssl_conf: Ssl);
    /// Returns the Ssl instance of the config and
    /// sets the port number for TLS
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::config::{OctaneConfig, Config};
    /// use std::time::Duration;
    ///
    /// let mut config = OctaneConfig::new();
    /// config
    ///     .ssl(443)
    ///     .key("key.pem")
    ///     .cert("cert.pem");
    /// ```
    ///
    /// Or with Octane struct
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use std::time::Duration;
    /// use octane::config::Config;
    ///
    /// let mut app = Octane::new();
    /// app
    ///     .ssl(443)
    ///     .key("key.pem")
    ///     .cert("cert.pem");
    /// ```
    fn ssl(&mut self, port: u16) -> &mut Ssl;
}
/// Octane config which can be used independently to
/// configure the server settings.
///
/// To apply the settings to the main struct,
/// make sure you run `app.with_config(config);` on the
/// main struct where config is the custom config you created
impl OctaneConfig {
    /// Creates a new config instance with default values
    pub fn new() -> Self {
        OctaneConfig {
            ssl: Ssl::new(),
            has_ssl: false,
            keep_alive: None,
            worker_threads: None,
            static_dir: HashMap::new(),
            file_404: PathBuf::new(),
        }
    }
    /// Appends a settings instance to self
    pub fn append(&mut self, settings: Self) {
        self.ssl = settings.ssl;
        self.keep_alive = settings.keep_alive;
        self.static_dir.extend(settings.static_dir);
    }

    /// Sets the number of worker threads, this is settings
    /// which will be applied to the `core_threads` method
    /// on the runtime builder struct
    /// https://docs.rs/tokio/0.2.13/tokio/runtime/struct.Builder.html#method.core_threads
    pub fn worker_threads(&mut self, threads: usize) -> &mut Self {
        self.worker_threads = Some(threads);
        self
    }

    /// Get the certs as a Vec<Certificate>, a user will not have to
    /// use this directly
    #[cfg(feature = "rustls")]
    pub fn get_cert(&self) -> std::io::Result<Vec<Certificate>> {
        if self.ssl.is_good() {
            let mut buf = std::io::BufReader::new(std::fs::File::open(&self.ssl.cert)?);
            certs(&mut buf)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Certs"))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid Certs",
            ))
        }
    }
    /// Get the private key as a Vec<PrivateKey>, a user will not have to
    /// use this directly
    #[cfg(feature = "rustls")]
    pub fn get_key(&self) -> std::io::Result<Vec<PrivateKey>> {
        use std::io::Read;
        if self.ssl.is_good() {
            let mut buf = std::io::BufReader::new(std::fs::File::open(&self.ssl.key)?);
            pkcs8_private_keys(&mut buf)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Key"))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid Key",
            ))
        }
    }
}

default!(OctaneConfig);
default!(Ssl);

impl Config for OctaneConfig {
    fn set_keepalive(&mut self, duration: Duration) {
        self.keep_alive = Some(duration);
    }
    fn add_static_dir(&mut self, loc: &'static str, dir_name: &'static str) {
        let loc_buf = PathBuf::from(loc);
        let dir_buf = PathBuf::from(dir_name);
        if let Some(paths) = self.static_dir.get_mut(&loc_buf) {
            paths.push(dir_buf)
        } else {
            self.static_dir.insert(loc_buf, vec![dir_buf]);
        }
    }
    fn set_404_file(&mut self, dir_name: &'static str) {
        self.file_404 = PathBuf::from(dir_name);
    }
    fn with_ssl_config(&mut self, ssl_conf: Ssl) {
        self.ssl.key = ssl_conf.key;
        self.ssl.cert = ssl_conf.cert;
    }
    fn ssl(&mut self, port: u16) -> &mut Ssl {
        self.ssl.port = port;
        &mut self.ssl
    }
}
