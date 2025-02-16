use std::sync::{Arc, Barrier};
use std::time::SystemTime;

use discord_presence::Client;
use discord_presence::DiscordError;
use thiserror::Error;

pub use discord_presence::models::Activity;

use crate::util::{hash, SystemTimeExt};

const MAX_MISSED_UPDATES: usize = 2;

pub trait ActivityBuilder {
    type Error;

    fn build(&mut self, activity: Activity) -> Result<Activity, Self::Error>;
}

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("already connected")]
    AlreadyConnected,
}

#[derive(Debug, Error)]
pub enum UpdateError<A> {
    #[error("not connected")]
    NotConnected,

    #[error(transparent)]
    Rpc(DiscordError),

    #[error(transparent)]
    Activity(A),
}

struct Inner {
    rpc: Client,
    missed_updates: usize,
    last_activity_hash: Option<u64>,
}

pub struct Rpc {
    inner: Option<Inner>,
    start: SystemTime,
}

impl Rpc {
    pub fn new() -> Self {
        let start = SystemTime::now();

        Self { inner: None, start }
    }

    pub fn connect(&mut self, id: u64) -> Result<(), ConnectError> {
        if self.inner.is_some() {
            return Err(ConnectError::AlreadyConnected);
        }

        let mut rpc = Client::new(id);
        let ready = Arc::new(Barrier::new(2));

        let _ready = rpc.on_ready({
            let ready = ready.clone();
            move |_| {
                ready.wait();
            }
        });

        rpc.start();
        ready.wait();

        self.inner = Some(Inner {
            rpc,
            missed_updates: 0,
            last_activity_hash: None,
        });

        Ok(())
    }

    pub fn update<A>(&mut self, builder: &mut A) -> Result<(), UpdateError<A::Error>>
    where
        A: ActivityBuilder,
    {
        let Some(ref mut inner) = self.inner else {
            return Err(UpdateError::NotConnected);
        };

        let activity = builder
            .build(Activity::new())
            .map_err(UpdateError::Activity)?;

        let hash = hash(&activity);

        match inner.last_activity_hash {
            Some(x) if x != hash => self.do_update(activity).map_err(UpdateError::Rpc),
            Some(_) if inner.missed_updates >= MAX_MISSED_UPDATES => {
                self.do_update(activity).map_err(UpdateError::Rpc)
            }
            Some(_) => {
                inner.missed_updates += 1;
                Ok(())
            }
            None => {
                inner.last_activity_hash = Some(hash);
                self.do_update(activity).map_err(UpdateError::Rpc)
            }
        }
    }

    fn do_update(&mut self, activity: Activity) -> Result<(), DiscordError> {
        let inner = self.inner.as_mut().unwrap();

        inner
            .rpc
            .set_activity(|_| {
                activity.timestamps(|x| x.start(self.start.duration_since_epoch().as_secs()))
            })
            .map(|_| ())
    }
}
