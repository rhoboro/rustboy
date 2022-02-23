#[macro_use]
mod debugger;
mod arithmetic;
mod cartridges;
mod cpu;
mod io;
mod lcd;
mod mother_board;
mod ppu;
mod sound;
mod timer;

type Address = u16;

pub use mother_board::{run, Config};
