use std::io;
use std::path::PathBuf;

///////////////////////////////////////////////////////////////////////////////

pub trait ChildExt {
    fn handle(&self) -> Option<ChildHandle>;
}

#[derive(Clone)]
pub struct ChildHandle(imp::ChildHandle);

impl ChildHandle {
    #[inline]
    pub fn cwd(&self) -> io::Result<PathBuf> {
        self.0.cwd()
    }
}

///////////////////////////////////////////////////////////////////////////////

macro_rules! platform {
    ($name:ident if $cfg:meta) => {
        #[cfg($cfg)]
        mod $name;
        #[cfg($cfg)]
        use self::$name as imp;
    };
}

platform!(linux if target_os = "linux");
