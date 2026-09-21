use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct Shared<T> {
    slot: Mutex<Option<T>>,
    update: Notify,
    closed: AtomicBool,
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Receiver<T>(Arc<Shared<T>>);

impl<T> Receiver<T> {
    pub fn closed(&self) -> bool {
        self.0.closed.load(Ordering::Acquire)
    }

    pub async fn recv(&mut self) -> Option<T> {
        loop {
            // We need to check if the pipe is closed before waiting for the writer
            // because if it is closed then the writer has been dropped and we will
            // never receive an "updated" notification.
            if self.closed() {
                return None;
            }

            {
                // SAFETY: We consider a poisoned lock a bug. There is nothing we
                //         can do with a poisoned value and it would be unsafe to
                //         try and make sense of the maybe-corrupted data inside.
                let mut slot = self.0.slot.lock().expect("poisoned lock");

                if let Some(value) = slot.take() {
                    return Some(value);
                }
            }

            self.0.update.notified().await;
        }
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        // SAFETY: `self.recv` does not utilize runtime-provided features like
        //         timers or IO.
        unsafe { crate::util::block_on(self.recv()) }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.0.closed.store(true, Ordering::Release);
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Sender<T>(Arc<Shared<T>>);

impl<T> Sender<T> {
    pub fn closed(&self) -> bool {
        self.0.closed.load(Ordering::Acquire)
    }

    pub fn send(&mut self, value: T) -> Result<(), T> {
        // SAFETY: We consider a poisoned lock a bug. There is nothing we
        //         can do with a poisoned value and it would be unsafe to
        //         try and make sense of the maybe-corrupted data inside.
        let mut slot = self.0.slot.lock().expect("poisoned lock");

        // Notify the reader of a value. In the case of multiple writes before
        // one read, `notify_one` guarantees that the reader will not receive
        // queued notifications. This way the reader will get woken up once and
        // see the last value written.
        self.0.update.notify_one();

        if self.closed() {
            return Err(value);
        }

        *slot = Some(value);
        Ok(())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.0.closed.store(true, Ordering::Release);

        // Notify on close so the reader can exit its indefinite wait for an
        // update that will never come.
        self.0.update.notify_one();
    }
}

///////////////////////////////////////////////////////////////////////////////

pub fn new<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        slot: Mutex::new(None),
        update: Notify::new(),
        closed: AtomicBool::new(false),
    });

    (Sender(Arc::clone(&shared)), Receiver(shared))
}
