use std::cmp::{max, min};
use std::time::{Duration, Instant};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone)]
pub struct Backoff<S> {
    strategy: S,
}

impl<S> Backoff<S> {
    pub fn new(strategy: S) -> Self {
        Self { strategy }
    }
}

impl<S> Backoff<S>
where
    S: Strategy,
{
    pub fn spin(&mut self) {
        let deadline = Instant::now() + self.strategy.tick();
        while Instant::now() < deadline {
            std::thread::yield_now();
        }
    }

    pub async fn sleep(&mut self) {
        tokio::time::sleep(self.strategy.tick()).await
    }

    pub fn blocking_sleep(&mut self) {
        std::thread::sleep(self.strategy.tick())
    }

    pub fn reset(&mut self) {
        self.strategy.reset();
    }
}

///////////////////////////////////////////////////////////////////////////////

mod private {
    use super::*;

    pub trait Strategy {
        fn tick(&mut self) -> Duration;
        fn reset(&mut self);
    }
}

pub trait Strategy: private::Strategy {}
impl<T> Strategy for T where T: private::Strategy {}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone)]
pub struct Exponential {
    base: f32,
    factor: f32,
}

impl Exponential {
    pub fn new(base: f32) -> Self {
        Self { base, factor: 0.0 }
    }
}

impl private::Strategy for Exponential {
    fn tick(&mut self) -> Duration {
        let t = Duration::from_secs_f32(self.base.powf(self.factor));
        self.factor += 1.0;
        t
    }

    fn reset(&mut self) {
        self.factor = 0.0;
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone)]
pub struct Min<S> {
    min: Duration,
    inner: S,
}

impl<S> Min<S> {
    pub fn new(min: Duration, inner: S) -> Self {
        Self { min, inner }
    }
}

impl<S> private::Strategy for Min<S>
where
    S: Strategy,
{
    fn tick(&mut self) -> Duration {
        max(self.inner.tick(), self.min)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone)]
pub struct Max<S> {
    max: Duration,
    inner: S,
}

impl<S> Max<S> {
    pub fn new(max: Duration, inner: S) -> Self {
        Self { max, inner }
    }
}

impl<S> private::Strategy for Max<S>
where
    S: Strategy,
{
    fn tick(&mut self) -> Duration {
        min(self.inner.tick(), self.max)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}
