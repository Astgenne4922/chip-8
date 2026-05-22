mod emulator;
mod menu;

use crossterm::event;
use device_query::{CallbackGuard, DeviceEvents, DeviceEventsHandler};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::cpu::{CPU, DISPLAY_HEIGHT, DISPLAY_WIDTH, Display, Keyboard, Timer};
use crate::ui::menu::Menu;
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::canvas::Canvas;

pub struct App {
    cpu: Arc<Mutex<CPU>>,
    display: Display,
    keyboard: Keyboard,
    delay_timer: Timer,
    sound_timer: Timer,
    menu: Menu,
}

impl Default for App {
    fn default() -> Self {
        let cpu = Arc::new(Mutex::new(CPU::default()));
        let (display, keyboard, delay_timer, sound_timer) = {
            let mut cpu = cpu.lock().unwrap();
            // cpu.load_rom(&std::path::PathBuf::from("./roms/tests/5-quirks.ch8"));
            cpu.load_rom(&std::path::PathBuf::from("./roms/tests/6-keypad.ch8"));
            // cpu.load_rom(&std::path::PathBuf::from("./roms/tests/7-beep.ch8"));

            (
                Arc::clone(cpu.display()),
                Arc::clone(cpu.keyboard()),
                cpu.delay_timer().clone(),
                cpu.sound_timer().clone(),
            )
        };

        Self {
            cpu,
            display,
            keyboard,
            delay_timer,
            sound_timer,
            menu: Menu::default(),
        }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        // // Timers thread decremented by one 60 times per second
        // self.timer_thread();
        // // CPU emulation running at 700 instructions per second
        // self.cpu_thread();
        // // Beeper thread
        // let _sound_device = self.sound_thread(); // The sound device thread will stop when dropped out of scope
        // // Using device_query to handle press/release of keys
        // let _key_handler = self.register_key_handler(); // The callbacks are unregistered when dropped out of scope

        loop {
            // Block rendering if the window is too small
            if !self.is_too_small(terminal)? {
                terminal.draw(|frame| self.draw(frame))?;

                // Normal UI key handling
                if event::poll(Duration::from_millis(0))?
                    && let Some(key) = event::read()?.as_key_press_event()
                {
                    if matches!(key.code, event::KeyCode::Esc) {
                        break;
                    }

                    self.menu.update(key)?;
                }
            }
        }

        Ok(())
    }

    fn is_too_small(&self, terminal: &mut DefaultTerminal) -> std::io::Result<bool> {
        let size = terminal.size()?;
        if size.height < DISPLAY_HEIGHT as u16 + 4 || size.width < DISPLAY_WIDTH as u16 * 2 + 2 {
            terminal.draw(|frame| {
                let par = Paragraph::new("too small")
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

        let title = TextLine::from_iter([Span::from("Chip-8 Emulator").bold(), Span::from(" (Press 'ESC' to quit)")]);
        frame.render_widget(title.centered(), top);

        let canvas = Canvas::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .x_bounds([0.0, DISPLAY_WIDTH as f64])
            .y_bounds([-(DISPLAY_HEIGHT as f64), 0.0])
            .paint(|ctx| {
                for y in 0..DISPLAY_HEIGHT {
                    for x in 0..DISPLAY_WIDTH {
                        if self.display.lock().unwrap()[y as usize][x as usize] {
                            ctx.print(x as f64, -(y as f64), "██");
                        }
                    }
                }
            });

        // frame.render_widget(canvas, area);
        frame.render_widget(&self.menu, area);
    }

    fn is_quit(&self) -> std::io::Result<bool> {
        if let Some(key) = event::read()?.as_key_event()
            && key.is_press()
            && matches!(key.code, event::KeyCode::Esc)
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
