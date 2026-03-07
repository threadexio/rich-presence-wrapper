use std::cmp::min;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::process::Command;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum Never {}

///////////////////////////////////////////////////////////////////////////////

pub trait SystemTimeExt {
    fn duration_since_epoch(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    fn duration_since_epoch(&self) -> Duration {
        self.duration_since(UNIX_EPOCH).unwrap()
    }
}

///////////////////////////////////////////////////////////////////////////////

pub struct Backoff {
    delay: Duration,
    max: Duration,
    factor: f32,
}

impl Backoff {
    pub fn new(initial: Duration, max: Duration, factor: f32) -> Self {
        Self {
            delay: initial,
            max,
            factor,
        }
    }

    pub fn advance(&mut self) {
        let new_delay = self.delay.mul_f32(self.factor);
        self.delay = min(self.max, new_delay);
    }

    pub fn get(&self) -> Duration {
        self.delay
    }

    pub fn blocking_sleep(&mut self) {
        std::thread::sleep(self.delay);
        self.advance();
    }

    pub async fn sleep(&mut self) {
        tokio::time::sleep(self.delay).await;
        self.advance();
    }
}

///////////////////////////////////////////////////////////////////////////////

pub fn process_cwd(pid: u32) -> io::Result<PathBuf> {
    let mut path = PathBuf::with_capacity(32);
    path.push("/proc");
    path.push(format!("{}", pid));
    path.push("cwd");
    std::fs::read_link(path)
}

pub fn home_dir() -> Option<&'static Path> {
    static HOME_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    HOME_DIR.get_or_init(dirs::home_dir).as_deref()
}

pub fn basename(x: &OsStr) -> Option<&OsStr> {
    Path::new(x).file_name()
}

pub fn find_repo_root(in_repo: &Path) -> Option<&Path> {
    in_repo.ancestors().find(|p| p.join(".git").is_dir())
}

pub async fn get_vcs_branch(repo: &Path) -> io::Result<Option<String>> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .await?;

    let stdout = str::from_utf8(&output.stdout)
        .map_err(|_| invalid_data())?
        .trim();

    if stdout.is_empty() {
        return Ok(None);
    }

    let branch = stdout.to_owned();
    Ok(Some(branch))
}

fn invalid_data() -> io::Error {
    io::Error::from(io::ErrorKind::InvalidData)
}
