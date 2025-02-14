use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use discord_presence::Client;
use discord_presence::DiscordError;
use thiserror::Error;

pub use discord_presence::models::Activity;

use crate::util::{hash, SystemTimeExt};

pub trait ActivityBuilder {
    type Error;

    fn build(&mut self, activity: Activity) -> Result<Activity, Self::Error>;
}

#[derive(Debug, Error)]
pub enum UpdateError<A>
where
    A: ActivityBuilder,
{
    #[error(transparent)]
    Rpc(DiscordError),

    #[error(transparent)]
    Activity(A::Error),
}

pub struct Rpc {
    rpc: Client,
    start: SystemTime,
    last_activity_hash: Option<u64>,
}

impl Rpc {
    pub async fn new(id: u64) -> Self {
        let start = SystemTime::now();

        let rpc = tokio::task::spawn_blocking(move || {
            let mut rpc = Client::new(id);
            let ready = Arc::new(AtomicBool::new(false));

            let _ready = rpc.on_ready({
                let ready = ready.clone();
                move |_| ready.store(true, Ordering::Relaxed)
            });

            rpc.start();

            while !ready.load(Ordering::Relaxed) {
                sleep(Duration::from_secs(1));
            }

            rpc
        })
        .await
        .unwrap();

        Self {
            rpc,
            start,
            last_activity_hash: None,
        }
    }

    pub async fn update<A>(&mut self, builder: &mut A) -> Result<(), UpdateError<A>>
    where
        A: ActivityBuilder,
    {
        let activity = builder
            .build(Activity::new())
            .map_err(UpdateError::Activity)?;

        let hash = hash(&activity);

        match self.last_activity_hash {
            Some(x) if x != hash => self.do_update(activity).map_err(UpdateError::Rpc),
            Some(_) => Ok(()),
            None => {
                self.last_activity_hash = Some(hash);
                self.do_update(activity).map_err(UpdateError::Rpc)
            }
        }
    }

    fn do_update(&mut self, activity: Activity) -> Result<(), DiscordError> {
        self.rpc
            .set_activity(|_| {
                activity.timestamps(|x| x.start(self.start.duration_since_epoch().as_secs()))
            })
            .map(|_| ())
    }
}
