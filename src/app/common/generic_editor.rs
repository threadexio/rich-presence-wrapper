use std::path::Path;
use std::time::{Duration, SystemTime};

use eyre::Result;
use tokio::time::sleep;

use crate::discord::*;
use crate::platform::ChildHandle;
use crate::util::{Never, SystemTimeExt, find_repo_root, get_vcs_branch, home_dir};

pub struct GenericEditor {
    pub discord: Discord,
    pub editor: ChildHandle,
    pub name: &'static str,
    pub logo: &'static str,
    pub options: Options,
}

pub struct Options {
    pub refresh_delay: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            refresh_delay: Duration::from_secs(5),
        }
    }
}

impl GenericEditor {
    pub async fn run(&mut self) -> Result<Never> {
        let start = SystemTime::now();

        loop {
            let mut activity = Activity::new()
                .name(self.name)
                .assets(Assets::new().large_image(self.logo))
                .timestamps(Timestamps::new().start(start.duration_since_epoch().as_secs() as i64))
                .activity_type(ActivityType::Playing)
                .party(Party::new().size([1, 1]));

            if let Ok(cwd) = self.editor.cwd() {
                let workspace = None
                    .or_else(|| {
                        find_repo_root(&cwd)
                            .and_then(|x| x.file_name())
                            .map(|x| x.to_owned())
                    })
                    .or_else(|| {
                        home_dir().map(|home| match cwd.strip_prefix(home) {
                            Ok(x) => Path::new("~").join(x).into_os_string(),
                            Err(_) => cwd.as_os_str().to_owned(),
                        })
                    })
                    .unwrap_or_else(|| cwd.as_os_str().to_owned());

                activity = activity.details(format!("In {}", workspace.display()));

                if let Ok(Some(vcs_branch)) = get_vcs_branch(&cwd) {
                    activity = activity.state(vcs_branch);
                }
            }

            self.discord.set_activity(activity).await?;
            sleep(self.options.refresh_delay).await;
        }
    }
}
