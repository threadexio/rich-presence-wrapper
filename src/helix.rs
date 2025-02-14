use std::borrow::Cow;
use std::io;
use std::path::Path;
use std::process::Stdio;

use tokio::process::{Child, Command};

use crate::rpc::{Activity, ActivityBuilder};
use crate::util::{find_repo_root, strip_home_dir, ChildExt};

pub struct Helix {
    process: Child,
}

impl Helix {
    pub fn new(mut helix: Command) -> io::Result<Self> {
        let process = helix
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        Ok(Self { process })
    }

    pub async fn wait(&mut self) {
        let _ = self.process.wait().await;
    }
}

impl ActivityBuilder for Helix {
    type Error = io::Error;

    fn build(&mut self, activity: Activity) -> Result<Activity, Self::Error> {
        let cwd = self.process.cwd()?;

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
