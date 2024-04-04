// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{
    collections::HashMap,
    path::PathBuf,
};

pub(crate) struct InputQuery {
    input: String,
    sender: InputQueryReplyTx,
}

#[derive(Clone, Debug)]
pub(crate) struct Input {
    pub(crate) inputs: Vec<PathBuf>,
    pub(crate) must_loop: bool,
}

pub(crate) struct InputCacheHandle {
    tx: InputQueryTx,
    handle: tokio::task::JoinHandle<Result<(), String>>,
}

impl InputCacheHandle {
    #[tracing::instrument(skip(self))]
    pub(crate) async fn finish(self) {
        tracing::trace!("Waiting for InputCache to finish");
        self.handle.await.expect("Failed to join task").expect("Task failed");
        tracing::trace!("InputCache finished");
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn sender(&self) -> InputQueryTx {
        self.tx.clone()
    }

    #[tracing::instrument(skip(self))]
    pub(crate) async fn get_inputs(&self, input: String) -> Result<Input, String> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.tx.send(InputQuery { input, sender: reply_tx }).await.expect("Internal communication should not fail");
        
        reply_rx.await.expect("Internal communication should not fail")
    }
}

#[derive(Clone, Debug)]
struct GeneratorReply {
    input: String,
    data: Result<Vec<PathBuf>, String>,
}

pub(crate) type InputQueryTx = tokio::sync::mpsc::Sender<InputQuery>;
type InputQueryRx = tokio::sync::mpsc::Receiver<InputQuery>;
type InputQueryReplyTx = tokio::sync::oneshot::Sender<Result<Input, String>>;
type InputQueryReplyRx = tokio::sync::oneshot::Receiver<Result<Input, String>>;

type InputGeneratorReplyTx = tokio::sync::mpsc::Sender<GeneratorReply>;
type InputGeneratorReplyRx = tokio::sync::mpsc::Receiver<GeneratorReply>;

enum InputMapEntry {
    Cached(Result<Vec<PathBuf>, String>),
    Generating(Vec<(InputQueryReplyTx, bool)>),
}

struct InputCache {
    inputs: HashMap<String, InputMapEntry>,
    rx: InputQueryRx,
    generator_channel: (InputGeneratorReplyTx, InputGeneratorReplyRx),
}

impl InputCache {
    pub(crate) fn new(files: Vec<PathBuf>, rx: InputQueryRx) -> Self {
        let inputs = {
            let mut i = HashMap::new();
            i.insert("files".to_string(), InputMapEntry::Cached(Ok(files)));
            i
        };

        Self {
            inputs,
            rx,
            generator_channel: tokio::sync::mpsc::channel(10),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn handle_request(&mut self) -> Result<bool, String> {
        tokio::select! {
            query = self.rx.recv() => {
                self.handle_input_query(query)
            },
            reply = self.generator_channel.1.recv() => {
                self.handle_generator_reply(reply)
            }
        } 
    }

    #[tracing::instrument(skip(self, query))]
    fn handle_input_query(&mut self, query: Option<InputQuery>) -> Result<bool, String> {
        let Some(query) = query else {
            return Ok(false);
        };

        let (query_name, must_loop) = match query.input.as_str() {
            "files" => Ok(("files", false)),
            "file" => Ok(("files", true)),
            "cargo_targets" => Ok(("cargo_targets", false)),
            "cargo_target" => Ok(("cargo_targets", true)),
            _ => Err(format!("{} not found in possible inputs", query.input)),
        }?;

        let sender = query.sender;
        match self.inputs.get_mut(&query.input) {
            Some(InputMapEntry::Cached(data)) => {
                sender
                    .send(
                        data.as_ref()
                            .map(|d| Input {
                                inputs: d.clone(),
                                must_loop,
                            })
                            .map_err(Clone::clone),
                    )
                    .expect("Failed to send internal message");
            }
            Some(InputMapEntry::Generating(data)) => data.push((sender, must_loop)),
            None => {
                let generator_tx = self.generator_channel.0.clone();

                match query_name {
                    "files" => unreachable!(
                    "files are always known, no need to fill this information in after the fact"
                ),
                    "cargo_targets" => {
                        tokio::spawn(async move {
                            generator_tx
                                .send(GeneratorReply {
                                    input: query_name.to_string(),
                                    data: Err("Not implemented yet!".to_string()),
                                })
                                .await
                                .expect("Failed to send internal message");
                        });

                        self.inputs.insert(
                            query_name.to_string(),
                            InputMapEntry::Generating(vec![(sender, must_loop)]),
                        );
                    }
                    _ => unreachable!("Unknown input provided by code"),
                };
            }
        };

        Ok(true)
    }

    #[tracing::instrument(skip(self))]
    fn handle_generator_reply(&mut self, reply: Option<GeneratorReply>) -> Result<bool, String> {
        let Some(reply) = reply else {
            return Ok(true);
        };

        let Some(InputMapEntry::Generating(to_notify)) = self
            .inputs
            .insert(reply.input, InputMapEntry::Cached(reply.data.clone()))
        else {
            unreachable!("Unexpected content in cache hashmap");
        };
        for (tx, must_loop) in to_notify {
            let reply = reply.data.clone();
            let to_send = reply.map(|inputs| Input { inputs, must_loop });
            tx.send(to_send)
                .expect("Internal communication should not fail");
        }

        Ok(true)
    }
}

#[tracing::instrument]
pub(crate) fn setup_input_cache(files: Vec<PathBuf>) -> InputCacheHandle {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let mut cache = InputCache::new(files, rx);

    let handle = tokio::spawn(async move {
        let _span = tracing::span!(tracing::Level::TRACE, "input_collector");
        loop {
            let result = cache.handle_request().await?;
            tracing::trace!("Handle request result: {result}");
            if !result {
                break;
            }
        }
        Ok(())
    });

    InputCacheHandle { tx, handle }
}
