use std::fmt::Write;

use super::layout::PaneLayout;

#[derive(Clone, Debug)]
pub struct ContextButton {
    pub hotkey: char,
    pub label: String,
}

impl ContextButton {
    pub fn new(hotkey: char, label: impl Into<String>) -> Self {
        Self {
            hotkey,
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub id: &'static str,
    pub label: String,
    pub hotkey: char,
    pub layout: PaneLayout,
    pub context_buttons: Vec<ContextButton>,
}

impl MenuItem {
    pub fn new(
        id: &'static str,
        label: impl Into<String>,
        hotkey: char,
        layout: PaneLayout,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            hotkey,
            layout,
            context_buttons: Vec::new(),
        }
    }

    pub fn with_context_buttons(
        mut self,
        buttons: impl IntoIterator<Item = ContextButton>,
    ) -> Self {
        self.context_buttons = buttons.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug)]
pub struct NavMenu {
    pub items: Vec<MenuItem>,
    active: usize,
}

impl NavMenu {
    pub fn new(items: Vec<MenuItem>, active: usize) -> Self {
        let active = if items.is_empty() {
            0
        } else {
            active.min(items.len() - 1)
        };
        Self { items, active }
    }

    pub fn render_menu_bar(&self) -> String {
        let mut output = String::new();
        let parts: Vec<String> = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let active_marker = if idx == self.active { '*' } else { ' ' };
                format!("[{active_marker}{}] {}", item.hotkey, item.label)
            })
            .collect();
        let _ = write!(output, "Menu {}", parts.join("  "));

        if let Some(active_item) = self.items.get(self.active) {
            if !active_item.context_buttons.is_empty() {
                let context: Vec<String> = active_item
                    .context_buttons
                    .iter()
                    .map(|btn| format!("({}) {}", btn.hotkey, btn.label))
                    .collect();
                let _ = write!(output, " | Actions: {}", context.join(", "));
            }
        }

        output
    }

    pub fn render_active_layout_into(&self, output: &mut String, width: u16, height: u16) {
        if let Some(item) = self.items.get(self.active) {
            let _ = writeln!(output, "");
            let _ = writeln!(output, "Active: {}", item.label);
            item.layout.render_into(output, width, height);
        }
    }

    pub fn select_by_hotkey(&mut self, hotkey: char) {
        if let Some((idx, _)) = self
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| item.hotkey == hotkey)
        {
            self.active = idx;
        }
    }

    pub fn active_item(&self) -> Option<&MenuItem> {
        self.items.get(self.active)
    }
}
