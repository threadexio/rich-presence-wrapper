use crate::app::generic_editor::GenericEditor;

use super::prelude::*;

pub struct Zed {
    ipc: Ipc,
}

impl Zed {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            ipc: Ipc::builder()
                .client_id("1342862237538193418")
                .finish()
                .await?,
        })
    }
}

#[async_trait::async_trait]
impl App for Zed {
    fn program(&mut self, _: &mut Command) -> Result<()> {
        Ok(())
    }

    async fn run(&mut self, pid: u32) -> Result<Never> {
        let mut generic_editor = GenericEditor {
            ipc: &mut self.ipc,
            pid,
            name: "zed",
            logo: "zed-logo",
        };

        generic_editor.run().await
    }
}
