use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::cpu::{CPU, Display};
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::canvas::Canvas;

pub struct App {
    exit: bool,
    cpu: CPU,
    display: Display,

    last_cpu_cycle: std::time::Instant,
    last_timer_decrement: std::time::Instant,
}

impl Default for App {
    fn default() -> Self {
        let mut cpu = CPU::default();
        cpu.load_rom(&std::path::PathBuf::from("./roms/test_opcode.ch8"));

        Self {
            exit: Default::default(),
            display: Arc::clone(cpu.display()),
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
            self.handle_events()?;

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
                self.cpu.one_clock_cycle(&[]);
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

        let title = TextLine::from_iter([Span::from("Chip-8 Emulator").bold(), Span::from(" (Press 'q' to quit)")]);
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
        if event::poll(Duration::from_millis(1))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    if let KeyCode::Char('q') = key_event.code {
                        self.exit = true
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
