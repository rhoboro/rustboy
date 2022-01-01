use crate::io::IO;

use std::fmt::{Debug, Formatter};

pub struct Lcd {}

impl IO for Lcd {}

impl Debug for Lcd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "Lcd")
    }
}
