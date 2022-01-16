use crate::io::IO;
use crate::Address;

use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};

pub trait LCD {
    fn draw(&mut self, frame_buffer: &[PixelData; 160 * 144]);
}

struct Terminal;

impl LCD for Terminal {
    fn draw(&mut self, frame_buffer: &[PixelData; 160 * 144]) {
        for y in 0..144 {
            for x in 0..160 {
                print!("{:?}", frame_buffer[x + (y * 160)]);
            }
            println!();
        }
    }
}

// RGBA
struct PixelData(u8, u8, u8, u8);

impl Debug for PixelData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

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

enum PPUMode {
    // Drawing後に 456 T-Cycles になるよう調整するための待機
    HBlank,
    // 擬似スキャンラインのスキャン中の待機。擬似スキャンラインも456 T-Cycles消費する
    VBlank,
    // OAMから描画するスプライトをスプライトバッファに格納
    OAMScan,
    // LCDにピクセルを転送
    Drawing,
}

enum Color {
    // 00
    White,
    // 10
    LightGray,
    // 01
    DarkGray,
    // 11
    Black,
}

impl Color {
    fn to_rgba(&self) -> PixelData {
        match self {
            Color::White => WHITE,
            Color::LightGray => LIGHT_GRAY,
            Color::DarkGray => DARK_GRAY,
            Color::Black => BLACK,
        }
    }
}

struct Pixel {
    // パレット適用前の値
    color: Color,
    // obp0 / obp1
    palette: u8,
    // 未使用
    // sprite: u8,
    // スプライトの bit7 の値
    background_priority: u8,
}

// タイルは 8 x 8 ピクセル。1ピクセルは2bitで4色。
// 先頭2バイトがタイル内の一番上の行に相当
// バイトごとのbitの位置が列に相当（0ビット目が一番右）
type Tile = [u8; 8 * 2];

struct VRAM {
    // タイルパターンテーブル
    // 0x8000 - 0x97FF
    // 下記の2パターンでアクセスされる
    // Method 8000(スプライトと背景で利用)
    // 0x8000 - 0x8FFF: ここのタイルは 0(0x8000) ~ 255(0x8FFF) のタイル番号で指定
    // 0x8000: 0
    // 0x8010: 1
    // 0x8020: 2
    // ...
    // 0x8FFF: 255
    //
    // Method 8800(背景とウインドウで利用)
    // 0x8800 - 0x97FF: ここのタイルは-128(0x8800) ~ 0(0x9000)~ 127(0x97FF) タイル番号で指定
    // 0x8800: -128
    // ...
    // 0x8FF0: -1
    // 0x9000: 0
    // 0x9010: 1
    // ...
    // 0x97FF: 127
    // TODO: 16 はハードコードせずに Tile から取得したい
    tile_pattern: [Tile; ((0x97FF + 1) - 0x8000) / 16],

    // 背景データ(32 x 32タイル)の格納先は2つあり、背景/ウィンドウで利用できる
    // 格納される値はタイルパターンテーブルのタイル番号
    // 0x9800 - 0x9BFF
    // 0x9C00 - 0x9FFF
    background_map_1: [u8; 32 * 32],
    background_map_2: [i8; 32 * 32],
}

pub struct PPU {
    lcd: Box<dyn LCD>,
    // 実際の画面と対応
    frame_buffer: [PixelData; 160 * 144],
    // スプライト属性テーブル (OAM - Object Attribute Memory)
    oam: [u8; 4 * 40],
    // TODO: VRAM 構造体にしたい
    vram: [u8; 8 * 1024],
    // スキャンラインごとのpushしたピクセル(0 - 160)
    x_position_counter: u16,
    // 8画素分の背景用FIFO
    fifo_background: VecDeque<Pixel>,
    // 8画素分のスプライト用FIFO(スプライトバッファ)
    fifo_sprite: VecDeque<Pixel>,

    // 以下はレジスタ
    // 0xFF40: LCD制御
    // 5bit: 1ならウィンドウも描画, 0ならウィンドウは無視
    // 4bit: 1なら Method 8000, 0なら Method 8800
    // 3bit: 1なら 0x9C00 - 0x9FFF, 0なら 0x9800 - 0x9BFF にある背景データを使う
    lcdc: u8,
    // 0xFF41: LCDステータス
    stat: u8,
    // 0xFF42: スクロールY座標
    scy: u8,
    // 0xFF43: スクロールX座標
    scx: u8,
    // 0xFF44: LCD Y座標
    // LCDの縦サイズは144だが10本の擬似スキャンラインがあるので 1 - 153 になる
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
            lcd: Box::new(Terminal {}),
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
            x_position_counter: 0,
            fifo_background: VecDeque::with_capacity(8),
            fifo_sprite: VecDeque::with_capacity(8),
        }
    }

    pub fn tick(&mut self, cycle: u8) {
        println!("{}", cycle);
    }

    fn scan_lines(&mut self) {
        loop {
            // ly レジスタが現在処理中の行
            // 1画面は154行（LCD144行 + 擬似スキャンライン10行）
            if self.ly >= 154 {
                break;
            }
            self.scan_line();
        }
    }

    // 1行(= 160 pixel)の描画
    // 1行のスキャンラインは 456 T-Cycle
    // ここでは frame_buffer に書き込む
    fn scan_line(&mut self) {
        // OAMをスキャンしてスプライトバッファに格納
        // 背景描画。Pixel Fetcher が背景FIFOに描画するピクセルを供給し続ける。
        // タイル番号を取得、タイルデータの1バイト目を取得、タイルデータの2バイト目を取得、対応するFIFOにプッシュ
        // FIFOは2つある（背景ウィンドウ用とスプライト用）
        // FIFOが埋まったらLCDにプッシュ。（2つのFIFOをマージすることもある）
        loop {
            if self.x_position_counter >= 159 {
                self.x_position_counter = 0;
                break;
            }
            // TODO: OAMスキャン
            let tile_number = self.fetch_tile_number();
            let tile_data = self.fetch_tile_data(tile_number);
            if self.fifo_background.is_empty() {
                self.push_fifo(tile_data);
                if !self.fifo_background.is_empty() {
                    // push Pixel to LCD
                    let offset = (160 * (self.ly as u16) + (8 * self.x_position_counter)) as usize;
                    for i in 0..=7 {
                        let pixel = self.fifo_background.pop_front();
                        self.frame_buffer[offset + i] = pixel.unwrap().color.to_rgba();
                    }
                    if offset + 7 == self.frame_buffer.len() - 1 {
                        self.lcd.draw(&self.frame_buffer);
                    }
                }
            }
        }
    }

    fn fetch_tile_number(&self) -> u8 {
        // ウィンドウは無視
        let base_address: u16 = if (self.lcdc & 0b00000100) != 0 {
            0x9C00
        } else {
            0x9800
        };
        // オフセット計算
        let mut tile_address = base_address + self.x_position_counter;
        tile_address += ((self.scx / 8) & 0x1F) as u16;
        tile_address += (32 * ((self.ly + self.scy) & 0xFF) / 8) as u16;
        self.read(tile_address)
    }
    fn fetch_tile_data(&self, tile_number: u8) -> (u8, u8) {
        let offset = 2 * ((self.ly + self.scy) % 8);
        let address = if (self.lcdc & 0b00010000) != 0 {
            0x8000 + ((tile_number as u16) * 16) + (offset as u16)
        } else {
            let base: u16 = 0x9000;
            ((base as i16) + ((tile_number as i16) * 16)) as u16 + (offset as u16)
        };
        let low = self.read(address);
        let high = self.read(address + 1);
        (low, high)
    }
    fn push_fifo(&mut self, pixel: (u8, u8)) {
        // fifoにいれる
        for offset in (0..=7).rev() {
            let color = match (((pixel.0 >> offset) & 0b1), ((pixel.1 >> offset) & 0b1)) {
                (0, 0) => Color::White,
                (1, 0) => Color::LightGray,
                (0, 1) => Color::DarkGray,
                (1, 1) => Color::Black,
                _ => unreachable!(),
            };
            self.fifo_background.push_back(Pixel {
                color,
                palette: self.obp0,
                background_priority: 0,
            })
        }
        self.x_position_counter += 1;
    }
}

impl IO for PPU {
    fn read(&self, address: Address) -> u8 {
        println!("Read: {:X?}", address);
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
        println!("Write: {:X?}, Data: {}", address, data);
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
