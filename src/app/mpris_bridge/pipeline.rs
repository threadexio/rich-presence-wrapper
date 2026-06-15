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
pub struct Builder<T> {
    _marker: PhantomData<fn() -> T>,
    stages: JoinSet<Result<()>>,

    input: Sink<T>,
    output: Source<T>,
}

impl<T> Default for Builder<T> {
    fn default() -> Self {
        let (input, output) = pipe();
        Self {
            _marker: PhantomData,
            stages: JoinSet::new(),
            input,
            output,
        }
    }
}

impl<T> Builder<T> {
    pub fn stage<S>(&mut self, stage: S)
    where
        S: StageBuilder<T>,
        S::Stage: 'static,
    {
        let (input, current_output) = pipe();
        let output = replace(&mut self.output, current_output);

        let mut stage = stage.build(input, output);
        self.stages.spawn_local(async move { stage.run().await });
    }

    pub fn build(self) -> (JoinSet<Result<()>>, Sink<T>, Source<T>) {
        let Self {
            _marker,
            stages,
            input,
            output,
        } = self;

        (stages, input, output)
    }
}

///////////////////////////////////////////////////////////////////////////////

pub fn builder<T>() -> Builder<T> {
    Builder::default()
}
