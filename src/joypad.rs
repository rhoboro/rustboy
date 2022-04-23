use std::fmt::{Debug, Formatter};

use crate::io::IO;
use crate::Address;

#[derive(Clone, Copy, Debug)]
struct Buttons {
    // Bit 5
    // ボタンモード選択中は0
    button: bool,
    // Bit 4
    // 方向モード選択中は0
    direction: bool,

    // 押されていたら0
    // Bit 3
    down_start: bool,
    // Bit 2
    up_select: bool,
    // Bit 1
    left_b: bool,
    // Bit 0
    right_a: bool,
}

pub struct JoyPad {
    buttons: Buttons,
}

impl JoyPad {
    pub fn new() -> Self {
        Self {
            buttons: Buttons::from(0b_0011_1111),
        }
    }
}

impl IO for JoyPad {
    fn read(&self, address: Address) -> u8 {
        debug_log!("Read JoyPad: {:X?}", address);
        u8::from(self.buttons)
    }
    fn write(&mut self, address: Address, data: u8) {
        debug_log!("Write JoyPad: {:X?}, Data: {}", address, data);
        self.buttons = Buttons::from(data)
    }
}

impl From<u8> for Buttons {
    fn from(v: u8) -> Self {
        Self {
            button: !((v & 0b_0010_0000) == 0b_0010_0000),
            direction: !((v & 0b_0001_0000) == 0b_0001_0000),
            down_start: !((v & 0b_0000_1000) == 0b_0000_1000),
            up_select: !((v & 0b_0000_0100) == 0b_0000_0100),
            left_b: !((v & 0b_0000_0010) == 0b_0000_0010),
            right_a: !((v & 0b_0000_0001) == 0b_0000_0001),
        }
    }
}

impl From<Buttons> for u8 {
    fn from(buttons: Buttons) -> Self {
        let mut v = 0b_0011_1111;
        if !buttons.button {
            v &= 0b_0001_1111;
        }
        if !buttons.direction {
            v &= 0b_0010_1111;
        }
        if !buttons.down_start {
            v &= 0b_0011_0111;
        }
        if !buttons.up_select {
            v &= 0b_0011_1011;
        }
        if !buttons.left_b {
            v &= 0b_0011_1101;
        }
        if !buttons.right_a {
            v &= 0b_0011_1110;
        }
        v
    }
}

impl Debug for JoyPad {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "JoyPad")
    }
}
