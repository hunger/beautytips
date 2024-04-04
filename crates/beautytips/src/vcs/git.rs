// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::path::{Path, PathBuf};
use std::os::unix::ffi::OsStrExt;

use crate::vcs;

#[derive(Debug, Default)]
pub struct Git { }

impl Git {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl vcs::Vcs for Git {
    fn name(&self) -> &str {
        "git"
    }

    #[tracing::instrument]
    async fn changed_files(
        &self,
        _current_directory: &Path,
        _from_revision: Option<&String>,
        _to_revision: Option<&String>,
    ) -> crate::Result<Vec<std::path::PathBuf>> {
        todo!()
    }

    #[tracing::instrument]
    async fn is_supported(&self, current_directory: &Path) -> bool {
        tokio::process::Command::new("git").args(["--version"]).current_dir(current_directory).status().await.is_ok()
    }

    #[tracing::instrument]
    async fn repository_root(&self, current_directory: &Path) -> Option<PathBuf> {
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(current_directory)
            .output()
            .await.ok()?;
        tracing::trace!("top level result: {output:?}");
        if output.status.success() {
            let output = std::ffi::OsStr::from_bytes(&output.stdout[..(output.stdout.len() - 1)]);
            let path = PathBuf::from(output);
            Some(path)
        } else {
            None
        }
    }
}
