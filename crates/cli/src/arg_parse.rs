// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::path::PathBuf;

use clap::Arg;

fn command() -> clap::Command {
    clap::command!()
        // Inputs:
        .arg(
            Arg::new("files-from")
                .long("files-from")
                .help("Source for files to check")
                .value_parser(["vcs"])
                .default_value("vcs"),
        )
        .arg(
            Arg::new("directory")
                .long("directory")
                .help("The directory to work in")
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("."),
        )
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
}
