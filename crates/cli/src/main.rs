// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use anyhow::Result;
use tracing_subscriber::prelude::*;

mod arg_parse;

struct DebugReporter {}

impl beautytips::Reporter for DebugReporter {
    fn report_start(&mut self, action_id: beautytips::ActionId) {
        println!("STARTED: {action_id}");
    }

    fn report_done(&mut self, action_id: beautytips::ActionId, result: beautytips::ActionResult) {
        println!("DONE   : {action_id} => {result:?}");
    }
}

fn main() -> Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    tracing_subscriber::registry().with(stdout_log).init();

    let actions = vec![
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("echo_test").unwrap(),
            command: ["/bin/sh", "-c", "echo -e \"baz\"; sleep 2; exit 0"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
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
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("file").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{files}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("files").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{files...}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("foobar").unwrap(),
            command: ["/bin/sh", "-c", "echo -e {{foobar...}}"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 0,
        },
    ];

    let reporter = DebugReporter {};

    beautytips::run(
        std::env::current_dir()?,
        beautytips::InputFiles::Vcs(beautytips::VcsInput::default()),
        &actions,
        Box::new(reporter),
    )?;

    Ok(())
}
