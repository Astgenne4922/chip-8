use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use device_query::{DeviceQuery as _, DeviceState};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Widget, canvas::Canvas},
};

use crate::cpu::{CPU, DISPLAY_HEIGHT, DISPLAY_WIDTH, Display, Keyboard, Timer};

pub struct Emulator {
    cpu: Arc<Mutex<CPU>>,
    display: Display,
    keyboard: Keyboard,
    delay_timer: Timer,
    sound_timer: Timer,
    _is_running: Arc<AtomicBool>,
    _threads: Vec<JoinHandle<()>>,
}

impl Emulator {
    pub fn new(rom: &Path) -> Self {
        let cpu = Arc::new(Mutex::new(CPU::default()));
        let (display, keyboard, delay_timer, sound_timer) = {
            let mut cpu = cpu.lock().unwrap();
            cpu.load_rom(rom);

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
            _is_running: Arc::new(AtomicBool::new(true)),
            _threads: Vec::new(),
        }
    }

    pub fn start_threads(&mut self) {
        // Timers thread decremented by one 60 times per second
        self._threads
            .push(timer_thread(&self.delay_timer, &self.sound_timer, &self._is_running));
        // CPU emulation running at 700 instructions per second
        self._threads.push(cpu_thread(&self.cpu, &self._is_running));
        // Beeper thread
        let _sound_device = sound_thread(&self.sound_timer); // The sound device thread will stop when dropped out of scope
        // Using device_query to handle press/release of keys
        // let _key_handler = register_key_handler(&self.keyboard); // The callbacks are unregistered when dropped out of scope
        self._threads.push(keyboard_thread(&self.keyboard, &self._is_running));
    }
}

impl Drop for Emulator {
    fn drop(&mut self) {
        self._is_running.store(false, Ordering::Relaxed);
        while let Some(thread) = self._threads.pop() {
            thread.join().expect("The thread should join flawlessly");
        }
    }
}

impl Widget for &Emulator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let canvas = Canvas::default()
            // .block(
            //     Block::default()
            //         .borders(Borders::ALL)
            //         .border_style(Style::default().fg(Color::White)),
            // )
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

        canvas.render(area, buf);
    }
}

fn timer_thread(delay_timer: &Timer, sound_timer: &Timer, is_running: &Arc<AtomicBool>) -> JoinHandle<()> {
    let delay_timer = delay_timer.clone();
    let sound_timer = sound_timer.clone();
    let is_running = Arc::clone(is_running);
    std::thread::spawn(move || {
        let mut last_time = std::time::Instant::now();

        while is_running.load(Ordering::Relaxed) {
            if std::time::Instant::now().duration_since(last_time).as_micros() >= 1_000_000 / 60 {
                last_time = std::time::Instant::now();

                sound_timer.decrement();
                delay_timer.decrement();
            }
        }
    })
}

fn sound_thread(sound_timer: &Timer) -> tinyaudio::OutputDevice {
    let sound_timer = sound_timer.clone();

    let params = tinyaudio::OutputDeviceParameters {
        channels_count: 2,
        sample_rate: 44100,
        channel_sample_count: 4410,
    };

    tinyaudio::run_output_device(params, {
        let mut clock = 0f32;
        move |data| {
            let vol = if sound_timer.get() == 0 { 0.0 } else { 0.1 };
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

fn cpu_thread(cpu: &Arc<Mutex<CPU>>, is_running: &Arc<AtomicBool>) -> JoinHandle<()> {
    let cpu = Arc::clone(cpu);
    let is_running = Arc::clone(is_running);
    std::thread::spawn(move || {
        let mut last_time = std::time::Instant::now();

        while is_running.load(Ordering::Relaxed) {
            if std::time::Instant::now().duration_since(last_time).as_micros() >= 1_000_000 / 700 {
                last_time = std::time::Instant::now();
                cpu.lock().unwrap().one_clock_cycle();
            }
        }
    })
}

fn keyboard_thread(keyboard: &Keyboard, is_running: &Arc<AtomicBool>) -> JoinHandle<()> {
    let keyboard = Arc::clone(keyboard);
    let is_running = Arc::clone(is_running);
    std::thread::spawn(move || {
        let device_state = DeviceState::new();
        let mut prev_keys = vec![];
        while is_running.load(Ordering::Relaxed) {
            let keys = device_state.get_keys();
            for key_state in &keys {
                if !prev_keys.contains(key_state)
                    && let Some(key) = map_keys(key_state)
                {
                    keyboard.lock().unwrap()[key] = false;
                }
            }
            for key_state in &prev_keys {
                if !keys.contains(key_state)
                    && let Some(key) = map_keys(key_state)
                {
                    keyboard.lock().unwrap()[key] = true;
                }
            }
            prev_keys = keys;
            std::thread::sleep(Duration::from_millis(10));
        }
    })
}

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
