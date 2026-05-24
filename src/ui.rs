mod emulator;
mod menu;

use crossterm::event;
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::time::Duration;

use crate::cpu::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
use crate::ui::emulator::Emulator;
use crate::ui::menu::Menu;
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::Stylize;
use ratatui::text::{Line as TextLine, Span};

#[derive(Default)]
pub enum State {
    #[default]
    Menu,
    Emulation,
}

#[derive(Default)]
pub struct App {
    state: State,
    menu: Menu,
    emulator: Option<Emulator>,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            // Block rendering if the window is too small
            if !self.is_too_small(terminal)? {
                terminal.draw(|frame| self.draw(frame))?;

                // Normal UI key handling
                if event::poll(Duration::from_millis(0))?
                    && let Some(key) = event::read()?.as_key_press_event()
                {
                    if matches!(key.code, event::KeyCode::Esc) {
                        match self.state {
                            State::Menu => break,
                            State::Emulation => {
                                self.state = State::Menu;
                                self.emulator = None;
                            }
                        }
                    }

                    match self.state {
                        State::Menu => {
                            let res = self.menu.update(key)?;
                            if let Some(rom) = res {
                                let mut emulator = Emulator::new(&rom);
                                emulator.start_threads();
                                self.emulator = Some(emulator);
                                self.state = State::Emulation;
                            }
                        }
                        State::Emulation => {}
                    };
                }
            }
        }

        Ok(())
    }

    fn is_too_small(&self, terminal: &mut DefaultTerminal) -> std::io::Result<bool> {
        let size = terminal.size()?;
        if size.height < DISPLAY_HEIGHT as u16 + 4 || size.width < DISPLAY_WIDTH as u16 * 2 + 2 {
            terminal.draw(|frame| {
                let par = Paragraph::new(format!("too small - {} x {}", size.width, size.height))
                    .centered()
                    .block(Block::bordered().padding(Padding::new(0, 0, size.height / 2, 0)));
                frame.render_widget(par, frame.area());
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Length(DISPLAY_HEIGHT as u16 + 2)])
            .spacing(1)
            .flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(DISPLAY_WIDTH as u16 * 2 + 2)])
            .spacing(1)
            .flex(Flex::Center);
        let [top, main] = frame.area().layout(&vertical);
        let [area] = main.layout(&horizontal);

        let title = TextLine::from_iter([
            Span::from("Chip-8 Emulator").bold(),
            Span::from(" (Press 'ESC' to quit)"),
        ]);
        frame.render_widget(title.centered(), top);

        let block = Block::bordered();
        let block_area = block.inner(area);
        frame.render_widget(&block, area);
        match self.state {
            State::Menu => frame.render_widget(&self.menu, block_area),
            State::Emulation => frame.render_widget(
                self.emulator.as_ref().expect("The emulator should be instantiated"),
                block_area,
            ),
        }
    }
}
