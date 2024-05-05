// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use anyhow::{Context, Result};
use tracing_subscriber::prelude::*;

mod arg_parse;
mod config;
mod reporter;

fn main() -> Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    let config = config::load_user_configuration()?;

    tracing_subscriber::registry()
        .with(stdout_log.with_filter(tracing_subscriber::filter::LevelFilter::TRACE))
        .init();

    let actions = config.action_group("test_me").unwrap();

    let reporter = reporter::Reporter::default();

    beautytips::run(
        std::env::current_dir()?,
        beautytips::InputFiles::Vcs(beautytips::VcsInput::default()),
        actions,
        Box::new(reporter),
    )?;

    Ok(())
}
