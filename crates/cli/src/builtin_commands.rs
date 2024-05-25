// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::ffi::OsString;

pub fn run_builtin_command(action: &str, arguments: &[OsString]) -> anyhow::Result<()> {
    eprintln!("Running builtin command {action} {arguments:?}");

    match action {
        "foo" => {
            eprintln!("I know what to do!");
            Ok(())
        }
        _ => Err(anyhow::anyhow!(format!(
            "{action} is not a builtin command"
        ))),
    }
}
