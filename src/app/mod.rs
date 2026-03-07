use async_trait::async_trait;
use eyre::Result;
use tokio::process::Command;

use crate::util::Never;

mod helix;
pub use self::helix::Helix;

mod zed;
pub use self::zed::Zed;

mod generic_editor;

#[async_trait]
pub trait App {
    fn program(&mut self, program: &mut Command) -> Result<()>;

    async fn run(&mut self, pid: u32) -> Result<Never>;
}

mod prelude {
    pub(crate) use super::App;
    pub(crate) use crate::ipc::*;
    pub(crate) use crate::util::*;
    pub(crate) use eyre::Result;
    pub(crate) use tokio::process::Command;
}
