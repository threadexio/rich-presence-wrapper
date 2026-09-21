#![allow(dead_code)]

use std::time::Duration;

use discord_rich_presence::error::Error as DiscordError;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use eyre::{Result, bail};
use tokio::task::JoinHandle;

use crate::util::backoff::{self, Backoff};
use crate::util::spsc;

pub use discord_rich_presence::activity::*;

///////////////////////////////////////////////////////////////////////////////

pub struct Discord {
    tx: spsc::Sender<Message>,
    task: Option<JoinHandle<Result<()>>>,
}

pub struct Builder<ClientId> {
    pub client_id: ClientId,
}

impl Discord {
    pub fn builder() -> Builder<()> {
        Builder { client_id: () }
    }
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

        let (tx, rx) = spsc::new();

        let inner = DiscordIpcClient::new(client_id);
        let task = tokio::task::spawn_blocking(move || Task { inner, rx }.run());

        Discord {
            tx,
            task: Some(task),
        }
    }
}

impl Discord {
    pub async fn set_activity(
        &mut self,
        activity: impl Into<Box<Activity<'static>>>,
    ) -> Result<()> {
        let activity = activity.into();
        self.send(Message::SetActivity { activity }).await
    }

    pub async fn clear_activity(&mut self) -> Result<()> {
        self.send(Message::ClearActivity).await
    }

    async fn send(&mut self, m: Message) -> Result<()> {
        match self.tx.send(m) {
            Ok(()) => Ok(()),

            Err(_) => match self.task.take() {
                Some(task) => task.await.map_err(Into::into).flatten(),
                None => bail!("errored previously"),
            },
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

enum Message {
    SetActivity { activity: Box<Activity<'static>> },
    ClearActivity,
}

struct Task {
    inner: DiscordIpcClient,
    rx: spsc::Receiver<Message>,
}

impl Task {
    fn run(&mut self) -> Result<()> {
        loop {
            match self.rx.blocking_recv() {
                Some(Message::SetActivity { activity }) => {
                    self.handle_set_activity(activity)?;
                }

                Some(Message::ClearActivity) => {
                    self.handle_clear_activity()?;
                }

                None => return Ok(()),
            }
        }
    }

    fn handle_set_activity(&mut self, activity: Box<Activity<'static>>) -> Result<()> {
        self.execute(|me| me.inner.set_activity(*activity.clone()))
    }

    fn handle_clear_activity(&mut self) -> Result<()> {
        trace!("clear activity");
        self.execute(|me| me.inner.clear_activity())
    }

    fn execute<O>(&mut self, mut f: impl FnMut(&mut Self) -> Result<O, DiscordError>) -> Result<O> {
        loop {
            match f(self) {
                Ok(output) => return Ok(output),

                Err(e) if is_retryable(&e) => {
                    debug!("ipc error: {e}");
                    self.reconnect()?;
                }

                Err(e) => return Err(e.into()),
            }
        }
    }

    fn reconnect(&mut self) -> Result<()> {
        let _ = self.inner.close();

        let mut backoff = Backoff::new(backoff::Max::new(
            Duration::from_secs(10),
            backoff::Min::new(Duration::from_secs(1), backoff::Exponential::new(1.5)),
        ));

        let mut i: u64 = 1;
        loop {
            trace!("reconnection attempt #{i}");
            i += 1;

            match self.inner.connect() {
                Ok(()) => {
                    trace!("connected");
                    break Ok(());
                }
                Err(e) if is_retryable(&e) => backoff.blocking_sleep(),
                Err(e) => break Err(e.into()),
            }
        }
    }
}

fn is_retryable(err: &DiscordError) -> bool {
    matches!(
        err,
        DiscordError::NotConnected
            | DiscordError::IPCConnectionFailed
            | DiscordError::IPCNotFound
            | DiscordError::ReadError(_)
            | DiscordError::WriteError(_)
            | DiscordError::FlushError(_)
    )
}
