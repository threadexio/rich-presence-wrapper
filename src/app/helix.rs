use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use eyre::{Context, Result};
use module::types::Overridable;
use module::Merge;
use serde::Deserialize;

use crate::app::common::generic_editor::GenericEditor;
use crate::config::cli;
use crate::config::Config;
use crate::discord::Discord;
use crate::platform::ChildExt;
use crate::platform::ChildHandle;
use crate::util::exit_status_to_code;
use crate::util::Never;

const CLIENT_ID: &str = "1339918035842105417";

#[derive(Debug, clap::Parser)]
#[command(name = "hx", disable_help_flag = true)]
pub struct Command {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<OsString>,
}

#[derive(Debug, Default, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct File {
    path: Option<Overridable<PathBuf>>,
    #[merge(rename = "client-id")]
    client_id: Option<Overridable<String>>,
}

pub async fn run(config: Config) -> Result<ExitCode> {
    let cli::Command::Helix(ref command) = config.command else {
        unreachable!()
    };

    let binary_path = env::var_os("_hx")
        .map(PathBuf::from)
        .or(config.helix.path.as_deref().cloned())
        .unwrap_or_else(|| PathBuf::from("hx"));

    let mut child = tokio::process::Command::new(binary_path)
        .args(&command.args)
        .spawn()
        .context("failed to spawn helix")?;

    tokio::spawn({
        let child = child.handle().expect("we have not waited the child");

        async move {
            let _ = rpc_task(config, child).await;
        }
    });

    let code = child.wait().await.map(exit_status_to_code)?;
    Ok(code)
}

async fn rpc_task(config: Config, editor: ChildHandle) -> Result<Never> {
    let mut generic_editor = GenericEditor {
        discord: Discord::builder()
            .client_id(
                config
                    .helix
                    .client_id
                    .as_deref()
                    .map(String::as_str)
                    .unwrap_or(CLIENT_ID),
            )
            .finish(),

        editor,

        name: "helix",
        logo: "helix-logo",

        options: Default::default(),
    };

    generic_editor.run().await
}
