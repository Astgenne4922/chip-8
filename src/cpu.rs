use std::path::Path;

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct OPCODE {
    code: u8,
    x: u8,
    y: u8,
    n: u8,
    nn: u8,
    nnn: u16,
}

#[derive(Clone)]
pub enum Key {
    ZERO,
    ONE,
    TWO,
    THREE,
    FOUR,
    FIVE,
    SIX,
    SEVEN,
    EIGHT,
    NINE,
    A,
    B,
    C,
    D,
    E,
    F,
}

impl PartialEq<u8> for Key {
    fn eq(&self, other: &u8) -> bool {
        let key: u8 = self.clone().into();
        key == *other
    }
}

impl From<Key> for u8 {
    fn from(val: Key) -> Self {
        match val {
            Key::ZERO => 0x0,
            Key::ONE => 0x1,
            Key::TWO => 0x2,
            Key::THREE => 0x3,
            Key::FOUR => 0x4,
            Key::FIVE => 0x5,
            Key::SIX => 0x6,
            Key::SEVEN => 0x7,
            Key::EIGHT => 0x8,
            Key::NINE => 0x9,
            Key::A => 0xA,
            Key::B => 0xB,
            Key::C => 0xC,
            Key::D => 0xD,
            Key::E => 0xE,
            Key::F => 0xF,
        }
    }
}

pub struct CPU {
    memory: [u8; 4096],
    registers: [u8; 16],
    pc: usize,
    i: u16,
    stack: Vec<u16>,
    display: [[bool; 64]; 32],
    delay_timer: u8,
    sound_timer: u8,
}

impl Default for CPU {
    fn default() -> Self {
        let mut memory = [0; 4096];
        memory[0x50..=0x9F].copy_from_slice(&FONT);

        Self {
            memory,
            registers: [0; 16],
            pc: 0x200,
            i: 0,
            stack: Vec::with_capacity(16),
            display: [[false; 64]; 32],
            delay_timer: 0,
            sound_timer: 0,
        }
    }
}

impl CPU {
    pub fn one_clock_cycle(&mut self, pressed_key: Option<Key>) {
        let instruction = self.fetch();
        let opcode = self.decode(instruction);
        self.execute(opcode, pressed_key);
    }

    pub fn is_beeping(&self) -> bool {
        self.sound_timer > 0
    }

    pub fn decrement_sound(&mut self) {
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn decrement_delay(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
    }

    pub fn load_rom(&mut self, rom_path: &Path) {
        let rom = std::fs::read(rom_path).unwrap();
        for (i, byte) in rom.iter().enumerate() {
            self.memory[0x200 + i] = *byte;
        }
    }

    pub fn display(&self) -> &[[bool; 64]; 32] {
        &self.display
    }

    fn fetch(&mut self) -> u16 {
        let instruction = ((self.memory[self.pc] as u16) << 8) + (self.memory[self.pc + 1] as u16);
        self.pc += 2;
        instruction
    }

    fn decode(&mut self, instruction: u16) -> OPCODE {
        OPCODE {
            code: ((instruction & 0xF000) >> 12) as u8,
            x: ((instruction & 0x0F00) >> 8) as u8,
            y: ((instruction & 0x00F0) >> 4) as u8,
            n: (instruction & 0x000F) as u8,
            nn: (instruction & 0x00FF) as u8,
            nnn: instruction & 0x0FFF,
        }
    }

    fn execute(&mut self, instruction: OPCODE, pressed_key: Option<Key>) {
        match instruction.code {
            0x00 => match instruction.nnn {
                // 00E0 -> Clear Screen
                0x0E0 => self.display = [[false; 64]; 32],
                // 00EE -> Return from subroutine
                0x0EE => {
                    self.pc = self.stack.pop().unwrap() as usize;
                }
                _ => unreachable!(),
            },
            // 1NNN -> Jump
            0x01 => self.pc = instruction.nnn as usize,
            // 2NNN -> Call subroutine
            0x02 => {
                self.stack.push(self.pc as u16);
                self.pc = instruction.nnn as usize;
            }
            // 3XNN -> Skip one if VX == NN
            0x03 => {
                self.pc += if self.registers[instruction.x as usize] == instruction.nn {
                    2
                } else {
                    0
                };
            }
            // 4XNN -> Skip one if VX != NN
            0x04 => {
                self.pc += if self.registers[instruction.x as usize] != instruction.nn {
                    2
                } else {
                    0
                };
            }
            // 5XY0 -> Skip one if VX == VY
            0x05 => {
                self.pc += if self.registers[instruction.x as usize] == self.registers[instruction.y as usize] {
                    2
                } else {
                    0
                };
            }
            // 6XNN -> Set VX to NN
            0x06 => self.registers[instruction.x as usize] = instruction.nn,
            // 7XNN -> Add NN to VX
            0x07 => {
                self.registers[instruction.x as usize] =
                    self.registers[instruction.x as usize].overflowing_add(instruction.nn).0;
            }
            0x08 => match instruction.n {
                // 8XY0 -> Set VX to VY
                0x00 => self.registers[instruction.x as usize] = self.registers[instruction.y as usize],
                // 8XY1 -> Set VX to VX OR VY
                0x01 => self.registers[instruction.x as usize] |= self.registers[instruction.y as usize],
                // 8XY2 -> Set VX to VX AND VY
                0x02 => self.registers[instruction.x as usize] &= self.registers[instruction.y as usize],
                // 8XY3 -> Set VX to VX XOR VY
                0x03 => self.registers[instruction.x as usize] ^= self.registers[instruction.y as usize],
                // 8XY4 -> Add VY to VX (with carry flag)
                0x04 => {
                    let (sum, carry) =
                        self.registers[instruction.x as usize].overflowing_add(self.registers[instruction.y as usize]);
                    self.registers[instruction.x as usize] = sum;
                    self.registers[0xF] = if carry { 1 } else { 0 };
                }
                // 8XY5 -> Subtract VY from VX (with carry flag)
                0x05 => {
                    let (sub, carry) =
                        self.registers[instruction.x as usize].overflowing_sub(self.registers[instruction.y as usize]);
                    self.registers[instruction.x as usize] = sub;
                    self.registers[0xF] = if carry { 0 } else { 1 };
                }
                // 8XY6 -> Set VX to VX << 1 (with carry flag)
                0x06 => {
                    self.registers[0xF] = (self.registers[instruction.x as usize] & 0x80) >> 7;
                    self.registers[instruction.x as usize] <<= 1;
                }
                // 8XY7 -> Subtract VX from VY (with carry flag)
                0x07 => {
                    let (sub, carry) =
                        self.registers[instruction.y as usize].overflowing_sub(self.registers[instruction.x as usize]);
                    self.registers[instruction.y as usize] = sub;
                    self.registers[0xF] = if carry { 0 } else { 1 };
                }
                // 8XYE -> Set VX to VX >> 1 (with carry flag)
                0x0E => {
                    self.registers[0xF] = self.registers[instruction.x as usize] & 0x1;
                    self.registers[instruction.x as usize] >>= 1;
                }
                _ => unreachable!(),
            },
            // 9XY0 -> Skip one if VX != VY
            0x09 => {
                self.pc += if self.registers[instruction.x as usize] != self.registers[instruction.y as usize] {
                    2
                } else {
                    0
                };
            }
            // ANNN -> Set I to NNN
            0x0A => self.i = instruction.nnn,
            // BNNN -> Jump to address NNN + V0
            0x0B => self.pc = (instruction.nnn + (self.registers[0x0] as u16)) as usize,
            // CXNN -> Sets VX to RAND & NN
            0x0C => self.registers[instruction.x as usize] = rand::random_range(0..255) & instruction.nn,
            // DXYN -> Draws the display
            0x0D => {
                let mut y = self.registers[instruction.y as usize] % 32;

                for i in 0..instruction.n as u16 {
                    let mut x = self.registers[instruction.x as usize] % 64;
                    if y >= 32 {
                        break;
                    }
                    let s = self.memory[(self.i + i) as usize];
                    for b in 0..8 {
                        if x >= 64 {
                            break;
                        }
                        self.display[y as usize][x as usize] = ((s & (1 << (7 - b))) >> (7 - b)) == 1;
                        if self.display[y as usize][x as usize] {
                            self.registers[0xF] = 1;
                        }
                        x += 1;
                    }
                    y += 1;
                }
            }
            0x0E => match instruction.nn {
                // EX9E -> Skip one if key VX is pressed
                0x9E => {
                    if let Some(key) = pressed_key {
                        self.pc += if key == self.registers[instruction.x as usize] {
                            2
                        } else {
                            0
                        };
                    }
                }
                // EXA1 -> Skip one if key VX is not pressed
                0xA1 => {
                    self.pc += if pressed_key.is_none_or(|k| k != self.registers[instruction.x as usize]) {
                        2
                    } else {
                        0
                    }
                }
                _ => unreachable!(),
            },
            0x0F => match instruction.nn {
                // FX07 -> Sets VX to delay timer
                0x07 => self.registers[instruction.x as usize] = self.delay_timer,
                // FX07 -> Sets delay timer to VX
                0x15 => self.delay_timer = self.registers[instruction.x as usize],
                // FX07 -> Sets sound timer to VX
                0x18 => self.sound_timer = self.registers[instruction.x as usize],
                // FX1E -> Add VX to I (with carry flag if "overflow" over 0x1000)
                0x1E => {
                    let (sum, carry) = self.i.overflowing_add(self.registers[instruction.x as usize] as u16);
                    self.i = sum;
                    self.registers[0xF] = if carry || sum >= 0x1000 { 1 } else { 0 };
                }
                // FX0A -> Block for key press
                0x0A => {
                    if let Some(key) = pressed_key {
                        self.registers[instruction.x as usize] = key.into();
                    } else {
                        self.pc -= 2;
                    }
                }
                // FX29 -> Sets I to font character for the value in VX
                0x29 => self.i = (self.registers[instruction.x as usize] * 5) as u16,
                // FX33 -> Sets address I, I + 1 and I + 2 to the 3 decimal digits of VX
                0x33 => {
                    self.memory[self.i as usize] = self.registers[instruction.x as usize] / 100;
                    self.memory[self.i as usize + 1] = (self.registers[instruction.x as usize] % 100) / 10;
                    self.memory[self.i as usize + 2] = self.registers[instruction.x as usize] % 10;
                }
                // FX55 -> Store registers 0 to X in memory
                0x55 => {
                    for x in 0..=instruction.x as u16 {
                        self.memory[(self.i + x) as usize] = self.registers[x as usize];
                    }
                }
                // FX66 -> Load registers 0 to X from memory
                0x65 => {
                    for x in 0..=instruction.x as u16 {
                        self.registers[x as usize] = self.memory[(self.i + x) as usize];
                    }
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
