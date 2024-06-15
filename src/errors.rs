// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{fmt, path::PathBuf};

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Error {
    kind: Box<ErrorKind>,
}

/// Note: this is intentionally not public.
enum ErrorKind {
    FileError {
        message: String,
        error: std::io::Error,
    },
    InputGeneratorError {
        input_query: String,
        message: String,
    },
    InvalidConfiguration {
        message: String,
    },
    UnexpectedExitCode {
        command: String,
        expected: i32,
        actual: i32,
    },
    DirectoryWalkError {
        error: ignore::Error,
        base_directory: PathBuf,
    },
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
            ErrorKind::DirectoryWalkError {
                error,
                base_directory,
            } => {
                write!(f, "Could not collect files in {base_directory:?}: {error}")
            }
            ErrorKind::FileError { message, error } => {
                write!(f, "{message}: {error}")
            }
            ErrorKind::InputGeneratorError {
                message,
                input_query,
            } => {
                write!(f, "Failed to generate input \"{input_query}\": {message}")
            }
            ErrorKind::InvalidConfiguration { message } => {
                write!(f, "Invalid configuration: {message}")
            }
            ErrorKind::UnexpectedExitCode {
                command,
                expected,
                actual,
            } => {
                write!(f, "Unexpected exit code {actual:?} when running {command}. Expected was {expected}")
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
    pub(crate) fn new_directory_walk(base_directory: PathBuf, error: ignore::Error) -> Self {
        ErrorKind::DirectoryWalkError {
            error,
            base_directory,
        }
        .into()
    }
    pub(crate) fn new_io_error(message: &str, error: std::io::Error) -> Self {
        ErrorKind::FileError {
            message: message.to_string(),
            error,
        }
        .into()
    }
    pub(crate) fn new_input_generator(input_query: String, message: String) -> Self {
        ErrorKind::InputGeneratorError {
            input_query,
            message,
        }
        .into()
    }
    pub(crate) fn new_invalid_configuration(message: String) -> Self {
        ErrorKind::InvalidConfiguration { message }.into()
    }
    pub(crate) fn new_unexpected_exit_code(command: &str, expected: i32, actual: i32) -> Self {
        ErrorKind::UnexpectedExitCode {
            command: command.to_string(),
            expected,
            actual,
        }
        .into()
    }
}

#[test]
fn error_send_sync() {
    fn f<T: Send + Sync>() {}
    f::<Error>();
}
