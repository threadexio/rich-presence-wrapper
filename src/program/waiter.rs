use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::Child;
use std::process::ExitStatus;

use nix::libc::pid_t;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;

#[derive(Clone)]
pub struct Waiter(Pid);

impl From<&'_ Child> for Waiter {
    fn from(process: &'_ Child) -> Self {
        let pid = Pid::from_raw(process.id() as pid_t);
        Self(pid)
    }
}

impl Waiter {
    pub fn wait(self) -> io::Result<ExitStatus> {
        loop {
            match waitpid(self.0, None) {
                Ok(WaitStatus::Exited(_, status)) => return Ok(ExitStatus::from_raw(status)),
                Ok(_) => continue,
                Err(e) => return Err(io::Error::from_raw_os_error(e as i32)),
            }
        }
    }
}
