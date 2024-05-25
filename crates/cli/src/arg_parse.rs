// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use clap::{Args, Parser, Subcommand};

use std::{ffi::OsString, path::PathBuf};

use crate::config::QualifiedActionId;

/// Where to get files to look at from
#[derive(Clone, Debug, Args)]
#[group(required = true, multiple = false)]
struct CliInputFiles {
    #[arg(long = "from-vcs")]
    #[allow(clippy::option_option)]
    vcs: Option<Option<String>>,
    #[arg(long = "from-files", num_args = 1.., value_name = "FILE")]
    files: Option<Vec<PathBuf>>,
    #[arg(long = "from-dir")]
    directory: Option<PathBuf>,
}

/// Where to get files to look at from
#[derive(Clone, Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
enum CliCommand {
    Builtin {
        action: String,
        arguments: Vec<OsString>,
    },
    ListActions,
    /// Doc comment
    ListFiles {
        #[command(flatten)]
        source: CliInputFiles,
    },
    Run {
        #[command(flatten)]
        source: CliInputFiles,
        #[arg(long = "actions", num_args = 1.., value_name = "ACTION")]
        actions: Vec<QualifiedActionId>,
    },
}

#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long = "debug", action = clap::ArgAction::Count, env = "BEAUTY_TIPS_LOG_LEVEL")]
    debug_level: u8,
    #[arg(long = "verbose", action = clap::ArgAction::Count)]
    verbosity_level: u8,

    #[command(subcommand)]
    action: CliCommand,
}

#[derive(Clone, Debug)]
pub enum Command {
    Builtin {
        action: String,
        arguments: Vec<OsString>,
    },
    ListFiles {
        source: beautytips::InputFiles,
    },
    ListActions {},
    RunActions {
        source: beautytips::InputFiles,
        actions: Vec<QualifiedActionId>,
    },
}

#[derive(Clone, Debug)]
pub struct CommandlineConfiguration {
    pub debug_level: u8,
    pub verbosity_level: u8,
    pub command: Command,
}

fn generate_input_files(inputs: &CliInputFiles) -> anyhow::Result<beautytips::InputFiles> {
    if let Some(vcs) = &inputs.vcs {
        // TODO: Handle from/to
        Ok(beautytips::InputFiles::Vcs(beautytips::VcsInput {
            tool: vcs.clone(),
            from_revision: None,
            to_revision: None,
        }))
    } else if let Some(files) = &inputs.files {
        Ok(beautytips::InputFiles::FileList(files.clone()))
    } else if let Some(directory) = &inputs.directory {
        Ok(beautytips::InputFiles::AllFiles(directory.clone()))
    } else {
        Err(anyhow::anyhow!(
            "Unknown input file list generation found on command line"
        ))
    }
}

pub fn command() -> anyhow::Result<CommandlineConfiguration> {
    let cli = Cli::parse();

    let command = match cli.action {
        CliCommand::Builtin { action, arguments } => Command::Builtin { action, arguments },
        CliCommand::ListActions => Command::ListActions {},
        CliCommand::ListFiles { source } => Command::ListFiles {
            source: generate_input_files(&source)?,
        },
        CliCommand::Run { source, actions } => Command::RunActions {
            source: generate_input_files(&source)?,
            actions,
        },
    };

    Ok(CommandlineConfiguration {
        debug_level: cli.debug_level,
        verbosity_level: cli.verbosity_level,
        command,
    })
}
