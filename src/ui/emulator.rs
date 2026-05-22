use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use device_query::{CallbackGuard, DeviceEvents, DeviceEventsHandler};

use crate::cpu::{CPU, Display, Keyboard, Timer};

pub struct Emulator {
    // cpu: Arc<Mutex<CPU>>,
    display: Display,
    // keyboard: Keyboard,
    // delay_timer: Timer,
    // sound_timer: Timer,
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

        // Timers thread decremented by one 60 times per second
        timer_thread(&delay_timer, &sound_timer);
        // CPU emulation running at 700 instructions per second
        cpu_thread(&cpu);
        // Beeper thread
        let _sound_device = sound_thread(&sound_timer); // The sound device thread will stop when dropped out of scope
        // Using device_query to handle press/release of keys
        let _key_handler = register_key_handler(&keyboard); // The callbacks are unregistered when dropped out of scope

        Self {
            // cpu,
            display,
            // keyboard,
            // delay_timer,
            // sound_timer,
        }
    }
}

fn timer_thread(delay_timer: &Timer, sound_timer: &Timer) {
    let delay_timer = delay_timer.clone();
    let sound_timer = sound_timer.clone();
    std::thread::spawn(move || {
        let mut last_time = std::time::Instant::now();

        loop {
            if std::time::Instant::now().duration_since(last_time).as_micros() >= 1_000_000 / 60 {
                last_time = std::time::Instant::now();

                sound_timer.decrement();
                delay_timer.decrement();
            }
        }
    });
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

fn cpu_thread(cpu: &Arc<Mutex<CPU>>) {
    let cpu = Arc::clone(cpu);
    std::thread::spawn(move || {
        let mut last_time = std::time::Instant::now();

        loop {
            if std::time::Instant::now().duration_since(last_time).as_micros() >= 1_000_000 / 700 {
                last_time = std::time::Instant::now();
                cpu.lock().unwrap().one_clock_cycle();
            }
        }
    });
}

#[allow(clippy::type_complexity)]
fn register_key_handler(
    keyboard: &Keyboard,
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
        let keyboard = Arc::clone(keyboard);
        device_handler.on_key_up(move |key| {
            if let Some(key) = map_keys(key) {
                keyboard.lock().unwrap()[key] = false;
            }
        })
    };

    let keydown_callback = {
        let keyboard = Arc::clone(keyboard);
        device_handler.on_key_down(move |key| {
            if let Some(key) = map_keys(key) {
                keyboard.lock().unwrap()[key] = true;
            }
        })
    };

    (keyup_callback, keydown_callback)
}
