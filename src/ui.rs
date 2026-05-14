use crossterm::event;
use device_query::{CallbackGuard, DeviceEvents, DeviceEventsHandler};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::cpu::{CPU, DISPLAY_HEIGHT, DISPLAY_WIDTH, Display, Keyboard, Timer};
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
        // Timers thread decremented by one 60 times per second
        self.timer_thread();
        // CPU emulation running at 700 instructions per second
        self.cpu_thread();
        // Beeper thread
        let _sound_device = self.sound_thread(); // The sound device thread will stop when dropped out of scope
        // Using device_query to handle press/release of keys
        let _ = self.register_key_handler(); // The callbacks are unregistered when dropped out of scope

        while !self.exit {
            // Block rendering if the window is too small
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

    fn sound_thread(&self) -> tinyaudio::OutputDevice {
        let sound_timer = self.sound_timer.clone();

        let params = tinyaudio::OutputDeviceParameters {
            channels_count: 2,
            sample_rate: 44100,
            channel_sample_count: 4410,
        };

        tinyaudio::run_output_device(params, {
            let mut clock = 0f32;
            move |data| {
                let vol = if sound_timer.get() == 0 { 0.0 } else { 0.03 };
                for samples in data.chunks_mut(params.channels_count) {
                    clock = (clock + 1.0) % params.sample_rate as f32;
                    let value = (clock * 440.0 * 2.0 * std::f32::consts::PI / params.sample_rate as f32).sin();
                    for sample in samples {
                        *sample = value * vol;
                    }
                }
            }
        })
        .unwrap()
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
