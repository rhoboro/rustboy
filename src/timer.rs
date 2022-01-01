use crate::io::IO;
use std::fmt::{Debug, Formatter};

pub struct Timer {}

impl IO for Timer {}

impl Debug for Timer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "Timer")
    }
}
