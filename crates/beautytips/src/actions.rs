// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{fmt::Display, path::PathBuf};

mod inputs;

#[derive(Clone, Debug)]
pub struct ActionId (String);

impl ActionId {
    pub fn new(input: &str) -> crate::Result<Self> {
        if input.chars().any(|c| !(('a'..='z').contains(&c)) && c != '_') {
            Err(crate::Error::new_invalid_configuration(format!("{input} is not a valid action id")))
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
    tracing::debug!("Sending message to reporter");
    sender
        .send(message)
        .await
        .expect("Failed to send message to reporter");
    tracing::trace!("Message sent");
}

// #[tracing::instrument]
async fn run_file_action(
    current_directory: PathBuf,
    sender: ActionUpdateSender,
    action: ActionDefinition,
    // input_sender: inputs::InputQueryTx,
) -> crate::Result<()> {
    tracing::debug!("run file action {}", action.id);

    sender
        .send(ActionUpdate::Started {
            action_id: action.id.clone(),
        })
        .await
        .expect("Failed to send start message to reporter");

    let Some(command) = action.command.first() else {
        tracing::error!("No command in {}", action.id);
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

    let output = tokio::process::Command::new(command)
        .current_dir(current_directory)
        .args(action.command.iter().skip(1))
        .output()
        .await
        .map_err(|e| crate::Error::new_process_failed(command, e))?;

    tracing::trace!("result of running {}: {output:?}", action.id);

    if output.status.code() != Some(action.expected_exit_code) {
        tracing::debug!("Unexpected return code for {}", action.id);
        let err = crate::Error::new_unexpected_exit_code(command, 0, output.status.code());
        report(
            &sender,
            ActionUpdate::Done {
                action_id: action.id.clone(),
                result: ActionResult::Error {
                    message: err.to_string(),
                },
            },
        )
        .await;
        return Err(err);
    }

    tracing::trace!("Success running {}", action.id);
    report(
        &sender,
        ActionUpdate::Done {
            action_id: action.id.clone(),
            result: ActionResult::Ok {
                stdout: output.stdout,
                stderr: output.stderr,
            },
        },
    )
    .await;
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
    actions: Vec<ActionDefinition>,
    files: Vec<PathBuf>,
) -> crate::Result<()> {
    tracing::trace!("Starting actions");
    let cache_handle = inputs::setup_input_cache(files);
    let mut join_set = tokio::task::JoinSet::new();

    for a in &actions {
            let cd = current_directory.clone();
            let tx = sender.clone();
            let a = a.clone();
            let id = a.id.clone();

            join_set.spawn(run_file_action(cd, tx, a /* cache_handle.sender() */));
            tracing::debug!("spawned {id} (=> in JS: {})", join_set.len());
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
