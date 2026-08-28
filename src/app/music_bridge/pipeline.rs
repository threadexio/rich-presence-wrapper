use std::mem::replace;

use tokio::sync::mpsc;

///////////////////////////////////////////////////////////////////////////////

pub struct Pipeline<T> {
    pub input: Sink<T>,
    pub output: Source<T>,
}

impl<T> Pipeline<T> {
    pub fn new() -> Self {
        let (input, output) = pipe();
        Self { input, output }
    }

    pub fn next(&mut self) -> (Source<T>, Sink<T>) {
        let (tx, rx) = pipe();
        let rx = replace(&mut self.output, rx);
        (rx, tx)
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Source<T>(mpsc::Receiver<T>);

impl<T> Source<T> {
    pub async fn pull(&mut self) -> Option<T> {
        self.0.recv().await
    }
}

#[derive(Debug)]
pub struct Sink<T>(mpsc::Sender<T>);

impl<T> Sink<T> {
    pub async fn push(&mut self, item: T) -> bool {
        self.0.send(item).await.is_ok()
    }
}

fn pipe<T>() -> (Sink<T>, Source<T>) {
    let (tx, rx) = mpsc::channel(1);
    (Sink(tx), Source(rx))
}
