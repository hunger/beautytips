// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use anyhow::{Context, Result};

mod arg_parse;

fn main() -> Result<()> {
    let config = arg_parse::parse_args();
    
    let ctx = beautytips::Context::new().context("Failed during basic setup")?;

    Ok(())
}
