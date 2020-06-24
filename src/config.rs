use crate::path::PathBuf;
use core::time::Duration;

pub struct OctaneConfig {
    pub keep_alive: Option<Duration>,
    pub static_dir: Vec<(Option<PathBuf>, String)>,
}

impl OctaneConfig {
    pub fn new() -> Self {
        OctaneConfig {
            keep_alive: None,
            static_dir: Vec::new(),
        }
    }

    pub fn set_keepalive(&mut self, duration: Duration) -> &mut Self {
        self.keep_alive = Some(duration);
        self
    }
    pub fn set_static_dir(&mut self, duration: Duration) -> &mut Self {
        self.keep_alive = Some(duration);
        self
    }
}

impl Default for OctaneConfig {
    fn default() -> Self {
        Self::new()
    }
}
