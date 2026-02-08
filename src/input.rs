// ============================================================================
// Input Handler - Keyboard event processing
// ============================================================================

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use crate::app::{App, InputMode};

pub struct InputHandler;

impl InputHandler {
    pub fn new() -> Self {
        Self
    }

    /// Poll for input with optional timeout
    pub fn poll(&self, timeout_ms: u64) -> Option<KeyEvent> {
        if event::poll(Duration::from_millis(timeout_ms)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                return Some(key);
            }
        }
        None
    }

    /// Process a key event and update app state
    pub fn process(&self, app: &mut App, key: KeyEvent) {
        match app.input_mode {
            InputMode::Insert => self.process_insert_mode(app, key),
            InputMode::Normal => self.process_normal_mode(app, key),
        }
    }

    fn process_insert_mode(&self, app: &mut App, key: KeyEvent) {
        match key.code {
            // Mode switching
            KeyCode::Esc => {
                app.enter_normal_mode();
            }

            // Navigation
            KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.next();
            }
            KeyCode::Up | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.previous();
            }
            KeyCode::Left | KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.move_cursor_left();
            }
            KeyCode::Right | KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.move_cursor_right();
            }
            KeyCode::Home | KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.move_cursor_start();
            }
            KeyCode::End | KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.move_cursor_end();
            }

            // Actions
            KeyCode::Enter => {
                app.launch_selection();
            }
            KeyCode::Tab => {
                app.toggle_mode();
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.toggle_tui_preference();
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.toggle_dormant();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.clear_input();
            }

            // Text input
            KeyCode::Char(c) => {
                app.insert_char(c);
            }
            KeyCode::Backspace => {
                app.backspace();
            }
            KeyCode::Delete => {
                app.delete_char();
            }

            _ => {}
        }
    }

    fn process_normal_mode(&self, app: &mut App, key: KeyEvent) {
        match key.code {
            // Mode switching
            KeyCode::Char('i') | KeyCode::Char('a') => {
                app.enter_insert_mode();
            }

            // Quit
            KeyCode::Char('q') => {
                app.quit();
            }
            KeyCode::Esc => {
                app.quit();
            }

            // Navigation
            KeyCode::Char('j') | KeyCode::Down => {
                app.next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.previous();
            }
            KeyCode::Char('g') => {
                app.go_top();
            }
            KeyCode::Char('G') => {
                app.go_bottom();
            }

            // Actions
            KeyCode::Enter => {
                app.launch_selection();
            }
            KeyCode::Tab => {
                app.toggle_mode();
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.toggle_tui_preference();
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                app.toggle_dormant();
            }

            _ => {}
        }
    }
}
