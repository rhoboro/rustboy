extern crate core;

pub use mother_board::{run, Config};

#[macro_use]
mod debugger;
mod arithmetic;
mod cartridges;
mod cpu;
mod interruption;
mod io;
mod joypad;
mod lcd;
mod mother_board;
mod ppu;
mod sound;
mod timer;

type Address = u16;
