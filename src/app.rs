// ============================================================================
// Application State and Logic - Optimized
// ============================================================================

use crate::db::Database;
use crate::system::{AppEntry, scan_apps, fuzzy_search_apps, FileSearcher};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Apps,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
}

pub struct App {
    // Input state
    pub input: String,
    pub input_mode: InputMode,
    pub cursor_pos: usize,

    // Mode state
    pub mode: Mode,
    pub selected_index: usize,
    pub show_dormant: bool,

    // Data
    pub all_apps: Vec<AppEntry>,
    pub filtered_apps: Vec<AppEntry>,
    pub filtered_files: Vec<String>,

    // File searcher (lazy, streaming)
    file_searcher: FileSearcher,

    // Database
    pub db: Database,

    // UI State
    pub should_quit: bool,
    pub should_launch: bool,
    pub launch_command: Option<(String, Vec<String>, bool)>, // (program, args, is_tui)
}

impl App {
    pub fn new() -> Self {
        let db = Database::new().expect("Failed to initialize database");
        
        // Only load apps on startup - files are lazy-loaded
        let apps = scan_apps(&db);

        Self {
            input: String::new(),
            input_mode: InputMode::Insert,
            cursor_pos: 0,
            mode: Mode::Apps,
            selected_index: 0,
            show_dormant: false,
            all_apps: apps.clone(),
            filtered_apps: apps,
            filtered_files: Vec::new(), // Start empty
            file_searcher: FileSearcher::new(),
            db,
            should_quit: false,
            should_launch: false,
            launch_command: None,
        }
    }

    // =========================================================================
    // Input Handling
    // =========================================================================

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
        self.update_search();
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input.remove(self.cursor_pos);
            self.update_search();
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
            self.update_search();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn move_cursor_start(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.update_search();
    }

    // =========================================================================
    // Mode Switching
    // =========================================================================

    pub fn enter_normal_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn enter_insert_mode(&mut self) {
        self.input_mode = InputMode::Insert;
    }

    #[allow(dead_code)]
    pub fn toggle_input_mode(&mut self) {
        self.input_mode = match self.input_mode {
            InputMode::Normal => InputMode::Insert,
            InputMode::Insert => InputMode::Normal,
        };
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Apps => Mode::Files,
            Mode::Files => Mode::Apps,
        };
        self.selected_index = 0;
        self.update_search();
    }

    pub fn toggle_dormant(&mut self) {
        self.show_dormant = !self.show_dormant;
        self.update_search();
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    pub fn next(&mut self) {
        let count = self.result_count();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    pub fn previous(&mut self) {
        let count = self.result_count();
        if count > 0 {
            self.selected_index = if self.selected_index == 0 {
                count - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn go_top(&mut self) {
        self.selected_index = 0;
    }

    pub fn go_bottom(&mut self) {
        let count = self.result_count();
        if count > 0 {
            self.selected_index = count - 1;
        }
    }

    // =========================================================================
    // Search & Filtering - OPTIMIZED
    // =========================================================================

    fn update_search(&mut self) {
        self.selected_index = 0;

        match self.mode {
            Mode::Apps => {
                let matched = if self.input.is_empty() {
                    self.all_apps.clone()
                } else {
                    fuzzy_search_apps(&self.input, &self.all_apps)
                        .into_iter()
                        .cloned()
                        .collect()
                };

                self.filtered_apps = matched
                    .into_iter()
                    .filter(|app| self.show_dormant || !app.is_dormant)
                    .collect();
            }
            Mode::Files => {
                // Streaming file search - only search when there's a query
                if self.input.is_empty() {
                    self.filtered_files.clear();
                } else {
                    // This is fast because it streams results and stops early
                    self.filtered_files = self.file_searcher.search(&self.input, 50);
                }
            }
        }
    }

    fn result_count(&self) -> usize {
        match self.mode {
            Mode::Apps => self.filtered_apps.len(),
            Mode::Files => self.filtered_files.len(),
        }
    }

    // =========================================================================
    // Actions
    // =========================================================================

    pub fn toggle_tui_preference(&mut self) -> bool {
        if let Mode::Apps = self.mode {
            if self.filtered_apps.is_empty() {
                return false;
            }
            let app = &self.filtered_apps[self.selected_index];
            let current_state = self.db.is_tui_app(&app.name);
            let _ = self.db.set_tui_mode(&app.name, !current_state);
            return true;
        }
        false
    }

    pub fn launch_selection(&mut self) {
        match self.mode {
            Mode::Apps => {
                if self.filtered_apps.is_empty() {
                    return;
                }
                let app = &self.filtered_apps[self.selected_index];

                // Update usage stats
                let _ = self.db.increment_usage(&app.name);

                // Determine if TUI
                let is_tui = if self.db.has_entry(&app.name) {
                    self.db.is_tui_app(&app.name)
                } else {
                    app.is_cli_only
                };

                // Parse exec command
                let clean_exec = app
                    .exec
                    .split_whitespace()
                    .filter(|s| !s.starts_with('%'))
                    .collect::<Vec<&str>>()
                    .join(" ");

                let args_owned = shell_words::split(&clean_exec).unwrap_or_default();
                if args_owned.is_empty() {
                    return;
                }

                let program = args_owned[0].clone();
                let args: Vec<String> = args_owned[1..].iter().cloned().collect();

                self.launch_command = Some((program, args, is_tui));
                self.should_launch = true;
            }
            Mode::Files => {
                if self.filtered_files.is_empty() {
                    return;
                }
                let file_path = self.filtered_files[self.selected_index].clone();

                self.launch_command = Some((
                    "kitty".to_string(),
                    vec!["-e".to_string(), "nvim".to_string(), file_path],
                    false,
                ));
                self.should_launch = true;
            }
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
