// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::collections::HashMap;

use anyhow::Result;
use tracing_subscriber::prelude::*;

mod arg_parse;
mod reporter;

fn main() -> Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    tracing_subscriber::registry()
        .with(stdout_log.with_filter(tracing_subscriber::filter::LevelFilter::WARN))
        .init();

    let actions = vec![
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("echo_test").unwrap(),
            command: ["/bin/sh", "-c", "echo -e \"baz\"; sleep 2; exit 0"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
            input_filters: HashMap::default(),
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("fail").unwrap(),
            command: [
                "/bin/sh",
                "-c",
                "echo -e \"This will fail\"; sleep 5; exit 10",
            ]
            .iter()
            .map(ToString::to_string)
            .collect(),
            expected_exit_code: 1,
            input_filters: HashMap::default(),
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("file").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{files}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
            input_filters: HashMap::default(),
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("files").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{files...}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
            input_filters: HashMap::default(),
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("foobar").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{foobar...}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
            input_filters: HashMap::default(),
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("file_filtered").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{files...}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
            input_filters: vec![(
                "files".to_string(),
                vec![glob::Pattern::new("*.ignoreme").unwrap()],
            )]
            .into_iter()
            .collect(),
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("cargo_targets").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{cargo_targets...}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
            input_filters: HashMap::default(),
        },
    ];

    let reporter = reporter::Reporter::default();

    beautytips::run(
        std::env::current_dir()?,
        beautytips::InputFiles::Vcs(beautytips::VcsInput::default()),
        &actions,
        Box::new(reporter),
    )?;

    Ok(())
}
