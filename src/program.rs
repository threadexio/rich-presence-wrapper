use std::env::{args_os, var_os};
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::{fmt, io};

use crate::rpc::{Activity, App};

#[derive(Debug, thiserror::Error)]
pub enum ProgramError {
    #[error("failed to spawn target program: {0}")]
    SpawnError(io::Error),

    #[error(
        "missing target program

You can specify a target program via:
 * renaming this executable to the target program,
 * symlinking the wrapper executable with the name of the target program, or
 * specifying the target program as the first argument."
    )]
    MissingProgram,

    #[error("missing target program path. expected the path of the target program to be given in the environment variable {0:?}

In order to avoid cycles, this program requires the path of the target program to be given in an environment variable."
    )]
    MissingProgramPath(OsString),

    #[error(
        "unknown program {0:?}

There is no rich presence support for the requested target program.

Supported target programs:
--------------------------
{available_programs}",
        available_programs = AvailablePrograms(AVAILABLE_PROGRAMS)
    )]
    UnknownProgram(OsString),
}

pub struct Program {
    process: Child,
}

impl Program {
    pub fn new() -> Result<(Self, ProgramApp), ProgramError> {
        let mut args = args_os();

        let mut arg0 = args.next().unwrap();
        if get_bin_path(&arg0).unwrap() == OsStr::new("rich-presence-wrapper") {
            arg0 = args.next().ok_or(ProgramError::MissingProgram)?;
        }

        let arg0_name = get_bin_path(&arg0).unwrap();

        let activity = program(arg0_name)
            .ok_or_else(|| ProgramError::UnknownProgram(arg0_name.to_os_string()))?;

        let var_name = make_path_env_var(arg0_name);
        let arg0_real_path =
            var_os(&var_name).ok_or_else(|| ProgramError::MissingProgramPath(var_name.clone()))?;

        let process = Command::new(arg0_real_path)
            .args(args)
            .env_remove(&var_name)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(ProgramError::SpawnError)?;

        let pid = process.id();

        let activity = ProgramApp(activity(pid));
        let program = Self { process };

        Ok((program, activity))
    }

    pub fn wait(mut self) -> io::Result<ExitStatus> {
        self.process.wait()
    }
}

pub struct ProgramApp(Box<dyn App + Send>);

impl App for ProgramApp {
    fn id(&self) -> u64 {
        self.0.id()
    }

    fn activity(&mut self, activity: &mut Activity) {
        self.0.activity(activity)
    }
}

fn make_path_env_var(cmd_name: &OsStr) -> OsString {
    let mut x = OsString::with_capacity(1 + cmd_name.len());
    x.push("_");
    x.push(cmd_name);
    x
}

fn get_bin_path(s: &OsStr) -> Option<&OsStr> {
    let p = Path::new(s);
    p.file_name()
}

struct AvailablePrograms(&'static [&'static str]);

impl fmt::Display for AvailablePrograms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|x| writeln!(f, "  {x}"))
    }
}

macro_rules! programs {
    (
        $(
            $($cmd:literal),+ => $mod:ident
        ),*
    ) => {
        $(
            mod $mod;
        )*

        const AVAILABLE_PROGRAMS: &[&str] = &[ $( $($cmd),+ ),* ];

        type ProgramConstructor = Box<dyn FnOnce(u32) -> Box<dyn App + Send>>;

        fn program(arg0: &OsStr) -> Option<ProgramConstructor> {
            match &*arg0.to_string_lossy() {
                $(
                    $(
                        $cmd => Some(Box::new(|pid| Box::new(self::$mod::new(pid)))),
                    )+
                )*
                _ => None
            }
        }
    }
}

programs! {
    "hx" => helix
}
