use std::fs;
use std::io;
use std::path::PathBuf;

use super::*;

impl ChildExt for tokio::process::Child {
    fn handle(&self) -> Option<super::ChildHandle> {
        Some(super::ChildHandle(ChildHandle { pid: self.id()? }))
    }
}

#[derive(Clone)]
pub struct ChildHandle {
    pid: u32,
}

impl ChildHandle {
    pub fn cwd(&self) -> io::Result<PathBuf> {
        match fs::read_link(format!("/proc/{}/cwd", self.pid)) {
            Ok(x) => Ok(x),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(io::Error::other("child died")),
            Err(e) => Err(e),
        }
    }
}
