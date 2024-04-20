// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::fmt;

/// `Result` from std, with the error type defaulting to xshell's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error returned by an `xshell` operation.
pub struct Error {
    kind: Box<ErrorKind>,
}

/// Note: this is intentionally not public.
enum ErrorKind {
    InputGeneratorError {
        input_query: String,
        message: String,
    },
    InvalidConfiguration {
        message: String,
    },
    ProcessFailed {
        command: String,
        error: std::io::Error,
    },
    UnexpectedExitCode {
        command: String,
        expected: i32,
        actual: i32,
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
            ErrorKind::InputGeneratorError {
                message,
                input_query,
            } => {
                write!(f, "Failed to generate input \"{input_query}\": {message}")
            }
            ErrorKind::InvalidConfiguration { message } => {
                write!(f, "Invalid configuration: {message}")
            }
            ErrorKind::ProcessFailed { command, error } => {
                write!(f, "Process {command} failed: {error}")
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
    pub(crate) fn new_process_failed(command: &str, error: std::io::Error) -> Self {
        ErrorKind::ProcessFailed {
            command: command.to_string(),
            error,
        }
        .into()
    }
    pub(crate) fn new_unexpected_exit_code(
        command: &str,
        expected: i32,
        actual: i32,
    ) -> Self {
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
