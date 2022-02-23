use std::convert::Into;
use std::fmt::{Debug, Formatter};

use crate::io::IO;
use crate::Address;

pub enum Peripheral {
    Joypad,
    Serial,
    Timer,
    LcdStatus,
    VBlank,
}

impl Peripheral {
    pub fn jump_address(&self) -> Address {
        match self {
            Peripheral::Joypad => 0x0060,
            Peripheral::Serial => 0x0058,
            Peripheral::Timer => 0x0050,
            Peripheral::LcdStatus => 0x0048,
            Peripheral::VBlank => 0x0040,
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct InterruptEnables {
    // https://gbdev.io/pandocs/Interrupts.html
    // Bit 4: Joypad   Interrupt Enable (INT $60)  (1=Enable)
    pub joypad: bool,
    // Bit 3: Serial   Interrupt Enable (INT $58)  (1=Enable)
    pub serial: bool,
    // Bit 2: Timer    Interrupt Enable (INT $50)  (1=Enable)
    pub timer: bool,
    // Bit 1: LCD STAT Interrupt Enable (INT $48)  (1=Enable)
    pub lcd_stat: bool,
    // Bit 0: VBlank   Interrupt Enable (INT $40)  (1=Enable)
    pub v_blank: bool,
}

impl From<u8> for InterruptEnables {
    fn from(v: u8) -> Self {
        Self {
            joypad: (v & 0b_0001_0000) == 0b_0001_0000,
            serial: (v & 0b_0000_1000) == 0b_0000_1000,
            timer: (v & 0b_0000_0100) == 0b_0000_0100,
            lcd_stat: (v & 0b_0000_0010) == 0b_0000_0010,
            v_blank: (v & 0b_0000_0001) == 0b_0000_0001,
        }
    }
}

impl From<InterruptEnables> for u8 {
    fn from(enables: InterruptEnables) -> Self {
        let mut v = 0b_0000_0000;
        if enables.joypad {
            v |= 0b_0001_0000;
        }
        if enables.serial {
            v |= 0b_0000_1000;
        }
        if enables.timer {
            v |= 0b_0000_0100;
        }
        if enables.lcd_stat {
            v |= 0b_0000_0010;
        }
        if enables.v_blank {
            v |= 0b_0000_0001;
        }
        v
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct InterruptFlags {
    // https://gbdev.io/pandocs/Interrupts.html
    // Bit 4: Joypad   Interrupt Request (INT $60)  (1=Request)
    pub joypad: bool,
    // Bit 3: Serial   Interrupt Request (INT $58)  (1=Request)
    pub serial: bool,
    // Bit 2: Timer    Interrupt Request (INT $50)  (1=Request)
    pub timer: bool,
    // Bit 1: LCD STAT Interrupt Request (INT $48)  (1=Request)
    pub lcd_stat: bool,
    // Bit 0: VBlank   Interrupt Request (INT $40)  (1=Request)
    pub v_blank: bool,
}

impl From<u8> for InterruptFlags {
    fn from(v: u8) -> Self {
        Self {
            joypad: (v & 0b_0001_0000) == 0b_0001_0000,
            serial: (v & 0b_0000_1000) == 0b_0000_1000,
            timer: (v & 0b_0000_0100) == 0b_0000_0100,
            lcd_stat: (v & 0b_0000_0010) == 0b_0000_0010,
            v_blank: (v & 0b_0000_0001) == 0b_0000_0001,
        }
    }
}

impl From<InterruptFlags> for u8 {
    fn from(flags: InterruptFlags) -> Self {
        let mut v = 0b_0000_0000;
        if flags.joypad {
            v |= 0b_0001_0000;
        }
        if flags.serial {
            v |= 0b_0000_1000;
        }
        if flags.timer {
            v |= 0b_0000_0100;
        }
        if flags.lcd_stat {
            v |= 0b_0000_0010;
        }
        if flags.v_blank {
            v |= 0b_0000_0001;
        }
        v
    }
}

pub struct Interruption {
    // 0xFF0F: 割り込みフラグ
    interrupts: InterruptFlags,
    // 0xFFFF: 割り込み有効フラグ
    enables: InterruptEnables,
}

impl Interruption {
    pub fn new() -> Self {
        Self {
            interrupts: InterruptFlags::default(),
            enables: InterruptEnables::default(),
        }
    }
    pub fn print_interrupt_flags(&self) {
        println!("InterruptFlags: 0b{:08b}", u8::from(self.interrupts));
    }
    pub fn print_interrupt_enables(&self) {
        println!("InterruptEnables: 0b{:08b}", u8::from(self.enables));
    }
}

impl IO for Interruption {
    fn read(&self, address: Address) -> u8 {
        match address {
            // 割り込みフラグ
            0xFF0F => self.interrupts.into(),
            // 割り込み有効
            0xFFFF => self.interrupts.into(),
            _ => unreachable!(),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        match address {
            // 割り込みフラグ
            0xFF0F => self.interrupts = InterruptFlags::from(data),
            // 割り込み有効
            0xFFFF => self.enables = InterruptEnables::from(data),
            _ => unreachable!(),
        }
    }
}

impl Debug for Interruption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Interruption")
    }
}
