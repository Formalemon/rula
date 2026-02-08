// ============================================================================
// Terminal Control - Low-level terminal operations
// ============================================================================

use std::io::{self, Write};
use crossterm::{
    cursor::{MoveTo, Show, Hide},
    terminal::{Clear, ClearType, size},
    QueueableCommand,
};
use crate::theme::*;

pub struct Terminal {
    stdout: io::Stdout,
    width: u16,
    height: u16,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        let (width, height) = size()?;
        let mut term = Self {
            stdout: io::stdout(),
            width,
            height,
        };
        term.setup()?;
        Ok(term)
    }

    fn setup(&mut self) -> io::Result<()> {
        // Hide cursor and clear screen
        self.stdout.queue(Hide)?;
        self.clear()?;
        self.flush()
    }

    pub fn clear(&mut self) -> io::Result<()> {
        // Fill entire screen with base color
        self.stdout.queue(Clear(ClearType::All))?;
        self.fill_background()?;
        Ok(())
    }

    fn fill_background(&mut self) -> io::Result<()> {
        // Fill the screen with base background color
        let bg = RosePineMoon::BASE.bg();
        let reset = RESET;

        for y in 0..self.height {
            self.stdout.queue(MoveTo(0, y))?;
            write!(self.stdout, "{}{}", bg, " ".repeat(self.width as usize))?;
        }
        write!(self.stdout, "{}", reset)?;
        Ok(())
    }

    pub fn move_to(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.stdout.queue(MoveTo(x, y)).map(|_| ())
    }

    pub fn write(&mut self, text: &str) -> io::Result<()> {
        write!(self.stdout, "{}", text)
    }

    pub fn write_at(&mut self, x: u16, y: u16, text: &str) -> io::Result<()> {
        self.move_to(x, y)?;
        self.write(text)
    }

    pub fn write_styled(&mut self, x: u16, y: u16, text: &str, style: &Style) -> io::Result<()> {
        self.move_to(x, y)?;
        self.write(&style.apply(text))
    }

    /// Draw a horizontal line with a specific character and color
    pub fn hline(&mut self, x: u16, y: u16, width: u16, ch: char, color: Color) -> io::Result<()> {
        self.move_to(x, y)?;
        let line: String = std::iter::repeat(ch).take(width as usize).collect();
        self.write(&styled(&line, color))
    }

    /// Draw a horizontal line with background color (subtle separator)
    #[allow(dead_code)]
    pub fn hline_bg(&mut self, x: u16, y: u16, width: u16, bg: Color) -> io::Result<()> {
        self.move_to(x, y)?;
        let spaces = " ".repeat(width as usize);
        self.write(&styled_bg(&spaces, RosePineMoon::MUTED, bg))
    }

    /// Clear a line and fill with background color
    #[allow(dead_code)]
    pub fn clear_line_bg(&mut self, y: u16, bg: Color) -> io::Result<()> {
        self.move_to(0, y)?;
        let spaces = " ".repeat(self.width as usize);
        self.write(&styled_bg(&spaces, RosePineMoon::TEXT, bg))
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        self.stdout.queue(Show)?;
        self.stdout.queue(Clear(ClearType::All))?;
        self.flush()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
