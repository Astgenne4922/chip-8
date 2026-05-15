use std::{collections::BTreeMap, path::PathBuf};

use crossterm::event::{self, KeyCode, KeyEvent};
use ratatui::{
    layout::{self, Constraint, Layout},
    widgets::{List, ListState, Scrollbar, ScrollbarState, StatefulWidget, Widget},
};

enum Side {
    Category,
    ROM,
}

pub struct Menu {
    rom_categories: Vec<String>,
    roms: Vec<Vec<PathBuf>>,
    category: usize,
    selected_rom: usize,
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
            selected_rom: 0,
            interactable_side: Side::Category,
        }
    }
}

impl Widget for &Menu {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let layout = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]);
        let [category, roms] = area.layout(&layout);

        self.render_category(category, buf);
        self.render_romlist(roms, buf);
    }
}

impl Menu {
    fn render_category(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut state = ListState::default().with_selected(Some(self.category));
        StatefulWidget::render(
            List::new(self.rom_categories.clone()).highlight_symbol(">>"),
            area,
            buf,
            &mut state,
        );

        let mut scroll_state = ScrollbarState::default()
            .content_length(self.rom_categories.len())
            .position(self.category);
        Scrollbar::default()
            .thumb_symbol("▐")
            .render(area, buf, &mut scroll_state);
    }

    fn render_romlist(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut state = ListState::default().with_selected(Some(self.selected_rom));
        StatefulWidget::render(
            List::new(
                self.roms[self.category]
                    .iter()
                    .map(|path| path.file_name().unwrap().to_string_lossy().to_string()),
            )
            .highlight_symbol(">>"),
            area,
            buf,
            &mut state,
        );

        let mut scroll_state = ScrollbarState::default()
            .content_length(self.roms[self.category].len())
            .position(self.selected_rom);
        Scrollbar::default()
            .thumb_symbol("▐")
            .render(area, buf, &mut scroll_state);
    }

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
            KeyCode::Enter | KeyCode::Right => self.interactable_side = Side::ROM,
            _ => {}
        };
    }

    fn update_roms(&mut self, key: KeyEvent) -> Option<PathBuf> {
        match key.code {
            KeyCode::Up => self.selected_rom = self.selected_rom.saturating_sub(1),
            KeyCode::Down => {
                if self.selected_rom < self.roms[self.category].len() - 1 {
                    self.selected_rom = self.selected_rom.saturating_add(1);
                }
            }
            KeyCode::Enter => return Some(self.roms[self.category][self.selected_rom].clone()),
            KeyCode::Backspace | KeyCode::Left => {
                self.interactable_side = Side::Category;
                self.selected_rom = 0;
            }
            _ => {}
        };

        None
    }
}
