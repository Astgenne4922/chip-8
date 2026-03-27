use crossterm::event::{self, KeyCode};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::cpu::{CPU, Display, Keyboard, Timer};
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::canvas::Canvas;

pub struct App {
    exit: bool,
    cpu: Arc<Mutex<CPU>>,
    display: Display,
    keyboard: Keyboard,
    delay_timer: Timer,
    sound_timer: Timer,
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
            exit: Default::default(),
            cpu,
            display,
            keyboard,
            delay_timer,
            sound_timer,
        }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        self.timer_thread();
        self.cpu_thread();

        while !self.exit {
            if !self.is_too_small(terminal)? {
                terminal.draw(|frame| self.draw(frame))?;

                if event::poll(Duration::from_millis(10))? {
                    self.handle_events()?;
                } else {
                    *self.keyboard.lock().unwrap() = [false; 16];
                }
            }
        }

        Ok(())
    }

    fn is_too_small(&self, terminal: &mut DefaultTerminal) -> std::io::Result<bool> {
        let size = terminal.size()?;
        if size.height < 36 || size.width < 130 {
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
                *self.keyboard.lock().unwrap() = [false; 16];

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

    fn timer_thread(&self) {
        let delay_timer = self.delay_timer.clone();
        let sound_timer = self.sound_timer.clone();
        std::thread::spawn(move || {
            let mut last_time = std::time::Instant::now();

            loop {
                if std::time::Instant::now().duration_since(last_time).as_micros() >= 1000000 / 60 {
                    last_time = std::time::Instant::now();

                    sound_timer.decrement();
                    delay_timer.decrement();
                }
            }
        });
    }

    fn cpu_thread(&self) {
        let cpu = Arc::clone(&self.cpu);
        std::thread::spawn(move || {
            let mut last_time = std::time::Instant::now();

            loop {
                if std::time::Instant::now().duration_since(last_time).as_micros() >= 1000000 / 700 {
                    last_time = std::time::Instant::now();
                    cpu.lock().unwrap().one_clock_cycle();
                }
            }
        });
    }
}
