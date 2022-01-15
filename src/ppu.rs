use crate::io::IO;
use crate::Address;

use std::fmt::{Debug, Formatter};

// RGBA
struct PixelData(u8, u8, u8, u8);

const WHITE: PixelData = PixelData(255, 255, 255, 0);
const LIGHT_GRAY: PixelData = PixelData(170, 170, 170, 0);
const DARK_GRAY: PixelData = PixelData(85, 85, 85, 0);
const BLACK: PixelData = PixelData(0, 0, 0, 0);

const WIDTH_LCD: u16 = 166;
const HEIGHT_LCD: u16 = 144;
const WIDTH_BG: u16 = 256;
const HEIGHT_BG: u16 = 256;
const WIDTH_WINDOW: u16 = 256;
const HEIGHT_WINDOW: u16 = 256;

pub struct PPU {
    frame_buffer: [PixelData; 160 * 144],
    // スプライト属性テーブル (OAM - Object Attribute Memory)
    oam: [u8; 4 * 40],
    vram: [u8; 8 * 1024],
    // レジスタ
    // 0xFF40: LCD制御
    lcdc: u8,
    // 0xFF41: LCDステータス
    stat: u8,
    // 0xFF42: スクロールY座標
    scy: u8,
    // 0xFF43: スクロールX座標
    scx: u8,
    // 0xFF44: LCDC Y座標
    ly: u8,
    // 0xFF45: LY比較
    lyc: u8,
    // 0xFF47: 背景パレットデータ
    bgp: u8,
    // 0xFF48: オブジェクトパレット0
    obp0: u8,
    // 0xFF49: オブジェクトパレット1
    obp1: u8,
    // 0xFF4A: ウィンドウY座標
    wy: u8,
    // 0xFF4B: ウィンドウX座標
    wx: u8,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            frame_buffer: [WHITE; 160 * 144],
            oam: [0; 4 * 40],
            vram: [0; 8 * 1024],
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
        }
    }
}

impl IO for PPU {
    fn read(&self, address: Address) -> u8 {
        println!("Read: {}", address);
        match address {
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                self.vram[(address - 0x8000) as usize]
            }
            0xFF40..=0xFF4B => {
                // レジスタ
                match address {
                    0xFF40 => self.lcdc,
                    0xFF41 => self.stat,
                    0xFF42 => self.scy,
                    0xFF43 => self.scx,
                    0xFF44 => self.ly,
                    0xFF45 => self.lyc,
                    0xFF47 => self.bgp,
                    0xFF48 => self.obp0,
                    0xFF49 => self.obp1,
                    0xFF4A => self.wy,
                    0xFF4B => self.wx,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        println!("Write: {}, Data: {}", address, data);
        match address {
            0xFE00..=0xFE9F => {
                self.oam[(address - 0xFE00) as usize] = data;
            }
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                self.vram[(address - 0x8000) as usize] = data;
            }
            0xFF40..=0xFF4B => {
                // レジスタ
                match address {
                    0xFF40 => self.lcdc = data,
                    0xFF41 => self.stat = data,
                    0xFF42 => self.scy = data,
                    0xFF43 => self.scx = data,
                    0xFF44 => self.ly = data,
                    0xFF45 => self.lyc = data,
                    0xFF47 => self.bgp = data,
                    0xFF48 => self.obp0 = data,
                    0xFF49 => self.obp1 = data,
                    0xFF4A => self.wy = data,
                    0xFF4B => self.wx = data,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Debug for PPU {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "Lcd")
    }
}
