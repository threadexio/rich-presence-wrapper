#[macro_use]
extern crate tracing;

use std::fs;
use std::io;
use std::process::ExitCode;

use eyre::{Context, Result};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cli::Args;
use crate::config::Config;

#[macro_use]
mod util;

mod app;
mod cli;
mod config;
mod consts;
mod discord;
mod platform;

///////////////////////////////////////////////////////////////////////////////

fn main() -> ExitCode {
    let args = Args::parse();

    let level_filter_layer = match args.log_level {
        cli::LogLevel::Off => LevelFilter::OFF,
        cli::LogLevel::Error => LevelFilter::ERROR,
        cli::LogLevel::Warn => LevelFilter::WARN,
        cli::LogLevel::Info => LevelFilter::INFO,
        cli::LogLevel::Debug => LevelFilter::DEBUG,
        cli::LogLevel::Trace => LevelFilter::TRACE,
    };

    let (console_fmt_layer, console_fmt_handle) = tracing_subscriber::reload::Layer::new(Some(
        tracing_subscriber::fmt::layer()
            .compact()
            .with_writer(io::stderr),
    ));

    let (log_file_fmt_layer, log_file_fmt_handle) = tracing_subscriber::reload::Layer::new(None);

    tracing_subscriber::registry()
        .with(level_filter_layer)
        .with(console_fmt_layer)
        .with(log_file_fmt_layer)
        .init();

    debug!("{args:#?}");

    if let Some(log_file) = args.log_file.as_ref()
        && let Err(e) = try2!({
            let writer = fs::File::options()
                .append(true)
                .create(true)
                .open(log_file)
                .with_context(|| log_file.display().to_string())?;

            log_file_fmt_handle
                .modify(|layer| {
                    *layer = Some(
                        tracing_subscriber::fmt::layer()
                            .compact()
                            .with_ansi(false)
                            .with_writer(writer),
                    );
                })
                .expect("subscriber should still exist");

            Result::<()>::Ok(())
        })
    {
        warn!("{e:#}");
    }

    match try2!({
        let config_path = Ok(args.config).or_else(|()| Config::default_path())?;

        let config = match config_path {
            Some(path) => Config::read(path).context("failed to read config")?,
            None => Config::default(),
        };

        debug!("{config:#?}");

        if !args.command.can_use_stdio_for_log() {
            console_fmt_handle
                .modify(|layer| *layer = None)
                .expect("subscriber should still exist");
        }

        run_local(app::run(&args.command, &config.app))
    }) {
        Ok(code) => code,
        Err(e) => {
            error!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run_local<F>(future: F) -> F::Output
where
    F: Future,
{
    let rt = tokio::runtime::LocalRuntime::new().unwrap();
    let output = rt.block_on(future);
    rt.shutdown_background();
    output
}
