use std::sync::{Arc, Barrier};
use std::time::SystemTime;

use discord_presence::Client;
use discord_presence::DiscordError;
use thiserror::Error;

pub use discord_presence::models::Activity;

use crate::util::SystemTimeExt;

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

pub struct Rpc {
    rpc: Option<Client>,
    start: SystemTime,
}

impl Rpc {
    pub fn new() -> Self {
        let start = SystemTime::now();

        Self { rpc: None, start }
    }

    pub fn connect(&mut self, id: u64) -> Result<(), ConnectError> {
        if self.rpc.is_some() {
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

        self.rpc = Some(rpc);
        Ok(())
    }

    pub fn update<A>(&mut self, builder: &mut A) -> Result<(), UpdateError<A::Error>>
    where
        A: ActivityBuilder,
    {
        let Some(ref mut rpc) = self.rpc else {
            return Err(UpdateError::NotConnected);
        };

        let activity = builder
            .build(Activity::new())
            .map_err(UpdateError::Activity)?;

        rpc.set_activity(|_| {
            activity.timestamps(|x| x.start(self.start.duration_since_epoch().as_secs()))
        })
        .map_err(UpdateError::Rpc)?;

        Ok(())
    }
}
