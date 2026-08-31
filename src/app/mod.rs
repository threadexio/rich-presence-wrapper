use std::process::ExitCode;

use clap::CommandFactory;
use eyre::Result;
use magic_args::{Extend, apply};
use module::Merge;
use serde::Deserialize;

///////////////////////////////////////////////////////////////////////////////

#[allow(dead_code)]
mod common;

macro_rules! app {
    ($mod:ident if $cfg:meta) => {
        #[cfg($cfg)]
        mod $mod;

        #[cfg(not($cfg))]
        mod $mod {
            use std::process::ExitCode;

            use eyre::{Result, bail};
            use module::Merge;
            use serde::Deserialize;

            #[derive(Debug, clap::Parser)]
            #[command(name = stringify!($mod), disable_help_flag = true)]
            pub struct Command {}

            #[derive(Debug, Default, Deserialize, Merge)]
            pub struct Config {}

            pub async fn run() -> Result<ExitCode> {
                bail!(
                    "this command is not compiled-in for this build of {}. see: `--version`",
                    env!("CARGO_BIN_NAME")
                )
            }
        }
    };
}

app!(helix if feature = "helix");
app!(zed if feature = "zed");
app!(music_bridge if feature = "music-bridge");
app!(lsp if feature = "lsp");

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    #[command(name = "hx")]
    #[cfg_attr(not(feature = "helix"), command(hide = true))]
    Helix(helix::Command),

    #[command(name = "zeditor")]
    #[cfg_attr(not(feature = "zed"), command(hide = true))]
    Zed(zed::Command),

    #[command(name = "music-bridge")]
    #[cfg_attr(not(feature = "music-bridge"), command(hide = true))]
    MusicBridge(music_bridge::Command),

    #[command(name = "lsp")]
    #[cfg_attr(not(feature = "lsp"), command(hide = true))]
    Lsp(lsp::Command),
}

impl Command {
    pub fn name(&self) -> &str {
        match self {
            Self::Helix(_) => "helix",
            Self::Zed(_) => "zed",
            Self::MusicBridge(_) => "music-bridge",
            Self::Lsp(_) => "lsp",
        }
    }

    pub fn can_use_stdio_for_log(&self) -> bool {
        // TODO: refactor to one matches statement
        #[allow(unused_mut)]
        let mut r = true;

        #[cfg(feature = "helix")]
        {
            r &= !matches!(self, Self::Helix(_));
        }

        #[cfg(feature = "lsp")]
        {
            r &= !matches!(self, Self::Lsp(_));
        }

        r
    }

    pub fn multicall_commands() -> [clap::Command; 2] {
        [helix::Command::command(), zed::Command::command()]
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Deserialize, Merge)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    helix: helix::Config,
    zed: zed::Config,
    music_bridge: music_bridge::Config,
    lsp: lsp::Config,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(command: &Command, config: &Config) -> Result<ExitCode> {
    let all = (command, config);

    match &command {
        Command::Helix(x) => apply(helix::run, all.extend(x).extend(&config.helix)).await,

        Command::Zed(x) => apply(zed::run, all.extend(x).extend(&config.zed)).await,

        Command::MusicBridge(x) => {
            apply(
                music_bridge::run,
                all.extend(x).extend(&config.music_bridge),
            )
            .await
        }

        Command::Lsp(x) => apply(lsp::run, all.extend(x).extend(&config.lsp)).await,
    }
}
