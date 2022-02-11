use crate::debug_log;
use crate::ppu::{FrameBuffer, PixelData, LCD};
use std::fmt::{Debug, Formatter};

impl Debug for PixelData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelData(255, 255, 255, 0) => write!(f, " "),
            PixelData(170, 170, 170, 0) => write!(f, "B"),
            PixelData(85, 85, 85, 0) => write!(f, "C"),
            PixelData(0, 0, 0, 0) => write!(f, "D"),
            _ => write!(f, ""),
        }
    }
}

enum BiColor {
    White = 0,
    Black = 1,
}

trait ToBiColor {
    // 白黒化
    fn bi_color(&self) -> BiColor;
}

impl ToBiColor for PixelData {
    fn bi_color(&self) -> BiColor {
        match self {
            PixelData(255, 255, 255, 0) => BiColor::White,
            PixelData(170, 170, 170, 0) => BiColor::White,
            PixelData(85, 85, 85, 0) => BiColor::Black,
            PixelData(0, 0, 0, 0) => BiColor::Black,
            _ => BiColor::White,
        }
    }
}

pub struct Terminal;

impl Terminal {
    pub fn new() -> Self {
        Terminal {}
    }
}

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

pub struct BrailleTerminal {
    // 8点点字の配列。配列の添字が点の位置に相当する。
    brailles: [[u32; 2]; 4],
}

impl BrailleTerminal {
    pub fn new() -> Self {
        BrailleTerminal {
            brailles: [
                // [
                //     char::from_u32(0x2801).unwrap(),
                //     char::from_u32(0x2808).unwrap(),
                // ],
                // [
                //     char::from_u32(0x2802).unwrap(),
                //     char::from_u32(0x2810).unwrap(),
                // ],
                // [
                //     char::from_u32(0x2804).unwrap(),
                //     char::from_u32(0x2820).unwrap(),
                // ],
                // [
                //     char::from_u32(0x2840).unwrap(),
                //     char::from_u32(0x2880).unwrap(),
                // ],
                [0x2801, 0x2808],
                [0x2802, 0x2810],
                [0x2804, 0x2820],
                [0x2840, 0x2880],
            ],
        }
    }
}

impl LCD for BrailleTerminal {
    fn draw(&self, frame_buffer: &FrameBuffer) {
        debug_log!("draw");
        // clear
        println!("\x1b[2J");
        let mut line_buffer = [0x2800; 80];
        for (y, line) in frame_buffer.iter().enumerate() {
            for (x, pixel) in line.iter().enumerate() {
                match pixel.bi_color() {
                    BiColor::Black => line_buffer[x / 2] |= self.brailles[y % 4][x % 2],
                    _ => (),
                }
            }
            if y % 4 == 3 {
                print!("{:03?} ", y - 3);
                for c in line_buffer {
                    print!("{:}", char::from_u32(c).unwrap());
                }
                println!();
                line_buffer = [0x2800; 80];
            }
        }
    }
}
