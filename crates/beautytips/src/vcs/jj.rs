// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{os::unix::ffi::OsStrExt, path::PathBuf};

use crate::vcs;

#[derive(Debug, Default)]
pub struct Jj { }

impl Jj {
    pub fn new() -> Self {
        Self {}
    }
}

impl vcs::Vcs for Jj {
    fn name(&self) -> &str {
        "jj"
    }

    fn changed_files(&self, _ctx: &crate::Context) -> Vec<std::path::PathBuf> {
        todo!()
    }

    fn is_supported(&self, ctx: &crate::Context) -> bool {
        xshell::cmd!(ctx.sh, "jj --version").quiet().output().is_ok()
    }

    fn repository_root(&self, ctx: &crate::Context) -> Option<std::path::PathBuf> {
        let output = xshell::cmd!(ctx.sh, "jj workspace root").quiet().output().ok()?;
        if output.status.success() {
            let output = std::ffi::OsStr::from_bytes(&output.stdout[..(output.stdout.len() - 1)]);
            let path = PathBuf::from(output);
            Some(path)
        } else {
            None
        }
    }
}
