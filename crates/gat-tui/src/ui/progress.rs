/// Progress bars and spinners for ongoing operations
///
/// Provides visual feedback for batch jobs, uploads, and long-running tasks

use super::{EmptyState, THEME};

/// A single progress bar
#[derive(Clone, Debug)]
pub struct ProgressBar {
    pub label: String,
    pub value: f64,      // 0.0 to 1.0
    pub show_percent: bool,
    pub status: ProgressStatus,
}

/// Progress status affecting color/symbol
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressStatus {
    Active,     // In progress
    Complete,   // Finished successfully
    Failed,     // Finished with error
    Paused,     // Temporarily stopped
}

impl ProgressStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            ProgressStatus::Active => "▶",
            ProgressStatus::Complete => "✓",
            ProgressStatus::Failed => "✗",
            ProgressStatus::Paused => "‖",
        }
    }

    pub fn bar_char(&self) -> &'static str {
        match self {
            ProgressStatus::Active => "█",
            ProgressStatus::Complete => "▓",
            ProgressStatus::Failed => "░",
            ProgressStatus::Paused => "▒",
        }
    }
}

/// Progress bar collection view
#[derive(Clone, Debug)]
pub struct ProgressBarView {
    pub title: Option<String>,
    pub bars: Vec<ProgressBar>,
    pub bar_width: usize,
    pub show_labels: bool,
    pub show_eta: bool,
    pub empty: Option<EmptyState>,
}

impl ProgressBarView {
    pub fn new() -> Self {
        Self {
            title: None,
            bars: Vec::new(),
            bar_width: 40,
            show_labels: true,
            show_eta: false,
            empty: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn add_progress(
        mut self,
        label: impl Into<String>,
        value: f64,
        status: ProgressStatus,
    ) -> Self {
        self.bars.push(ProgressBar {
            label: label.into(),
            value: value.clamp(0.0, 1.0),
            show_percent: true,
            status,
        });
        self
    }

    pub fn bar_width(mut self, width: usize) -> Self {
        self.bar_width = width;
        self
    }

    pub fn hide_labels(mut self) -> Self {
        self.show_labels = false;
        self
    }

    pub fn with_eta(mut self) -> Self {
        self.show_eta = true;
        self
    }

    pub fn with_empty_state(mut self, empty: EmptyState) -> Self {
        self.empty = Some(empty);
        self
    }

    pub fn has_bars(&self) -> bool {
        !self.bars.is_empty()
    }

    /// Render progress bars
    #[cfg(feature = "fancy-ui")]
    pub fn render_lines(&self) -> Vec<String> {
        if self.bars.is_empty() {
            if let Some(empty) = &self.empty {
                return empty.render_lines(&THEME);
            }
            return vec!["(no jobs)".to_string()];
        }

        let mut lines = Vec::new();

        if let Some(title) = &self.title {
            lines.push(title.clone());
            lines.push("".to_string());
        }

        // Find longest label for alignment
        let max_label_len = if self.show_labels {
            self.bars.iter().map(|b| b.label.len()).max().unwrap_or(0)
        } else {
            0
        };

        for bar in &self.bars {
            let filled = (bar.value * self.bar_width as f64) as usize;
            let empty = self.bar_width.saturating_sub(filled);

            let bar_char = bar.status.bar_char();
            let bar_visual = format!("{}{}", bar_char.repeat(filled), " ".repeat(empty));

            let percent = if bar.show_percent {
                format!(" {:3.0}%", bar.value * 100.0)
            } else {
                String::new()
            };

            let line = if self.show_labels {
                let label_padding = " ".repeat(max_label_len.saturating_sub(bar.label.len()));
                format!(
                    "{} {}{} [{}]{}",
                    bar.status.symbol(),
                    bar.label,
                    label_padding,
                    bar_visual,
                    percent
                )
            } else {
                format!("{} [{}]{}", bar.status.symbol(), bar_visual, percent)
            };

            lines.push(line);
        }

        lines
    }

    /// Simple fallback for minimal builds
    #[cfg(not(feature = "fancy-ui"))]
    pub fn render_lines(&self) -> Vec<String> {
        if self.bars.is_empty() {
            if let Some(empty) = &self.empty {
                return empty.render_lines(&THEME);
            }
            return vec!["(no jobs)".to_string()];
        }

        let mut lines = Vec::new();

        if let Some(title) = &self.title {
            lines.push(title.clone());
        }

        for bar in &self.bars {
            lines.push(format!(
                "{} {}: {:.0}%",
                bar.status.symbol(),
                bar.label,
                bar.value * 100.0
            ));
        }

        lines
    }
}

impl Default for ProgressBarView {
    fn default() -> Self {
        Self::new()
    }
}

/// Spinner for indefinite operations
#[derive(Clone, Debug)]
pub struct SpinnerView {
    pub label: String,
    pub frame: usize,
    pub style: SpinnerStyle,
}

/// Spinner animation styles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpinnerStyle {
    Dots,       // ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏
    Line,       // -\|/
    Arrow,      // ←↖↑↗→↘↓↙
    Box,        // ◰◳◲◱
    Circle,     // ◐◓◑◒
    Custom,     // Custom frames
}

impl SpinnerStyle {
    pub fn frames(&self) -> &'static [&'static str] {
        match self {
            SpinnerStyle::Dots => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            SpinnerStyle::Line => &["-", "\\", "|", "/"],
            SpinnerStyle::Arrow => &["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
            SpinnerStyle::Box => &["◰", "◳", "◲", "◱"],
            SpinnerStyle::Circle => &["◐", "◓", "◑", "◒"],
            SpinnerStyle::Custom => &["⟳"],
        }
    }

    pub fn frame_char(&self, frame: usize) -> &'static str {
        let frames = self.frames();
        frames[frame % frames.len()]
    }
}

impl SpinnerView {
    pub fn new(label: impl Into<String>, style: SpinnerStyle) -> Self {
        Self {
            label: label.into(),
            frame: 0,
            style,
        }
    }

    pub fn tick(&mut self) {
        self.frame += 1;
    }

    pub fn render(&self) -> String {
        format!("{} {}", self.style.frame_char(self.frame), self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_empty() {
        let view = ProgressBarView::new();
        let lines = view.render_lines();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_progress_single_bar() {
        let view = ProgressBarView::new()
            .add_progress("Task 1", 0.5, ProgressStatus::Active);

        let lines = view.render_lines();
        assert!(lines.iter().any(|l| l.contains("Task 1")));
        assert!(lines.iter().any(|l| l.contains("50%")));
    }

    #[test]
    fn test_progress_multiple_bars() {
        let view = ProgressBarView::new()
            .add_progress("Job A", 0.25, ProgressStatus::Active)
            .add_progress("Job B", 0.75, ProgressStatus::Active)
            .add_progress("Job C", 1.0, ProgressStatus::Complete);

        let lines = view.render_lines();
        assert!(lines.iter().any(|l| l.contains("Job A")));
        assert!(lines.iter().any(|l| l.contains("Job B")));
        assert!(lines.iter().any(|l| l.contains("Job C")));
    }

    #[test]
    fn test_progress_status_symbols() {
        assert_eq!(ProgressStatus::Active.symbol(), "▶");
        assert_eq!(ProgressStatus::Complete.symbol(), "✓");
        assert_eq!(ProgressStatus::Failed.symbol(), "✗");
        assert_eq!(ProgressStatus::Paused.symbol(), "‖");
    }

    #[test]
    fn test_spinner_frames() {
        let mut spinner = SpinnerView::new("Loading", SpinnerStyle::Dots);
        let frame1 = spinner.render();
        spinner.tick();
        let frame2 = spinner.render();
        assert_ne!(frame1, frame2);
    }

    #[test]
    fn test_spinner_styles() {
        let dots = SpinnerStyle::Dots.frames();
        assert_eq!(dots.len(), 10);

        let line = SpinnerStyle::Line.frames();
        assert_eq!(line.len(), 4);

        let circle = SpinnerStyle::Circle.frames();
        assert_eq!(circle.len(), 4);
    }

    #[test]
    fn test_progress_clamping() {
        let view = ProgressBarView::new()
            .add_progress("Over", 1.5, ProgressStatus::Active)
            .add_progress("Under", -0.5, ProgressStatus::Active);

        let lines = view.render_lines();
        // Should clamp to 0-100%
        assert!(lines.iter().any(|l| l.contains("100%")));
        assert!(lines.iter().any(|l| l.contains("0%")));
    }

    #[test]
    fn test_progress_with_title() {
        let view = ProgressBarView::new()
            .with_title("Batch Jobs")
            .add_progress("Task", 0.5, ProgressStatus::Active);

        let lines = view.render_lines();
        assert!(lines.iter().any(|l| l.contains("Batch Jobs")));
    }
}
