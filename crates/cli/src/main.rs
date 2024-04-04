// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::io::Write;

use anyhow::Result;
use beautytips::{ActionId, Reporter};
use tracing_subscriber::prelude::*;

mod arg_parse;

struct DebugReporter { }

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
            command: [
                "/bin/sh",
                "-c",
                "echo -e \"Foobar\"; sleep 2; exit 0"
            ]
            .iter()
            .map(ToString::to_string)
            .collect(),
            expected_exit_code: 0,
        },
        beautytips::ActionDefinition {
            id: beautytips::ActionId::new("fail").unwrap(),
            command: ["/bin/sh", "-c", "echo -e \"This will fail\"; sleep 5; exit 10"]
                .iter()
                .map(ToString::to_string)
                .collect(),
            expected_exit_code: 1,
        },
    ];

    let mut reporter = DebugReporter { };

    beautytips::run(
        std::env::current_dir()?,
        beautytips::InputFiles::Vcs(beautytips::VcsInput::default()),
        actions,
        Box::new(reporter),
    )?;

    Ok(())
}
