use crate::app::generic_editor::GenericEditor;

use super::prelude::*;

pub struct Helix {
    ipc: Ipc,
}

impl Helix {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            ipc: Ipc::builder()
                .client_id("1339918035842105417")
                .finish()
                .await?,
        })
    }
}

#[async_trait::async_trait]
impl App for Helix {
    fn program(&mut self, _: &mut Command) -> Result<()> {
        Ok(())
    }

    async fn run(&mut self, pid: u32) -> Result<Never> {
        let mut generic_editor = GenericEditor {
            ipc: &mut self.ipc,
            pid,
            name: "helix",
            logo: "helix-logo",
        };

        generic_editor.run().await
    }
}
