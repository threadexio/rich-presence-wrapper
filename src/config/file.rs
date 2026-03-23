use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use eyre::{Context, Result};
use module::{Context as _, Merge};
use module_util::evaluator::Evaluator;
use serde::Deserialize;

#[derive(Default, Deserialize, Merge)]
#[serde(default, rename_all = "kebab-case")]
pub struct File {
    #[cfg(feature = "helix")]
    pub helix: crate::app::helix::File,

    #[cfg(feature = "zed")]
    pub zed: crate::app::zed::File,

    #[cfg(feature = "mpris-bridge")]
    #[merge(rename = "mpris-bridge")]
    pub mpris_bridge: crate::app::mpris_bridge::File,
}

pub fn parse(config_path: &Path) -> Result<File> {
    #[derive(Deserialize)]
    struct Module {
        #[serde(default)]
        imports: Vec<PathBuf>,

        #[serde(flatten)]
        file: File,
    }

    let mut evaluator: Evaluator<DisplayPath, File> = Evaluator::with(File::default());
    let mut evaluated = HashSet::new();

    evaluator.import(DisplayPath(config_path.to_path_buf()));

    while let Some(path) = evaluator.next() {
        if !evaluated.insert(path.0.clone()) {
            continue;
        }

        let text = match fs::read_to_string(&path.0) {
            Ok(x) => x,
            Err(e) if e.kind() == io::ErrorKind::NotFound && path.0 == config_path => continue,
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path))?,
        };

        let dirname = path
            .0
            .parent()
            .expect("the path must have at least one component since read() succeeded");

        let Module { imports, file } =
            toml::from_str(&text).with_context(|| format!("failed to parse {path}"))?;

        let imports = imports
            .into_iter()
            .map(|x| dirname.join(x))
            .map(DisplayPath)
            .collect();

        evaluator
            .eval(path.clone(), imports, file)
            .with_trace(|| evaluator.trace(path))?;
    }

    let x = evaluator
        .finish()
        .expect("there must be at least one value");

    Ok(x)
}

#[derive(Clone)]
struct DisplayPath(PathBuf);

impl fmt::Display for DisplayPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.display().fmt(f)
    }
}
