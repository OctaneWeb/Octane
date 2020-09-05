use crate::constants::closures_lock;
use crate::default;
use colored::*;
use core::time::Duration;
use std::path::PathBuf;
#[cfg(feature = "rustls")]
use tokio_rustls::rustls::{
    internal::pemfile::{certs, rsa_private_keys},
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
    pub ssl: Ssl,
    worker_threads: Option<usize>,
    pub extensions: Vec<Extensions>,
}

pub enum Extensions {
    WebSockets,
}
/// Shared config trait which allows us to use the config
/// methods on the Octane server struct too as it has a
/// config field by default
pub trait Config {
    /// Sets the keepalive duration for a keepalive request,
    /// by default, a 5 second keep alive is set
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
            keep_alive: Some(Duration::from_secs(5)),
            worker_threads: None,
            extensions: Vec::new(),
        }
    }
    /// Appends a settings instance to self
    pub fn append(&mut self, settings: Self) {
        self.ssl = settings.ssl;
        self.keep_alive = settings.keep_alive;
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
    /// use this directly, this is used and done for them
    #[cfg(feature = "rustls")]
    pub fn get_cert(&self) -> std::io::Result<Vec<Certificate>> {
        self.ssl.validate();
        let mut buf = std::io::BufReader::new(std::fs::File::open(&self.ssl.cert)?);
        certs(&mut buf)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Certs"))
    }
    /// Get the private key as a Vec<PrivateKey>, a user will not have to
    /// use this directly
    #[cfg(feature = "rustls")]
    pub fn get_key(&self) -> std::io::Result<Vec<PrivateKey>> {
        self.ssl.validate();
        let mut buf = std::io::BufReader::new(std::fs::File::open(&self.ssl.key)?);
        rsa_private_keys(&mut buf)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Key"))
    }

    pub fn add_extension(&mut self, ext: Extensions) -> &mut Self {
        self.extensions.push(ext);
        self
    }

    pub fn startup_string(&self, ssl: bool, port: u16) -> String {
        let mut final_string = String::new();
        final_string.push_str(
            format!(
                "\n\r{} {}\n\r\n{}\n\n",
                "Starting".bold().blue(),
                "Octane".green().bold(),
                "Configurations".red().bold()
            )
            .as_str(),
        );
        if let Some(x) = self.keep_alive {
            final_string.push_str(
                format!(
                    "{}: {}s\n",
                    "-> Keep-alive".blue(),
                    x.as_secs_f64().to_string().green(),
                )
                .as_str(),
            );
        } else {
            final_string.push_str(
                format!("{}: {}\n", "-> Keep-alive".blue(), "Disabled".green(),).as_str(),
            );
        }
        if let Some(x) = self.worker_threads {
            final_string.push_str(
                format!(
                    "{}: {}\n",
                    "-> Worker-threads".blue(),
                    x.to_string().green(),
                )
                .as_str(),
            );
        } else {
            final_string.push_str(
                format!(
                    "{}: {}\n",
                    "-> Worker-threads".blue(),
                    "2 * Number of cores available in the CPU".green(),
                )
                .as_str(),
            );
        }
        if ssl {
            final_string.push_str(
                format!(
                    "{}: {} {}\n",
                    "-> TLS".blue(),
                    "enabled at".green(),
                    self.ssl.port.to_string().red().bold()
                )
                .as_str(),
            );
        } else {
            final_string.push_str(format!("{}: {} \n", "TLS".red(), "disabled".green()).as_str());
        }
        closures_lock(|map| {
            final_string.push_str(
                format!(
                    "{}: {} paths\n",
                    "-> Serving".blue(),
                    map.len().to_string().red().bold()
                )
                .as_str(),
            );
        });
        final_string.push_str(
            format!(
                "\n{} at {}:{}\n",
                "Listening".red(),
                "localhost".blue(),
                port.to_string().red().bold()
            )
            .as_str(),
        );

        final_string
    }
}

default!(OctaneConfig);
default!(Ssl);

impl Config for OctaneConfig {
    fn set_keepalive(&mut self, duration: Duration) {
        self.keep_alive = Some(duration);
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
