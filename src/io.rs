use std::fmt::{Debug, Formatter};

use crate::Address;

pub trait IO {
    fn read(&self, address: Address) -> u8;
    fn write(&mut self, address: Address, data: u8);
}

impl Debug for dyn IO {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IO")
    }
}

// アドレスバスは16bit
// データバスは8bit
pub trait Bus {
    fn read(&self, _address: Address) -> u8;
    fn write(&self, _address: Address, _data: u8);
}

impl Debug for dyn Bus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
