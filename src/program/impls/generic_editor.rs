use std::borrow::Cow;

use crate::rpc::{Activity, Party};
use crate::util::*;

pub struct GenericEditor {
    pub logo: &'static str,
    pub pid: u32,
}

impl GenericEditor {
    pub fn activity(&mut self, activity: &mut Activity) {
        let Ok(cwd) = process_cwd(self.pid) else {
            return;
        };

        let repo_root = find_repo_root(&cwd);

        let workspace = repo_root
            .and_then(|x| x.file_name())
            .map(|x| x.to_string_lossy())
            .or_else(|| {
                strip_home_dir(&cwd).map(|x| {
                    if x.is_empty() {
                        Cow::Borrowed("~/")
                    } else {
                        x.to_string_lossy()
                    }
                })
            })
            .unwrap_or_else(|| cwd.to_string_lossy());

        activity.details = Some(format!("In {}", workspace));
        activity.small.image = Some("edit".to_owned());
        activity.large.image = Some(self.logo.to_owned());
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
