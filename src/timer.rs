use crate::debug_log;
use crate::io::IO;
use crate::Address;
use std::fmt::{Debug, Formatter};

pub struct Timer {}

impl IO for Timer {
    fn read(&self, address: Address) -> u8 {
        debug_log!("Read Timer: {:X?}", address);
        0
    }
    fn write(&mut self, address: Address, data: u8) {
        debug_log!("Write Timer: {:X?}, Data: {}", address, data);
    }
}

impl Debug for Timer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "Timer")
    }
}
