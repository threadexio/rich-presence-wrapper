#[macro_use]
extern crate tracing;

use std::path::Path;
use std::process::ExitCode;
use std::{fs, io};

use eyre::{Context, ContextCompat, Result};
use magic_args::{Extend, apply};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cli::Args;
use crate::config::Config;
use crate::util::{PathJoin, cache_dir, config_dir};

#[macro_use]
mod util;

mod app;
mod cli;
mod config;
mod discord;
mod platform;

///////////////////////////////////////////////////////////////////////////////

async fn _main(args: &Args) -> Result<ExitCode> {
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

    trace!("{config:#?}");

    let all = (&args, &config);

    match &args.command {
        #[cfg(feature = "helix")]
        cli::Command::Helix(x) => apply(app::helix::run, all.extend(x)).await,

        #[cfg(feature = "zed")]
        cli::Command::Zed(x) => apply(app::zed::run, all.extend(x)).await,

        #[cfg(feature = "mpris-bridge")]
        cli::Command::MprisBridge(x) => {
            apply(
                app::mpris_bridge::run,
                all.extend(x).extend(&config.mpris_bridge),
            )
            .await
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    let app_name = match args.command {
        cli::Command::Helix(_) => "helix",
        cli::Command::Zed(_) => "zed",
        cli::Command::MprisBridge(_) => "mpris-bridge",
    };

    let level_filter_layer = match args.log_level {
        cli::LogLevel::Off => LevelFilter::OFF,
        cli::LogLevel::Error => LevelFilter::ERROR,
        cli::LogLevel::Warn => LevelFilter::WARN,
        cli::LogLevel::Info => LevelFilter::INFO,
        cli::LogLevel::Debug => LevelFilter::DEBUG,
        cli::LogLevel::Trace => LevelFilter::TRACE,
    };

    let console_fmt_layer = match args.command {
        cli::Command::Helix(_) => None,
        _ => Some(tracing_subscriber::fmt::layer().compact()),
    };

    let (log_file_fmt_layer, log_file_fmt_handle) = tracing_subscriber::reload::Layer::new(None);

    tracing_subscriber::registry()
        .with(level_filter_layer)
        .with(console_fmt_layer)
        .with(log_file_fmt_layer)
        .init();

    let r = try2!({
        let cache_dir = cache_dir()
            .map(|x| x.join(env!("CARGO_BIN_NAME")))
            .context("cache directory not set")?;

        match fs::create_dir(&cache_dir) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                return Err(e).with_context(|| {
                    format!("failed to create directory '{}'", cache_dir.display())
                });
            }
        }

        let log_file_path = cache_dir.as_path().join(format!("{app_name}.log"));

        let log_file = fs::File::options()
            .append(true)
            .create(true)
            .open(&log_file_path)
            .with_context(|| format!("failed to open log file '{}'", log_file_path.display()))?;

        log_file_fmt_handle
            .modify(|layer| {
                *layer = Some(
                    tracing_subscriber::fmt::layer()
                        .compact()
                        .with_ansi(false)
                        .with_writer(log_file),
                );
            })
            .expect("subscriber should still exist");

        Result::<()>::Ok(())
    });

    if let Err(e) = r {
        warn!("{e:#}");
    }

    let rt = tokio::runtime::LocalRuntime::new().unwrap();
    let r = rt.block_on(_main(&args));
    rt.shutdown_background();

    match r {
        Ok(code) => code,
        Err(e) => {
            error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}
