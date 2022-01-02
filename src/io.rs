use crate::Address;
use std::fmt::{Debug, Formatter};

pub trait IO {
    fn read(&self, address: Address) -> u8;
    fn write(&mut self, address: Address, data: u8);
}

impl Debug for dyn IO {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IO")
    }
}
