use std::process::ExitCode;

use eyre::Result;

use crate::config::Config;

mod app;
mod config;
mod discord;
mod platform;
mod util;

async fn try_main(config: Config) -> Result<ExitCode> {
    match config.command {
        #[cfg(feature = "helix")]
        config::cli::Command::Helix(_) => app::helix::run(config).await,

        #[cfg(feature = "zed")]
        config::cli::Command::Zed(_) => app::zed::run(config).await,

        #[cfg(feature = "mpris-bridge")]
        config::cli::Command::MprisBridge(_) => app::mpris_bridge::run(config).await,
    }
}

fn _main() -> Result<ExitCode> {
    let config = Config::read()?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap();

    let code = rt.block_on(async move { try_main(config).await });
    rt.shutdown_background();
    code
}

fn main() -> ExitCode {
    match _main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
