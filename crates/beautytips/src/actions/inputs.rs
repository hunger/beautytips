// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{collections::HashMap, path::PathBuf};

pub(crate) struct InputQueryMessage {
    input: String,
    tx: InputQueryReplyTx,
}

#[derive(Clone)]
pub(crate) struct InputQuery(InputQueryTx);

impl InputQuery {
    #[tracing::instrument(skip(self))]
    pub(crate) async fn inputs(&self, input: String) -> InputQueryReplyMessage {
        tracing::trace!("Querying values for input \"{input}\"");
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.0
            .send(InputQueryMessage {
                input,
                tx: reply_tx,
            })
            .await
            .expect("Internal communication should not fail");

        reply_rx
            .await
            .expect("Internal communication should not fail")
    }
}

pub(crate) struct InputCacheHandle {
    tx: InputQueryTx,
    handle: tokio::task::JoinHandle<Result<(), String>>,
}

impl InputCacheHandle {
    #[tracing::instrument(skip(self))]
    pub(crate) async fn finish(self) {
        tracing::trace!("Waiting for InputCache to finish");
        drop(self.tx);
        
        self.handle
            .await
            .expect("Failed to join task")
            .expect("Task failed");
        tracing::trace!("InputCache finished");
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn query(&self) -> InputQuery {
        InputQuery(self.tx.clone())
    }
}

#[derive(Clone, Debug)]
struct GeneratorReply {
    input: String,
    data: InputQueryReplyMessage,
}

type InputQueryTx = tokio::sync::mpsc::Sender<InputQueryMessage>;
type InputQueryRx = tokio::sync::mpsc::Receiver<InputQueryMessage>;
type InputQueryReplyMessage = Result<Vec<PathBuf>, String>;
type InputQueryReplyTx = tokio::sync::oneshot::Sender<InputQueryReplyMessage>;
// type InputQueryReplyRx = tokio::sync::oneshot::Receiver<InputQueryReplyType>;

type InputGeneratorReplyTx = tokio::sync::mpsc::Sender<GeneratorReply>;
type InputGeneratorReplyRx = tokio::sync::mpsc::Receiver<GeneratorReply>;

enum InputMapEntry {
    Cached(InputQueryReplyMessage),
    Generating(Vec<InputQueryReplyTx>),
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
    fn handle_input_query(&mut self, query: Option<InputQueryMessage>) -> Result<bool, String> {
        let Some(query) = query else {
            return Ok(false);
        };

        let sender = query.tx;
        match self.inputs.get_mut(&query.input) {
            Some(InputMapEntry::Cached(data)) => {
                sender
                    .send(data.clone())
                    .expect("Failed to send internal message");
            }
            Some(InputMapEntry::Generating(data)) => data.push(sender),
            None => {
                let generator_tx = self.generator_channel.0.clone();
                let query_name = query.input.clone();
                let qn = query_name.clone();

                match query_name.as_str() {
                    "files" => unreachable!(),
                    "cargo_targets" => {
                        tokio::spawn(async move {
                            generator_tx
                                .send(GeneratorReply {
                                    input: qn,
                                    data: Err("Not implemented yet!".to_string()),
                                })
                                .await
                                .expect("Failed to send internal message");
                        });

                        self.inputs
                            .insert(query_name, InputMapEntry::Generating(vec![sender]));
                    }
                    _ => {
                        sender
                            .send(Err(format!("Input '{query_name}' is not supported")))
                            .expect("Failed to send internal message");
                    }
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

        tracing::trace!("Handle generator reply for {}", reply.input);

        let Some(InputMapEntry::Generating(to_notify)) = self
            .inputs
            .insert(reply.input, InputMapEntry::Cached(reply.data.clone()))
        else {
            unreachable!("Unexpected content in cache hashmap");
        };
        for tx in to_notify {
            tx.send(reply.data.clone())
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
