use std::io;

use super::*;

impl ChildExt for tokio::process::Child {
    fn handle(&self) -> Option<super::ChildHandle> {
        Some(super::ChildHandle(ChildHandle { pid: self.id()? }))
    }
}

#[derive(Clone)]
pub struct ChildHandle {
    pid: u32
}

impl ChildHandle {
    pub fn cwd(&self) -> io::Result<PathBuf> {
        darwin_libproc::pid_cwd(u32::cast_signed(self.pid))
    }
}