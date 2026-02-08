// ============================================================================
// ROSE PINE MOON - Color Palette
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to ANSI truecolor escape sequence (foreground)
    pub fn fg(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    /// Convert to ANSI truecolor escape sequence (background)
    pub fn bg(&self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }
}

// Reset codes
pub const RESET: &str = "\x1b[0m";
#[allow(dead_code)]
pub const RESET_FG: &str = "\x1b[39m";
#[allow(dead_code)]
pub const RESET_BG: &str = "\x1b[49m";
#[allow(dead_code)]
pub const CLEAR_SCREEN: &str = "\x1b[2J";
#[allow(dead_code)]
pub const CLEAR_LINE: &str = "\x1b[2K";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";
#[allow(dead_code)]
pub const CURSOR_HOME: &str = "\x1b[H";

// Rose Pine Moon Palette
pub struct RosePineMoon;

impl RosePineMoon {
    // Backgrounds - flowing from dark to light
    pub const BASE: Color = Color::new(35, 33, 54);        // #232136 - Deepest background
    #[allow(dead_code)]
    pub const SURFACE: Color = Color::new(42, 39, 63);     // #2a273f - Slightly lifted
    #[allow(dead_code)]
    pub const OVERLAY: Color = Color::new(57, 53, 82);     // #393552 - Interactive elements
    #[allow(dead_code)]
    pub const HIGHLIGHT_LOW: Color = Color::new(42, 40, 62);   // #2a283e
    pub const HIGHLIGHT_MED: Color = Color::new(68, 65, 90);   // #44415a
    #[allow(dead_code)]
    pub const HIGHLIGHT_HIGH: Color = Color::new(86, 82, 110); // #56526e

    // Foregrounds - flowing from muted to bright
    pub const MUTED: Color = Color::new(110, 106, 134);    // #6e6a86 - Comments, hints
    pub const SUBTLE: Color = Color::new(144, 140, 170);   // #908caa - Secondary text
    pub const TEXT: Color = Color::new(224, 222, 244);     // #e0def4 - Primary text

    // Accents - each with a distinct purpose
    pub const LOVE: Color = Color::new(235, 111, 146);     // #eb6f92 - Errors, quit
    pub const GOLD: Color = Color::new(246, 193, 119);     // #f6c177 - Files mode, warnings
    #[allow(dead_code)]
    pub const ROSE: Color = Color::new(234, 154, 151);     // #ea9a97 - Soft highlights
    pub const PINE: Color = Color::new(62, 143, 176);      // #3e8fb0 - Insert mode, TUI
    #[allow(dead_code)]
    pub const FOAM: Color = Color::new(156, 207, 216);     // #9ccfd8 - Apps mode, info
    #[allow(dead_code)]
    pub const IRIS: Color = Color::new(196, 167, 231);     // #c4a7e7 - Normal mode, hints
}

// Style builder for easy styling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
}

impl Style {
    pub fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
        }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    #[allow(dead_code)]
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    #[allow(dead_code)]
    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    #[allow(dead_code)]
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    #[allow(dead_code)]
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn apply(&self, text: &str) -> String {
        let mut result = String::new();

        if self.bold {
            result.push_str("\x1b[1m");
        }
        if self.dim {
            result.push_str("\x1b[2m");
        }
        if self.italic {
            result.push_str("\x1b[3m");
        }
        if self.underline {
            result.push_str("\x1b[4m");
        }
        if let Some(fg) = self.fg {
            result.push_str(&fg.fg());
        }
        if let Some(bg) = self.bg {
            result.push_str(&bg.bg());
        }

        result.push_str(text);
        result.push_str(RESET);

        result
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

// Convenience functions for common styles
pub fn styled(text: &str, fg: Color) -> String {
    Style::new().fg(fg).apply(text)
}

#[allow(dead_code)]
pub fn styled_bg(text: &str, fg: Color, bg: Color) -> String {
    Style::new().fg(fg).bg(bg).apply(text)
}
