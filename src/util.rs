use std::ffi::{OsStr, OsString};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::io;
use tokio::process::Child;

pub trait ChildExt {
    fn cwd(&self) -> io::Result<PathBuf>;
}

impl ChildExt for Child {
    fn cwd(&self) -> io::Result<PathBuf> {
        let id = self
            .id()
            .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?;

        std::fs::read_link(format!("/proc/{id}/cwd"))
    }
}

pub trait SystemTimeExt {
    fn duration_since_epoch(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    fn duration_since_epoch(&self) -> Duration {
        self.duration_since(UNIX_EPOCH).unwrap()
    }
}

pub fn find_repo_root(path: &Path) -> Option<&Path> {
    path.ancestors().find(|root| root.join(".git").is_dir())
}

pub fn strip_home_dir(path: &Path) -> Option<&Path> {
    static HOME_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

    let home = HOME_DIR.get_or_init(dirs::home_dir).as_deref()?;
    path.strip_prefix(home).ok()
}

pub fn hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub fn env<K, V>(var: K, default: impl FnOnce() -> V) -> OsString
where
    K: AsRef<OsStr>,
    V: Into<OsString>,
{
    std::env::var_os(var).unwrap_or_else(|| default().into())
}
