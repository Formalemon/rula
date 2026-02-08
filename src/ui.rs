// ============================================================================
// UI Renderer - Optimized with Cached DB Lookups
// ============================================================================

use crate::app::{App, InputMode, Mode};
use crate::terminal::Terminal;
use crate::theme::*;
use std::io;
use std::collections::HashMap;

pub struct Ui {
    term: Terminal,
    width: u16,
    height: u16,
    // Cache TUI status to avoid DB queries during rendering
    tui_cache: HashMap<String, bool>,
}

const COL_CONTENT_START: u16 = 2;
const ROW_INPUT: u16 = 1;
const ROW_RESULTS_START: u16 = 3;

impl Ui {
    pub fn new() -> io::Result<Self> {
        let term = Terminal::new()?;
        let (width, height) = term.size();
        Ok(Self { 
            term, 
            width, 
            height,
            tui_cache: HashMap::new(),
        })
    }

    pub fn render(&mut self, app: &App) -> io::Result<()> {
        // Refresh TUI cache before rendering
        self.refresh_tui_cache(app);

        self.term.clear()?;
        self.draw_border()?;
        self.draw_input_row(app)?;
        self.draw_results(app)?;

        if app.input_mode == InputMode::Insert {
            let cursor_x = self.calculate_cursor_x(app);
            self.term.write(crate::theme::SHOW_CURSOR)?;
            self.term.move_to(cursor_x, ROW_INPUT)?;
            self.term.write(RESET)?;
        } else {
            self.term.write(crate::theme::HIDE_CURSOR)?;
        }

        self.term.flush()?;
        Ok(())
    }

    // Cache TUI status for all visible apps to avoid DB queries during rendering
    fn refresh_tui_cache(&mut self, app: &App) {
        self.tui_cache.clear();
        
        if app.mode == Mode::Apps {
            for app_entry in &app.filtered_apps {
                let is_tui = if app.db.has_entry(&app_entry.name) {
                    app.db.is_tui_app(&app_entry.name)
                } else {
                    app_entry.is_cli_only
                };
                self.tui_cache.insert(app_entry.name.clone(), is_tui);
            }
        }
    }

    fn get_tui_status(&self, app_name: &str) -> bool {
        self.tui_cache.get(app_name).copied().unwrap_or(false)
    }

    // ========================================================================
    // Drawing Components
    // ========================================================================

    fn draw_border(&mut self) -> io::Result<()> {
        let w = self.width;
        let h = self.height;
        let color = RosePineMoon::HIGHLIGHT_MED;

        self.term.write_styled(0, 0, "╭", &Style::new().fg(color))?;
        self.term.write_styled(w - 1, 0, "╮", &Style::new().fg(color))?;
        self.term.write_styled(0, h - 1, "╰", &Style::new().fg(color))?;
        self.term.write_styled(w - 1, h - 1, "╯", &Style::new().fg(color))?;

        if w > 2 {
            self.term.hline(1, 0, w - 2, '─', color)?;
            self.term.hline(1, h - 1, w - 2, '─', color)?;
        }

        if h > 2 {
            let vertical = "│";
            let style = Style::new().fg(color);
            for y in 1..(h - 1) {
                self.term.write_styled(0, y, vertical, &style)?;
                self.term.write_styled(w - 1, y, vertical, &style)?;
            }
        }
        Ok(())
    }

    fn draw_input_row(&mut self, app: &App) -> io::Result<()> {
        let mut x = COL_CONTENT_START;
        let (prompt_text, prompt_color) = match app.mode {
            Mode::Apps => ("Apps > ", RosePineMoon::LOVE),
            Mode::Files => ("Files > ", RosePineMoon::GOLD),
        };

        self.term.write_at(x, ROW_INPUT, &Style::new().fg(prompt_color).bold().apply(prompt_text))?;
        x += prompt_text.len() as u16;

        let input_style = if app.input_mode == InputMode::Insert {
            Style::new().fg(RosePineMoon::TEXT)
        } else {
            Style::new().fg(RosePineMoon::SUBTLE)
        };
        self.term.write_at(x, ROW_INPUT, &input_style.apply(&app.input))?;
        Ok(())
    }

    fn calculate_cursor_x(&self, app: &App) -> u16 {
        let mut x = COL_CONTENT_START;
        let prompt_len = match app.mode {
            Mode::Apps => 7,
            Mode::Files => 8,
        };
        x += prompt_len;
        x += app.cursor_pos as u16;
        x
    }

    // ========================================================================
    // Results List (Optimized rendering)
    // ========================================================================

    fn draw_results(&mut self, app: &App) -> io::Result<()> {
        let max_render_row = self.height.saturating_sub(1);
        let list_height = max_render_row.saturating_sub(ROW_RESULTS_START);

        let all_items = match app.mode {
            Mode::Apps => self.prepare_app_items(app, 50),
            Mode::Files => self.prepare_file_items(app, 50),
        };

        // Calculate optimal start_index for scrolling
        let mut start_index = app.selected_index;
        let mut current_view_height = 0;

        for i in (0..=app.selected_index).rev() {
            if let Some((icon, text, aux_text, _, _)) = all_items.get(i) {
                let item_height = self.measure_item_height(icon, text, aux_text);
                
                if current_view_height + item_height > list_height {
                    break; 
                }
                current_view_height += item_height;
                start_index = i;
            }
        }

        // Render visible items
        let mut current_row = ROW_RESULTS_START;

        for (_idx, (icon, text, aux_text, is_selected, is_tui)) in all_items.iter().enumerate().skip(start_index) {
            if current_row >= max_render_row {
                break;
            }

            // Selection indicator
            let indicator = if *is_selected { "> " } else { "  " };
            let ind_style = if *is_selected { 
                Style::new().fg(RosePineMoon::LOVE).bold() 
            } else { 
                Style::new() 
            };
            self.term.write_at(COL_CONTENT_START, current_row, &ind_style.apply(indicator))?;

            // Icon
            let mut x = COL_CONTENT_START + 2;
            if !icon.is_empty() {
                let icon_color = if *is_tui { RosePineMoon::PINE } else { RosePineMoon::SUBTLE };
                self.term.write_at(x, current_row, &Style::new().fg(icon_color).apply(icon))?;
                x += icon.chars().count() as u16 + 1;
            }

            // Main text
            let name_style = if *is_selected {
                Style::new().fg(RosePineMoon::TEXT).bold()
            } else {
                Style::new().fg(RosePineMoon::SUBTLE)
            };
            self.term.write_at(x, current_row, &name_style.apply(text))?;
            x += text.chars().count() as u16 + 1;

            // Path with smart wrapping
            if !aux_text.is_empty() {
                let available_width = (self.width.saturating_sub(x).saturating_sub(1)) as usize;
                
                if aux_text.len() <= available_width {
                    let path_style = Style::new().fg(RosePineMoon::MUTED);
                    self.term.write_at(x, current_row, &path_style.apply(aux_text))?;
                    current_row += 1; 
                } else {
                    let split_idx = aux_text[..available_width].rfind('/').unwrap_or(available_width);
                    
                    let part1 = &aux_text[..split_idx];
                    let path_style = Style::new().fg(RosePineMoon::MUTED);
                    self.term.write_at(x, current_row, &path_style.apply(part1))?;
                    current_row += 1;

                    if current_row < max_render_row {
                        let part2 = &aux_text[split_idx..];
                        let avail_2 = (self.width.saturating_sub(x).saturating_sub(1)) as usize;
                        let part2_display = if part2.len() > avail_2 {
                            format!("{}...", &part2[..avail_2.saturating_sub(3)])
                        } else {
                            part2.to_string()
                        };
                        self.term.write_at(x, current_row, &path_style.apply(&part2_display))?;
                        current_row += 1;
                    }
                }
            } else {
                current_row += 1;
            }
        }
        
        // Clear remaining space
        while current_row < max_render_row {
             let space_count = (self.width.saturating_sub(2)) as usize;
             self.term.write_at(1, current_row, &" ".repeat(space_count))?;
             current_row += 1;
        }

        Ok(())
    }

    fn measure_item_height(&self, icon: &str, text: &str, aux_text: &str) -> u16 {
        if aux_text.is_empty() {
            return 1;
        }

        let mut x = COL_CONTENT_START + 2;
        if !icon.is_empty() {
            x += icon.chars().count() as u16 + 1;
        }
        x += text.chars().count() as u16 + 1;

        let available_width = (self.width.saturating_sub(x).saturating_sub(1)) as usize;
        
        if aux_text.len() <= available_width {
            1
        } else {
            2
        }
    }

    fn prepare_app_items(&self, app: &App, max: u16) -> Vec<(String, String, String, bool, bool)> {
        let start_index = if app.selected_index >= max as usize {
            app.selected_index - (max as usize) + 1
        } else {
            0
        };

        app.filtered_apps
            .iter()
            .enumerate()
            .skip(start_index)
            .take(max as usize)
            .map(|(i, entry)| {
                let is_selected = i == app.selected_index;
                let is_tui = self.get_tui_status(&entry.name);
                let icon = if is_tui { "\u{e795}" } else { "" };
                (icon.to_string(), entry.name.clone(), "".to_string(), is_selected, is_tui)
            })
            .collect()
    }

    fn prepare_file_items(&self, app: &App, max: u16) -> Vec<(String, String, String, bool, bool)> {
        let start_index = if app.selected_index >= max as usize {
            app.selected_index - (max as usize) + 1
        } else {
            0
        };

        app.filtered_files
            .iter()
            .enumerate()
            .skip(start_index)
            .take(max as usize)
            .map(|(i, path_str)| {
                let is_selected = i == app.selected_index;
                let path = std::path::Path::new(path_str);
                
                let name = path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_str.clone());
                
                let parent = path.parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                ("".to_string(), name, parent, is_selected, false)
            })
            .collect()
    }
}
