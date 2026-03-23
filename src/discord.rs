#![allow(dead_code)]

use std::time::Duration;

use discord_rich_presence::error::Error as DiscordError;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use eyre::{bail, Result};
use tokio::sync::{mpsc, oneshot};

pub use discord_rich_presence::activity::*;

use crate::util::{Backoff, Never};

pub struct Builder<ClientId> {
    pub client_id: ClientId,
}

impl<ClientId> Builder<ClientId> {
    pub fn client_id<T>(self, client_id: T) -> Builder<T> {
        Builder { client_id }
    }
}

impl<T> Builder<T>
where
    T: AsRef<str>,
{
    pub fn finish(self) -> Discord {
        let Self { client_id } = self;
        let client_id = client_id.as_ref();

        let (tx, rx) = mpsc::channel(16);
        let (error_tx, error_rx) = oneshot::channel();

        let mut task = DiscordTask {
            inner: DiscordIpcClient::new(client_id),
            rx,
        };

        tokio::task::spawn_blocking(move || {
            let err = task.run().unwrap_err();
            let _ = error_tx.send(err);
        });

        Discord {
            tx,
            error: Some(error_rx),
        }
    }
}

pub struct Discord {
    tx: mpsc::Sender<DiscordMessage>,
    error: Option<oneshot::Receiver<eyre::Error>>,
}

impl Discord {
    pub fn builder() -> Builder<()> {
        Builder { client_id: () }
    }

    pub async fn set_activity(
        &mut self,
        activity: impl Into<Box<Activity<'static>>>,
    ) -> Result<()> {
        let activity = activity.into();
        self.send(DiscordMessage::SetActivity { activity }).await
    }

    pub async fn clear_activity(&mut self) -> Result<()> {
        self.send(DiscordMessage::ClearActivity).await
    }

    async fn send(&mut self, m: DiscordMessage) -> Result<()> {
        let Err(_) = self.tx.send(m).await else {
            return Ok(());
        };

        let error = self.error.take().expect("error thrown previously");
        let error = error
            .await
            .expect("a dead IPC task should always return an error");

        Err(error)
    }
}

enum DiscordMessage {
    SetActivity { activity: Box<Activity<'static>> },
    ClearActivity,
}

struct DiscordTask {
    inner: DiscordIpcClient,
    rx: mpsc::Receiver<DiscordMessage>,
}

impl DiscordTask {
    fn run(&mut self) -> Result<Never> {
        loop {
            match self.rx.blocking_recv() {
                Some(DiscordMessage::SetActivity { activity }) => {
                    self.handle_set_activity(activity)?;
                }

                Some(DiscordMessage::ClearActivity) => {
                    self.handle_clear_activity()?;
                }

                None => bail!("ipc closed"),
            }
        }
    }

    fn handle_set_activity(&mut self, activity: Box<Activity<'static>>) -> Result<()> {
        self.execute(|me| me.inner.set_activity(*activity.clone()))
    }

    fn handle_clear_activity(&mut self) -> Result<()> {
        self.execute(|me| me.inner.clear_activity())
    }

    fn execute<O>(&mut self, mut f: impl FnMut(&mut Self) -> Result<O, DiscordError>) -> Result<O> {
        loop {
            match f(self) {
                Ok(output) => return Ok(output),

                Err(
                    DiscordError::NotConnected
                    | DiscordError::IPCConnectionFailed
                    | DiscordError::IPCNotFound,
                ) => {
                    self.connect()?;
                }

                Err(
                    DiscordError::ReadError(_)
                    | DiscordError::WriteError(_)
                    | DiscordError::FlushError(_),
                ) => {
                    let _ = self.inner.close();
                    self.connect()?;
                }

                Err(e) => return Err(e.into()),
            }
        }
    }

    fn connect(&mut self) -> Result<()> {
        let mut backoff = Backoff::new(Duration::from_secs(5), Duration::from_secs(30), 1.5);

        loop {
            match self.inner.connect() {
                Ok(()) => return Ok(()),
                Err(DiscordError::IPCNotFound | DiscordError::IPCConnectionFailed) => {
                    backoff.blocking_sleep()
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}
