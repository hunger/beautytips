// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

pub(crate) mod actions;
pub(crate) mod util;
pub(crate) mod vcs;

use std::{collections::HashMap, path::PathBuf};

use actions::ActionUpdateReceiver;
pub use actions::{
    inputs::InputFilters, ActionDefinition, ActionDefinitionIterator, OutputCondition,
};

use anyhow::Context;

type Result<T> = std::result::Result<T, anyhow::Error>;
type SendableResult<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Default)]
pub struct VcsInput {
    /// The version control tool to use (or None for auto-detect)
    pub tool: Option<String>,
    /// The revision to start the comparison from (or None for default)
    pub from_revision: Option<String>,
    /// The revision to stop the comparison at (or None for default)
    pub to_revision: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ExecutionContext {
    pub root_directory: PathBuf,
    pub extra_environment: HashMap<String, String>,
    pub files_to_process: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum InputFiles {
    Vcs(VcsInput),
    FileList(Vec<PathBuf>),
    AllFiles(PathBuf),
}

impl Default for InputFiles {
    fn default() -> Self {
        Self::Vcs(VcsInput::default())
    }
}

pub use actions::ActionResult;

/// Report results of an Action
pub trait Reporter {
    fn report_start(&mut self, taction_id: String);
    fn report_done(&mut self, action_id: String, result: ActionResult);

    fn finish(&mut self);
}

/// Collect the input files based on `Context` and configuration
///
/// # Errors
///
/// Mostly `InvalidConfiguration`, but others are possible when data collection fails.
#[tracing::instrument]
async fn collect_input_files_impl(
    current_directory: PathBuf,
    inputs: InputFiles,
) -> Result<ExecutionContext> {
    assert!(current_directory.is_absolute());

    let mut context = match inputs {
        InputFiles::Vcs(config) => vcs::find_changed_files(current_directory, config).await,
        InputFiles::FileList(files) => Ok(ExecutionContext {
            root_directory: current_directory,
            extra_environment: HashMap::from([(
                "BEAUTYTIPS_INPUT".to_string(),
                "files".to_string(),
            )]),
            files_to_process: files,
        }),
        InputFiles::AllFiles(base_dir) => {
            let files = ignore::WalkBuilder::new(base_dir.clone())
                .build()
                .map(|d| d.map(ignore::DirEntry::into_path))
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("Failed to walk directory tree below '{base_dir:?}'")?;
            Ok(ExecutionContext {
                root_directory: current_directory,
                extra_environment: HashMap::from([(
                    "BEAUTYTIPS_INPUT".to_string(),
                    "dir".to_string(),
                )]),
                files_to_process: files,
            })
        }
    }?;

    let root_directory = tokio::fs::canonicalize(&context.root_directory)
        .await
        .context(format!("Could not canonicalize '{:?}", context.root_directory))?;

    std::env::set_current_dir(&root_directory).
        context(format!(
                "Failed to set current directory to {:?}",
                context.root_directory
            ))?;

    let mut canonical_files = Vec::new();
    for f in &context.files_to_process {
        let meta = tokio::fs::metadata(&f)
            .await
            .context(format!("Failed to get metadata for {f:?}"))?;
        if meta.is_dir() {
            continue;
        }

        let f = tokio::fs::canonicalize(&f)
            .await
            .context(format!("Could not canonicalize {f:?}"))?;

        if f.is_absolute() {
            if f.starts_with(&root_directory) {
                canonical_files.push(f);
            }
        } else if !f.starts_with("..") {
            canonical_files.push(root_directory.join(f));
        }
    }
    context.files_to_process = canonical_files;

    Ok(context)
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

    reporter.finish();
    tracing::trace!("Local reporter task is done");
}

/// Collect files only
///
/// # Errors
///
/// Mostly `InvalidConfiguration`, but others are possible when data collection fails.
///
/// # Panics
///
/// Panics whenever tokio decides to panic.
#[tracing::instrument]
pub fn collect_input_files<'a>(
    current_directory: PathBuf,
    inputs: InputFiles,
) -> Result<(PathBuf, Vec<PathBuf>)> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime setup failed")
        .block_on(async move {
            let _span = tracing::span!(tracing::Level::TRACE, "tokio_runtime");
            tracing::trace!("Inside tokio runtime block");

            collect_input_files_impl(current_directory, inputs).await
        })
        .map(|mut context| {
            (
                std::mem::take(&mut context.root_directory),
                std::mem::take(&mut context.files_to_process),
            )
        })
}

/// Run beautytips
///
/// # Errors
///
/// Mostly `InvalidConfiguration`, but others are possible when data collection fails.
///
/// # Panics
///
/// Panics whenever tokio decides to panic.
#[tracing::instrument(skip(reporter))]
pub fn run<'a>(
    current_directory: PathBuf,
    inputs: InputFiles,
    actions: actions::ActionDefinitionIterator<'a>,
    reporter: Box<dyn Reporter>,
) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime setup failed")
        .block_on(async move {
            let _span = tracing::span!(tracing::Level::TRACE, "tokio_runtime");
            tracing::trace!("Inside tokio runtime block");

            let context = collect_input_files_impl(current_directory, inputs).await?;

            tracing::debug!(
                "Detected root directory: {:?} with changed files: {:?}",
                context.root_directory,
                context.files_to_process
            );

            // # Safety: actions are valid during the entire time the
            // o runtime is up. So it should be safe to treat the `actions`
            // as static.
            let actions = unsafe {
                std::mem::transmute::<
                    actions::ActionDefinitionIterator<'a>,
                    actions::ActionDefinitionIterator<'static>,
                >(actions)
            };

            let (tx, rx) = tokio::sync::mpsc::channel(10);
            let runner = tokio::task::spawn(async move {
                let _span = tracing::span!(tracing::Level::TRACE, "runner_task");

                tracing::debug!("Runner task started");

                let result = actions::run(context, tx, actions).await;

                tracing::debug!("Runner task finished");

                result
            });

            handle_reports(reporter, rx).await;
            runner.await.expect("Join Error")
        })
}
