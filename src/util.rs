#![allow(unused)]

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::str::from_utf8;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub trait SystemTimeExt {
    fn duration_since_epoch(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    fn duration_since_epoch(&self) -> Duration {
        self.duration_since(UNIX_EPOCH).unwrap()
    }
}

pub trait PathExt {
    fn is_empty(&self) -> bool;
}

impl PathExt for Path {
    fn is_empty(&self) -> bool {
        self.as_os_str().is_empty()
    }
}

pub trait ChildExt {
    fn cwd(&self) -> io::Result<PathBuf>;
}

impl ChildExt for Child {
    fn cwd(&self) -> io::Result<PathBuf> {
        process_cwd(self.id())
    }
}

pub fn process_cwd(pid: u32) -> io::Result<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd"))
}

pub fn find_repo_root(path: &Path) -> Option<&Path> {
    path.ancestors().find(|root| root.join(".git").is_dir())
}

pub fn strip_home_dir(path: &Path) -> Option<&Path> {
    static HOME_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

    let home = HOME_DIR.get_or_init(dirs::home_dir).as_deref()?;
    path.strip_prefix(home).ok()
}

pub fn get_vcs_branch(repo_root: &Path) -> io::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()?;

    let stdout = from_utf8(&output.stdout).map_err(|_| io_error(io::ErrorKind::InvalidData))?;

    let branch = stdout.trim().to_owned();
    Ok(branch)
}

pub fn io_error(kind: io::ErrorKind) -> io::Error {
    io::Error::from(kind)
}
