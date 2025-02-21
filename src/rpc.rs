use std::sync::{Arc, Barrier};
use std::time::SystemTime;

use discord_presence::models::Activity as DiscordActivity;
use discord_presence::{Client, DiscordError};
use thiserror::Error;

use crate::util::SystemTimeExt;

#[derive(Debug, Default, Clone)]
pub struct Asset {
    pub text: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Party {
    pub size: u32,
    pub capacity: u32,
}

#[derive(Debug, Default, Clone)]
pub struct Activity {
    pub details: Option<String>,
    pub state: Option<String>,

    pub small: Asset,
    pub large: Asset,

    pub party: Option<Party>,
}

fn to_discord_activity(p: Activity, mut activity: DiscordActivity) -> DiscordActivity {
    if let Some(value) = p.details {
        activity = activity.details(value);
    }

    if let Some(value) = p.state {
        activity = activity.state(value);
    }

    activity = activity.assets(|mut x| {
        if let Some(value) = p.small.text {
            x = x.small_text(value);
        }

        if let Some(value) = p.small.image {
            x = x.small_image(value);
        }

        if let Some(value) = p.large.text {
            x = x.large_text(value);
        }

        if let Some(value) = p.large.image {
            x = x.large_image(value);
        }

        x
    });

    activity = activity.party(|mut x| {
        if let Some(party) = p.party {
            x = x.size((party.size, party.capacity));
        }

        x
    });

    activity
}

pub trait App {
    fn id(&self) -> u64;

    fn activity(&mut self, activity: &mut Activity);
}

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("already connected")]
    AlreadyConnected,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("not connected")]
    NotConnected,

    #[error(transparent)]
    Rpc(DiscordError),
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

    pub fn update<A>(&mut self, builder: &mut A) -> Result<(), UpdateError>
    where
        A: App + ?Sized,
    {
        let Some(ref mut rpc) = self.rpc else {
            return Err(UpdateError::NotConnected);
        };

        let mut p = Activity::default();
        builder.activity(&mut p);

        rpc.set_activity(|x| {
            to_discord_activity(p, x)
                .timestamps(|x| x.start(self.start.duration_since_epoch().as_secs()))
        })
        .map_err(UpdateError::Rpc)?;

        Ok(())
    }
}
