mod cartridges;
mod cpu;
mod mother_board;
type Address = u16;
pub use mother_board::{run, Config};
