use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::str::from_utf8;
use std::sync::OnceLock;

use crate::rpc::{Activity, ActivityBuilder};

pub struct Helix {
    process: Child,
}

impl Helix {
    pub fn new(mut helix: Command) -> io::Result<Self> {
        let process = helix
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(Self { process })
    }

    pub fn wait(mut self) -> io::Result<ExitStatus> {
        self.process.wait()
    }

    pub fn activity_builder(&self) -> HelixActivity {
        let id = self.process.id();

        HelixActivity { id }
    }
}

pub struct HelixActivity {
    id: u32,
}

impl ActivityBuilder for HelixActivity {
    type Error = io::Error;

    fn build(&mut self, activity: Activity) -> Result<Activity, Self::Error> {
        let cwd = get_process_cwd(self.id)?;

        let repo_root = find_repo_root(&cwd);

        let workspace = repo_root
            .and_then(|x| x.file_name())
            .map(|x| x.to_string_lossy())
            .or_else(|| strip_home_dir(&cwd).map(|x| x.to_string_lossy()))
            .unwrap_or_else(|| cwd.to_string_lossy());

        let mut activity = activity
            .details(format!("In {}", workspace))
            .assets(|x| x.large_image("helix-logo").small_image("edit"))
            .party(|x| x.size((1, 1)))
            .instance(true);

        if let Some(root) = repo_root {
            let branch = get_vcs_branch(root)?;
            activity = activity.state(branch);
        }

        Ok(activity)
    }
}

fn get_process_cwd(id: u32) -> io::Result<PathBuf> {
    std::fs::read_link(format!("/proc/{id}/cwd"))
}

fn find_repo_root(path: &Path) -> Option<&Path> {
    path.ancestors().find(|root| root.join(".git").is_dir())
}

fn strip_home_dir(path: &Path) -> Option<&Path> {
    static HOME_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

    let home = HOME_DIR.get_or_init(dirs::home_dir).as_deref()?;
    path.strip_prefix(home).ok()
}

fn get_vcs_branch(repo_root: &Path) -> io::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()?;

    let stdout = from_utf8(&output.stdout).map_err(|_| io_error(io::ErrorKind::InvalidData))?;

    let branch = stdout.trim().to_owned();
    Ok(branch)
}

fn io_error(kind: io::ErrorKind) -> io::Error {
    io::Error::from(kind)
}
