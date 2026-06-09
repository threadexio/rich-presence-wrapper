use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches, ValueEnum};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
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

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, clap::Parser)]
#[command(name = env!("CARGO_BIN_NAME"), disable_help_subcommand = true)]
pub struct Args {
    #[clap(
        long,
        help = "Specify an alternate configuration file.",
        env = "RICH_PRESENCE_WRAPPER_CONFIG"
    )]
    pub config: Option<PathBuf>,

    #[clap(
        long = "level",
        help = "Set the log level.",
        env = "RICH_PRESENCE_WRAPPER_LOG_LEVEL",
        default_value = "info"
    )]
    pub log_level: LogLevel,

    #[clap(subcommand)]
    pub command: Command,
}

impl Args {
    pub fn parse() -> Self {
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
                log_level: LogLevel::Off,
                command: Command::from_arg_matches(&matches)
                    .expect("exactly one subcommand must match"),
            },
        }
    }
}
