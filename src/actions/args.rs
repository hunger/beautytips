// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

// cSpell: ignore concatcp dotdotdot starstar

use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

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
            .map(|a| shell_words::quote(&a.current().to_string_lossy()).to_string())
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
    root_directory: &Path,
    input_filters: &inputs::InputFilters,
) -> crate::SendableResult<Option<(Vec<PathBuf>, bool)>> {
    static EMPTY: Vec<glob::Pattern> = vec![];

    if arg.starts_with("{{") && arg.ends_with("}}") {
        let input_name = &arg[2..(arg.len() - 2)];
        let (input_name, is_array) = if input_name.ends_with("...") {
            (&input_name[0..(input_name.len() - 3)], true)
        } else {
            (input_name, false)
        };

        let current_filters = input_filters.get(input_name).unwrap_or(&EMPTY);
        let match_options = {
            let mut opt = glob::MatchOptions::new();
            opt.require_literal_separator = true;
            opt
        };

        let paths = inputs
            .inputs(input_name.to_string())
            .await
            .map_err(|e| format!("Failed to get inputs for {input_name:?}: {e}"))?
            .into_iter()
            .filter(|p| {
                let Ok(rel_path) = p.strip_prefix(root_directory) else {
                    return false;
                };
                current_filters.is_empty()
                    || current_filters
                        .iter()
                        .any(|f| f.matches_path_with(rel_path, match_options.clone()))
            })
            .collect();

        Ok(Some((paths, is_array)))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip(inputs))]
pub(crate) async fn parse_arg(
    arg: &str,
    inputs: inputs::InputQuery,
    root_directory: &Path,
    input_filters: &inputs::InputFilters,
) -> crate::SendableResult<Option<Vec<Arg>>> {
    let argument_parts = split_arg(arg);

    let mut result = Vec::new();

    if argument_parts.len() == 1 {
        let arg = &argument_parts[0];
        if let Some((paths, is_array)) =
            input_arg(arg, inputs, root_directory, input_filters).await?
        {
            if paths.is_empty() {
                return Ok(None);
            }

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
            if let Some((paths, is_array)) =
                input_arg(p, inputs.clone(), root_directory, input_filters).await?
            {
                if paths.is_empty() {
                    return Ok(None);
                }

                if is_array {
                    let total = paths
                        .iter()
                        .map(|p| shell_words::quote(&p.to_string_lossy()).to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    for a in &mut extended_arg {
                        a.push_str(&total);
                    }
                } else {
                    let mut new_extended_arg = Vec::with_capacity(extended_arg.len() * paths.len());

                    for p in &paths {
                        let extension = shell_words::quote(&p.to_string_lossy()).to_string();
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

    Ok(Some(result))
}

#[tracing::instrument(skip(inputs))]
pub(crate) async fn parse_args(
    args: &[String],
    inputs: inputs::InputQuery,
    root_directory: &Path,
    input_filters: &inputs::InputFilters,
) -> crate::SendableResult<Option<Args>> {
    let mut parsed_args = Vec::with_capacity(args.len() - 1);

    for a in args.iter().skip(1) {
        let filtered_args = parse_arg(a, inputs.clone(), root_directory, input_filters).await?;
        if let Some(filtered_args) = filtered_args {
            parsed_args.extend_from_slice(&filtered_args);
        } else {
            return Ok(None);
        }
    }

    Ok(Some(Args(parsed_args)))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

    const ROOT_DIR: &str = if cfg!(windows) {
        "C:\\51bb3d94"
    } else {
        "/tmp/51bb3d94"
    };

    const PATH_0: &str = &const_format::concatcp!(ROOT_DIR, std::path::MAIN_SEPARATOR, "README.md");
    const PATH_1: &str = &const_format::concatcp!(ROOT_DIR, std::path::MAIN_SEPARATOR, "main.rs");
    const PATH_2: &str =
        &const_format::concatcp!(ROOT_DIR, std::path::MAIN_SEPARATOR, "Cargo.toml");
    const PATH_3: &str = &const_format::concatcp!(
        ROOT_DIR,
        std::path::MAIN_SEPARATOR,
        "docs",
        std::path::MAIN_SEPARATOR,
        "doc.md"
    );
    const PATH_4: &str = &const_format::concatcp!(
        ROOT_DIR,
        std::path::MAIN_SEPARATOR,
        "docs",
        std::path::MAIN_SEPARATOR,
        "sections",
        std::path::MAIN_SEPARATOR,
        "one.md"
    );

    async fn test_input_arg(
        arg: &str,
        filters: &[&str],
    ) -> crate::SendableResult<Option<(Vec<PathBuf>, bool)>> {
        let input_cache = inputs::setup_input_cache(
            PathBuf::from(ROOT_DIR),
            vec![
                PathBuf::from(PATH_0),
                PathBuf::from(PATH_1),
                PathBuf::from(PATH_2),
                PathBuf::from(PATH_3),
                PathBuf::from(PATH_4),
            ],
        );

        let filter = HashMap::from([(
            "files".to_string(),
            filters
                .iter()
                .map(|f| glob::Pattern::new(f).unwrap())
                .collect(),
        )]);

        let root_directory = PathBuf::from(ROOT_DIR);

        let result = input_arg(arg, input_cache.query(), &root_directory, &filter).await;
        result
    }

    #[tokio::test]
    async fn test_input_arg_none() {
        let result = test_input_arg("foo", &[]).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_input_arg_files() {
        let (paths, is_array) = test_input_arg("{{files}}", &[]).await.unwrap().unwrap();

        assert_eq!(is_array, false);
        assert_eq!(paths.len(), 5);
        assert_eq!(paths[0].to_string_lossy(), PATH_0);
        assert_eq!(paths[1].to_string_lossy(), PATH_1);
        assert_eq!(paths[2].to_string_lossy(), PATH_2);
        assert_eq!(paths[3].to_string_lossy(), PATH_3);
        assert_eq!(paths[4].to_string_lossy(), PATH_4);
    }

    #[tokio::test]
    async fn test_input_arg_files_dotdotdot() {
        let (paths, is_array) = test_input_arg("{{files...}}", &[]).await.unwrap().unwrap();

        assert_eq!(is_array, true);
        assert_eq!(paths.len(), 5);
        assert_eq!(paths[0].to_string_lossy(), PATH_0);
        assert_eq!(paths[1].to_string_lossy(), PATH_1);
        assert_eq!(paths[2].to_string_lossy(), PATH_2);
        assert_eq!(paths[3].to_string_lossy(), PATH_3);
        assert_eq!(paths[4].to_string_lossy(), PATH_4);
    }

    #[tokio::test]
    async fn test_input_arg_files_filter_starstar_star_md() {
        let (paths, is_array) = test_input_arg("{{files...}}", &["**/*.md"])
            .await
            .unwrap()
            .unwrap();

        assert_eq!(is_array, true);
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].to_string_lossy(), PATH_0);
        assert_eq!(paths[1].to_string_lossy(), PATH_3);
        assert_eq!(paths[2].to_string_lossy(), PATH_4);
    }

    #[tokio::test]
    async fn test_input_arg_files_filter_star_md() {
        let (paths, is_array) = test_input_arg("{{files...}}", &["*.md"])
            .await
            .unwrap()
            .unwrap();

        assert_eq!(is_array, true);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].to_string_lossy(), PATH_0);
    }

    #[tokio::test]
    async fn test_input_arg_files_filter_docs_md() {
        let (paths, is_array) = test_input_arg("{{files...}}", &["docs/*.md"])
            .await
            .unwrap()
            .unwrap();

        assert_eq!(is_array, true);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].to_string_lossy(), PATH_3);
    }

    #[tokio::test]
    async fn test_input_arg_files_filter_docs_starstar_md() {
        let (paths, is_array) = test_input_arg("{{files...}}", &["docs/**/*.md"])
            .await
            .unwrap()
            .unwrap();

        assert_eq!(is_array, true);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].to_string_lossy(), PATH_3);
        assert_eq!(paths[1].to_string_lossy(), PATH_4);
    }

    #[tokio::test]
    async fn test_input_arg_files_filter_github_actions() {
        let (paths, is_array) =
            test_input_arg("{{files...}}", &[".github/**/*.yaml", ".github/**/*.yml"])
                .await
                .unwrap()
                .unwrap();

        assert_eq!(is_array, true);
        assert_eq!(paths.len(), 0);
    }
}
