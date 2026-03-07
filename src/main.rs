use std::collections::VecDeque;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::process::ExitCode;

use eyre::{bail, Context, ContextCompat, Result};
use tokio::process::Command;

use crate::app::{App, Helix, Zed};
use crate::util::basename;

mod app;
mod ipc;
mod util;

fn main() -> ExitCode {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap();

    let code = rt.block_on(async move {
        match try_main().await {
            Ok(x) => x,
            Err(e) => {
                eprintln!("error: {e:#}");
                ExitCode::FAILURE
            }
        }
    });

    rt.shutdown_background();
    code
}

async fn try_main() -> Result<ExitCode> {
    let mut args: VecDeque<_> = env::args_os().collect();

    if basename(args.front().expect("missing argv[0]"))
        .expect("argv[0] should have a final component")
        == env!("CARGO_BIN_NAME")
    {
        args.pop_front();
    }

    let code = run_program(args.iter()).await?;
    Ok(ExitCode::from(code as u8))
}

async fn run_program<I>(mut argv: I) -> Result<i32>
where
    I: Iterator,
    I::Item: AsRef<OsStr>,
{
    let arg0 = argv.next().context("missing target program")?;

    let program = basename(arg0.as_ref()).unwrap();

    let mut app: Box<dyn App + Send> = match program.to_string_lossy().as_ref() {
        "hx" => Box::new(Helix::new().await?),
        "zeditor" => Box::new(Zed::new().await?),
        _ => bail!("unknown target program"),
    };

    let env_var_name = program_env_var_name(program);
    let path = env::var_os(&env_var_name)
        .with_context(|| format!("please set ${}", env_var_name.display()))?;

    let mut child = Command::new(&path);

    child.arg0(arg0).args(argv);
    child.env_remove(env_var_name);

    unsafe {
        child.pre_exec(|| {
            nix::sys::prctl::set_pdeathsig(nix::sys::signal::Signal::SIGHUP)
                .map_err(|e| io::Error::from_raw_os_error(e as i32))?;

            Ok(())
        });
    }

    app.program(&mut child)?;

    let mut child = child
        .spawn()
        .with_context(|| format!("failed to spawn {}", path.display()))?;

    // SAFETY: We have not polled the child at all, so the PID must still be
    //         available. As per the docs of `Child::id()`.
    let pid = child.id().expect("child's pid should be available");

    tokio::select! {
        r = app.run(pid) => {
            let err = r.unwrap_err();
            Err(err)
        }

        r = child.wait() => {
            let status = r.context("failed to wait child")?;
            let code = status.code().unwrap_or(-1);
            Ok(code)
        }
    }
}

fn program_env_var_name(program: &OsStr) -> OsString {
    let prefix = OsStr::new("_");

    let mut out = OsString::with_capacity(prefix.len().strict_add(program.len()));
    out.push(prefix);
    out.push(program);
    out
}
