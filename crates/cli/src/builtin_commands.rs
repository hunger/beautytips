// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{ffi::OsString, path::PathBuf};

use anyhow::Context;

type Args = Vec<(String, String)>;
type Inputs = Vec<PathBuf>;

fn parse_arguments(
    arguments: &[OsString],
) -> anyhow::Result<(Args, Inputs)> {
    let mut parse_args = true;
    let mut key: Option<String> = None;
    let mut args = vec![];
    let mut inputs = vec![];

    let separator = OsString::from(&"--");

    for a in arguments {
        if a == &separator {
            if let Some(key) = key {
                return Err(anyhow::anyhow!(format!("Incomplete argument \"{key}\"")));
            }
            parse_args = false;
            continue;
        }
        if parse_args {
            let a = a
                .clone()
                .into_string()
                .map_err(|_| anyhow::anyhow!("Failed to convert an argument"))?;

            if let Some(k) = &key {
                args.push((k.clone(), a));
            } else {
                if !a.starts_with("--") {
                    return Err(anyhow::anyhow!(format!(
                        "Argument {a} does not start with \"--\""
                    )));
                }
                if let Some(equal_sign) = a.find('=') {
                    let k = &a[2..equal_sign];
                    let v = &a[(equal_sign + 1)..];
                    args.push((k.to_string(), v.to_string()));
                } else {
                    key = Some(a[2..].to_string());
                }
            }
        } else {
            inputs.push(PathBuf::from(a));
        }
    }
    Ok((args, inputs))
}

fn parse_size(input: &str) -> anyhow::Result<u64> {
    let last_char = input.as_bytes()[input.len() - 1];
    let factor = match last_char {
        b'k' | b'K' => 1024,
        b'm' | b'M' => 1024 * 1024,
        b'g' | b'G' => 1024 * 1024 * 1024,
        b't' | b'T' => 1024 * 1024 * 1024 * 1024,
        _ => 1,
    };

    let to_parse = if factor == 1 { input } else { &input[..(input.len() - 1)] };
    let base = to_parse.parse::<u64>().context("Failed to parse size")?;

    Ok(base * factor)
}

fn check_large_files(args: &[(String, String)], inputs: &[PathBuf]) -> anyhow::Result<i32> {
    let mut size = 0;
    for (k, v) in args {
        match k.as_str() {
            "size" => { size = parse_size(v)?; },
            _ => { return Err(anyhow::anyhow!(format!("Unexpected argument {k}={v}"))); },
        }
    }

    let mut large_files = 0;
    for p in inputs {
        let meta = p.metadata()?;
        if meta.len() > size {
            eprintln!("{p:?}: {} bytes too big", meta.len() - size);
            large_files += 1;
        }
    }
    Ok(large_files)
}

pub fn run_builtin_command(action: &str, arguments: &[OsString]) -> anyhow::Result<i32> {
    let (args, inputs) = parse_arguments(arguments)?;

    match action {
        "check-large-files" => check_large_files(&args, &inputs),
        _ => Err(anyhow::anyhow!(format!(
            "{action} is not a builtin command"
        ))),
    }
}
