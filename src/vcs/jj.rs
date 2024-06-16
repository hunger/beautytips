// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

// spell-checker:ignore interdiff

use std::path::{Path, PathBuf};

use crate::vcs;

#[derive(Debug, Default)]
pub struct Jj {}

impl Jj {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl vcs::Vcs for Jj {
    fn name(&self) -> &str {
        "jj"
    }

    #[tracing::instrument]
    async fn changed_files(
        &self,
        current_directory: &Path,
        from_revision: Option<&String>,
        to_revision: Option<&String>,
    ) -> crate::Result<Vec<std::path::PathBuf>> {
        let from = from_revision.map_or("--from=@-".to_string(), |fr| format!("--from={fr}"));
        let to = to_revision.map_or("--to=@".to_string(), |to| format!("--to={to}"));

        let output = tokio::process::Command::new(self.name())
            .args(["--color=never", "interdiff", "-s", &from, &to])
            .current_dir(current_directory)
            .output()
            .await
            .map_err(|e| {
                crate::Error::new_io_error(&format!("Could not run {}", self.name()), e)
            })?;

        tracing::trace!("changed files result: {output:?}");

        if let Some(actual) = output.status.code() {
            if actual != 0 {
                return Err(crate::Error::new_unexpected_exit_code(
                    self.name(),
                    0,
                    actual,
                ));
            }
        }

        Ok(super::output_to_string(&output.stdout)
            .lines()
            .filter(|l| l.len() > 2 && &l[0..2] != "D ")
            .map(|l| PathBuf::from(&l[2..]))
            .collect())
    }

    #[tracing::instrument]
    async fn repository_root(&self, current_directory: &Path) -> Option<std::path::PathBuf> {
        let output = tokio::process::Command::new(self.name())
            .args(["--color=never", "workspace", "root"])
            .current_dir(current_directory)
            .output()
            .await
            .ok()?;

        tracing::trace!("top level result: {output:?}");

        output
            .status
            .success()
            .then_some(PathBuf::from(&super::output_to_string(&output.stdout)))
    }
}
