mod cartridges;
mod cpu;
mod io;
mod mother_board;
mod ppu;
mod sound;
mod timer;

type Address = u16;

pub use mother_board::{run, Config};
