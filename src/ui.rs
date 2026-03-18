use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{self, KeyCode};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::cpu::{CPU, Display, Keyboard};
use crate::emu::keyboard::Key;
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
        let mut last_tick = Instant::now();
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

            let timeout = Duration::from_millis(1).saturating_sub(last_tick.elapsed());
            if !event::poll(timeout)? {
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
                last_tick = Instant::now();
                continue;
            }

            self.handle_events()?;
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
        if let Some(key) = event::read()?.as_key_event() {
            if matches!(key.code, KeyCode::Esc) && key.is_press() {
                self.exit = true;
            } else {
                let keypad_key = match key.code {
                    KeyCode::Char('1') => Some(Key::ONE),
                    KeyCode::Char('2') => Some(Key::TWO),
                    KeyCode::Char('3') => Some(Key::THREE),
                    KeyCode::Char('4') => Some(Key::C),
                    KeyCode::Char('q') => Some(Key::FOUR),
                    KeyCode::Char('w') => Some(Key::FIVE),
                    KeyCode::Char('e') => Some(Key::SIX),
                    KeyCode::Char('r') => Some(Key::D),
                    KeyCode::Char('a') => Some(Key::SEVEN),
                    KeyCode::Char('s') => Some(Key::EIGHT),
                    KeyCode::Char('d') => Some(Key::NINE),
                    KeyCode::Char('f') => Some(Key::E),
                    KeyCode::Char('z') => Some(Key::A),
                    KeyCode::Char('x') => Some(Key::ZERO),
                    KeyCode::Char('c') => Some(Key::B),
                    KeyCode::Char('v') => Some(Key::F),
                    _ => None,
                };
                if let Some(keypad_key) = keypad_key {
                    self.keyboard.lock().unwrap().insert(keypad_key, key.is_press());
                }
            }
        }

        Ok(())
    }
}
