use crate::io::IO;
use crate::Address;

use std::fmt::{Debug, Formatter};

pub struct Lcd {}

impl IO for Lcd {
    fn read(&self, address: Address) -> u8 {
        println!("Read: {}", address);
        0
    }
    fn write(&mut self, address: Address, data: u8) {
        println!("Write: {}, Data: {}", address, data);
    }
}

impl Debug for Lcd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "Lcd")
    }
}
