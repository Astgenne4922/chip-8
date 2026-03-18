use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

use crate::emu::{
    keyboard::Key,
    memory::{RAM, Registers},
};

type Opcode = (u8, u8, u8, u8, u8, u16);

pub type Display = Arc<Mutex<[[bool; 64]; 32]>>;

pub struct CPU {
    ram: RAM,
    registers: Registers,
    pc: usize,
    i: u16,
    stack: Vec<u16>,
    display: Display,
    delay_timer: u8,
    sound_timer: Arc<AtomicU8>,
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
            delay_timer: 0,
            sound_timer: Arc::new(AtomicU8::new(0)),
        }
    }
}

impl CPU {
    pub fn one_clock_cycle(&mut self, pressed_keys: &[Key]) {
        let instruction = self.fetch();
        let opcode = self.decode(instruction);
        self.execute(opcode, pressed_keys);
    }

    pub fn decrement_sound(&mut self) {
        if self.sound_timer.fetch_sub(1, Ordering::SeqCst) == u8::MAX {
            self.sound_timer.store(0, Ordering::Relaxed);
        }
    }

    pub fn decrement_delay(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
    }

    pub fn load_rom(&mut self, rom_path: &Path) {
        let rom = std::fs::read(rom_path).unwrap();
        self.ram.put(0x200, &rom);
    }

    pub fn display(&self) -> &Display {
        &self.display
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

    fn execute(&mut self, instruction: Opcode, pressed_keys: &[Key]) {
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
            (0x8, _, _, 0x1) => self.registers.write(x, self.registers.read(x) | self.registers.read(y)),
            // 8XY2 -> Set VX to VX AND VY
            (0x8, _, _, 0x2) => self.registers.write(x, self.registers.read(x) & self.registers.read(y)),
            // 8XY3 -> Set VX to VX XOR VY
            (0x8, _, _, 0x3) => self.registers.write(x, self.registers.read(x) ^ self.registers.read(y)),
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
                if carry {
                    self.registers.write(0xFu8, if carry { 0 } else { 1 });
                }
            }
            // 8XY6 -> Set VX to VX >> 1 (with carry flag)
            (0x8, _, _, 0x6) => {
                let vx = self.registers.read(x);
                self.registers.write(0xFu8, (vx & 0b10000000) >> 7);
                self.registers.write(x, vx >> 1);
            }
            // 8XY7 -> Subtract VX from VY (with carry flag)
            (0x8, _, _, 0x7) => {
                let (sub, carry) = self.registers.read(y).overflowing_sub(self.registers.read(x));
                self.registers.write(y, sub);
                self.registers.write(0xFu8, if carry { 0 } else { 1 });
            }
            // 8XYE -> Set VX to VX << 1 (with carry flag)
            (0x8, _, _, 0xE) => {
                let vx = self.registers.read(x);
                self.registers.write(0xFu8, vx & 0x1);
                self.registers.write(x, vx << 1);
            }
            // 9XY0 -> Skip one if VX != VY
            (0x9, _, _, 0x0) => {
                if self.registers.read(x) != self.registers.read(x) {
                    self.pc += 2;
                }
            }
            // ANNN -> Set I to NNN
            (0xA, _, _, _) => self.i = nnn,
            // BNNN -> Jump to address NNN + V0
            (0xB, _, _, _) => self.pc = (nnn + (self.registers.read(0x0u8) as u16)) as usize,
            // CXNN -> Sets VX to RAND & NN
            (0xC, _, _, _) => self.registers.write(x, rand::random_range(0..255) & nn),
            // DXYN -> Draws the display
            (0xD, _, _, _) => {
                let mut vy = self.registers.read(y) % 32;

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
                        display[vy as usize][vx as usize] = ((s & (1 << (7 - b))) >> (7 - b)) == 1;
                        if display[vy as usize][vx as usize] {
                            self.registers.write(0xFu8, 1);
                        }
                        vx += 1;
                    }
                    vy += 1;
                }
            }
            // EX9E -> Skip one if key VX is pressed
            (0xE, _, 0x9, 0xE) => {
                if pressed_keys.contains(&self.registers.read(x).into()) {
                    self.pc += 2;
                }
            }
            // EXA1 -> Skip one if key VX is not pressed
            (0xE, _, 0xA, 0x1) => {
                if !pressed_keys.contains(&self.registers.read(x).into()) {
                    self.pc += 2;
                }
            }
            // FX07 -> Sets VX to delay timer
            (0xF, _, 0x0, 0x7) => self.registers.write(x, self.delay_timer),
            // FX07 -> Sets delay timer to VX
            (0xF, _, 0x0, 0xA) => self.delay_timer = self.registers.read(x),
            // FX07 -> Sets sound timer to VX
            (0xF, _, 0x1, 0x5) => self
                .sound_timer
                .store(self.registers.read(x), std::sync::atomic::Ordering::Relaxed),
            // FX1E -> Add VX to I (with carry flag if "overflow" over 0x1000)
            (0xF, _, 0x1, 0x8) => {
                let (sum, carry) = self.i.overflowing_add(self.registers.read(x).into());
                self.i = sum;
                self.registers.write(0xFu8, if carry || sum >= 0x1000 { 1 } else { 0 });
            }
            // FX0A -> Block for key press
            (0xF, _, 0x1, 0xE) => {
                if let Some(key) = pressed_keys.first() {
                    self.registers.write(x, *key);
                } else {
                    self.pc -= 2;
                }
            }
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
                    self.ram.write(self.i + x, self.registers.read(x));
                }
            }
            // FX66 -> Load registers 0 to X from memory
            (0xF, _, 0x6, 0x5) => {
                for x in 0..=x as u16 {
                    self.registers.write(x, self.ram.read(self.i + x));
                }
            }
            _ => unreachable!(),
        }
    }
}
