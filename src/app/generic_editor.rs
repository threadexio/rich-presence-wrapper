use std::path::Path;
use std::time::{Duration, SystemTime};

use eyre::Context;
use tokio::time::sleep;

use super::prelude::*;

pub struct GenericEditor<'a> {
    pub ipc: &'a mut Ipc,
    pub pid: u32,
    pub name: &'static str,
    pub logo: &'static str,
}

impl GenericEditor<'_> {
    pub async fn run(&mut self) -> Result<Never> {
        let start = SystemTime::now();

        loop {
            let cwd = process_cwd(self.pid).context("failed to get cwd")?;

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

            let mut activity = Activity::new()
                .name(self.name)
                .details(format!("In {}", workspace.display()))
                .assets(Assets::new().large_image(self.logo))
                .timestamps(Timestamps::new().start(start.duration_since_epoch().as_secs() as i64))
                .activity_type(ActivityType::Playing)
                .party(Party::new().size([1, 1]));

            if let Some(vcs_branch) = get_vcs_branch(&cwd).await? {
                activity = activity.state(vcs_branch);
            }

            self.ipc.set_activity(activity).await?;

            sleep(Duration::from_secs(5)).await;
        }
    }
}
