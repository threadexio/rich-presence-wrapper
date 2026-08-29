use std::mem::replace;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

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
struct PipeShared<T> {
    slot: Mutex<Option<T>>,
    update: Notify,
    closed: AtomicBool,
}

#[derive(Debug)]
pub struct Source<T>(Arc<PipeShared<T>>);

impl<T> Source<T> {
    pub async fn pull(&mut self) -> Option<T> {
        loop {
            // We need to check if the pipe is closed before waiting for the
            // writer because if it is closed then the writer has been dropped
            // and we will never receive an "updated" notification.
            if self.closed() {
                return None;
            }

            self.0.update.notified().await;

            // We need to check if the pipe is closed after waiting to catch a
            // writer being dropped after we have started waiting.
            if self.closed() {
                return None;
            }

            {
                // SAFETY: We consider a poisoned lock a bug. There is nothing we can do
                //         with a poisoned value and it would be unsafe to try and make
                //         sense of the maybe-corrupted data inside.
                let mut slot = self.0.slot.lock().expect("poisoned lock");

                if let Some(value) = slot.take() {
                    return Some(value);
                }
            }
        }
    }

    pub fn closed(&self) -> bool {
        self.0.closed.load(Ordering::Acquire)
    }
}

impl<T> Drop for Source<T> {
    fn drop(&mut self) {
        self.0.closed.store(true, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct Sink<T>(Arc<PipeShared<T>>);

impl<T> Sink<T> {
    pub fn push(&mut self, item: T) -> bool {
        {
            // SAFETY: We consider a poisoned lock a bug. There is nothing we
            //         can do with a poisoned value and it would be unsafe to
            //         try and make sense of the maybe-corrupted data inside.
            let mut slot = self.0.slot.lock().expect("poisoned lock");
            *slot = Some(item);

            // Drop the guard of `slot` here.
        }

        // Notify the reader of a value. In the case of multiple writes before
        // one read, `notify_one` guarantees that the reader will not receive
        // queued notifications. This way the reader will get woken up once and
        // see the last value written.
        self.0.update.notify_one();

        !self.closed()
    }

    pub fn closed(&self) -> bool {
        self.0.closed.load(Ordering::Acquire)
    }
}

impl<T> Drop for Sink<T> {
    fn drop(&mut self) {
        self.0.closed.store(true, Ordering::Release);

        // Notify on close so the reader can exit its indefinite wait for an
        // update that will never come.
        self.0.update.notify_one();
    }
}

fn pipe<T>() -> (Sink<T>, Source<T>) {
    let shared = Arc::new(PipeShared {
        slot: Mutex::new(None),
        update: Notify::new(),
        closed: AtomicBool::new(false),
    });

    (Sink(Arc::clone(&shared)), Source(shared))
}
