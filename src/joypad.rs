use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::{io, thread};

use crate::io::IO;
use crate::joypad::Status::{Selected, Unselected};
use crate::Address;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Status {
    // 0
    Selected,
    // 1
    Unselected,
}

impl From<bool> for Status {
    fn from(v: bool) -> Self {
        if v {
            Unselected
        } else {
            Selected
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Buttons {
    // 1. 0x30(0b_0011_0000) が書き込まれる
    // 2. ボタンの場合 0x10(0b_0001_0000) が書き込まれる(方向の場合0x01、両方の場合0x00)
    // 3. キーの状態を下位4ビットに書き込む
    // 4. buttons & 0x0F を読み取る

    // Bit 5
    // ボタンモード選択中は0
    button: Status,
    // Bit 4
    // 方向モード選択中は0
    direction: Status,

    // 押されていたら0
    // Bit 3
    down_start: Status,
    // Bit 2
    up_select: Status,
    // Bit 1
    left_b: Status,
    // Bit 0
    right_a: Status,
}

struct Cache {
    val: Option<char>,
}

pub struct JoyPad {
    buttons: Buttons,
    rx: Receiver<String>,
    // 1度の走査で複数回読み込まれる(最初の読み込みで入力を安定させ、後で読み込んだ方の値が実際に使われる)
    cache: RefCell<Cache>,
}

impl JoyPad {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        thread::spawn(move || loop {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();
            tx.send(buffer).unwrap();
        });
        Self {
            rx,
            buttons: Buttons::from(0b_0011_1111),
            cache: RefCell::new(Cache { val: Option::None }),
        }
    }

    pub fn handle_key_event(&self, data: u8) -> Result<u8, TryRecvError> {
        let c = match self.cache.borrow().val {
            Some(c) => {
                debug_log!("Read JoyPad: Cache: {:?}", c);
                c
            }
            None => match self.rx.try_recv() {
                Ok(key) => match key.chars().next() {
                    Some(c) => c,
                    _ => '\0',
                },
                Err(_) => '\0',
            },
        };
        if c == '\0' {
            debug_log!("Read JoyPad: NoKey: {:?}", c);
            return Ok(data | 0x0F);
        } else {
            debug_log!("Read JoyPad: set: {:?}", c);
            self.cache.borrow_mut().val = Some(c);
        }
        match u8::from(self.buttons) & 0x30 {
            0x00 => {
                debug_log!("JoyPad pushed 0x00: {:?}", c);
                Ok(data)
            }
            0x10 => {
                debug_log!("JoyPad pushed 0x10: {:?}", c);
                match c {
                    '\n' => Ok(data | 0b_0000_0111),
                    ' ' => Ok(data | 0b_0000_1011),
                    'b' => Ok(data | 0b_0000_1101),
                    'a' => Ok(data | 0b_0000_1110),
                    _ => Ok(data | 0x0F),
                }
            }
            0x20 => {
                debug_log!("JoyPad pushed 0x20: {:?}", c);
                match c {
                    'j' => Ok(data | 0b_0000_0111),
                    'k' => Ok(data | 0b_0000_1011),
                    'l' => Ok(data | 0b_0000_1110),
                    'h' => Ok(data | 0b_0000_1101),
                    _ => Ok(data | 0x0F),
                }
            }
            _ => {
                debug_log!("JoyPad pushed _: {:?}, {:?}", u8::from(self.buttons), c);
                Ok(data | 0x0F)
            }
        }
    }
}

impl IO for JoyPad {
    fn read(&self, address: Address) -> u8 {
        match self.handle_key_event(u8::from(self.buttons)) {
            Ok(data) => {
                debug_log!("Read JoyPad: {:X?}, Data: {:08b}", address, data);
                data
            }
            Err(e) => panic!("Read JoyPad: {:?}", e),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        debug_log!("Write JoyPad: {:X?}, Data: {}", address, data);
        self.buttons = Buttons::from(data);
        if data == 0x30 {
            debug_log!("Write JoyPad: Reset");
            self.cache.borrow_mut().val = Option::None;
        }
        debug_log!(
            "Writed JoyPad: {:X?}, Data: {:08b}",
            address,
            u8::from(self.buttons)
        );
    }
}

impl From<u8> for Buttons {
    fn from(v: u8) -> Self {
        Self {
            button: Status::from((v & 0b_0010_0000) == 0b_0010_0000),
            direction: Status::from((v & 0b_0001_0000) == 0b_0001_0000),
            down_start: Status::from((v & 0b_0000_1000) == 0b_0000_1000),
            up_select: Status::from((v & 0b_0000_0100) == 0b_0000_0100),
            left_b: Status::from((v & 0b_0000_0010) == 0b_0000_0010),
            right_a: Status::from((v & 0b_0000_0001) == 0b_0000_0001),
        }
    }
}

impl From<Buttons> for u8 {
    fn from(buttons: Buttons) -> Self {
        let mut v = 0b_0011_1111;
        if buttons.button == Selected {
            v &= 0b_0001_1111;
        }
        if buttons.direction == Selected {
            v &= 0b_0010_1111;
        }
        if buttons.down_start == Selected {
            v &= 0b_0011_0111;
        }
        if buttons.up_select == Selected {
            v &= 0b_0011_1011;
        }
        if buttons.left_b == Selected {
            v &= 0b_0011_1101;
        }
        if buttons.right_a == Selected {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::joypad::Status::{Selected, Unselected};

    #[test]
    fn test_buttons_from() {
        assert_eq!(
            Buttons::from(0b_0000_1111),
            Buttons {
                button: Selected,
                direction: Selected,
                down_start: Unselected,
                up_select: Unselected,
                left_b: Unselected,
                right_a: Unselected,
            }
        );
        assert_eq!(
            Buttons::from(0b_0010_1111),
            Buttons {
                button: Unselected,
                direction: Selected,
                down_start: Unselected,
                up_select: Unselected,
                left_b: Unselected,
                right_a: Unselected,
            }
        );
        assert_eq!(
            Buttons::from(0b_0001_1011),
            Buttons {
                button: Selected,
                direction: Unselected,
                down_start: Unselected,
                up_select: Selected,
                left_b: Unselected,
                right_a: Unselected,
            }
        );
        assert_eq!(
            Buttons::from(0b_1101_1110),
            Buttons {
                button: Selected,
                direction: Unselected,
                down_start: Unselected,
                up_select: Unselected,
                left_b: Unselected,
                right_a: Selected,
            }
        );
    }

    #[test]
    fn test_buttons_into() {
        assert_eq!(
            u8::from(Buttons {
                button: Selected,
                direction: Selected,
                down_start: Unselected,
                up_select: Unselected,
                left_b: Unselected,
                right_a: Unselected,
            }),
            0b_0000_1111
        );
        assert_eq!(
            u8::from(Buttons {
                button: Unselected,
                direction: Selected,
                down_start: Unselected,
                up_select: Unselected,
                left_b: Selected,
                right_a: Unselected,
            }),
            0b_0010_1101
        );
    }
}
