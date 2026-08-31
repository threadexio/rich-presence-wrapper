use std::path::PathBuf;

use clap::{FromArgMatches, ValueEnum};

use crate::app;

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

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, clap::Parser)]
#[command(
    name = env!("CARGO_BIN_NAME"),
    version = crate::consts::VERSION,
    long_version = crate::consts::LONG_VERSION,
    disable_help_subcommand = true,
    subcommand_required = true,
)]
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
        default_value = "info",
        global = true
    )]
    pub level: LogLevel,

    #[clap(subcommand)]
    pub command: app::Command,
}

impl Args {
    pub fn command() -> clap::Command {
        clap::Command::new(env!("CARGO_BIN_NAME"))
            .multicall(true)
            .subcommand_required(true)
            .disable_help_subcommand(true)
            .subcommand(<Self as clap::CommandFactory>::command())
            .subcommands(app::Command::multicall_commands())
    }

    pub fn parse() -> Self {
        let command = Self::command();
        let matches = command.get_matches();

        match matches.subcommand().expect("subcommand is required") {
            (env!("CARGO_BIN_NAME"), matches) => {
                Args::from_arg_matches(matches).expect("these are the matches for Args")
            }
            (_, _) => Args {
                config: None,
                level: LogLevel::Off,
                command: app::Command::from_arg_matches(&matches)
                    .expect("exactly one subcommand must match"),
            },
        }
    }
}
