/// ASCII bar chart visualization
///
/// Renders horizontal bar charts using Unicode block characters
/// for visual representation of metrics and analytics results.

use super::{EmptyState, THEME};

/// A single bar in the chart
#[derive(Clone, Debug)]
pub struct BarData {
    pub label: String,
    pub value: f64,
    pub color_hint: ColorHint,
}

/// Color hint for bar styling
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorHint {
    Good,    // Green tones
    Warning, // Yellow tones
    Critical, // Red tones
    Neutral,  // Default
}

impl ColorHint {
    pub fn symbol(&self) -> &'static str {
        match self {
            ColorHint::Good => "▓",
            ColorHint::Warning => "▒",
            ColorHint::Critical => "░",
            ColorHint::Neutral => "█",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ColorHint::Good => "Good",
            ColorHint::Warning => "Warning",
            ColorHint::Critical => "Critical",
            ColorHint::Neutral => "Normal",
        }
    }
}

/// Horizontal bar chart renderer
#[derive(Clone, Debug)]
pub struct BarChartView {
    pub title: Option<String>,
    pub bars: Vec<BarData>,
    pub max_value: Option<f64>,
    pub bar_width: usize,
    pub show_values: bool,
    pub show_legend: bool,
    pub value_suffix: String,
    pub empty: Option<EmptyState>,
}

impl BarChartView {
    pub fn new() -> Self {
        Self {
            title: None,
            bars: Vec::new(),
            max_value: None,
            bar_width: 40,
            show_values: true,
            show_legend: false,
            value_suffix: String::new(),
            empty: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn add_bar(
        mut self,
        label: impl Into<String>,
        value: f64,
        color_hint: ColorHint,
    ) -> Self {
        self.bars.push(BarData {
            label: label.into(),
            value,
            color_hint,
        });
        self
    }

    pub fn max_value(mut self, max: f64) -> Self {
        self.max_value = Some(max);
        self
    }

    pub fn bar_width(mut self, width: usize) -> Self {
        self.bar_width = width;
        self
    }

    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    pub fn with_legend(mut self) -> Self {
        self.show_legend = true;
        self
    }

    pub fn value_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.value_suffix = suffix.into();
        self
    }

    pub fn with_empty_state(mut self, empty: EmptyState) -> Self {
        self.empty = Some(empty);
        self
    }

    pub fn has_bars(&self) -> bool {
        !self.bars.is_empty()
    }

    /// Render the bar chart as ASCII art lines
    #[cfg(feature = "fancy-ui")]
    pub fn render_lines(&self) -> Vec<String> {
        if self.bars.is_empty() {
            if let Some(empty) = &self.empty {
                return empty.render_lines(&THEME);
            }
            return vec!["(empty chart)".to_string()];
        }

        let mut lines = Vec::new();

        // Add title if present
        if let Some(title) = &self.title {
            lines.push(title.clone());
            lines.push("".to_string());
        }

        // Calculate max value for scaling
        let max_val = self.max_value.unwrap_or_else(|| {
            self.bars
                .iter()
                .map(|b| b.value)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(1.0)
        });

        // Find longest label for alignment
        let max_label_len = self
            .bars
            .iter()
            .map(|b| b.label.len())
            .max()
            .unwrap_or(0);

        // Render each bar
        for bar in &self.bars {
            let normalized = if max_val > 0.0 {
                (bar.value / max_val).min(1.0)
            } else {
                0.0
            };

            let filled_width = (normalized * self.bar_width as f64) as usize;
            let symbol = bar.color_hint.symbol();

            // Build the bar
            let bar_visual = symbol.repeat(filled_width);
            let padding = " ".repeat(self.bar_width.saturating_sub(filled_width));

            // Format the line
            let label_padding = " ".repeat(max_label_len.saturating_sub(bar.label.len()));
            let line = if self.show_values {
                format!(
                    "{}{} │{}{}│ {:.1}{}",
                    bar.label, label_padding, bar_visual, padding, bar.value, self.value_suffix
                )
            } else {
                format!("{}{} │{}{}│", bar.label, label_padding, bar_visual, padding)
            };

            lines.push(line);
        }

        // Add scale indicator
        lines.push("".to_string());
        let scale_line = format!(
            "{}0{}{}",
            " ".repeat(max_label_len + 3),
            " ".repeat(self.bar_width.saturating_sub(2)),
            if max_val < 1000.0 {
                format!("{:.1}{}", max_val, self.value_suffix)
            } else {
                format!("{:.0}{}", max_val, self.value_suffix)
            }
        );
        lines.push(scale_line);

        // Add legend if requested
        if self.show_legend {
            lines.push("".to_string());
            lines.push("Legend:".to_string());

            let mut hints: Vec<_> = self.bars.iter()
                .map(|b| b.color_hint)
                .collect();
            hints.sort_by_key(|h| *h as u8);
            hints.dedup();

            for hint in hints {
                lines.push(format!("  {} = {}", hint.symbol(), hint.label()));
            }
        }

        lines
    }

    /// Render simple fallback for minimal builds
    #[cfg(not(feature = "fancy-ui"))]
    pub fn render_lines(&self) -> Vec<String> {
        if self.bars.is_empty() {
            if let Some(empty) = &self.empty {
                return empty.render_lines(&THEME);
            }
            return vec!["(empty chart)".to_string()];
        }

        let mut lines = Vec::new();

        if let Some(title) = &self.title {
            lines.push(title.clone());
        }

        lines.push("Values:".to_string());
        for bar in &self.bars {
            lines.push(format!(
                "  {}: {:.1}{}",
                bar.label, bar.value, self.value_suffix
            ));
        }

        lines
    }
}

impl Default for BarChartView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barchart_empty() {
        let chart = BarChartView::new();
        let lines = chart.render_lines();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_barchart_single_bar() {
        let chart = BarChartView::new()
            .add_bar("Test", 50.0, ColorHint::Good);

        let lines = chart.render_lines();
        assert!(lines.iter().any(|l| l.contains("Test")));
    }

    #[test]
    fn test_barchart_multiple_bars() {
        let chart = BarChartView::new()
            .add_bar("Metric A", 75.0, ColorHint::Good)
            .add_bar("Metric B", 50.0, ColorHint::Warning)
            .add_bar("Metric C", 25.0, ColorHint::Critical);

        let lines = chart.render_lines();

        // Should have all three metrics
        assert!(lines.iter().any(|l| l.contains("Metric A")));
        assert!(lines.iter().any(|l| l.contains("Metric B")));
        assert!(lines.iter().any(|l| l.contains("Metric C")));
    }

    #[test]
    fn test_barchart_with_title() {
        let chart = BarChartView::new()
            .with_title("Test Chart")
            .add_bar("Data", 100.0, ColorHint::Neutral);

        let lines = chart.render_lines();
        assert!(lines.iter().any(|l| l.contains("Test Chart")));
    }

    #[test]
    fn test_barchart_with_suffix() {
        let chart = BarChartView::new()
            .add_bar("Power", 500.0, ColorHint::Good)
            .value_suffix(" MW");

        let lines = chart.render_lines();
        assert!(lines.iter().any(|l| l.contains("MW")));
    }

    #[test]
    fn test_barchart_with_legend() {
        let chart = BarChartView::new()
            .add_bar("A", 10.0, ColorHint::Good)
            .add_bar("B", 20.0, ColorHint::Warning)
            .with_legend();

        let lines = chart.render_lines();
        assert!(lines.iter().any(|l| l.contains("Legend")));
    }

    #[test]
    fn test_barchart_custom_max() {
        let chart = BarChartView::new()
            .add_bar("Test", 50.0, ColorHint::Neutral)
            .max_value(100.0);

        let lines = chart.render_lines();
        // Should show scale up to 100
        assert!(lines.iter().any(|l| l.contains("100")));
    }

    #[test]
    fn test_barchart_hide_values() {
        let chart = BarChartView::new()
            .add_bar("Test", 50.0, ColorHint::Neutral)
            .show_values(false);

        let lines = chart.render_lines();
        // Visual bar should still be present
        assert!(!lines.is_empty());
    }
}
