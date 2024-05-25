// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use anyhow::{Context, Result};
use tracing_subscriber::prelude::*;

mod arg_parse;
mod builtin_commands;
mod config;
mod reporter;

fn main() -> Result<()> {
    let command = arg_parse::command().context("Failed to parse command line arguments")?;

    let max_level = match command.debug_level {
        0 => tracing_subscriber::filter::LevelFilter::ERROR,
        1 => tracing_subscriber::filter::LevelFilter::WARN,
        2 => tracing_subscriber::filter::LevelFilter::INFO,
        3 => tracing_subscriber::filter::LevelFilter::DEBUG,
        _ => tracing_subscriber::filter::LevelFilter::TRACE,
    };

    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    let config = config::load_user_configuration()?;

    tracing_subscriber::registry()
        .with(stdout_log.with_filter(max_level))
        .init();

    match command.command {
        arg_parse::Command::Builtin { action, arguments } => {
            let exit_code = builtin_commands::run_builtin_command(&action, &arguments)?;
            std::process::exit(exit_code);
        }
        arg_parse::Command::ListActions {} => {
            for ag in config.action_groups.keys() {
                println!("{ag} (group)");
            }
            for a in config.action_map.keys() {
                println!("{a}");
            }

            Ok(())
        }
        arg_parse::Command::ListFiles { source } => {
            let (root_dir, files) =
                beautytips::collect_input_files(std::env::current_dir()?, source)?;
            println!("root directory: {root_dir:?}");
            for f in &files {
                println!("{f:?}");
            }
            Ok(())
        }
        arg_parse::Command::RunActions {
            source: inputs,
            actions,
        } => {
            let reporter = reporter::Reporter::default();

            let actions = config.named_actions(&actions)?;

            beautytips::run(
                std::env::current_dir()?,
                inputs,
                actions,
                Box::new(reporter),
            )?;

            Ok(())
        }
    }
}
