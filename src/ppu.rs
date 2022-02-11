use crate::arithmetic::{AddSigned, ToSigned};
use crate::debug_log;
use crate::io::IO;
use crate::Address;

use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::vec::IntoIter;

const REFRESH_CYCLE: u64 = 70224;
const WHITE: PixelData = PixelData(255, 255, 255, 0);
const LIGHT_GRAY: PixelData = PixelData(170, 170, 170, 0);
const DARK_GRAY: PixelData = PixelData(85, 85, 85, 0);
const BLACK: PixelData = PixelData(0, 0, 0, 0);

const WIDTH_LCD: u16 = 160;
const HEIGHT_LCD: u16 = 144;
const WIDTH_TILE: u16 = 8;
const HEIGHT_TILE: u16 = 8;
const WIDTH_BG: u16 = 256;
const HEIGHT_BG: u16 = 256;
const WIDTH_WINDOW: u16 = 256;
const HEIGHT_WINDOW: u16 = 256;

pub type FrameBuffer = [[PixelData; WIDTH_LCD as usize]; HEIGHT_LCD as usize];
pub trait LCD {
    /// 描画が必要なタイミングで実行される
    fn draw(&self, frame_buffer: &FrameBuffer);
}

// RGBA
#[derive(Clone, Copy)]
pub struct PixelData(pub u8, pub u8, pub u8, pub u8);

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

enum TileDataSelect {
    Method8000,
    Method8800,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy)]
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

struct TileLine {
    low: u8,
    high: u8,
}

impl IntoIterator for TileLine {
    type Item = Color;
    type IntoIter = IntoIter<Color>;

    fn into_iter(self) -> Self::IntoIter {
        let mut v = Vec::with_capacity(8);
        for offset in (0..WIDTH_TILE).rev() {
            let color = match (((self.low >> offset) & 0b1), ((self.high >> offset) & 0b1)) {
                (0, 0) => Color::White,
                (1, 0) => Color::LightGray,
                (0, 1) => Color::DarkGray,
                (1, 1) => Color::Black,
                _ => unreachable!(),
            };
            v.push(color)
        }
        v.into_iter()
    }
}

pub struct PPU {
    lcd: Box<dyn LCD>,
    // ずっと起動していると溢れる
    clock: u64,
    // 70224 T-cycle ごとに1回描画するため、次の描画時の clock を記録する
    clock_next_target: u64,
    // 実際の画面と対応
    frame_buffer: FrameBuffer,
    // スプライト属性テーブル (OAM - Object Attribute Memory)
    oam: [u8; 4 * 40],
    // VRAM は 0x8000 - 0x9FFF の 8KB
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
    // 背景データ(32 x 32タイル)の格納先は2つあり、背景/ウィンドウで利用できる
    // 格納される値はタイルパターンテーブルのタイル番号
    // 0x9800 - 0x9BFF
    // 0x9C00 - 0x9FFF
    vram: [u8; 8 * 1024],
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
    scy: u16,
    // 0xFF43: スクロールX座標
    scx: u16,
    // 0xFF44: LCD Y座標
    // LCDの縦サイズは144だが10本の擬似スキャンラインがあるので 1 - 153 になる
    ly: u16,
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
    pub fn new(lcd: Box<dyn LCD>) -> Self {
        Self {
            lcd,
            clock: 0,
            clock_next_target: REFRESH_CYCLE,
            frame_buffer: [[WHITE; 160]; 144],
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
            fifo_background: VecDeque::with_capacity(WIDTH_TILE as usize),
            fifo_sprite: VecDeque::with_capacity(WIDTH_TILE as usize),
        }
    }

    fn tile_data_select(&self) -> TileDataSelect {
        if (self.lcdc & 0b00010000) != 0 {
            TileDataSelect::Method8000
        } else {
            TileDataSelect::Method8800
        }
    }
    fn background_data_select(&self) -> Address {
        if (self.lcdc & 0b00001000) != 0 {
            // 0x9C00 - 0x9FFF の1024個
            0x9C00
        } else {
            // 0x9800 - 0x9BFF の1024個
            0x9800
        }
    }

    pub fn print_vram(&self) {
        println!("{:?}", self.vram);
    }

    pub fn tick(&mut self, cycle: u8) {
        self.clock += cycle as u64;
        if self.clock_next_target <= self.clock {
            debug_log!("LCD REFRESH!!!");
            self.clock_next_target += REFRESH_CYCLE;
            self.scan_lines();
        }
    }

    fn scan_lines(&mut self) {
        self.ly = 0;
        loop {
            // ly レジスタが現在処理中の行
            // 1画面は154行（LCD144行 + 擬似スキャンライン10行）
            if self.ly >= HEIGHT_LCD {
                break;
            }
            self.scan_line(self.ly);
            self.ly += 1;
        }
        self.lcd.draw(&self.frame_buffer);
    }

    // 1行(= 160 pixel)の描画
    // 1行のスキャンラインは 456 T-Cycle
    // ここでは frame_buffer に書き込む
    fn scan_line(&mut self, ly: u16) {
        // OAMをスキャンしてスプライトバッファに格納
        // 背景描画。Pixel Fetcher が背景FIFOに描画するピクセルを供給し続ける。
        // タイル番号を取得、タイルデータの1バイト目を取得、タイルデータの2バイト目を取得、対応するFIFOにプッシュ
        // FIFOは2つある（背景ウィンドウ用とスプライト用）
        // FIFOが埋まったらLCDにプッシュ。（2つのFIFOをマージすることもある）

        // スキャンラインごとのLCDにpushしたピクセル数(0 - 160)
        let mut rx = 0u16;
        loop {
            if rx >= WIDTH_LCD - 1 {
                break;
            }
            // TODO: OAMスキャン
            let tile_number = self.fetch_tile_number(ly, rx);
            let tile_data = self.fetch_tile_data(tile_number, ly);
            if self.fifo_background.is_empty() {
                assert_eq!(self.fifo_background.len(), 0);
                self.push_fifo(tile_data);
            }
            if !self.fifo_background.is_empty() {
                // Push Pixel to LCD
                let mut discarded = self.scx % 8;
                while self.fifo_background.len() > 0 {
                    let pixel = self.fifo_background.pop_front();
                    if discarded > 0 {
                        discarded -= 1;
                        continue;
                    }
                    self.frame_buffer[ly as usize][rx as usize] = pixel.unwrap().color.to_rgba();
                    rx += 1;
                }
            }
        }
    }

    fn fetch_tile_number(&self, ly: u16, rx: u16) -> u8 {
        self.read(tile_number_address(
            self.background_data_select(),
            ly,
            rx,
            self.scx,
            self.scy,
        ))
    }
    fn fetch_tile_data(&self, tile_number: u8, ly: u16) -> TileLine {
        let address = tile_number_to_address(tile_number, self.tile_data_select(), ly, self.scy);
        let low = self.read(address);
        let high = self.read(address + 1);
        TileLine { low, high }
    }
    fn push_fifo(&mut self, tile_line: TileLine) {
        // 8画素分のピクセルデータを fifo にいれ、pushしたピクセル数を返す
        for color in tile_line {
            self.fifo_background.push_back(Pixel {
                color,
                palette: self.obp0,
                background_priority: 0,
            });
        }
    }
}

impl IO for PPU {
    fn read(&self, address: Address) -> u8 {
        match address {
            0xFE00..=0xFE9F => {
                let data = self.oam[(address - 0xFE00) as usize];
                debug_log!("Read Vram: {:X?}, Data: {}", address, data);
                data
            }
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                let data = self.vram[(address - 0x8000) as usize];
                debug_log!("Read Vram: {:X?}, Data: {}", address, data);
                data
            }
            0xFF40..=0xFF4B => {
                // レジスタ
                match address {
                    0xFF40 => self.lcdc,
                    0xFF41 => self.stat,
                    0xFF42 => self.scy.try_into().unwrap(),
                    0xFF43 => self.scx.try_into().unwrap(),
                    0xFF44 => self.ly.try_into().unwrap(),
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
        match address {
            0xFE00..=0xFE9F => {
                debug_log!("Write Vram: {:X?}, Data: {}", address, data);
                self.oam[(address - 0xFE00) as usize] = data;
            }
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                debug_log!("Write Vram: {:X?}, Data: {}", address, data);
                self.vram[(address - 0x8000) as usize] = data;
            }
            0xFF40..=0xFF4B => {
                // レジスタ
                match address {
                    0xFF40 => self.lcdc = data,
                    0xFF41 => self.stat = data,
                    0xFF42 => self.scy = data as u16,
                    0xFF43 => self.scx = data as u16,
                    0xFF44 => self.ly = data as u16,
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

fn tile_number_address(base_address: Address, ly: u16, rx: u16, scx: u16, scy: u16) -> Address {
    // オフセット計算
    let offset_x = (rx / WIDTH_TILE) + ((scx / WIDTH_TILE) & 0x001F);
    let offset_y = 32 * (((ly + scy) & 0xFF) / HEIGHT_TILE);
    let tile_address = base_address + ((offset_x + offset_y) & 0x03FF);
    tile_address
}

fn tile_number_to_address(tile_number: u8, method: TileDataSelect, ly: u16, scy: u16) -> Address {
    let base_address = match method {
        TileDataSelect::Method8000 => 0x8000 + tile_number.to_unsigned_u16().wrapping_mul(16),
        TileDataSelect::Method8800 => {
            0x9000u16.add_signed_u16(tile_number.to_signed_u16().wrapping_mul(16))
        }
    };
    // 1画素あたり2bitなので1行ズレると2バイト後ろのデータになる
    let offset = 2 * ((ly + scy) % HEIGHT_TILE);
    base_address + offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_number_address() {
        assert_eq!(tile_number_address(0x9800, 0, 0, 0, 0), 0x9800);
        assert_eq!(tile_number_address(0x9800, 1, 1, 0, 0), 0x9800);
        assert_eq!(tile_number_address(0x9800, 8, 1, 0, 0), 0x9820);
        assert_eq!(tile_number_address(0x9800, 20, 2, 0, 0), 0x9840);
        assert_eq!(tile_number_address(0x9800, 144, 0, 0, 0), 0x9A40);
    }

    #[test]
    fn test_tile_data_address_m8800() {
        assert_eq!(
            tile_number_to_address(127, TileDataSelect::Method8800, 0, 0),
            0x97F0,
        );
        assert_eq!(
            tile_number_to_address(0, TileDataSelect::Method8800, 0, 0),
            0x9000
        );
        assert_eq!(
            tile_number_to_address(1, TileDataSelect::Method8800, 0, 0),
            0x9010
        );
        assert_eq!(
            tile_number_to_address(1, TileDataSelect::Method8800, 2, 0),
            0x9014
        );
        assert_eq!(
            tile_number_to_address(32, TileDataSelect::Method8800, 5, 0),
            0x920A
        );
        assert_eq!(
            tile_number_to_address(-1i8 as u8, TileDataSelect::Method8800, 0, 0),
            0x8FF0
        );
        assert_eq!(
            tile_number_to_address(-2i8 as u8, TileDataSelect::Method8800, 0, 0),
            0x8FE0
        );
        assert_eq!(
            tile_number_to_address(-128i8 as u8, TileDataSelect::Method8800, 0, 0),
            0x8800,
        );
    }

    #[test]
    fn test_tile_data_address_m8000() {
        assert_eq!(
            tile_number_to_address(0x00, TileDataSelect::Method8000, 0, 0),
            0x8000
        );
        assert_eq!(
            tile_number_to_address(0x02, TileDataSelect::Method8000, 0, 0),
            0x8020
        );
        assert_eq!(
            tile_number_to_address(0xFF, TileDataSelect::Method8000, 0, 0),
            0x8FF0
        );
    }

    #[test]
    fn test_tile_line() {
        let tl = TileLine {
            low: 0b11001100,
            high: 0b10101010,
        };
        let mut it = tl.into_iter();
        assert_eq!(it.next(), Some(Color::Black));
        assert_eq!(it.next(), Some(Color::LightGray));
        assert_eq!(it.next(), Some(Color::DarkGray));
        assert_eq!(it.next(), Some(Color::White));
        assert_eq!(it.next(), Some(Color::Black));
        assert_eq!(it.next(), Some(Color::LightGray));
        assert_eq!(it.next(), Some(Color::DarkGray));
        assert_eq!(it.next(), Some(Color::White));
        assert_eq!(it.next(), None);
    }
}
