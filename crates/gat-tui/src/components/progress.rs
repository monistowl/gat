/// Progress and indicator components

#[derive(Clone, Debug)]
pub struct ProgressWidget {
    pub current: u64,
    pub total: u64,
    pub label: String,
    pub id: String,
}

impl ProgressWidget {
    pub fn new(id: impl Into<String>, total: u64) -> Self {
        ProgressWidget {
            current: 0,
            total,
            label: String::new(),
            id: id.into(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn increment(&mut self) {
        if self.current < self.total {
            self.current += 1;
        }
    }

    pub fn set_progress(&mut self, current: u64) {
        self.current = current.min(self.total);
    }

    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f64 / self.total as f64) * 100.0
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }

    pub fn reset(&mut self) {
        self.current = 0;
    }
}

/// Status indicator (success, error, warning, info)
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
pub struct StatusWidget {
    pub level: StatusLevel,
    pub message: String,
    pub id: String,
}

impl StatusWidget {
    pub fn new(id: impl Into<String>) -> Self {
        StatusWidget {
            level: StatusLevel::Info,
            message: String::new(),
            id: id.into(),
        }
    }

    pub fn set_success(mut self, message: impl Into<String>) -> Self {
        self.level = StatusLevel::Success;
        self.message = message.into();
        self
    }

    pub fn set_error(mut self, message: impl Into<String>) -> Self {
        self.level = StatusLevel::Error;
        self.message = message.into();
        self
    }

    pub fn set_warning(mut self, message: impl Into<String>) -> Self {
        self.level = StatusLevel::Warning;
        self.message = message.into();
        self
    }

    pub fn set_info(mut self, message: impl Into<String>) -> Self {
        self.level = StatusLevel::Info;
        self.message = message.into();
        self
    }

    pub fn symbol(&self) -> &'static str {
        match self.level {
            StatusLevel::Success => "✓",
            StatusLevel::Error => "✗",
            StatusLevel::Warning => "⚠",
            StatusLevel::Info => "ℹ",
        }
    }
}
