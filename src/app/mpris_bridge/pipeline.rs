use std::{marker::PhantomData, mem::replace};

use eyre::Result;
use tokio::{sync::watch, task::JoinSet};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Source<T>(watch::Receiver<Option<T>>);

impl<T> Source<T>
where
    T: Clone,
{
    pub async fn pull(&mut self) -> Option<T> {
        loop {
            if self.0.changed().await.is_err() {
                break None;
            }

            match &*self.0.borrow_and_update() {
                Some(item) => break Some(item.clone()),
                None => continue,
            }
        }
    }
}

#[derive(Debug)]
pub struct Sink<T>(watch::Sender<Option<T>>);

impl<T> Sink<T> {
    pub fn push(&mut self, item: T) -> bool {
        self.0.send(Some(item)).is_ok()
    }
}

pub fn pipe<T>() -> (Sink<T>, Source<T>) {
    let (tx, rx) = watch::channel(None);
    (Sink(tx), Source(rx))
}

///////////////////////////////////////////////////////////////////////////////

pub trait StageBuilder<T> {
    type Stage: Stage<T>;

    fn build(self, sink: Sink<T>, source: Source<T>) -> Self::Stage;
}

pub trait Stage<T> {
    fn run(&mut self) -> impl Future<Output = Result<()>>;
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Pipeline<T> {
    _marker: PhantomData<fn() -> T>,
    stages: JoinSet<Result<()>>,
}

#[derive(Debug)]
pub struct Builder<T> {
    _marker: PhantomData<fn() -> T>,
    stages: JoinSet<Result<()>>,

    input: Sink<T>,
    output: Source<T>,
}

impl<T> Pipeline<T> {
    pub fn builder() -> Builder<T> {
        let (input, output) = pipe();

        Builder {
            _marker: PhantomData,
            stages: JoinSet::new(),
            input,
            output,
        }
    }
}

impl<T> Builder<T> {
    pub fn stage<S>(mut self, stage: S) -> Self
    where
        S: StageBuilder<T>,
        S::Stage: 'static,
    {
        let (input, current_output) = pipe();
        let output = replace(&mut self.output, current_output);

        let mut stage = stage.build(input, output);
        self.stages.spawn_local(async move { stage.run().await });

        self
    }

    pub fn build(self) -> (Pipeline<T>, Sink<T>, Source<T>) {
        let Self {
            _marker,
            stages,
            input,
            output,
        } = self;

        (
            Pipeline {
                _marker: PhantomData,
                stages,
            },
            input,
            output,
        )
    }
}

impl<T> Pipeline<T> {
    pub async fn wait(self) -> Result<()> {
        let Self {
            _marker,
            mut stages,
        } = self;

        let mut error = None;
        while let Some(r) = stages.join_next().await {
            match r.map_err(Into::into).flatten() {
                Ok(()) => continue,
                Err(e) if error.is_none() => error = Some(e),
                Err(e) => {
                    warn!("another pipeline stage errored with: {e:#}");
                }
            }
        }

        match error {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }
}
