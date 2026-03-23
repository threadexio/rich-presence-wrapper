use std::path::Path;

use eyre::{ContextCompat, Result};

use crate::util::{config_dir, PathJoin};

pub mod cli;
pub mod file;

#[derive(Debug)]
pub struct Config {
    #[cfg(feature = "helix")]
    pub helix: crate::app::helix::File,

    #[cfg(feature = "zed")]
    pub zed: crate::app::zed::File,

    #[cfg(feature = "mpris-bridge")]
    pub mpris_bridge: crate::app::mpris_bridge::File,

    pub command: cli::Command,
}

impl Config {
    pub fn read() -> Result<Self> {
        let args = cli::parse();

        let config_path = match args.config {
            Some(x) => x,
            None => [
                config_dir().context("failed to get the user config directory")?,
                Path::new(env!("CARGO_BIN_NAME")),
                Path::new("config.toml"),
            ]
            .join(),
        };

        let file = file::parse(&config_path)?;

        Ok(Self {
            #[cfg(feature = "helix")]
            helix: file.helix,

            #[cfg(feature = "zed")]
            zed: file.zed,

            #[cfg(feature = "mpris-bridge")]
            mpris_bridge: file.mpris_bridge,

            command: args.command,
        })
    }
}
