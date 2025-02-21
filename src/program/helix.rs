use crate::rpc::{Activity, App, Party};
use crate::util::*;

pub struct Helix {
    pid: u32,
}

pub fn new(pid: u32) -> Helix {
    Helix { pid }
}

impl App for Helix {
    fn id(&self) -> u64 {
        1339918035842105417
    }

    fn activity(&mut self, activity: &mut Activity) {
        let Ok(cwd) = get_process_cwd(self.pid) else {
            return;
        };

        let repo_root = find_repo_root(&cwd);

        let workspace = repo_root
            .and_then(|x| x.file_name())
            .map(|x| x.to_string_lossy())
            .or_else(|| strip_home_dir(&cwd).map(|x| x.to_string_lossy()))
            .unwrap_or_else(|| cwd.to_string_lossy());

        activity.details = Some(format!("In {}", workspace));
        activity.small.image = Some("edit".to_owned());
        activity.large.image = Some("helix-logo".to_owned());
        activity.party = Some(Party {
            size: 1,
            capacity: 1,
        });

        if let Some(root) = repo_root {
            if let Ok(branch) = get_vcs_branch(root) {
                activity.state = Some(branch);
            }
        }
    }
}
