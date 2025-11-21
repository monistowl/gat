use std::fmt::Write;

use anyhow::Result;

use crate::command_runner::{spawn_command, CommandHandle};

use super::{EmptyState, TableView, Tooltip, THEME};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionMode {
    DryRun,
    Full,
}

impl ExecutionMode {
    fn as_label(&self) -> &'static str {
        match self {
            ExecutionMode::DryRun => "Dry-run",
            ExecutionMode::Full => "Full run",
        }
    }
}

pub struct CommandModal {
    pub title: String,
    pub prompt: String,
    pub run_hotkey: char,
    pub execution_mode: ExecutionMode,
    pub command_text: Vec<String>,
    pub help: Tooltip,
    output: Vec<String>,
    handle: Option<CommandHandle>,
    max_output_rows: usize,
}

impl CommandModal {
    pub fn new(title: impl Into<String>, prompt: impl Into<String>, run_hotkey: char) -> Self {
        Self {
            title: title.into(),
            prompt: prompt.into(),
            run_hotkey,
            execution_mode: ExecutionMode::DryRun,
            command_text: Vec::new(),
            help: Tooltip::new(
                "Enter gat-cli commands, one flag per line to keep things readable.",
            ),
            output: Vec::new(),
            handle: None,
            max_output_rows: 6,
        }
    }

    pub fn with_help(mut self, help: Tooltip) -> Self {
        self.help = help;
        self
    }

    pub fn with_command_text(mut self, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.command_text = lines.into_iter().map(|l| l.into()).collect();
        self
    }

    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    pub fn submit(&mut self) -> Result<()> {
        let invocation = self.build_invocation();
        self.output.clear();
        self.output.push(format!(
            "{} [{}]",
            self.execution_mode.as_label(),
            invocation.join(" ")
        ));

        match spawn_command(invocation) {
            Ok(handle) => {
                self.handle = Some(handle);
                self.capture_output();
            }
            Err(err) => {
                self.output.push(format!("error: {err}"));
            }
        }

        Ok(())
    }

    pub fn capture_output(&mut self) {
        if let Some(handle) = &self.handle {
            for line in handle.poll() {
                self.output.push(line);
            }
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        let _ = writeln!(&mut output, "[{}]", self.title);
        let _ = writeln!(&mut output, "{} {}", THEME.accent, self.prompt);

        let _ = writeln!(&mut output, "\n{} Command text (multiline):", THEME.muted);
        for line in &self.command_text {
            let _ = writeln!(&mut output, "│ {}", line);
        }

        let mode_line = format!(
            "{} {}    {} {}",
            self.render_radio(ExecutionMode::DryRun),
            ExecutionMode::DryRun.as_label(),
            self.render_radio(ExecutionMode::Full),
            ExecutionMode::Full.as_label(),
        );
        let _ = writeln!(&mut output, "Mode: {}", mode_line);
        let _ = writeln!(
            &mut output,
            "[{}] Run (hotkey: {})",
            self.run_hotkey.to_ascii_uppercase(),
            self.run_hotkey
        );

        let _ = writeln!(&mut output, "{}", self.help.render());
        let _ = writeln!(&mut output, "Examples:");
        let _ = writeln!(
            &mut output,
            "  gat-cli datasets list --format table --limit 5"
        );
        let _ = writeln!(
            &mut output,
            "  gat-cli derms envelope --grid-file case33bw.arrow --out envelope.parquet"
        );

        let _ = writeln!(&mut output, "\n{} Output (scroll to review):", THEME.muted);
        for line in self.render_output_table() {
            let _ = writeln!(&mut output, "{}", line);
        }

        output
    }

    fn build_invocation(&self) -> Vec<String> {
        let raw = self
            .command_text
            .iter()
            .filter(|line| !line.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");

        if raw.is_empty() {
            return vec!["echo".to_string(), "No command provided".to_string()];
        }

        let tokens: Vec<String> = raw.split_whitespace().map(|t| t.to_string()).collect();
        if self.execution_mode == ExecutionMode::DryRun {
            let mut dry_tokens = vec!["echo".to_string(), "DRY-RUN:".to_string()];
            dry_tokens.extend(tokens);
            dry_tokens
        } else {
            tokens
        }
    }

    fn render_radio(&self, mode: ExecutionMode) -> &'static str {
        if self.execution_mode == mode {
            "●"
        } else {
            "○"
        }
    }

    fn render_output_table(&self) -> Vec<String> {
        let mut table = TableView::new(["#", "Stream"]).with_empty_state(EmptyState::new(
            "No output yet",
            [
                "Submit a dry-run to preview the invocation.",
                "Full runs will stream logs into this table.",
            ],
        ));
        let take = self.output.len().saturating_sub(self.max_output_rows);
        let start = if take > 0 { take } else { 0 };

        for (idx, line) in self.output.iter().enumerate().skip(start) {
            table = table.add_row([format!("{}", idx + 1), line.clone()]);
        }

        table.render_lines()
    }
}
