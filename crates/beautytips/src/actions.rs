// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{fmt::Display, path::PathBuf};

mod args;
mod inputs;

#[derive(Clone, Debug)]
pub struct ActionId(String);

impl ActionId {
    /// Create a new `ActionId`
    ///
    /// # Errors
    ///
    /// Raise an invaliv configuration error if the action id contains anything
    /// but lowercase ASCII letters or '_'.
    pub fn new(input: &str) -> crate::Result<Self> {
        if input.chars().any(|c| !c.is_ascii_lowercase() && c != '_') {
            Err(crate::Error::new_invalid_configuration(format!(
                "{input} is not a valid action id"
            )))
        } else {
            Ok(ActionId(input.to_string()))
        }
    }
}

impl Display for ActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct ActionDefinition {
    pub id: ActionId,
    pub command: Vec<String>,
    pub expected_exit_code: i32,
    pub input_filters: inputs::InputFilters,
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
        action_id: ActionId,
    },
    Done {
        action_id: ActionId,
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

#[tracing::instrument(skip(inputs))]
async fn run_single_action(
    current_directory: PathBuf,
    sender: ActionUpdateSender,
    action: &'static ActionDefinition,
    inputs: inputs::InputQuery,
) -> crate::Result<()> {
    tracing::debug!("running action '{}': {:?}", action.id, action.command);

    sender
        .send(ActionUpdate::Started {
            action_id: action.id.clone(),
        })
        .await
        .expect("Failed to send start message to reporter");

    if std::env::var("SKIP")
        .unwrap_or_default()
        .split('\n')
        .any(|s| s == action.id.to_string())
    {
        tracing::trace!("Skipping '{}'", action.id);
        report(
            &sender,
            ActionUpdate::Done {
                action_id: action.id.clone(),
                result: ActionResult::Skipped,
            },
        )
        .await;
        return Ok(());
    }

    let Some(command) = action.command.first() else {
        tracing::error!("No command in action '{}'", action.id);
        let message = format!("No command defined in action '{}'", action.id);
        sender
            .send(ActionUpdate::Done {
                action_id: action.id.clone(),
                result: ActionResult::Error {
                    message: message.clone(),
                },
            })
            .await
            .expect("Failed to send message to reporter");
        return Err(crate::Error::new_invalid_configuration(message));
    };

    let args = args::parse_args(&action.command, inputs, &action.input_filters).await;
    let mut args = match args {
        Ok(Some(args)) => args,
        Ok(None) => {
            sender
                .send(ActionUpdate::Done {
                    action_id: action.id.clone(),
                    result: ActionResult::NotApplicable,
                })
                .await
                .expect("Failed to send message to reporter");
            return Ok(());
        }
        Err(e) => {
            sender
                .send(ActionUpdate::Done {
                    action_id: action.id.clone(),
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
            .output()
            .await
            .map_err(|e| crate::Error::new_io_error(&format!("Could not start '{command}'"), e))?;

        tracing::trace!(
            "result of running action '{}' ({} {}): {output:?}",
            action.id,
            command,
            args.print()
        );

        if output.status.code() != Some(action.expected_exit_code) {
            tracing::debug!("Unexpected return code for action '{}'", action.id);
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
        report(
            &sender,
            ActionUpdate::Done {
                action_id: action.id.clone(),
                result: ActionResult::Warn { stdout, stderr },
            },
        )
        .await;
    } else {
        tracing::trace!("Success running '{}'", action.id);
        report(
            &sender,
            ActionUpdate::Done {
                action_id: action.id.clone(),
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
    current_directory: PathBuf,
    sender: ActionUpdateSender,
    actions: &'static [ActionDefinition],
    files: Vec<PathBuf>,
) -> crate::Result<()> {
    tracing::trace!("Starting actions");
    let cache_handle = inputs::setup_input_cache(current_directory.clone(), files);
    let mut join_set = tokio::task::JoinSet::new();

    for a in actions {
        let cd = current_directory.clone();
        let tx = sender.clone();

        join_set.spawn(run_single_action(cd, tx, a, cache_handle.query()));
    }

    tracing::trace!("All actions started");

    drop(sender);

    tracing::trace!("Joining actions: {}", join_set.len());

    while let Some(r) = join_set.join_next().await {
        tracing::debug!("joined => in JS: {}", join_set.len());
        r.expect("Join Error found")?;
    }

    cache_handle.finish().await;

    tracing::trace!("Done running actions");
    Ok(())
}
