use std::collections::HashMap;

pub fn get_keyboard() -> HashMap<Key, bool> {
    HashMap::from([
        (Key::ZERO, false),
        (Key::ONE, false),
        (Key::TWO, false),
        (Key::THREE, false),
        (Key::FOUR, false),
        (Key::FIVE, false),
        (Key::SIX, false),
        (Key::SEVEN, false),
        (Key::EIGHT, false),
        (Key::NINE, false),
        (Key::A, false),
        (Key::B, false),
        (Key::C, false),
        (Key::D, false),
        (Key::E, false),
        (Key::F, false),
    ])
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

impl From<u8> for Key {
    fn from(val: u8) -> Self {
        match val {
            0x0 => Key::ZERO,
            0x1 => Key::ONE,
            0x2 => Key::TWO,
            0x3 => Key::THREE,
            0x4 => Key::FOUR,
            0x5 => Key::FIVE,
            0x6 => Key::SIX,
            0x7 => Key::SEVEN,
            0x8 => Key::EIGHT,
            0x9 => Key::NINE,
            0xA => Key::A,
            0xB => Key::B,
            0xC => Key::C,
            0xD => Key::D,
            0xE => Key::E,
            0xF => Key::F,
            _ => unreachable!(),
        }
    }
}
