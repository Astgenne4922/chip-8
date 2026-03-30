use crossterm::event;
use device_query::{CallbackGuard, DeviceEvents, DeviceEventsHandler};
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
        // Using device_query to handle press/release of keys
        let _ = self.register_key_handler(); // The callbacks are unregistered when dropped out of scope

        while !self.exit {
            if !self.is_too_small(terminal)? {
                terminal.draw(|frame| self.draw(frame))?;

                // Normal UI key handling
                if event::poll(Duration::from_millis(10))? {
                    self.handle_events()?;
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
            && matches!(key.code, event::KeyCode::Esc)
        {
            self.exit = true;
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

    #[allow(clippy::type_complexity)]
    fn register_key_handler(
        &self,
    ) -> (
        CallbackGuard<impl Fn(&device_query::Keycode)>,
        CallbackGuard<impl Fn(&device_query::Keycode)>,
    ) {
        fn map_keys(key: &device_query::Keycode) -> Option<usize> {
            use device_query::Keycode;
            match key {
                Keycode::Key1 => Some(1),
                Keycode::Key2 => Some(2),
                Keycode::Key3 => Some(3),
                Keycode::Key4 => Some(0xC),
                Keycode::Q => Some(4),
                Keycode::W => Some(5),
                Keycode::E => Some(6),
                Keycode::R => Some(0xD),
                Keycode::A => Some(7),
                Keycode::S => Some(8),
                Keycode::D => Some(9),
                Keycode::F => Some(0xE),
                Keycode::Z => Some(0xA),
                Keycode::X => Some(0),
                Keycode::C => Some(0xB),
                Keycode::V => Some(0xF),
                _ => None,
            }
        }

        let device_handler = DeviceEventsHandler::new(Duration::from_millis(10)).unwrap();

        let keyup_callback = {
            let keyboard = Arc::clone(&self.keyboard);
            device_handler.on_key_up(move |key| {
                if let Some(key) = map_keys(key) {
                    keyboard.lock().unwrap()[key] = false;
                }
            })
        };

        let keydown_callback = {
            let keyboard = Arc::clone(&self.keyboard);
            device_handler.on_key_down(move |key| {
                if let Some(key) = map_keys(key) {
                    keyboard.lock().unwrap()[key] = true;
                }
            })
        };

        (keyup_callback, keydown_callback)
    }
}
