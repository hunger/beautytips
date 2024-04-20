// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

mod actions;
mod errors;
pub(crate) mod util;
pub(crate) mod vcs;

use std::path::PathBuf;

use actions::ActionUpdateReceiver;
pub use actions::{ActionDefinition, ActionId};
pub use errors::{Error, Result};

#[derive(Clone, Debug, Default)]
pub struct VcsInput {
    /// The version control tool to use (or None for auto-detect)
    pub tool: Option<String>,
    /// The revision to start the comparision from (or None for default)
    pub from_revision: Option<String>,
    /// The revision to stop the comparision at (or None for default)
    pub to_revision: Option<String>,
}

#[derive(Clone, Debug)]
pub enum InputFiles {
    Vcs(VcsInput),
}

impl Default for InputFiles {
    fn default() -> Self {
        Self::Vcs(VcsInput::default())
    }
}

pub use actions::ActionResult;

/// Report results of an Action
pub trait Reporter {
    fn report_start(&mut self, action_id: ActionId);
    fn report_done(&mut self, action_id: ActionId, result: ActionResult);
}

/// Collect the input files based on `Context` and configuration
///
/// # Errors
///
/// Mostly `InvalidConfiguration`, but others are possible when data collection fails.
#[tracing::instrument]
async fn collect_input_files(
    current_directory: PathBuf,
    inputs: InputFiles,
) -> Result<(PathBuf, Vec<PathBuf>)> {
    match inputs {
        InputFiles::Vcs(config) => vcs::find_files_changed(current_directory, config).await,
    }
}

#[tracing::instrument(skip(reporter))]
async fn handle_reports(mut reporter: Box<dyn Reporter>, mut rx: ActionUpdateReceiver) {
    tracing::trace!("running local reporter task");
    loop {
        let _span = tracing::span!(tracing::Level::TRACE, "reporter_callback_handler");
        let Some(m) = rx.recv().await else {
            tracing::trace!("reporter is done");
            break;
        };
        match m {
            actions::ActionUpdate::Started { action_id } => {
                tracing::debug!("action {action_id} start");
                reporter.report_start(action_id);
            }
            actions::ActionUpdate::Done { action_id, result } => {
                tracing::debug!("action {action_id} complete: {result:?}");
                reporter.report_done(action_id, result);
            }
        }
    }

    tracing::trace!("Local reporter task is done");
}

/// Run beautytips
///
/// # Errors
///
/// Mostle `InvalidConfiguration`, but others are possible when data collection fails.
///
/// # Panics
///
/// Panics whenever tokio decides to panic.
#[tracing::instrument(skip(reporter))]
pub fn run<'a>(
    current_directory: PathBuf,
    inputs: InputFiles,
    actions: &'a [actions::ActionDefinition],
    reporter: Box<dyn Reporter>,
) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime setup failed")
        .block_on(async move {
            let _span = tracing::span!(tracing::Level::TRACE, "tokio_runtime");
            tracing::trace!("Inside tokio runtime block");

            let (root_directory, files) = collect_input_files(current_directory, inputs).await?;

            // # Safety: actions are valid during the entire time the
            // o runtime is up. So it should be safe to treat the `actions`
            // as static.
            let actions = unsafe {
                std::mem::transmute::<
                    &'a [actions::ActionDefinition],
                    &'static [actions::ActionDefinition],
                >(actions)
            };

            tracing::debug!(
                "detected root directory: {root_directory:?} with changed files: {files:?}"
            );

            let (tx, rx) = tokio::sync::mpsc::channel(10);
            let runner = tokio::task::spawn(async move {
                let _span = tracing::span!(tracing::Level::TRACE, "runner_task");

                actions::run(root_directory, tx, actions, files).await
            });

            handle_reports(reporter, rx).await;
            runner.await.expect("Join Error")
        })
}
