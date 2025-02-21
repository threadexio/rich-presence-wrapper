use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub trait SystemTimeExt {
    fn duration_since_epoch(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    fn duration_since_epoch(&self) -> Duration {
        self.duration_since(UNIX_EPOCH).unwrap()
    }
}
