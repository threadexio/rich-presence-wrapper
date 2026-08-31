use std::path::PathBuf;

use eyre::Result;
use module::Merge;
use serde::Deserialize;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Deserialize, Merge)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    #[serde(flatten)]
    pub app: crate::app::Config,
}

impl Config {
    pub fn read(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let x = module_util::file::toml([path])?.expect("evaluated at least one path");
        Ok(x)
    }

    pub fn default_path() -> Result<PathBuf> {
        #[allow(dead_code)]
        fn user_config_path() -> Result<PathBuf> {
            use crate::util::{PathJoin, config_dir};
            use eyre::ContextCompat;
            use std::path::Path;

            Ok([
                config_dir().context("failed to get user config directory")?,
                Path::new(env!("CARGO_BIN_NAME")),
                Path::new("config.toml"),
            ]
            .join())
        }

        cfg_select! {
            debug_assertions => {
                eyre::bail!("there is no default configuration file for debug builds. please specify one manually.")
            },

            _ => {
                user_config_path()
            }
        }
    }
}
