use std::time::Duration;

use tokio::process::Command;
use tokio::time::{interval, MissedTickBehavior};

const APP_ID: u64 = 1339918035842105417;
const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

mod helix;
mod rpc;
mod util;

use self::helix::Helix;
use self::rpc::Rpc;
use self::util::env;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args = std::env::args_os().peekable();
    let _arg0 = args.next().unwrap();

    let mut helix = Command::new(env("HELIX", || "hx"));
    helix.args(args).env_remove("HELIX");

    let mut helix = Helix::new(helix).unwrap();
    let mut rpc = Rpc::new(APP_ID).await;

    let mut update = interval(UPDATE_INTERVAL);
    update.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = helix.wait() => break,
            _ = update.tick() => { let _ = rpc.update(&mut helix).await; }
        }
    }
}
