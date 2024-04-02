// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::path::PathBuf;
use std::os::unix::ffi::OsStrExt;

use crate::vcs;

#[derive(Debug, Default)]
pub struct Git { }

impl Git {
    pub fn new() -> Self {
        Self {}
    }
}

impl vcs::Vcs for Git {
    fn name(&self) -> &str {
        "git"
    }

    fn changed_files(&self, _ctx: &crate::Context) -> Vec<PathBuf> {
        todo!()
    }

    fn is_supported(&self, ctx: &crate::Context) -> bool {
        xshell::cmd!(ctx.sh, "git --version").quiet().output().is_ok()
    }

    fn repository_root(&self, ctx: &crate::Context) -> Option<PathBuf> {
        let output = xshell::cmd!(ctx.sh, "git rev-parse --show-toplevel").quiet().output().ok()?;
        if output.status.success() {
            let output = std::ffi::OsStr::from_bytes(&output.stdout[..(output.stdout.len() - 1)]);
            let path = PathBuf::from(output);
            Some(path)
        } else {
            None
        }
    }
}
