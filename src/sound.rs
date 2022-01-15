use crate::io::IO;
use crate::Address;
use std::fmt::{Debug, Formatter};

pub struct Sound {}

impl IO for Sound {
    fn read(&self, address: Address) -> u8 {
        println!("Read: {:X?}", address);
        0
    }
    fn write(&mut self, address: Address, data: u8) {
        println!("Write: {:X?}, Data: {}", address, data);
    }
}

impl Debug for Sound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "Sound")
    }
}
