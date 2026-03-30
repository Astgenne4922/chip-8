use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

use crate::memory::{RAM, Registers};

type Opcode = (u8, u8, u8, u8, u8, u16);

pub type Display = Arc<Mutex<[[bool; 64]; 32]>>;
pub type Keyboard = Arc<Mutex<[bool; 16]>>;
pub struct Timer(Arc<AtomicU8>);

impl Default for Timer {
    fn default() -> Self {
        Self(Arc::new(AtomicU8::new(0)))
    }
}

impl Clone for Timer {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl Timer {
    pub fn set(&self, value: u8) {
        self.0.store(value, Ordering::Relaxed);
    }

    pub fn get(&self) -> u8 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn decrement(&self) {
        if self.0.fetch_sub(1, Ordering::SeqCst) == u8::MAX {
            self.0.store(0, Ordering::Relaxed);
        }
    }
}

enum State {
    Running,
    Waiting,
    Pressed(u8),
}

pub struct CPU {
    ram: RAM,
    registers: Registers,
    pc: usize,
    i: u16,
    stack: Vec<u16>,
    display: Display,
    keyboard: Keyboard,
    delay_timer: Timer,
    sound_timer: Timer,
    state: State,
}

impl Default for CPU {
    fn default() -> Self {
        Self {
            ram: RAM::default(),
            registers: Registers::default(),
            pc: 0x200,
            i: 0,
            stack: Vec::with_capacity(16),
            display: Arc::new(Mutex::new([[false; 64]; 32])),
            keyboard: Arc::new(Mutex::new([false; 16])),
            delay_timer: Timer::default(),
            sound_timer: Timer::default(),
            state: State::Running,
        }
    }
}

impl CPU {
    pub fn one_clock_cycle(&mut self) {
        let instruction = self.fetch();
        let opcode = self.decode(instruction);
        self.execute(opcode);
    }

    pub fn load_rom(&mut self, rom_path: &Path) {
        let rom = std::fs::read(rom_path).unwrap();
        self.ram.put(0x200, &rom);
    }

    pub fn display(&self) -> &Display {
        &self.display
    }

    pub fn keyboard(&self) -> &Keyboard {
        &self.keyboard
    }

    pub fn delay_timer(&self) -> &Timer {
        &self.delay_timer
    }

    pub fn sound_timer(&self) -> &Timer {
        &self.sound_timer
    }

    fn fetch(&mut self) -> u16 {
        let instruction = ((self.ram.read(self.pc) as u16) << 8) + (self.ram.read(self.pc + 1) as u16);
        self.pc += 2;
        instruction
    }

    fn decode(&mut self, instruction: u16) -> Opcode {
        (
            ((instruction & 0xF000) >> 12) as u8,
            ((instruction & 0x0F00) >> 8) as u8,
            ((instruction & 0x00F0) >> 4) as u8,
            (instruction & 0x000F) as u8,
            (instruction & 0x00FF) as u8,
            instruction & 0x0FFF,
        )
    }

    fn execute(&mut self, instruction: Opcode) {
        let (code, x, y, n, nn, nnn) = instruction;
        match (code, x, y, n) {
            // 0000 -> Call SYS routine (NOT IMPLEMENTED)
            (0x0, 0x0, 0x0, 0x0) => unimplemented!(),
            // 00E0 -> Clear Screen
            (0x0, 0x0, 0xE, 0x0) => *self.display.lock().unwrap() = [[false; 64]; 32],
            // 00EE -> Return from subroutine
            (0x0, 0x0, 0xE, 0xE) => self.pc = self.stack.pop().unwrap() as usize,
            // 1NNN -> Jump
            (0x1, _, _, _) => self.pc = nnn as usize,
            // 2NNN -> Call subroutine
            (0x2, _, _, _) => {
                self.stack.push(self.pc as u16);
                self.pc = nnn as usize;
            }
            // 3XNN -> Skip one if VX == NN
            (0x3, _, _, _) => {
                if self.registers.read(x) == nn {
                    self.pc += 2;
                }
            }
            // 4XNN -> Skip one if VX != NN
            (0x4, _, _, _) => {
                if self.registers.read(x) != nn {
                    self.pc += 2;
                }
            }
            // 5XY0 -> Skip one if VX == VY
            (0x5, _, _, 0x0) => {
                if self.registers.read(x) == self.registers.read(y) {
                    self.pc += 2;
                }
            }
            // 6XNN -> Set VX to NN
            (0x6, _, _, _) => self.registers.write(x, nn),
            // 7XNN -> Add NN to VX
            (0x7, _, _, _) => self.registers.write(x, self.registers.read(x).overflowing_add(nn).0),
            // 8XY0 -> Set VX to VY
            (0x8, _, _, 0x0) => self.registers.write(x, self.registers.read(y)),
            // 8XY1 -> Set VX to VX OR VY
            (0x8, _, _, 0x1) => {
                self.registers.write(x, self.registers.read(x) | self.registers.read(y));
                self.registers.write(0xFu8, 0);
            }
            // 8XY2 -> Set VX to VX AND VY
            (0x8, _, _, 0x2) => {
                self.registers.write(x, self.registers.read(x) & self.registers.read(y));
                self.registers.write(0xFu8, 0);
            }
            // 8XY3 -> Set VX to VX XOR VY
            (0x8, _, _, 0x3) => {
                self.registers.write(x, self.registers.read(x) ^ self.registers.read(y));
                self.registers.write(0xFu8, 0);
            }
            // 8XY4 -> Add VY to VX (with carry flag)
            (0x8, _, _, 0x4) => {
                let (sum, carry) = self.registers.read(x).overflowing_add(self.registers.read(y));
                self.registers.write(x, sum);
                self.registers.write(0xFu8, if carry { 1 } else { 0 });
            }
            // 8XY5 -> Subtract VY from VX (with carry flag)
            (0x8, _, _, 0x5) => {
                let (sub, carry) = self.registers.read(x).overflowing_sub(self.registers.read(y));
                self.registers.write(x, sub);
                self.registers.write(0xFu8, if carry { 0 } else { 1 });
            }
            // 8XY6 -> Set VX to VY >> 1 (with carry flag)
            (0x8, _, _, 0x6) => {
                let vy = self.registers.read(y);
                self.registers.write(x, vy >> 1);
                self.registers.write(0xFu8, vy & 0b00000001);
            }
            // 8XY7 -> Subtract VX from VY and set the result to VX (with carry flag)
            (0x8, _, _, 0x7) => {
                let (sub, carry) = self.registers.read(y).overflowing_sub(self.registers.read(x));
                self.registers.write(x, sub);
                self.registers.write(0xFu8, if carry { 0 } else { 1 });
            }
            // 8XYE -> Set VX to VY << 1 (with carry flag)
            (0x8, _, _, 0xE) => {
                let vy = self.registers.read(y);
                self.registers.write(x, vy << 1);
                self.registers.write(0xFu8, (vy & 0b10000000) >> 7);
            }
            // 9XY0 -> Skip one if VX != VY
            (0x9, _, _, 0x0) => {
                if self.registers.read(x) != self.registers.read(y) {
                    self.pc += 2;
                }
            }
            // ANNN -> Set I to NNN
            (0xA, _, _, _) => self.i = nnn,
            // BNNN -> Jump to address NNN + V0
            (0xB, _, _, _) => self.pc = (nnn + (self.registers.read(0x0u8) as u16)) as usize,
            // CXNN -> Sets VX to RAND & NN
            (0xC, _, _, _) => self.registers.write(x, rand::random_range(0..=255) & nn),
            // DXYN -> Draws the display
            (0xD, _, _, _) => {
                let mut vy = self.registers.read(y) % 32;
                self.registers.write(0xFu8, 0);

                let mut display = self.display.lock().unwrap();
                for i in 0..n as u16 {
                    let mut vx = self.registers.read(x) % 64;
                    if vy >= 32 {
                        break;
                    }
                    let s = self.ram.read(self.i + i);
                    for b in 0..8 {
                        if vx >= 64 {
                            break;
                        }
                        let pixel = ((s & (1 << (7 - b))) >> (7 - b)) == 1;
                        if pixel {
                            if display[vy as usize][vx as usize] {
                                self.registers.write(0xFu8, 1);
                            }
                            display[vy as usize][vx as usize] = !display[vy as usize][vx as usize];
                        }
                        vx += 1;
                    }
                    vy += 1;
                }
            }
            // EX9E -> Skip one if key VX is pressed
            (0xE, _, 0x9, 0xE) => {
                if self.keyboard.lock().unwrap()[self.registers.read(x) as usize] {
                    self.pc += 2;
                }
            }
            // EXA1 -> Skip one if key VX is not pressed
            (0xE, _, 0xA, 0x1) => {
                if !self.keyboard.lock().unwrap()[self.registers.read(x) as usize] {
                    self.pc += 2;
                }
            }
            // FX07 -> Sets VX to delay timer
            (0xF, _, 0x0, 0x7) => self.registers.write(x, self.delay_timer.get()),
            // FX15 -> Sets delay timer to VX
            (0xF, _, 0x1, 0x5) => self.delay_timer.set(self.registers.read(x)),
            // FX18 -> Sets sound timer to VX
            (0xF, _, 0x1, 0x8) => self.sound_timer.set(self.registers.read(x)),
            // FX1E -> Add VX to I (with carry flag if "overflow" over 0x1000)
            (0xF, _, 0x1, 0xE) => {
                let (sum, carry) = self.i.overflowing_add(self.registers.read(x).into());
                self.i = sum;
                self.registers.write(0xFu8, if carry || sum >= 0x1000 { 1 } else { 0 });
            }
            // FX0A -> Block for key press
            (0xF, _, 0x0, 0xA) => match self.state {
                State::Running => {
                    if !self.keyboard.lock().unwrap().iter().any(|pressed| *pressed) {
                        self.state = State::Waiting;
                    }
                    self.pc -= 2;
                }
                State::Waiting => {
                    let key = self
                        .keyboard
                        .lock()
                        .unwrap()
                        .iter()
                        .enumerate()
                        .find(|(_, pressed)| **pressed)
                        .map(|(key, _)| key as u8);

                    if let Some(pressed_key) = key {
                        self.state = State::Pressed(pressed_key);
                    }
                    self.pc -= 2;
                }
                State::Pressed(key) => {
                    if !self.keyboard.lock().unwrap()[key as usize] {
                        self.state = State::Running;
                        self.registers.write(x, key);
                    } else {
                        self.pc -= 2;
                    }
                }
            },
            // FX29 -> Sets I to font character for the value in VX
            (0xF, _, 0x2, 0x9) => self.i = (self.registers.read(x) * 5).into(),
            // FX33 -> Sets address I, I + 1 and I + 2 to the 3 decimal digits of VX
            (0xF, _, 0x3, 0x3) => {
                let vx = self.registers.read(x);
                self.ram.write(self.i, vx / 100);
                self.ram.write(self.i + 1, (vx % 100) / 10);
                self.ram.write(self.i + 2, vx % 10);
            }
            // FX55 -> Store registers 0 to X in memory
            (0xF, _, 0x5, 0x5) => {
                for x in 0..=x as u16 {
                    self.ram.write(self.i, self.registers.read(x));
                    self.i += 1;
                }
            }
            // FX66 -> Load registers 0 to X from memory
            (0xF, _, 0x6, 0x5) => {
                for x in 0..=x as u16 {
                    self.registers.write(x, self.ram.read(self.i));
                    self.i += 1;
                }
            }
            _ => unreachable!(),
        }
    }
}
