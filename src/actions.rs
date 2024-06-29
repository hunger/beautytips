// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use anyhow::Context;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

mod args;
pub(crate) mod inputs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputCondition {
    Never,
    Success,
    Failure,
    Always,
}

#[derive(Clone, Debug, Eq)]
pub struct ActionDefinition {
    pub id: String,
    pub source: String,
    pub description: String,
    pub run_sequentially: bool,
    pub command: Vec<String>,
    pub show_output: OutputCondition,
    pub expected_exit_code: i32,
    pub input_filters: inputs::InputFilters,
}

impl PartialOrd for ActionDefinition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ActionDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id) && self.source.eq(&other.source)
    }
}

impl Ord for ActionDefinition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let cmp = self.id.cmp(&other.id);
        if cmp == std::cmp::Ordering::Equal {
            self.source.cmp(&other.source)
        } else {
            cmp
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActionDefinitionIterator<'a> {
    actions: &'a [ActionDefinition],
    indices: Vec<usize>,
    current_item: usize,
}

impl<'a> ActionDefinitionIterator<'a> {
    #[must_use]
    pub fn new(actions: &'a [ActionDefinition], indices: HashSet<usize>) -> Self {
        let indices = {
            let mut i: Vec<usize> = Vec::from_iter(indices);
            i.sort_unstable();
            i
        };

        Self {
            actions,
            indices,
            current_item: 0,
        }
    }
}

impl<'a> Iterator for ActionDefinitionIterator<'a> {
    type Item = &'a ActionDefinition;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.current_item;
        self.current_item += 1;
        self.indices.get(cur).and_then(|i| self.actions.get(*i))
    }
}

#[derive(Clone, Debug)]
pub enum ActionResult {
    Ok { stdout: Vec<u8>, stderr: Vec<u8> },
    Skipped,
    NotApplicable,
    Warn { stdout: Vec<u8>, stderr: Vec<u8> },
    Error { message: String },
}

#[derive(Clone, Debug)]
pub(crate) enum ActionUpdate {
    Started {
        action_id: String,
    },
    Done {
        action_id: String,
        result: ActionResult,
    },
}
pub(crate) type ActionUpdateSender = tokio::sync::mpsc::Sender<ActionUpdate>;
pub(crate) type ActionUpdateReceiver = tokio::sync::mpsc::Receiver<ActionUpdate>;

#[tracing::instrument]
async fn report(sender: &ActionUpdateSender, message: ActionUpdate) {
    sender
        .send(message)
        .await
        .expect("Failed to send message to reporter");
}

pub(crate) async fn has_unfiltered_input(
    inputs: &inputs::InputQuery,
    input_filters: &inputs::InputFilters,
    root_directory: &Path,
) -> bool {
    for k in input_filters.inputs() {
        if input_filters
            .filtered(k, inputs, root_directory)
            .await
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            return false;
        }
    }
    true
}

#[tracing::instrument(skip(inputs))]
async fn run_single_action(
    current_directory: PathBuf,
    extra_environment: Arc<HashMap<String, String>>,
    sender: ActionUpdateSender,
    action: &'static ActionDefinition,
    inputs: inputs::InputQuery,
) -> crate::Result<()> {
    tracing::debug!("running action '{}': {:?}", action.id, action.command);
    let action_id = format!("{}/{}", action.source, action.id);

    sender
        .send(ActionUpdate::Started {
            action_id: action_id.clone(),
        })
        .await
        .expect("Failed to send start message to reporter");

    if !has_unfiltered_input(&inputs, &action.input_filters, &current_directory).await {
        sender
            .send(ActionUpdate::Done {
                action_id: action_id.clone(),
                result: ActionResult::NotApplicable,
            })
            .await
            .expect("Failed to send message to reporter");
        return Ok(());
    }

    if std::env::var("SKIP")
        .unwrap_or_default()
        .split(',')
        .any(|s| s == action_id)
    {
        tracing::trace!("Skipping '{}'", action_id);
        report(
            &sender,
            ActionUpdate::Done {
                action_id: action_id.clone(),
                result: ActionResult::Skipped,
            },
        )
        .await;
        return Ok(());
    }

    let Some(command) = action.command.first() else {
        tracing::error!("No command in action '{}'", action_id);
        let message = format!("No command defined in action '{action_id}'");
        sender
            .send(ActionUpdate::Done {
                action_id: action_id.clone(),
                result: ActionResult::Error {
                    message: message.clone(),
                },
            })
            .await
            .expect("Failed to send message to reporter");
        return Err(anyhow::anyhow!(format!("Invalid configuration: {message}")));
    };

    let args = args::parse_args(
        &action.command,
        inputs,
        &current_directory,
        &action.input_filters,
    )
    .await;

    let mut args = match args {
        Ok(args) => args,
        Err(e) => {
            sender
                .send(ActionUpdate::Done {
                    action_id: action_id.clone(),
                    result: ActionResult::Error {
                        message: format!("Argument parsing failed: {e}"),
                    },
                })
                .await
                .expect("Failed to send message to reporter");
            return Ok(());
        }
    };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut invalid_exit_code = false;

    loop {
        let output = tokio::process::Command::new(command)
            .current_dir(current_directory.clone())
            .args(args.args_iter())
            .envs(extra_environment.iter())
            .output()
            .await
            .context(format!("Could not start '{command}"))?;

        tracing::trace!(
            "result of running action '{}' ({} {}): {output:?}",
            action_id,
            command,
            args.print()
        );

        if output.status.code() != Some(action.expected_exit_code) {
            tracing::debug!("Unexpected return code for action '{}'", action_id);
            invalid_exit_code = true;
        }

        stdout.extend_from_slice(&output.stdout);
        if !stdout.ends_with(b"\n") {
            stdout.push(b'\n');
        }
        stderr.extend_from_slice(&output.stderr);
        if !stderr.ends_with(b"\n") {
            stderr.push(b'\n');
        }

        if args.increment() {
            break;
        }
    }

    if invalid_exit_code {
        tracing::trace!("Failure running '{}'", action_id);
        if action.show_output == OutputCondition::Never
            || action.show_output == OutputCondition::Success
        {
            stdout = Vec::new();
            stderr = Vec::new();
        }

        report(
            &sender,
            ActionUpdate::Done {
                action_id: action_id.clone(),
                result: ActionResult::Warn { stdout, stderr },
            },
        )
        .await;
    } else {
        tracing::trace!("Success running '{}'", action_id);
        if action.show_output == OutputCondition::Never
            || action.show_output == OutputCondition::Failure
        {
            stdout = Vec::new();
            stderr = Vec::new();
        }

        report(
            &sender,
            ActionUpdate::Done {
                action_id: action_id.clone(),
                result: ActionResult::Ok { stdout, stderr },
            },
        )
        .await;
    }
    Ok(())
}

/// Run actions on `files`
///
/// # Errors
///
/// Not sure yet.
#[tracing::instrument]
pub async fn run(
    mut context: crate::ExecutionContext,
    sender: ActionUpdateSender,
    actions: ActionDefinitionIterator<'static>,
) -> crate::Result<()> {
    tracing::trace!("Starting actions");
    let cache_handle = inputs::setup_input_cache(
        context.root_directory.clone(),
        std::mem::take(&mut context.files_to_process),
    );
    let mut join_set = tokio::task::JoinSet::new();

    let extra_environment = Arc::new(context.extra_environment);

    // parallel phase:
    tracing::trace!("Entering parallel run phase");
    for a in actions.clone().filter(|ad| !ad.run_sequentially) {
        let cd = context.root_directory.clone();
        let ee = extra_environment.clone();
        let tx = sender.clone();

        tracing::trace!("Spawning task for action {}", a.id);

        join_set.spawn(run_single_action(cd, ee, tx, a, cache_handle.query()));
    }

    tracing::trace!("Joining actions: {}", join_set.len());

    while let Some(r) = join_set.join_next().await {
        tracing::debug!("joined => in JS: {}", join_set.len());
        r.expect("Join Error found")?;
    }

    // sequential phase:
    tracing::trace!("Entering sequential run phase");
    for a in actions.filter(|ad| ad.run_sequentially) {
        let cd = context.root_directory.clone();
        let ee = extra_environment.clone();
        let tx = sender.clone();

        tracing::trace!("Spawning task for action {}", a.id);

        run_single_action(cd, ee, tx, a, cache_handle.query()).await?;
    }

    tracing::trace!("All actions started");

    drop(sender);

    cache_handle.finish().await;

    tracing::trace!("Done running actions");
    Ok(())
}
