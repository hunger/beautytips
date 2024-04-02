// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{fmt, path::PathBuf};

/// `Result` from std, with the error type defaulting to xshell's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by an `xshell` operation.
pub struct Error {
    kind: Box<ErrorKind>,
}

/// Note: this is intentionally not public.
enum ErrorKind {
    CurrentDirectory { err: xshell::Error },
    NotADirectory { path: PathBuf },
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        let kind = Box::new(kind);
        Error { kind }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            ErrorKind::CurrentDirectory { err } => {
                write!(f, "{err}")
            }
            ErrorKind::NotADirectory { path } => {
                write!(f, "{path:?} is not a directory")
            }
        }?;
        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for Error {}

/// `pub(crate)` constructors, visible only in this crate.
impl Error {
    pub(crate) fn new_current_directory(err: xshell::Error) -> Self {
        ErrorKind::CurrentDirectory { err }.into()        
    }
    pub(crate) fn new_not_a_directory(path: PathBuf) -> Self {
        ErrorKind::NotADirectory { path }.into()
    }
}

#[test]
fn error_send_sync() {
    fn f<T: Send + Sync>() {}
    f::<Error>();
}
