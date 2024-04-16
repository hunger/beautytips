// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use crate::actions::inputs;

enum ParseArgs {
    Outside,
    OneOpenBrace,
    Inside,
    OneClosingBrace,
}

pub(crate) fn parse_arg(arg: &str,
    inputs: inputs::InputQuery,
) -> Vec<String> {
    let mut state = ParseArgs::Outside;
    let mut current = String::new();
    let mut result = Vec::new();

    for a in arg.chars() {
        state = match a {
            '{' => {
                match state {
                    ParseArgs::Outside => ParseArgs::OneOpenBrace,
                    ParseArgs::OneOpenBrace => {
                        if !current.is_empty() {
                            result.push(current);
                        }
                        ParseArgs::Inside
                    },
                    _ => state,
                }
            },
            '}' => {
                match state {
                    ParseArgs::OneOpenBrace => {
                        current += "{}";
                        ParseArgs::Outside
                    }
                    ParseArgs::Inside => {
                        current += "}";
                        ParseArgs::OneClosingBrace
                    }
                    ParseArgs::OneClosingBrace => {
                        current += "}";
                        result.push(current);
                        current = String::new();
                        ParseArgs::Outside
                    }
                    ParseArgs::Ouside => {
                        current += "}";
                    }
                }
            },
            _ => {
                match state {
                    ParseArgs::OneOpenBrace => {
                        current += '{';
                        current += a;
                        ParseArgs::Outside
                    },
                    ParseArgs::OneClosingBrace => {
                        current += a;
                        ParseArgs::Inside
                    },
                    _ => {
                        current += a;
                        state
                    },
                }
            }
        }
    }

    result
}
