use super::{AppShell, CommandModal, ContextButton, MenuItem, NavMenu, PaneLayout, Tooltip};

pub struct PaneContext {
    pub default_tooltip: Option<Tooltip>,
    pub command_modal: Option<CommandModal>,
}

impl PaneContext {
    pub fn new() -> Self {
        Self {
            default_tooltip: None,
            command_modal: None,
        }
    }

    pub fn with_tooltip(mut self, tooltip: Tooltip) -> Self {
        self.default_tooltip = Some(tooltip);
        self
    }

    pub fn with_modal(mut self, modal: CommandModal) -> Self {
        self.command_modal = Some(modal);
        self
    }
}

pub trait PaneView {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn hotkey(&self) -> char;
    fn layout(&self, context: &PaneContext) -> PaneLayout;

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        None
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        Vec::new()
    }
}

pub struct PanelRegistry {
    context: PaneContext,
    panes: Vec<Box<dyn PaneView>>,
}

impl PanelRegistry {
    pub fn new(context: PaneContext) -> Self {
        Self {
            context,
            panes: Vec::new(),
        }
    }

    pub fn register(mut self, pane: impl PaneView + 'static) -> Self {
        self.panes.push(Box::new(pane));
        self
    }

    pub fn build_menu(&self) -> NavMenu {
        let items: Vec<MenuItem> = self
            .panes
            .iter()
            .map(|pane| {
                let mut item = MenuItem::new(
                    pane.id(),
                    pane.label(),
                    pane.hotkey(),
                    pane.layout(&self.context),
                );

                let buttons = pane.context_buttons(&self.context);
                if !buttons.is_empty() {
                    item = item.with_context_buttons(buttons);
                }

                if let Some(tooltip) = pane.tooltip(&self.context) {
                    item = item.with_tooltip(tooltip);
                }

                item
            })
            .collect();

        NavMenu::new(items, 0)
    }

    pub fn into_shell(mut self, title: impl Into<String>) -> AppShell {
        let nav_menu = self.build_menu();
        let mut shell = AppShell::new(title, nav_menu);

        if let Some(tooltip) = self.context.default_tooltip.take() {
            shell = shell.with_tooltip(tooltip);
        }

        if let Some(modal) = self.context.command_modal.take() {
            shell = shell.with_modal(modal);
        }

        shell
    }
}
