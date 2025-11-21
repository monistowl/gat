use std::fmt::Write;

use super::Pane;

#[derive(Clone, Debug)]
pub struct Sidebar {
    pub title: String,
    pub collapsed: bool,
    pub lines: Vec<String>,
}

impl Sidebar {
    pub fn new(title: impl Into<String>, collapsed: bool) -> Self {
        Self {
            title: title.into(),
            collapsed,
            lines: Vec::new(),
        }
    }

    pub fn lines(mut self, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.lines = lines.into_iter().map(|l| l.into()).collect();
        self
    }

    pub fn collapse(mut self) -> Self {
        self.collapsed = true;
        self
    }

    fn render_into(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent * 2);
        if self.collapsed {
            let _ = writeln!(output, "{}▍ {} (collapsed)", pad, self.title);
            return;
        }

        let _ = writeln!(output, "{}▍ {}", pad, self.title);
        for line in &self.lines {
            let _ = writeln!(output, "{}  {}", pad, line);
        }
    }
}

#[derive(Clone, Debug)]
pub struct SubTabs {
    pub labels: Vec<String>,
    pub active: usize,
    pub compact_active: Option<usize>,
}

impl SubTabs {
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>, active: usize) -> Self {
        Self {
            labels: labels.into_iter().map(|l| l.into()).collect(),
            active,
            compact_active: None,
        }
    }

    pub fn with_compact_active(mut self, active: usize) -> Self {
        self.compact_active = Some(active);
        self
    }

    pub fn render(&self) -> String {
        self.render_for_view(false)
    }

    pub fn render_for_view(&self, compact: bool) -> String {
        let active = if compact {
            self.compact_active.unwrap_or(self.active)
        } else {
            self.active
        };
        self.render_labels(active)
    }

    fn render_labels(&self, active: usize) -> String {
        let active = active.min(self.labels.len().saturating_sub(1));
        let rendered: Vec<String> = self
            .labels
            .iter()
            .enumerate()
            .map(|(idx, label)| {
                if idx == active {
                    format!("[{}]", label)
                } else {
                    label.to_string()
                }
            })
            .collect();
        rendered.join("  ")
    }
}

#[derive(Clone, Debug)]
pub struct ResponsiveRules {
    pub wide_threshold: u16,
    pub tall_threshold: u16,
    pub expand_visuals_on_wide: bool,
}

impl Default for ResponsiveRules {
    fn default() -> Self {
        Self {
            wide_threshold: 100,
            tall_threshold: 30,
            expand_visuals_on_wide: true,
        }
    }
}

impl ResponsiveRules {
    pub fn should_expand(&self, width: u16, height: u16) -> bool {
        width >= self.wide_threshold || height >= self.tall_threshold
    }
}

#[derive(Clone, Debug)]
pub struct PaneLayout {
    pub primary: Pane,
    pub secondary: Option<Pane>,
    pub sidebar: Option<Sidebar>,
    pub subtabs: Option<SubTabs>,
    pub responsive: ResponsiveRules,
}

impl PaneLayout {
    pub fn new(primary: Pane) -> Self {
        Self {
            primary,
            secondary: None,
            sidebar: None,
            subtabs: None,
            responsive: ResponsiveRules::default(),
        }
    }

    pub fn with_secondary(mut self, secondary: Pane) -> Self {
        self.secondary = Some(secondary);
        self
    }

    pub fn with_sidebar(mut self, sidebar: Sidebar) -> Self {
        self.sidebar = Some(sidebar);
        self
    }

    pub fn with_subtabs(mut self, subtabs: SubTabs) -> Self {
        self.subtabs = Some(subtabs);
        self
    }

    pub fn with_responsive_rules(mut self, rules: ResponsiveRules) -> Self {
        self.responsive = rules;
        self
    }

    pub fn render_into(&self, output: &mut String, width: u16, height: u16) {
        let expand_visuals =
            self.responsive.expand_visuals_on_wide && self.responsive.should_expand(width, height);

        if let Some(subtabs) = &self.subtabs {
            let compact = !expand_visuals;
            let _ = writeln!(output, "Subtabs: {}", subtabs.render_for_view(compact));
        }

        self.primary.render_into(output, 0, expand_visuals);

        if let Some(secondary) = &self.secondary {
            let _ = writeln!(output, "");
            let _ = writeln!(output, "⇄ Secondary pane (swapped when menu changes)");
            secondary.render_into(output, 1, expand_visuals);
        }

        if let Some(sidebar) = &self.sidebar {
            let _ = writeln!(output, "");
            sidebar.render_into(output, 1);
        }

        if expand_visuals {
            let _ = writeln!(
                output,
                "\nResponsive: visual widgets expanded to fill available space ({width}x{height})."
            );
        }
    }
}
