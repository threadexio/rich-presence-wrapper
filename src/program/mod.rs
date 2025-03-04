use eyre::{ContextCompat, Result};

use crate::rpc::{Activity, App};

mod args;
mod impls;
mod waiter;

use self::waiter::Waiter;

trait Inner: App + Send {
    fn waiter(&self) -> Waiter;
}

pub fn run() -> Result<Program> {
    let (arg0, args) = args::parse().context(
        "missing target program

You can specify a target program via:
 * renaming this executable to the target program,
 * symlinking the wrapper executable with the name of the target program, or
 * specifying the target program as the first argument.",
    )?;

    let inner = impls::run(arg0, args)?;
    Ok(Program(inner))
}

pub struct Program(Box<dyn Inner>);

impl Program {
    pub fn waiter(&self) -> Waiter {
        self.0.waiter()
    }
}

impl App for Program {
    fn id(&self) -> u64 {
        self.0.id()
    }

    fn activity(&mut self, activity: &mut Activity) {
        self.0.activity(activity);
    }
}
