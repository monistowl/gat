/// Tabbed interface component

#[derive(Clone, Debug)]
pub struct Tab {
    pub title: String,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct TabsWidget {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub id: String,
}

impl TabsWidget {
    pub fn new(id: impl Into<String>) -> Self {
        TabsWidget {
            tabs: Vec::new(),
            active_tab: 0,
            id: id.into(),
        }
    }

    pub fn add_tab(&mut self, title: impl Into<String>, tab_id: impl Into<String>) {
        self.tabs.push(Tab {
            title: title.into(),
            id: tab_id.into(),
        });
    }

    pub fn with_tabs(mut self, tabs: Vec<(String, String)>) -> Self {
        self.tabs = tabs
            .into_iter()
            .map(|(title, id)| Tab {
                title,
                id,
            })
            .collect();
        self
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
    }

    pub fn next_tab(&mut self) {
        if self.active_tab < self.tabs.len().saturating_sub(1) {
            self.active_tab += 1;
        }
    }

    pub fn prev_tab(&mut self) {
        if self.active_tab > 0 {
            self.active_tab -= 1;
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_tab_index(&self) -> usize {
        self.active_tab
    }
}
