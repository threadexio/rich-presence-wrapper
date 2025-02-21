use std::process::ExitCode;
use std::thread::{sleep, spawn};
use std::time::Duration;

const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

mod program;
mod rpc;
mod util;

use self::program::Program;
use self::rpc::{App, Rpc};

fn main() -> ExitCode {
    let (program, mut app) = match Program::new() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    spawn(move || {
        let mut rpc = Rpc::new();

        loop {
            sleep(UPDATE_INTERVAL);

            let _ = rpc.connect(app.id());
            let _ = rpc.update(&mut app);
        }
    });

    let status = program.wait().unwrap();
    let code = status.code().unwrap();
    ExitCode::from(code as u8)
}
