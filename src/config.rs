use core::time::Duration;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

#[cfg(feature = "rustls")]
use tokio_rustls::rustls::{
    internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys},
    Certificate, PrivateKey,
};

/// SSL struct to manage and validate key and certs
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
    pub fn key(&mut self, path: &str) -> &mut Self {
        self.key = PathBuf::from(path);
        self
    }
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
            .unwrap();
        let cert_ext = self
            .cert
            .as_path()
            .extension()
            .and_then(OsStr::to_str)
            .unwrap();
        if key_ext != "pem" && cert_ext != "pem" {
            panic!("Invalid key file, {:?}", "bad extension");
        } else {
            return true;
        }
    }
}

/// An independent OctaneConfig struct that can be used
/// **Note**: If you won't push the independently made config
/// then they won't take place, make sure to push them
/// to the main server struct
/// ```rust,no_run
/// let mut app = Octane::new();
/// let mut config = OctaneConfig::new();
/// app.with_config(config);
/// ```
pub struct OctaneConfig {
    pub keep_alive: Option<Duration>,
    pub static_dir: HashMap<PathBuf, Vec<PathBuf>>,
    pub ssl: Ssl,
}

/// Shared config trait which allows us to use these
/// methods on the Octane (main server struct) too.
pub trait Config {
    /// Add a directory to server as a static folder
    /// to server files from
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut config = OctaneConfig::new();
    /// config.add_static_dir("/", "templates");
    /// ```
    ///
    /// Or with Octane struct
    ///
    /// ```rust,no_run
    /// let mut app = Octane::new();
    /// app.add_static_dir("/", "templates");
    /// ```
    fn add_static_dir(&mut self, loc: &'static str, dir_name: &'static str);
    /// Sets the keepalive duration for a keepalive request
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let mut config = OctaneConfig::new();
    /// config.set_keepalive(Duration::new(5, 0));
    /// ```
    ///
    /// Or with Octane struct
    ///
    /// ```rust,no_run
    /// let mut app = Octane::new();
    /// app.set_keepalive(Duration::new(5, 0));
    /// ```
    fn set_keepalive(&mut self, duration: Duration);
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
}

/// Octane config which can be used independently to
/// configure the server settings.
///
/// To apply the settings to the main struct,
/// make sure you run `app.with_config(config);` on the
/// main struct where config is the custom config you created
impl OctaneConfig {
    pub fn new() -> Self {
        OctaneConfig {
            ssl: Ssl::new(),
            keep_alive: None,
            static_dir: HashMap::new(),
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
