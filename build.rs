use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

///////////////////////////////////////////////////////////////////////////////

fn main() {
    rerun_if_env_changed("CARGO_PKG_VERSION");
    rerun_if_changed(".git/HEAD");

    export_build_info();
}

fn export_build_info() {
    let mut writer = File::options()
        .write(true)
        .truncate(true)
        .create(true)
        .open(out_dir().join("build_info.rs"))
        .unwrap();

    writeln!(
        &mut writer,
        "pub const VERSION: &'static str = {:?};",
        version()
    )
    .unwrap();

    writeln!(
        &mut writer,
        "pub const LONG_VERSION: &'static str = {:?};",
        long_version()
    )
    .unwrap();

    writeln!(
        &mut writer,
        "pub const RUSTC_VERSION: &'static str = {:?};",
        rustc_version()
    )
    .unwrap();

    {
        write!(
            &mut writer,
            "pub const FEATURES: &'static [&'static str] = &["
        )
        .unwrap();

        for feature in features() {
            write!(&mut writer, "{feature:?},").unwrap();
        }

        writeln!(&mut writer, "];").unwrap()
    }
}

///////////////////////////////////////////////////////////////////////////////

fn run<I>(argv: I) -> String
where
    I: IntoIterator,
    I::Item: AsRef<OsStr>,
{
    let mut argv = argv.into_iter();

    let argv0 = argv.next().unwrap();

    let output = Command::new(argv0)
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();

    if !output.status.success() {
        panic!("run() failed")
    }

    str::from_utf8(&output.stdout)
        .unwrap()
        .trim_end()
        .to_owned()
}

fn rerun_if_changed<P>(path: P)
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    println!("cargo::rerun-if-changed={}", path.display());
}

fn rerun_if_env_changed(var: &str) {
    println!("cargo::rerun-if-env-changed={var}");
}

fn version() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();

    CACHE.get_or_init(|| run(["./scripts/version.py"])).as_str()
}

fn long_version() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();

    CACHE
        .get_or_init(|| {
            let mut s = String::new();
            s.push_str(version());
            s.push(' ');
            s.push('(');
            {
                let mut iter = features();

                if let Some(x) = iter.next() {
                    s.push('+');
                    s.push_str(x);

                    for x in iter {
                        s.push_str(" +");
                        s.push_str(x);
                    }
                }
            }
            s.push(')');
            s.push(' ');
            s.push('(');
            s.push_str(rustc_version());
            s.push(')');
            s
        })
        .as_str()
}

fn rustc_version() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();

    CACHE.get_or_init(|| run(["rustc", "--version"])).as_str()
}

fn features() -> impl Iterator<Item = &'static str> {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();

    CACHE
        .get_or_init(|| {
            env::var("CARGO_CFG_FEATURE")
                .unwrap()
                .split(',')
                .map(str::trim)
                .map(ToOwned::to_owned)
                .collect()
        })
        .iter()
        .map(String::as_str)
}

fn out_dir() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();

    CACHE
        .get_or_init(|| PathBuf::from(env::var_os("OUT_DIR").expect("$OUT_DIR not set")))
        .as_path()
}
