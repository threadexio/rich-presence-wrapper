use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches};

#[derive(Debug, clap::Parser)]
#[command(name = env!("CARGO_BIN_NAME"), disable_help_subcommand = true)]
pub struct Args {
    #[clap(
        long,
        help = "Specify an alternate configuration file.",
        env = "RICH_PRESENCE_WRAPPER_CONFIG"
    )]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    #[cfg(feature = "helix")]
    #[command(name = "hx")]
    Helix(crate::app::helix::Command),

    #[cfg(feature = "zed")]
    #[command(name = "zeditor")]
    Zed(crate::app::zed::Command),

    #[cfg(feature = "mpris-bridge")]
    #[command(name = "mpris-bridge")]
    MprisBridge(crate::app::mpris_bridge::Command),
}

pub fn parse() -> Args {
    let app_commands: [clap::Command; _] = [
        #[cfg(feature = "helix")]
        crate::app::helix::Command::command(),
        #[cfg(feature = "zed")]
        crate::app::zed::Command::command(),
        #[cfg(feature = "mpris-bridge")]
        crate::app::mpris_bridge::Command::command(),
    ];

    let command = clap::Command::new(env!("CARGO_BIN_NAME"))
        .multicall(true)
        .subcommand_required(true)
        .disable_help_subcommand(true)
        .subcommand(Args::command())
        .subcommands(app_commands);

    let matches = command.get_matches();

    match matches.subcommand().expect("subcommand is required") {
        (env!("CARGO_BIN_NAME"), matches) => {
            Args::from_arg_matches(matches).expect("these are the matches for Args")
        }
        (_, _) => Args {
            config: None,
            command: Command::from_arg_matches(&matches)
                .expect("exactly one subcommand must match"),
        },
    }
}
