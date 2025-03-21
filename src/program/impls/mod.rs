#![allow(unused)]

use std::ffi::OsString;
use std::fmt;
use std::process::Command;

use eyre::{bail, ContextCompat, Result};

use super::args::{Arg0, Args};
use super::Inner;

pub mod generic_editor;

struct AvailablePrograms;

impl fmt::Display for AvailablePrograms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const COLUMNS: usize = 3;

        for (i, x) in AVAILABLE_PROGRAMS.iter().enumerate() {
            if i % COLUMNS == 1 {
                writeln!(f, "  {x}")?;
            } else {
                write!(f, "  {x}")?;
            }
        }

        Ok(())
    }
}

macro_rules! programs {
    ($(
        $($bin:literal),+ => $mod:ident ($cfg:meta)
    )*) => {
        $(
            #[cfg($cfg)]
            mod $mod;
        )*

        const AVAILABLE_PROGRAMS: &[&str] = &[
            $(
                $(
                    #[cfg($cfg)]
                    $bin,
                )+
            )*
        ];

        pub fn run(arg0: Arg0, args: Args) -> Result<Box<dyn Inner>> {
            match arg0.binary_name().to_str() {
                $(
                    $(
                        #[cfg($cfg)]
                        Some($bin) => Ok(Box::new(self::$mod::new(arg0, args)?)),
                    )+
                )*
                _ => bail!("unknown target program

There is no rich presence support for the requested target program.

Supported programs:
-------------------
{AvailablePrograms}")
            }
        }
    };
}

programs! {
    "hx" => helix (feature = "helix")
    "zeditor" => zed (feature = "zed")
}

fn real_binary_from_env(arg0: &Arg0) -> Result<Command> {
    let mut env_var = OsString::new();
    env_var.push("_");
    env_var.push(arg0.binary_name());

    let real_arg0 = std::env::var_os(&env_var).with_context(|| {
        format!("missing target program path. expected the path of the target program to be given in the environment variable {env_var:?}

In order to avoid cycles, this program requires the path of the target program to be given in an environment variable.")
    })?;

    let mut x = Command::new(real_arg0);
    x.env_remove(&env_var);
    Ok(x)
}

#[allow(unused_imports)]
mod prelude {
    pub(super) use std::process::{Child, Command, Stdio};

    pub(super) use crate::rpc::{Activity, App};

    pub(super) use super::super::{
        args::{Arg0, Args},
        waiter::Waiter,
        Inner,
    };

    pub(super) use super::real_binary_from_env;
}
