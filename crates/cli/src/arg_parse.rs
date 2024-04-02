// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{ffi::OsString, path::PathBuf};

use clap::{Arg, ArgAction, ArgGroup};

#[derive(Clone, Debug)]
pub enum Inputs {
    Vcs {
        vcs: Option<String>,
        from: Option<String>,
        to: Option<String>,
    },
    FileSystem {
        root_directory: PathBuf,
        use_ignore_files: bool,
    },
    Files(Vec<PathBuf>),
    StdIn,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub inputs: Inputs,
}

fn command() -> clap::Command {
    clap::command!()
        // Inputs:
        .arg(
            Arg::new("from-vcs")
                .help("Get files to proceess from version control system")
                .long("from-vcs")
                .default_missing_value("auto-detect")
                .value_name("VCS")
                .num_args(0..=1)
                .require_equals(true),
        )
        .arg(
            Arg::new("from")
                .help("The first version in version control to look up files from")
                .long("from")
                .value_name("VERSION")
                .num_args(1)
                .require_equals(true)
                .requires("from-vcs"),
        )
        .arg(
            Arg::new("to")
                .help("The last version in version control to look up files from")
                .long("to")
                .value_name("VERSION")
                .num_args(1)
                .require_equals(true)
                .requires("from-vcs"),
        )
        .arg(
            Arg::new("from-files")
                .help("A list of files to precoess")
                .long("from-files")
                .value_name("FILE")
                .num_args(1)
                .value_delimiter(',')
                .require_equals(true),
        )
        .arg(
            Arg::new("from-fs")
                .help("Scan the filesystem for files to process")
                .long("from-fs")
                .value_name("DIRECTORY")
                .num_args(1)
                .action(ArgAction::Set)
                .require_equals(true),
        )
        .arg(
            Arg::new("use-ignore-files")
                .help("Use ignore files")
                .long("use-ignore-files")
                .default_missing_value("true")
                .default_value("true")
                .action(ArgAction::Set)
                .require_equals(true)
                .requires("from-fs"),
        )
        .arg(
            Arg::new("from-stdin")
                .help("Get file names from stdin (one file per line)")
                .long("from-stdin")
                .action(ArgAction::SetTrue)
        )
        .group(ArgGroup::new("inputs").args(["from-vcs", "from-files", "from-fs", "from-stdin"]))
}

pub fn parse_args() -> anyhow::Result<Config> {
    _parse_args(std::env::args_os())
}

fn _parse_args(args: impl Iterator<Item = OsString>) -> anyhow::Result<Config> {
    let matches = command().try_get_matches_from(args)?;

    eprintln!("Matches: {matches:?}");
    Ok(Config {
        inputs: Inputs::StdIn,
    })
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn test_command_version() {
        let result = command().try_get_matches_from(vec!["foo", "--version"]);
        assert!(result.is_err());
        let Err(e) = result else {
            unreachable!();
        };
        assert_eq!(e.kind(), ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_command_help() {
        let result = command().try_get_matches_from(vec!["foo", "--help"]);
        assert!(result.is_err());
        let Err(e) = result else {
            unreachable!();
        };
        assert_eq!(e.kind(), ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_command_no_inputs() {
        let result = command().try_get_matches_from(vec!["foo"]);
        eprintln!("{result:?}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_input_vcs_ok() {
        let result = command().try_get_matches_from(vec!["foo", "--from-vcs"]);
        eprintln!("{result:?}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_input_files_ok() {
        let result = command().try_get_matches_from(vec!["foo", "--from-files=foo,bar"]);
        eprintln!("{result:?}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_input_filesystem_ok() {
        let result = command().try_get_matches_from(vec!["foo", "--from-fs=/tmp/foobar"]);
        eprintln!("{result:?}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_input_stdin_ok() {
        let result = command().try_get_matches_from(vec!["foo", "--from-stdin"]);
        eprintln!("{result:?}");
        assert!(result.is_ok());
    }
}
