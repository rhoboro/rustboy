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
        let mut buf = String::new();

        // clear
        buf += "\x1b[2J";
        for (i, line) in frame_buffer.iter().enumerate() {
            buf += &format!("{:03?}", i);
            for pixel in line {
                buf += &format!("{:?}", pixel);
            }
            buf += &format!("\n");
        }
        eprintln!("{}", buf);
    }
}

/// 8点点字で標準出力に描画する
pub struct BrailleTerminal {
    brailles: [[u32; 2]; 4],
}

impl BrailleTerminal {
    pub fn new() -> Self {
        BrailleTerminal {
            brailles: [
                // Unicodeの8点点字の配列。配列の添字が点の位置に相当。
                // 下位8bitが点の位置を表し、論理和がとれる。
                // [0x2801, 0x2808],
                // [0x2802, 0x2810],
                // [0x2804, 0x2820],
                // [0x2840, 0x2880],
                ['⠁' as u32, '⠈' as u32],
                ['⠂' as u32, '⠐' as u32],
                ['⠄' as u32, '⠠' as u32],
                ['⡀' as u32, '⢀' as u32],
            ],
        }
    }
}

impl LCD for BrailleTerminal {
    fn draw(&self, frame_buffer: &FrameBuffer) {
        debug_log!("draw");
        // TODO: capacityの指定
        let mut buf = String::new();
        // clear
        buf += "\x1b[2J";
        // 点のない点字で初期化
        let mut line_buffer = [0x2800; 80];
        for (y, line) in frame_buffer.iter().enumerate() {
            for (x, pixel) in line.iter().enumerate() {
                match pixel.bi_color() {
                    BiColor::Black => line_buffer[x / 2] |= self.brailles[y % 4][x % 2],
                    BiColor::White => (),
                }
            }
            if y % 4 == 3 {
                buf += &format!("{:03?}", y - 3);
                for c in line_buffer {
                    buf += &format!("{:}", char::from_u32(c).unwrap());
                }
                buf += &format!("\n");
                line_buffer = [0x2800; 80];
            }
        }
        eprintln!("{}", buf);
    }
}
