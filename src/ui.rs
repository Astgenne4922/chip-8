use crossterm::event::{self, KeyCode};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::sync::Arc;
use std::time::Duration;

use crate::cpu::{CPU, Display, Keyboard};
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::canvas::Canvas;

pub struct App {
    exit: bool,
    cpu: CPU,
    display: Display,
    keyboard: Keyboard,

    last_cpu_cycle: std::time::Instant,
    last_timer_decrement: std::time::Instant,
}

impl Default for App {
    fn default() -> Self {
        let mut cpu = CPU::default();
        // cpu.load_rom(&std::path::PathBuf::from("./roms/test_opcode.ch8"));
        cpu.load_rom(&std::path::PathBuf::from(
            "./roms/programs/BMP Viewer - Hello (C8 example) [Hap, 2005].ch8",
        ));

        Self {
            exit: Default::default(),
            display: Arc::clone(cpu.display()),
            keyboard: Arc::clone(cpu.keyboard()),
            cpu,
            last_cpu_cycle: std::time::Instant::now(),
            last_timer_decrement: std::time::Instant::now(),
        }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            let size = terminal.size()?;
            if size.height < 36 || size.width < 130 {
                terminal.draw(|frame| {
                    let par = Paragraph::new("too small")
                        .centered()
                        .block(Block::bordered().padding(Padding::new(0, 0, size.height / 2, 0)));
                    frame.render_widget(par, frame.area());
                })?;
                continue;
            }
            terminal.draw(|frame| self.draw(frame))?;

                if std::time::Instant::now()
                    .duration_since(self.last_timer_decrement)
                    .as_micros()
                    >= 1000000 / 60
                {
                    self.last_timer_decrement = std::time::Instant::now();
                    self.cpu.decrement_delay();
                    self.cpu.decrement_sound();
                }

                if std::time::Instant::now()
                    .duration_since(self.last_cpu_cycle)
                    .as_micros()
                    >= 1000000 / 700
                {
                    self.last_cpu_cycle = std::time::Instant::now();
                    self.cpu.one_clock_cycle();
                }

            *self.keyboard.lock().unwrap() = [false; 16];
            if event::poll(Duration::from_millis(100))? {
            self.handle_events()?;
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Length(32 + 2)])
            .spacing(1)
            .flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(128 + 2)])
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
            .x_bounds([0.0, 64.0])
            .y_bounds([-32.0, 0.0])
            .paint(|ctx| {
                for y in 0..32 {
                    for x in 0..64 {
                        if self.display.lock().unwrap()[y][x] {
                            ctx.print(x as f64, -(y as f64), "██");
                        }
                    }
                }
            });

        frame.render_widget(canvas, area);
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        if let Some(key) = event::read()?.as_key_event()
            && key.is_press()
        {
            if matches!(key.code, KeyCode::Esc) {
                self.exit = true;
            } else {
                let keypad_key: Option<usize> = match key.code {
                    KeyCode::Char('1') => Some(1),
                    KeyCode::Char('2') => Some(2),
                    KeyCode::Char('3') => Some(3),
                    KeyCode::Char('4') => Some(0xC),
                    KeyCode::Char('q') => Some(4),
                    KeyCode::Char('w') => Some(5),
                    KeyCode::Char('e') => Some(6),
                    KeyCode::Char('r') => Some(0xD),
                    KeyCode::Char('a') => Some(7),
                    KeyCode::Char('s') => Some(8),
                    KeyCode::Char('d') => Some(9),
                    KeyCode::Char('f') => Some(0xE),
                    KeyCode::Char('z') => Some(0xA),
                    KeyCode::Char('x') => Some(0),
                    KeyCode::Char('c') => Some(0xB),
                    KeyCode::Char('v') => Some(0xF),
                    _ => None,
                };
                if let Some(keypad_key) = keypad_key {
                    self.keyboard.lock().unwrap()[keypad_key] = true;
                }
            }
        }

        Ok(())
    }
}
