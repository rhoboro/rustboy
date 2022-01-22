use crate::debug_log;
use crate::ppu::{FrameBuffer, LCD};

pub struct Terminal;

impl LCD for Terminal {
    fn draw(&self, frame_buffer: &FrameBuffer) {
        debug_log!("draw");
        // clear
        println!("\x1b[2J");
        for (i, line) in frame_buffer.iter().enumerate() {
            print!("{:03?} ", i);
            for pixel in line {
                print!("{:?}", pixel);
            }
            println!();
        }
    }
}
