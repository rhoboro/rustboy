use crate::io::IO;
use std::fmt::{Debug, Formatter};

pub struct Sound {}

impl IO for Sound {}

impl Debug for Sound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "Sound")
    }
}
