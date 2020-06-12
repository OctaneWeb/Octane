use core::time::Duration;

pub struct OctaneConfig {
    pub keep_alive: Option<Duration>,
}

impl OctaneConfig {
    pub fn new() -> Self {
        OctaneConfig { keep_alive: None }
    }

    pub fn set_keepalive(&mut self, duration: Duration) -> &mut Self {
        self.keep_alive = Some(duration);
        self
    }
}
