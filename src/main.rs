#[macro_use]
extern crate tracing;

use std::path::Path;
use std::process::ExitCode;

use eyre::{ContextCompat, Result};
use magic_args::apply;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cli::Args;
use crate::config::Config;
use crate::util::{ExtendTuple, PathJoin, config_dir};

mod app;
mod cli;
mod config;
mod discord;
mod platform;
mod util;

///////////////////////////////////////////////////////////////////////////////

fn _main() -> Result<ExitCode> {
    let args = Args::parse();

    let level_filter = match args.log_level {
        cli::LogLevel::Off => LevelFilter::OFF,
        cli::LogLevel::Error => LevelFilter::ERROR,
        cli::LogLevel::Warn => LevelFilter::WARN,
        cli::LogLevel::Info => LevelFilter::INFO,
        cli::LogLevel::Debug => LevelFilter::DEBUG,
        cli::LogLevel::Trace => LevelFilter::TRACE,
    };

    tracing_subscriber::registry()
        .with(level_filter)
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    trace!("{args:#?}");

    let config_path = match args.config {
        Some(ref x) => x.clone(),
        None => [
            config_dir().context("failed to get the user config directory")?,
            Path::new(env!("CARGO_BIN_NAME")),
            Path::new("config.toml"),
        ]
        .join(),
    };

    let config = Config::read(&config_path)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let all = (&args, &config);

    let code = rt.block_on(async {
        match &args.command {
            #[cfg(feature = "helix")]
            cli::Command::Helix(x) => apply(app::helix::run, all.extend(x)).await,

            #[cfg(feature = "zed")]
            cli::Command::Zed(x) => apply(app::zed::run, all.extend(x)).await,

            #[cfg(feature = "mpris-bridge")]
            cli::Command::MprisBridge(x) => apply(app::mpris_bridge::run, all.extend(x)).await,
        }
    });

    rt.shutdown_background();
    code
}

fn main() -> ExitCode {
    match _main() {
        Ok(code) => code,
        Err(e) => {
            error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}
