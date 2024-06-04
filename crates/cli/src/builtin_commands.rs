// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;

type Args = Vec<(String, String)>;
type Inputs = Vec<PathBuf>;

fn parse_arguments(arguments: &[OsString]) -> anyhow::Result<(Args, Inputs)> {
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

    let to_parse = if factor == 1 {
        input
    } else {
        &input[..(input.len() - 1)]
    };
    let base = to_parse.parse::<u64>().context("Failed to parse size")?;

    Ok(base * factor)
}

fn is_true(input: &str) -> bool {
    let input = input.to_lowercase();
    (&input == "true") || (&input == "1") || (&input == "on")
}

fn check_large_files(
    args: &[(String, String)],
    inputs: &[PathBuf],
    verbosity: u8,
) -> anyhow::Result<i32> {
    let mut size = 0;
    for (k, v) in args {
        match k.as_str() {
            "size" => {
                size = parse_size(v)?;
            }
            _ => {
                return Err(anyhow::anyhow!(format!("Unexpected argument {k}={v}")));
            }
        }
    }

    let mut large_files = 0;
    for p in inputs {
        let meta = p.metadata()?;
        let actual_size = meta.len();

        if actual_size > size {
            eprintln!("{p:?}: {} bytes too big", actual_size - size);
            large_files += 1;
        } else if verbosity > 0 {
            eprintln!("{p:?}: {actual_size} bytes, OK");
        }
    }
    Ok(large_files)
}

fn open_for_check(path: &Path) -> anyhow::Result<std::io::BufReader<std::fs::File>> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .append(false)
        .truncate(false)
        .open(path)
        .context("Failed to read file {p:?}")?;
    Ok(std::io::BufReader::new(file))
}

fn handle_bom(args: &[(String, String)], inputs: &[PathBuf], verbosity: u8) -> anyhow::Result<i32> {
    let fix = {
        let mut fix = false;
        for (k, v) in args {
            match k.as_str() {
                "fix" => fix = is_true(v),
                _ => {
                    return Err(anyhow::anyhow!(format!("Unexpected argument {k}={v}")));
                }
            }
        }
        fix
    };
    if verbosity > 1 {
        eprintln!("Fixing mode {}", if fix { "enabled" } else { "disabled" });
    }

    let mut unfixed_boms = 0;
    for p in inputs {
        let mut buf = open_for_check(p)?;
        let mut start_bytes = [0_u8; 3];
        let read_result = buf.read_exact(&mut start_bytes);
        match read_result {
            Ok(()) => {
                if start_bytes == [b'\xef', b'\xbb', b'\xbf'] {
                    if fix {
                        let mut contents = vec![];
                        if buf.read_to_end(&mut contents).is_ok() {
                            drop(buf);

                            let file = std::fs::OpenOptions::new()
                                .read(true)
                                .write(true)
                                .create(false)
                                .append(false)
                                .truncate(true)
                                .open(p)
                                .context("Failed to write file {p:?}")?;
                            let mut buf = std::io::BufWriter::new(file);
                            buf.write_all(&contents).context("Failed to write data")?;
                            eprintln!("{p:?}: byte order mark removed");
                            continue;
                        }
                    } else {
                        eprintln!("{p:?}: byte order mark found");
                    }
                    unfixed_boms += 1;
                } else if verbosity > 0 {
                    eprintln!("{p:?}: no byte order mark, OK");
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                if verbosity > 0 {
                    eprintln!("{p:?}: too short for a byte order mark");
                }
            }
            Err(e) => return Err(e).context("Failed to read byte oder mark"),
        }
    }
    Ok(unfixed_boms)
}

#[derive(Clone, Debug, Default)]
struct IsBinary {
    total_bytes: usize,
    odd_bytes: usize,
    early_decision: bool,
    expected_utf8_bytes: usize,
}

impl IsBinary {
    pub fn is_binary(&mut self, b: u8) -> bool {
        self.total_bytes += 1;

        if self.early_decision {
            return self.early_decision;
        }

        if b == b'\0' {
            self.early_decision = true;
            return true;
        }

        if self.expected_utf8_bytes > 0 {
            self.expected_utf8_bytes -= 1;
            if b & 0b1100_0000 == 0b1000_0000 {
                self.odd_bytes += 1;
            }
        } else {
            match b {
                b if b & 0b1111_0000 == 0b1110_0000 => {
                    self.expected_utf8_bytes = 3;
                }
                b if b & 0b1110_0000 == 0b1100_0000 => {
                    self.expected_utf8_bytes = 2;
                }
                b if b & 0b1110_0000 == 0b1100_0000 => {
                    self.expected_utf8_bytes = 1;
                }
                b if b >= 32 || [b'\n', b'\r', b'\t', 7, 12].contains(&b) => { /* do nothing */ }
                _ => {
                    self.odd_bytes += 1;
                }
            }
        }

        false
    }

    pub fn final_verdict(self) -> bool {
        self.early_decision || ((self.total_bytes / 10) * 3 > self.odd_bytes) // 30% odd bytes might happen in text;-)
    }
}

const LINE_ENDING_NAMES: [&str; 4] = ["cr", "crlf", "lf", "auto"];
const LINE_ENDING_STRINGS: [&str; 4] = ["\r", "\r\n", "\n", "auto"];
const LF: u8 = b'\n';
const CR: u8 = b'\r';

#[derive(Clone, Debug, Default)]
struct IsMixedLineEnding {
    end_counts: [usize; 3],
    last_byte: u8,
}

impl IsMixedLineEnding {
    pub fn count_line_endings(&mut self, byte: u8) {
        let last = self.last_byte;
        self.last_byte = byte;

        match (last, byte) {
            (b'\r', b'\n') => self.end_counts[1] += 1,
            (b'\r', _) => self.end_counts[0] += 1,
            (_, b'\n') => self.end_counts[2] += 1,
            (_, _) => { /* do nothing */ }
        }
    }

    pub fn final_verdict(mut self) -> (bool, usize) {
        self.count_line_endings(b'\0');
        eprintln!("Final counts: {:?}", self.end_counts);
        let is_mixed = self.end_counts.into_iter().filter(|c| *c > 0).count() > 1;
        let majority_index = self
            .end_counts
            .into_iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map_or(0, |(i, _)| i);
        eprintln!(
            "Final counts: {:?} => {}",
            self.end_counts, LINE_ENDING_NAMES[majority_index]
        );
        (is_mixed, majority_index)
    }
}

fn detect_mixed_line_endings(contents: &[u8]) -> (bool, bool, usize) {
    let mut binary_checker = IsBinary::default();
    let mut mixed_line_end_checker = IsMixedLineEnding::default();

    for b in contents {
        if binary_checker.is_binary(*b) {
            break;
        }
        mixed_line_end_checker.count_line_endings(*b);
    }

    if binary_checker.final_verdict() {
        (true, false, 0)
    } else {
        let (mixed, index) = mixed_line_end_checker.final_verdict();
        (false, mixed, index)
    }
}

fn fix_mixed_line_endings(contents: &[u8], fix_index: usize) -> Vec<u8> {
    assert!(fix_index < 3);

    let mut changed = Vec::with_capacity(contents.len());
    let mut last_was_cr = false;
    for b in contents {
        match *b {
            CR => {
                last_was_cr = true;
            }
            LF => {
                last_was_cr = false;
                changed.extend_from_slice(LINE_ENDING_STRINGS[fix_index].as_bytes());
            }
            b => {
                if last_was_cr {
                    changed.extend_from_slice(LINE_ENDING_STRINGS[fix_index].as_bytes());
                    last_was_cr = false;
                }
                changed.push(b)
            }
        }
    }

    changed
}

fn handle_mixed_line_endings(
    args: &[(String, String)],
    inputs: &[PathBuf],
    verbosity: u8,
) -> anyhow::Result<i32> {
    let (fix, expected_index) = {
        let mut fix = false;
        let mut expected_index = 0;

        for (k, v) in args {
            match k.as_str() {
                "fix" => {
                    if let Some(pos) = LINE_ENDING_NAMES.iter().position(|r| r == v) {
                        fix = true;
                        expected_index = pos;
                    } else {
                        return Err(anyhow::anyhow!(format!("Unknown fix mode {v}")));
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(format!("Unexpected argument {k}={v}")));
                }
            }
        }
        (fix, expected_index)
    };

    let mut mixed_line_endings = 0;
    for p in inputs {
        let mut buf = open_for_check(p)?;
        let mut contents = vec![];
        buf.read_to_end(&mut contents)
            .context("Failed to read data from file")?;
        drop(buf);

        let (is_binary, is_mixed, majority_index) = detect_mixed_line_endings(&contents);

        if is_binary {
            if verbosity > 0 {
                eprintln!("{p:?}: binary file, SKIPPING");
            }
            continue;
        }

        if !is_mixed {
            if verbosity > 0 {
                eprintln!("{p:?}: {} only, OK", LINE_ENDING_NAMES[majority_index]);
            }
            continue;
        }

        if fix {
            let fix_index = if expected_index == 3 {
                majority_index
            } else {
                expected_index
            };

            let new_contents = fix_mixed_line_endings(&contents, fix_index);

            let file = std::fs::OpenOptions::new()
                .read(false)
                .write(true)
                .create(false)
                .append(false)
                .truncate(true)
                .open(p)
                .context("Failed to write file {p:?}")?;
            let mut buf = std::io::BufWriter::new(file);
            buf.write_all(&new_contents).context("Failed to write data")?;
            eprintln!("{p:?}: FIXED to {}", LINE_ENDING_NAMES[fix_index]);
            continue;
        } else {
            mixed_line_endings += 1;
            eprintln!(
                "{p:?}: mixed with {} being the majority FAIL",
                LINE_ENDING_NAMES[majority_index]
            );
        }
    }

    Ok(mixed_line_endings)
}

pub fn run_builtin_command(
    action: &str,
    arguments: &[OsString],
    verbosity: u8,
) -> anyhow::Result<i32> {
    let (args, inputs) = parse_arguments(arguments)?;

    match action {
        "large-files" => check_large_files(&args, &inputs, verbosity),
        "bom" => handle_bom(&args, &inputs, verbosity),
        "mixed-line-endings" => handle_mixed_line_endings(&args, &inputs, verbosity),
        _ => Err(anyhow::anyhow!(format!(
            "{action} is not a builtin command"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_line_endings_empty_file() {
        let input = vec![];

        assert_eq!(detect_mixed_line_endings(&input), (false, false, 2));
    }

    #[test]
    fn test_detect_line_endings_binary_file() {
        let input = vec![0, 42, 10, 255, 128, 52];

        assert_eq!(detect_mixed_line_endings(&input), (true, false, 0));
    }

    #[test]
    fn test_detect_line_endings_lf_only() {
        let input = "a\nb\nc\n".as_bytes();
        assert_eq!(detect_mixed_line_endings(input), (false, false, 2));
    }

    #[test]
    fn test_detect_line_endings_crlf_only() {
        let input = "a\r\nb\r\nc\r\n".as_bytes();
        assert_eq!(detect_mixed_line_endings(input), (false, false, 1));
    }

    #[test]
    fn test_detect_line_endings_cr_only() {
        let input = "a\rb\rc\r".as_bytes();
        assert_eq!(detect_mixed_line_endings(input), (false, false, 0));
    }

    #[test]
    fn test_detect_line_endings_all_of_them() {
        let input = "a\rb\r\nc\n".as_bytes();
        assert_eq!(detect_mixed_line_endings(input), (false, true, 2));
    }

    #[test]
    fn test_fix_line_endings_cr() {
        let input = "a\rb\r\nc\n".as_bytes();
        assert_eq!(&fix_mixed_line_endings(input, 0), b"a\rb\rc\r");
    }

    #[test]
    fn test_fix_line_endings_crlf() {
        let input = "a\rb\r\nc\n".as_bytes();
        assert_eq!(&fix_mixed_line_endings(input, 1), b"a\r\nb\r\nc\r\n");
    }

    #[test]
    fn test_fix_line_endings_lf() {
        let input = "a\rb\r\nc\n".as_bytes();
        assert_eq!(&fix_mixed_line_endings(input, 2), b"a\nb\nc\n");
    }
}
