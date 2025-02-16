use std::{
    ffi::{OsStr, OsString},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub trait SystemTimeExt {
    fn duration_since_epoch(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    fn duration_since_epoch(&self) -> Duration {
        self.duration_since(UNIX_EPOCH).unwrap()
    }
}

pub fn env<K, V>(var: K, default: impl FnOnce() -> V) -> OsString
where
    K: AsRef<OsStr>,
    V: Into<OsString>,
{
    std::env::var_os(var).unwrap_or_else(|| default().into())
}
