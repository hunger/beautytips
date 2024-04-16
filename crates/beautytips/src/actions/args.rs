// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::actions::inputs;

#[derive(Clone, Debug)]
pub(crate) struct Arg {
    values: Vec<OsString>,
    current_pos: RefCell<usize>,
}

impl Arg {
    fn new(values: Vec<OsString>) -> Self {
        assert!(!values.is_empty());

        Self {
            values,
            current_pos: RefCell::new(0),
        }
    }

    fn current(&self) -> &OsStr {
        let cp = *self.current_pos.borrow();
        self.values
            .get(cp)
            .map(OsString::as_os_str)
            .expect("cp can not be invalid")
    }

    fn increment(&self) -> bool {
        let p = *self.current_pos.borrow() + 1;
        if p >= self.values.len() {
            *self.current_pos.borrow_mut() = 0;
            true
        } else {
            *self.current_pos.borrow_mut() = p;
            false
        }
    }
}

#[derive(Debug)]
pub(crate) struct Args(Vec<Arg>);

impl Args {
    pub(crate) fn increment(&mut self) -> bool {
        for a in &self.0 {
            if !a.increment() {
                return false;
            }
        }
        true
    }

    pub(crate) fn args_iter(&self) -> impl Iterator<Item = &OsStr> {
        self.0.iter().map(Arg::current)
    }

    pub(crate) fn print(&self) -> String {
        self.0
            .iter()
            .map(|a| shell_escape::escape(a.current().to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

enum ParseArgs {
    Outside,
    OneOpenBrace,
    Inside,
    OneClosingBrace,
}

fn split_arg(arg: &str) -> Vec<String> {
    let mut state = ParseArgs::Outside;
    let mut current = String::new();
    let mut result = Vec::new();

    for a in arg.chars() {
        state = match a {
            '{' => match state {
                ParseArgs::Outside => ParseArgs::OneOpenBrace,
                ParseArgs::OneOpenBrace => {
                    if !current.is_empty() {
                        result.push(current);
                        current = String::new();
                    }
                    current.push_str("{{");
                    ParseArgs::Inside
                }
                _ => state,
            },
            '}' => match state {
                ParseArgs::OneOpenBrace => {
                    current.push_str("{}");
                    ParseArgs::Outside
                }
                ParseArgs::Inside => {
                    current.push('}');
                    ParseArgs::OneClosingBrace
                }
                ParseArgs::OneClosingBrace => {
                    current.push('}');
                    result.push(current);
                    current = String::new();
                    ParseArgs::Outside
                }
                ParseArgs::Outside => {
                    current.push('}');
                    ParseArgs::Outside
                }
            },
            a => match state {
                ParseArgs::OneOpenBrace => {
                    current.push('{');
                    current.push(a);
                    ParseArgs::Outside
                }
                ParseArgs::OneClosingBrace => {
                    current.push(a);
                    ParseArgs::Inside
                }
                _ => {
                    current.push(a);
                    state
                }
            },
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

#[tracing::instrument(skip(inputs))]
async fn input_arg(
    arg: &str,
    inputs: inputs::InputQuery,
) -> crate::Result<Option<(Vec<PathBuf>, bool)>> {
    if arg.starts_with("{{") && arg.ends_with("}}") {
        let input_name = &arg[2..(arg.len() - 2)];
        tracing::debug!("{arg} is an input argument with name: {input_name}");
        let (input_name, is_array) = if input_name.ends_with("...") {
            (&input_name[0..(input_name.len() - 3)], true)
        } else {
            (input_name, false)
        };

        let paths = inputs.inputs(input_name.to_string()).await.map_err(|e| {
            crate::Error::new_input_generator(input_name.to_string(), e.to_string())
        })?;

        Ok(Some((paths, is_array)))
    } else {
        tracing::trace!("{arg} is no input argument");
        Ok(None)
    }
}

#[tracing::instrument(skip(inputs))]
pub(crate) async fn parse_arg(arg: &str, inputs: inputs::InputQuery) -> crate::Result<Vec<Arg>> {
    let argument_parts = split_arg(arg);

    let mut result = Vec::new();

    if argument_parts.len() == 1 {
        let arg = &argument_parts[0];
        if let Some((paths, is_array)) = input_arg(arg, inputs).await? {
            if is_array {
                result.extend(
                    paths
                        .iter()
                        .map(|p| Arg::new(vec![p.clone().into_os_string()])),
                );
            } else {
                result.push(Arg::new(
                    paths.iter().map(|p| p.clone().into_os_string()).collect(),
                ));
            }
        } else {
            result.push(Arg::new(vec![arg.into()]));
        }
    } else {
        let mut extended_arg = vec![String::new()];

        for p in &argument_parts {
            if let Some((paths, is_array)) = input_arg(p, inputs.clone()).await? {
                if is_array {
                    let total = paths
                        .iter()
                        .map(|p| shell_escape::escape(p.to_string_lossy()))
                        .collect::<Vec<_>>()
                        .join(" ");
                    for a in &mut extended_arg {
                        a.push_str(&total);
                    }
                } else {
                    let mut new_extended_arg = Vec::with_capacity(extended_arg.len() * paths.len());

                    for p in &paths {
                        let extension = shell_escape::escape(p.to_string_lossy());
                        for a in &extended_arg {
                            new_extended_arg.push(a.clone() + &extension);
                        }
                    }

                    extended_arg = new_extended_arg;
                }
            } else {
                for a in &mut extended_arg {
                    a.push_str(p);
                }
            }
        }

        result.push(Arg::new(extended_arg.iter().map(Into::into).collect()));
    }

    Ok(result)
}

#[tracing::instrument(skip(inputs))]
pub(crate) async fn parse_args(args: &[String], inputs: inputs::InputQuery) -> crate::Result<Args> {
    let mut parsed_args = Vec::with_capacity(args.len() - 1);

    for a in args.iter().skip(1) {
        parsed_args.extend_from_slice(&parse_arg(a, inputs.clone()).await?[..]);
    }

    Ok(Args(parsed_args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_args_ok() {
        assert_eq!(
            vec![
                "test".to_string(),
                "{{files}}".to_string(),
                "foobar".to_string()
            ],
            split_arg("test{{files}}foobar")
        );
    }

    #[test]
    fn test_split_args_ok_array() {
        assert_eq!(
            vec![
                "test".to_string(),
                "{{files...}}".to_string(),
                "foobar".to_string()
            ],
            split_arg("test{{files...}}foobar")
        );
    }
}
