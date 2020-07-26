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
/// feature (rustl or openssl)
///
/// By default, the Config struct has an ssl field so
/// you don't have to use it directly but if you want
/// to then you can and then append it to the config
#[derive(Clone)]
pub struct Ssl {
    pub key: PathBuf,
    pub cert: PathBuf,
}

impl Ssl {
    fn new() -> Self {
        Ssl {
            key: PathBuf::new(),
            cert: PathBuf::new(),
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
    pub fn is_good(&self) -> bool {
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
        } else {
            true
        }
    }
}

/// An independent OctaneConfig struct that can be used
/// **Note**: If you won't push the independently made config
/// then they won't take place, make sure to push them
/// to the main server struct like the following
///
/// ```no_run
/// use octane::server::Octane;
/// use octane::config::OctaneConfig;
///
/// let mut app = Octane::new();
/// let mut config = OctaneConfig::new();
/// app.with_config(config);
/// ```
pub struct OctaneConfig {
    pub keep_alive: Option<Duration>,
    pub static_dir: HashMap<PathBuf, Vec<PathBuf>>,
    pub ssl: Ssl,
    pub file_404: PathBuf,
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
    fn with_ssl_config(&mut self, ssl_conf: Ssl);
}

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
}

/// Octane config which can be used independently to
/// configure the server settings.
///
/// To apply the settings to the main struct,
/// make sure you run `app.with_config(config);` on the
/// main struct where config is the custom config you created
impl OctaneConfig {
    /// Creates a new empty config structure which octane
    /// can use to customise determine some values to run the server
    pub fn new() -> Self {
        OctaneConfig {
            ssl: Ssl::new(),
            keep_alive: None,
            static_dir: HashMap::new(),
            file_404: PathBuf::new(),
        }
    }

    pub fn append(&mut self, settings: Self) {
        self.ssl = settings.ssl;
        self.keep_alive = settings.keep_alive;
        self.static_dir.extend(settings.static_dir);
    }

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

    #[cfg(feature = "rustls")]
    pub fn get_key(&self) -> std::io::Result<Vec<PrivateKey>> {
        use std::io::Read;
        if self.ssl.is_good() {
            let mut buf = std::io::BufReader::new(std::fs::File::open(&self.ssl.key)?);
            let mut string = String::new();
            buf.read_to_string(&mut string)?;
            //println!("{:?}", string);
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

impl Default for OctaneConfig {
    fn default() -> Self {
        Self::new()
    }
}
