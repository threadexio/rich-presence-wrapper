use std::process::ExitCode;
use std::thread::{sleep, spawn};
use std::time::Duration;

use eyre::Result;

const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

mod program;
mod rpc;
mod util;

use self::rpc::{App, Rpc};

fn main() -> ExitCode {
    color_eyre::install().unwrap();

    match try_main() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<ExitCode> {
    let mut program = program::run()?;
    let waiter = program.waiter();

    let mut rpc = Rpc::new();

    spawn(move || loop {
        sleep(UPDATE_INTERVAL);

        let _ = rpc.connect(program.id());
        let _ = rpc.update(&mut program);
    });

    let code = waiter
        .wait()
        .map(|x| x.code().unwrap_or(127) as u8)
        .unwrap_or(128);

    Ok(ExitCode::from(code))
}
