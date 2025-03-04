use super::generic_editor::GenericEditor;
use super::prelude::*;

use eyre::Result;

pub struct Helix {
    process: Child,
    inner: GenericEditor,
}

pub fn new(arg0: Arg0, args: Args) -> Result<Helix> {
    let process = real_binary_from_env(&arg0)?.args(args).spawn()?;

    let inner = GenericEditor {
        pid: process.id(),
        logo: "helix-logo",
    };

    Ok(Helix { process, inner })
}

impl Inner for Helix {
    fn waiter(&self) -> Waiter {
        Waiter::from(&self.process)
    }
}

impl App for Helix {
    fn id(&self) -> u64 {
        1339918035842105417
    }

    fn activity(&mut self, activity: &mut Activity) {
        self.inner.activity(activity);
    }
}
