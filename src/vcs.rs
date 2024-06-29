// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

// spell-checker:ignore vcses

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};

mod git;
mod jj;

#[allow(clippy::module_name_repetitions)]
pub type BoxedVcs = Box<dyn Vcs + Sync + Send>;
#[allow(clippy::module_name_repetitions)]
pub type DynVcs = &'static (dyn Vcs + Sync + Send);

static KNOWN_VCSES: OnceLock<Vec<BoxedVcs>> = OnceLock::new();

pub fn output_to_string(input: &[u8]) -> String {
    // SAFETY: This is OS output, it should be OK to convert to an OsStr (I hope)
    let output = unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(input) };

    let output = output.to_string_lossy().to_string();
    let output = output.strip_suffix('\n').unwrap_or(&output);
    let output = output.strip_suffix('\r').unwrap_or(output);

    output.to_string()
}

/// Trait used to support different version control systems
#[async_trait::async_trait]
pub trait Vcs {
    /// The name of the version control system
    fn name(&self) -> &str;

    /// Find changed files in the `root_directory`
    ///
    /// # Errors
    ///
    /// Reports an error if the data could not get retrieved.
    async fn changed_files(
        &self,
        current_directory: &Path,
        from_revision: Option<&String>,
        to_revision: Option<&String>,
    ) -> crate::Result<Vec<PathBuf>>;

    /// Find the directory root
    async fn repository_root(&self, current_directory: &Path) -> Option<PathBuf>;
}

#[must_use]
fn known_vcses() -> Vec<DynVcs> {
    KNOWN_VCSES
        .get_or_init(|| vec![Box::new(jj::Jj::new()), Box::new(git::Git::new())])
        .iter()
        .map(Box::as_ref)
        .collect()
}

async fn helper(vcs: DynVcs, current_directory: &Path) -> Option<(DynVcs, PathBuf)> {
    vcs.repository_root(current_directory)
        .await
        .map(|r| (vcs, r))
}

#[must_use]
async fn auto_detect_vcs(current_directory: &Path) -> Option<(DynVcs, PathBuf)> {
    futures::future::join_all(
        known_vcses()
            .into_iter()
            .map(|vcs| helper(vcs, current_directory)),
    )
    .await
    .into_iter()
    .flatten()
    .next()
}

#[must_use]
fn vcs_by_name(name: &str) -> Option<DynVcs> {
    known_vcses().into_iter().find(|v| v.name() == name)
}

#[tracing::instrument]
async fn vcs_for_configuration(
    current_directory: &Path,
    config: crate::VcsInput,
) -> crate::Result<(DynVcs, PathBuf)> {
    if let Some(tool) = &config.tool {
        tracing::debug!("Looking for VCS {tool}");
        let Some(vcs) = vcs_by_name(tool) else {
            return Err(anyhow::anyhow!(format!("Version control system '{tool}' is not supported")));
        };

        let Some(root_path) = vcs.repository_root(current_directory).await else {
            return Err(anyhow::anyhow!(format!("No repository of version control system '{tool}' found")));
        };

        Ok((vcs, root_path))
    } else {
        tracing::debug!("Auto-detecting VCS");
        auto_detect_vcs(current_directory).await.ok_or(anyhow::anyhow!("Could not auto-detect a supported version control system"))
    }
}

/// Find all the files that changed based on the `VcsInput` configuration
///
/// # Errors
///
/// Reports invalid configuration errors or others when the data could not get retrieved
#[tracing::instrument]
pub(crate) async fn find_files_changed(
    current_directory: PathBuf,
    config: crate::VcsInput,
) -> crate::Result<crate::ExecutionContext> {
    let to_rev = config.to_revision.clone();
    let from_rev = config.from_revision.clone();

    let (vcs, repo_path) = vcs_for_configuration(&current_directory, config).await?;
    tracing::trace!(
        "Using {} to look up changed files in {repo_path:?}...",
        vcs.name()
    );

    let files_to_process = vcs
        .changed_files(&repo_path, from_rev.as_ref(), to_rev.as_ref())
        .await?;

    Ok(crate::ExecutionContext {
        root_directory: repo_path,
        extra_environment: HashMap::from([
            ("BEAUTYTIPS_INPUT".to_string(), "vcs".to_string()),
            ("BEAUTYTIPS_VCS".to_string(), vcs.name().to_string()),
            (
                "BEAUTYTIPS_VCS_FROM_REV".to_string(),
                from_rev.unwrap_or_default(),
            ),
            (
                "BEAUTYTIPS_VCS_TO_REV".to_string(),
                to_rev.unwrap_or_default(),
            ),
        ]),
        files_to_process,
    })
}
