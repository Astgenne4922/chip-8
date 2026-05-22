use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{List, ListState, Scrollbar, ScrollbarState, StatefulWidget, Widget},
};

#[allow(clippy::upper_case_acronyms)]
enum Side {
    Category,
    ROM,
}

pub struct Menu {
    rom_categories: Vec<String>,
    roms: Vec<Vec<PathBuf>>,
    category: usize,
    selected_rom: Option<usize>,
    interactable_side: Side,
}

impl Default for Menu {
    fn default() -> Self {
        let mut roms = Vec::new();

        // List sub-folders in roms folder
        let dirs: Vec<PathBuf> = std::fs::read_dir("./roms")
            .unwrap()
            .flatten()
            .map(|dir| dir.path().to_path_buf())
            .collect();

        for dir in &dirs {
            // Get all the roms in each sub-folder
            let romlist: Vec<PathBuf> = dir
                .read_dir()
                .unwrap()
                .flatten()
                .filter(|rom| rom.path().extension().unwrap() == "ch8")
                .map(|rom| rom.path().to_path_buf())
                .collect();
            roms.push(romlist);
        }

        Self {
            rom_categories: dirs
                .iter()
                .map(|dir| dir.file_name().unwrap().to_string_lossy().to_string())
                .collect(),
            roms,
            category: 0,
            selected_rom: None,
            interactable_side: Side::Category,
        }
    }
}

impl Widget for &Menu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]);
        let [category, roms] = area.layout(&layout);

        render_list(self.rom_categories.clone(), Some(self.category), category, buf);
        render_list(
            self.roms[self.category]
                .iter()
                .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
                .collect(),
            self.selected_rom,
            roms,
            buf,
        );
    }
}

impl Menu {
    pub fn update(&mut self, key: KeyEvent) -> std::io::Result<Option<PathBuf>> {
        match self.interactable_side {
            Side::Category => self.update_category(key),
            Side::ROM => return Ok(self.update_roms(key)),
        }

        Ok(None)
    }

    fn update_category(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.category = self.category.saturating_sub(1),
            KeyCode::Down => {
                if self.category < self.rom_categories.len() - 1 {
                    self.category = self.category.saturating_add(1);
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                self.interactable_side = Side::ROM;
                self.selected_rom = Some(0);
            }
            _ => {}
        };
    }

    fn update_roms(&mut self, key: KeyEvent) -> Option<PathBuf> {
        match key.code {
            KeyCode::Up => self.selected_rom = self.selected_rom.map(|i| i.saturating_sub(1)),
            KeyCode::Down => {
                if self
                    .selected_rom
                    .is_some_and(|i| i < self.roms[self.category].len() - 1)
                {
                    self.selected_rom = self.selected_rom.map(|i| i.saturating_add(1));
                }
            }
            KeyCode::Enter => return Some(self.roms[self.category][self.selected_rom.unwrap()].clone()),
            KeyCode::Backspace | KeyCode::Left => {
                self.interactable_side = Side::Category;
                self.selected_rom = None;
            }
            _ => {}
        };

        None
    }
}

fn render_list(list: Vec<String>, selected_index: Option<usize>, area: Rect, buf: &mut Buffer) {
    let len = list.len();
    let mut state = ListState::default().with_selected(selected_index);
    StatefulWidget::render(
        List::new(list)
            .highlight_symbol(">>")
            .highlight_style(Style::new().fg(Color::Black).bg(Color::White)),
        area,
        buf,
        &mut state,
    );

    if let Some(selected_index) = selected_index {
        let mut scroll_state = ScrollbarState::default().content_length(len).position(selected_index);
        Scrollbar::default()
            .thumb_symbol("▐")
            .render(area, buf, &mut scroll_state);
    }
}
