use std::env::{args_os, ArgsOs};
use std::ffi::{OsStr, OsString};
use std::iter::{FusedIterator, Peekable};
use std::path::{Path, PathBuf};

pub fn parse() -> Option<(Arg0, Args)> {
    let mut args = args_os().peekable();
    args.next_if(|x| Path::new(x).file_name().unwrap() == "rich-presence-wrapper");

    let arg0 = args.next().map(PathBuf::from)?;
    let arg0 = arg0.file_name()?.to_os_string();
    let arg0 = Arg0(arg0);

    let args = Args(args);

    Some((arg0, args))
}

#[derive(Debug)]
pub struct Arg0(OsString);

impl Arg0 {
    pub fn binary_name(&self) -> &OsStr {
        &self.0
    }
}

#[derive(Debug)]
pub struct Args(Peekable<ArgsOs>);

impl Iterator for Args {
    type Item = OsString;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.len()))
    }
}

impl ExactSizeIterator for Args {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl FusedIterator for Args {}
