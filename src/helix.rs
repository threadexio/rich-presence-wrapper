use std::borrow::Cow;
use std::io;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};

use crate::rpc::{Activity, ActivityBuilder};
use crate::util::{find_repo_root, get_process_cwd, strip_home_dir};

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

        let activity = activity
            .details(format!("In {}", workspace(&cwd)))
            .assets(|x| x.large_image("helix-logo").small_image("edit"))
            .party(|x| x.size((1, 1)))
            .instance(true);

        Ok(activity)
    }
}

fn workspace(path: &Path) -> Cow<'_, str> {
    let repo_root = || {
        find_repo_root(path)
            .and_then(|x| x.file_name())
            .map(|x| x.to_string_lossy())
    };

    let stripped_home = || strip_home_dir(path).map(|x| x.to_string_lossy());

    repo_root()
        .or_else(stripped_home)
        .unwrap_or_else(|| path.to_string_lossy())
}
