use std::process::ExitCode;
use std::thread::{sleep, spawn};
use std::time::Duration;

const APP_ID: u64 = 1339918035842105417;
const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

mod program;
mod rpc;
mod util;

use self::program::Program;
use self::rpc::Rpc;

fn main() -> ExitCode {
    let program = match Program::new() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    let mut activity = program.activity_builder();

    spawn(move || {
        let mut rpc = Rpc::new();

        loop {
            sleep(UPDATE_INTERVAL);

            let _ = rpc.connect(APP_ID);
            let _ = rpc.update(&mut activity);
        }
    });

    let status = program.wait().unwrap();
    let code = status.code().unwrap();
    ExitCode::from(code as u8)
}
