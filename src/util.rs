#![allow(dead_code)]

use std::cmp::min;
use std::future::pending;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, ExitStatus};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::time::{Instant, sleep_until};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy)]
pub enum Never {}

///////////////////////////////////////////////////////////////////////////////

pub trait SystemTimeExt {
    fn duration_since_epoch(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    fn duration_since_epoch(&self) -> Duration {
        self.duration_since(UNIX_EPOCH).unwrap()
    }
}

///////////////////////////////////////////////////////////////////////////////

pub struct Backoff {
    delay: Duration,
    max: Duration,
    factor: f32,
}

impl Backoff {
    pub fn new(initial: Duration, max: Duration, factor: f32) -> Self {
        Self {
            delay: initial,
            max,
            factor,
        }
    }

    pub fn advance(&mut self) {
        let new_delay = self.delay.mul_f32(self.factor);
        self.delay = min(self.max, new_delay);
    }

    pub fn blocking_sleep(&mut self) {
        std::thread::sleep(self.delay);
        self.advance();
    }
}

///////////////////////////////////////////////////////////////////////////////

pub fn home_dir() -> Option<&'static Path> {
    static HOME_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    HOME_DIR.get_or_init(dirs::home_dir).as_deref()
}

pub fn config_dir() -> Option<&'static Path> {
    static CONFIG_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    CONFIG_DIR.get_or_init(dirs::config_dir).as_deref()
}

///////////////////////////////////////////////////////////////////////////////

pub fn exit_status_to_code(x: ExitStatus) -> ExitCode {
    x.code()
        .map(|x| x as u8)
        .map(ExitCode::from)
        .unwrap_or(ExitCode::FAILURE)
}

///////////////////////////////////////////////////////////////////////////////

pub trait PathJoin {
    fn join(self) -> PathBuf;
}

impl<I> PathJoin for I
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
    I::IntoIter: Clone,
{
    fn join(self) -> PathBuf {
        let iter = self.into_iter();

        let capacity: usize = iter.clone().map(|x| x.as_ref().as_os_str().len()).sum();

        let mut path = PathBuf::with_capacity(capacity);
        iter.for_each(|x| path.push(x.as_ref()));
        path
    }
}

///////////////////////////////////////////////////////////////////////////////

pub trait ExtendTuple<T> {
    type Output;

    fn extend(self, item: T) -> Self::Output;
}

macro_rules! impl_extend_tuple {
    ($($T:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($T,)* T> ExtendTuple<T> for ($($T,)*) {
            type Output = ($($T,)* T,);

            fn extend(self, item: T) -> Self::Output {
                let ($($T,)*) = self;
                ($($T,)* item,)
            }
        }
    };
}

impl_extend_tuple!();
impl_extend_tuple!(T1);
impl_extend_tuple!(T1, T2);
impl_extend_tuple!(T1, T2, T3);
impl_extend_tuple!(T1, T2, T3, T4);
impl_extend_tuple!(T1, T2, T3, T4, T5);
impl_extend_tuple!(T1, T2, T3, T4, T5, T6);
impl_extend_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_extend_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);

///////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! try2 {
    (async move $e:block) => {
        (async move { $e }).await
    };
    (async $e:block) => {
        (async { $e }).await
    };
    (move $e:block) => {
        (move || $e)()
    };
    ($e:block) => {
        (|| $e)()
    };
}

///////////////////////////////////////////////////////////////////////////////

pub struct OneshotTimer {
    duration: Duration,
    started_at: Option<Instant>,
}

impl OneshotTimer {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            started_at: None,
        }
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.started_at.map(|x| x + self.duration)
    }

    pub fn expired(&self) -> bool {
        self.deadline().is_some_and(|x| Instant::now() > x)
    }

    pub fn started(&self) -> bool {
        self.started_at.is_some()
    }

    pub async fn wait(&mut self) {
        match self.deadline() {
            Some(deadline) => {
                sleep_until(deadline).await;
                self.started_at = None;
            }
            None => pending().await,
        }
    }

    pub fn restart(&mut self) {
        self.started_at = Some(Instant::now());
    }
}

///////////////////////////////////////////////////////////////////////////////

pub fn capitalize_words(s: &str) -> String {
    let mut out = String::with_capacity(s.len());

    let mut last: Option<char> = None;

    for c in s.chars() {
        match (last, c) {
            (Some(last), c) => match (last.is_whitespace(), c.is_whitespace()) {
                (false, _) => out.push(c),
                (true, false) => out.extend(c.to_uppercase()),
                (true, true) => {}
            },

            (None, c) => out.extend(c.to_uppercase()),
        }

        last = Some(c);
    }

    out
}

///////////////////////////////////////////////////////////////////////////////

pub const fn r#false() -> bool {
    false
}

pub const fn r#true() -> bool {
    true
}
