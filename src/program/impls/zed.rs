use super::generic_editor::GenericEditor;
use super::prelude::*;

use eyre::Result;

pub struct Zed {
    process: Child,
    inner: GenericEditor,
}

pub fn new(arg0: Arg0, args: Args) -> Result<Zed> {
    let process = real_binary_from_env(&arg0)?.args(args).spawn()?;

    let inner = GenericEditor {
        pid: process.id(),
        logo: "zed-logo",
    };

    Ok(Zed { process, inner })
}

impl Inner for Zed {
    fn waiter(&self) -> Waiter {
        Waiter::from(&self.process)
    }
}

impl App for Zed {
    fn id(&self) -> u64 {
        1342862237538193418
    }

    fn activity(&mut self, activity: &mut Activity) {
        self.inner.activity(activity);
    }
}
