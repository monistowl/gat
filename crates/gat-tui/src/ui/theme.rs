#[derive(Clone, Debug)]
pub struct Theme {
    pub accent: &'static str,
    pub muted: &'static str,
    pub heavy_border: &'static str,
    pub light_border: &'static str,
    pub padding: usize,
    pub table_gap: &'static str,
    pub empty_icon: &'static str,
}

impl Theme {
    pub const fn new() -> Self {
        Self {
            accent: "▍",
            muted: "·",
            heavy_border: "━",
            light_border: "─",
            padding: 2,
            table_gap: " │ ",
            empty_icon: "◌",
        }
    }

    /// Get a theme based on terminal capabilities
    /// Uses UTF-8 if locale supports it, falls back to ASCII
    pub fn auto() -> Self {
        if Self::supports_utf8() {
            Self::new()
        } else {
            Self::ascii()
        }
    }

    /// ASCII-only theme for terminals without UTF-8 support
    pub const fn ascii() -> Self {
        Self {
            accent: "|",
            muted: ".",
            heavy_border: "=",
            light_border: "-",
            padding: 2,
            table_gap: " | ",
            empty_icon: "o",
        }
    }

    fn supports_utf8() -> bool {
        // Check LANG environment variable
        if let Ok(lang) = std::env::var("LANG") {
            if lang.contains("UTF") || lang.contains("utf") {
                return true;
            }
        }
        // Check LC_ALL
        if let Ok(lc) = std::env::var("LC_ALL") {
            if lc.contains("UTF") || lc.contains("utf") {
                return true;
            }
        }
        // Check LC_CTYPE
        if let Ok(lc) = std::env::var("LC_CTYPE") {
            if lc.contains("UTF") || lc.contains("utf") {
                return true;
            }
        }
        // Default: assume UTF-8 is available
        true
    }

    pub fn indent(&self, depth: usize) -> String {
        " ".repeat(depth * self.padding)
    }

    pub fn frame_title(&self, title: &str) -> String {
        let rail = self.heavy_border.repeat(4);
        format!("┏{} {} {}┓", rail, title, rail)
    }

    pub fn divider(&self, length: usize) -> String {
        self.light_border.repeat(length)
    }
}

#[derive(Clone, Debug)]
pub struct EmptyState {
    pub label: String,
    pub guidance: Vec<String>,
}

impl EmptyState {
    pub fn new(
        label: impl Into<String>,
        guidance: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            label: label.into(),
            guidance: guidance.into_iter().map(|g| g.into()).collect(),
        }
    }

    pub fn render_lines(&self, theme: &Theme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("{} {}", theme.empty_icon, self.label));
        for tip in &self.guidance {
            lines.push(format!("{} {}", theme.muted, tip));
        }
        lines
    }
}

use once_cell::sync::Lazy;

pub static THEME: Lazy<Theme> = Lazy::new(Theme::auto);
