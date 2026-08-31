use std::path::PathBuf;

use eyre::{Result, bail};
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
        cfg_select! {
            debug_assertions => {
                bail!("there is no default configuration file for debug builds. please specify one manually.")
            },

            _ => {
                use std::path::Path;
                use crate::util::{PathJoin, config_dir};

                [
                    crate::util::config_dir().context("failed to get the user config directory")?,
                    Path::new(env!("CARGO_BIN_NAME")),
                    Path::new("config.toml"),
                ]
                .join()
            }
        }
    }
}
