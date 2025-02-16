use std::process::{Command, ExitCode};
use std::thread::{sleep, spawn};
use std::time::Duration;

const APP_ID: u64 = 1339918035842105417;
const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

mod helix;
mod rpc;
mod util;

use self::helix::Helix;
use self::rpc::Rpc;
use self::util::env;

fn main() -> ExitCode {
    let mut args = std::env::args_os().peekable();
    let _arg0 = args.next().unwrap();

    let mut helix = Command::new(env("HELIX", || "hx"));
    helix.args(args).env_remove("HELIX");

    let helix = Helix::new(helix).unwrap();
    let mut activity = helix.activity_builder();

    spawn(move || {
        let mut rpc = Rpc::new();

        loop {
            sleep(UPDATE_INTERVAL);

            let _ = rpc.connect(APP_ID);
            let _ = rpc.update(&mut activity);
        }
    });

    let status = helix.wait().unwrap();
    let code = status.code().unwrap();
    ExitCode::from(code as u8)
}
