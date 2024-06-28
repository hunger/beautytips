// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::path::{Path, PathBuf};

use crate::vcs;

use anyhow::Context;

pub fn zero_split_files(output: &[u8]) -> Vec<PathBuf> {
    output
        .split(|i| *i == 0)
        .filter(|s| !s.is_empty())
        .map(|s| PathBuf::from(&super::output_to_string(s)))
        .collect()
}

#[derive(Debug, Default)]
pub struct Git {}

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
        current_directory: &Path,
        from_revision: &Option<String>,
        to_revision: &Option<String>,
    ) -> crate::Result<Vec<std::path::PathBuf>> {
        let args = {
            let mut tmp = vec![
                "diff".to_string(),
                "--name-only".to_string(),
                "--no-ext-diff".to_string(),
                "-z".to_string(),
            ];
            match (from_revision, to_revision) {
                (None, None) => { /* do nothing */ }
                (Some(from), None) => tmp.push(from.clone()),
                (None, Some(to)) => {
                    tmp.push(format!("{to}~"));
                    tmp.push(to.clone());
                }
                (Some(from), Some(to)) => {
                    tmp.push(from.clone());
                    tmp.push(to.clone());
                }
            };
            tmp
        };

        let output = tokio::process::Command::new("git")
            .args(args)
            .current_dir(current_directory)
            .output()
            .await
            .context("Failed to run git")?;

        tracing::trace!("diff {from_revision:?} {to_revision:?} => {output:?}");

        if output.status.success() {
            return Ok(zero_split_files(&output.stdout));
        }
        Ok(vec![])
    }

    #[tracing::instrument]
    async fn repository_root(&self, current_directory: &Path) -> Option<PathBuf> {
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
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
