// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::io::{self, Write};

use crossterm::{cursor, style, terminal};

#[derive(Default)]
pub struct Reporter {
    running: Vec<String>,
    has_status: bool,
}

impl Reporter {
    fn print_status(&mut self) {
        self.clear_status();

        let (width, _) = terminal::size().unwrap_or((80, 40));
        let mut running = self.running.join(", ");
        let max_running = usize::from(width) - 15;

        if running.len() > max_running {
            running.truncate(max_running);
            running.push_str("...");
        }

        crossterm::queue!(
            io::stdout(),
            cursor::SavePosition,
            style::Print(format!("Running {running}")),
        )
        .expect("print failed");

        io::stdout().flush().expect("Flushing failed");
        self.has_status = true;
    }

    fn clear_status(&mut self) {
        if self.has_status {
            crossterm::queue!(
                io::stdout(),
                cursor::RestorePosition,
                terminal::Clear(terminal::ClearType::FromCursorDown),
            )
            .expect("print failed");
        }
        self.has_status = false;
    }
}

fn to_str(input: &[u8]) -> String {
    let input = if input.ends_with(b"\n") {
        &input[..input.len() - 1]
    } else {
        input
    };

    let s = String::from_utf8_lossy(input).trim().to_string();
    if s.is_empty() {
        return s;
    }

    let indent = "    ";
    let s = s
        .split('\n')
        .collect::<Vec<_>>()
        .join(&format!("\n{indent}"));
    format!("{indent}{s}")
}

fn stdout_and_err_to_str(stdout: &[u8], stderr: &[u8]) -> String {
    let mut output = to_str(stdout);
    if output.is_empty() {
        output = to_str(stderr);
    } else {
        output = format!("{output}\n{}", to_str(stderr));
    }
    if !output.is_empty() {
        output = format!("\n{output}",);
    }

    output
}

impl beautytips::Reporter for Reporter {
    fn report_start(&mut self, action_id: String) {
        self.running.push(action_id);
        self.print_status();
    }

    fn report_done(&mut self, action_id: String, result: beautytips::ActionResult) {
        self.clear_status();

        self.running = self
            .running
            .iter()
            .filter(|id| *id != &action_id)
            .cloned()
            .collect();

        match result {
            beautytips::ActionResult::Ok { stdout, stderr } => {
                let output = stdout_and_err_to_str(&stdout, &stderr);
                crossterm::queue!(
                    io::stdout(),
                    style::SetForegroundColor(style::Color::Green),
                    style::Print(format!("✅ {action_id} [OK]")),
                    style::SetForegroundColor(style::Color::DarkGrey),
                    style::Print(output),
                    style::Print('\n'),
                    style::ResetColor
                )
                .expect("print failed");
            }
            beautytips::ActionResult::Skipped => {
                crossterm::queue!(
                    io::stdout(),
                    style::SetForegroundColor(style::Color::Blue),
                    style::Print(format!("🦥 {action_id} [SKIPPED]\n")),
                    style::ResetColor,
                )
                .expect("print failed");
            }
            beautytips::ActionResult::NotApplicable => {
                crossterm::queue!(
                    io::stdout(),
                    style::SetForegroundColor(style::Color::Blue),
                    style::Print(format!("🚙 {action_id} [NOT APPLICABLE]\n")),
                    style::ResetColor,
                )
                .expect("print failed");
            }
            beautytips::ActionResult::Warn { stdout, stderr } => {
                let output = stdout_and_err_to_str(&stdout, &stderr);
                crossterm::queue!(
                    io::stdout(),
                    style::SetForegroundColor(style::Color::Yellow),
                    style::Print(format!("💡 {action_id} [WARN]")),
                    style::SetForegroundColor(style::Color::DarkGrey),
                    style::Print(output),
                    style::Print('\n'),
                    style::ResetColor,
                )
                .expect("print failed");
            }
            beautytips::ActionResult::Error { message } => {
                crossterm::queue!(
                    io::stdout(),
                    style::SetForegroundColor(style::Color::Red),
                    style::Print(format!("🚨 {action_id} [ERROR]: {message}\n")),
                    style::ResetColor,
                )
                .expect("print failed");
            }
        };

        if !self.running.is_empty() {
            self.print_status();
        }
    }

    fn finish(&mut self) {
        self.clear_status();
    }
}
