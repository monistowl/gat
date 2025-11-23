use crate::models::Theme;

/// Color definitions
pub struct Colors {
    pub primary: (u8, u8, u8),    // Cyan
    pub success: (u8, u8, u8),    // Green
    pub warning: (u8, u8, u8),    // Yellow
    pub error: (u8, u8, u8),      // Red
    pub neutral: (u8, u8, u8),    // Gray
    pub text: (u8, u8, u8),       // Text color
    pub background: (u8, u8, u8), // Background color
}

impl Colors {
    pub fn light() -> Self {
        Colors {
            primary: (0, 150, 200),      // Cyan
            success: (100, 200, 100),    // Green
            warning: (200, 200, 0),      // Yellow
            error: (200, 100, 100),      // Red
            neutral: (150, 150, 150),    // Gray
            text: (50, 50, 50),          // Dark text
            background: (255, 255, 255), // White background
        }
    }

    pub fn dark() -> Self {
        Colors {
            primary: (100, 200, 255), // Cyan
            success: (100, 255, 100), // Green
            warning: (255, 255, 100), // Yellow
            error: (255, 100, 100),   // Red
            neutral: (150, 150, 150), // Gray
            text: (220, 220, 220),    // Light text
            background: (30, 30, 30), // Dark background
        }
    }
}

pub fn get_colors(theme: Theme) -> Colors {
    match theme {
        Theme::Light => Colors::light(),
        Theme::Dark => Colors::dark(),
    }
}

/// Typography styles
pub enum TextStyle {
    Title,
    Body,
    Muted,
    Mono,
}

impl TextStyle {
    pub fn bold(&self) -> bool {
        matches!(self, TextStyle::Title)
    }

    pub fn dim(&self) -> bool {
        matches!(self, TextStyle::Muted)
    }
}
