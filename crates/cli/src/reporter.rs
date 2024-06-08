// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

use std::io::Write;

#[derive(Default)]
pub struct Reporter {
    running: Vec<String>,
    has_status: bool,
}

impl Reporter {
    fn print_status(&mut self) {
        self.clear_status();

        let (width, _) = termion::terminal_size().unwrap_or((80, 40));
        let mut running = self.running.join(", ");
        let max_running = usize::from(width) - 15;

        if running.len() > max_running {
            running.truncate(max_running);
            running.push_str("...");
        }
        print!("{}Running: {running}", termion::cursor::Save);
        std::io::stdout().flush().expect("Flushing failed");
        self.has_status = true;
    }

    fn clear_status(&mut self) {
        if self.has_status {
            print!(
                "{}{}",
                termion::cursor::Restore,
                termion::clear::AfterCursor
            );
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
        output = format!(
            "\n{}{output}",
            termion::color::Fg(termion::color::LightBlack)
        );
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
                println!(
                    "{}✅ {action_id} [OK]{output}{}",
                    termion::color::Fg(termion::color::Green),
                    termion::color::Fg(termion::color::Reset)
                );
            }
            beautytips::ActionResult::Skipped => {
                println!(
                    "{}🦥 {action_id} [SKIPPED]{}",
                    termion::color::Fg(termion::color::Blue),
                    termion::color::Fg(termion::color::Reset)
                );
            }
            beautytips::ActionResult::NotApplicable => {
                println!(
                    "{}🚙 {action_id} [NOT APPLICABLE]{}",
                    termion::color::Fg(termion::color::Blue),
                    termion::color::Fg(termion::color::Reset)
                );
            }
            beautytips::ActionResult::Warn { stdout, stderr } => {
                let output = stdout_and_err_to_str(&stdout, &stderr);
                println!(
                    "{}💡 {action_id} [WARN]{output}{}",
                    termion::color::Fg(termion::color::LightYellow),
                    termion::color::Fg(termion::color::Reset)
                );
            }
            beautytips::ActionResult::Error { message } => {
                println!(
                    "{}🚨 {action_id} [ERROR]: {message}{}",
                    termion::color::Fg(termion::color::Red),
                    termion::color::Fg(termion::color::Reset),
                );
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
