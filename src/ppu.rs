use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::rc::Weak;
use std::vec::IntoIter;

use crate::arithmetic::{AddSigned, ToSigned};
use crate::io::{Bus, IO};
use crate::Address;

const WHITE: PixelData = PixelData(255, 255, 255, 0);
const LIGHT_GRAY: PixelData = PixelData(170, 170, 170, 0);
const DARK_GRAY: PixelData = PixelData(85, 85, 85, 0);
const BLACK: PixelData = PixelData(0, 0, 0, 0);

const WIDTH_LCD: u16 = 160;
const HEIGHT_LCD: u16 = 144;
const HEIGHT_LCD_MARGIN: u16 = 10;
const WIDTH_TILE: u16 = 8;
const HEIGHT_TILE: u16 = 8;
const WIDTH_BG: u16 = 256;
const HEIGHT_BG: u16 = 256;
const WIDTH_WINDOW: u16 = 256;
const HEIGHT_WINDOW: u16 = 256;
const SCANLINE_CYCLE: u64 = 456;

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum TileDataSelect {
    Method8000,
    Method8800,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TileMapSelect {
    Method9C00,
    Method9800,
}

impl From<TileMapSelect> for Address {
    fn from(v: TileMapSelect) -> Self {
        if v == TileMapSelect::Method9C00 {
            // 0x9C00 - 0x9FFF の1024個
            0x9C00
        } else {
            // 0x9800 - 0x9BFF の1024個
            0x9800
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SpriteSize {
    Normal,
    Tall,
}

impl From<SpriteSize> for u16 {
    fn from(v: SpriteSize) -> Self {
        if v == SpriteSize::Tall {
            16
        } else {
            8
        }
    }
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
    background_priority: bool,
}

// タイルは 8 x 8 ピクセル。1ピクセルは2bitで4色。
// 先頭2バイトがタイル内の一番上の行に相当
// バイトごとのbitの位置が列に相当（0ビット目が一番右）
type Tile = [u8; 8 * 2];

struct TileLine {
    low: u8,
    high: u8,
}

#[derive(Debug, Clone, Copy)]
struct Sprite {
    y_position: u16,
    x_position: u16,
    tile_number: u8,
    flags: u8,
}

impl Sprite {
    pub const BASE_ADDRESS: Address = 0x8000;

    fn oam_scan(oam: &[u8; 4 * 40], ly: u16, lcdc: LcdControl) -> Vec<Sprite> {
        let mut sprite_buffer = Vec::with_capacity(10);
        for sprite_bytes in oam.chunks(4) {
            debug_log!("sprite_bytes: {:?}", sprite_bytes);
            match Sprite::new(sprite_bytes, ly, lcdc) {
                Some(sprite) => {
                    sprite_buffer.push(sprite);
                    if sprite_buffer.len() >= 10 {
                        break;
                    }
                }
                _ => (),
            }
        }
        sprite_buffer
    }
    fn new(bytes: &[u8], ly: u16, lcdc: LcdControl) -> Option<Self> {
        if bytes.len() != 4 {
            return Option::None;
        }
        let sprite = Self {
            y_position: bytes[0].into(),
            x_position: bytes[1].into(),
            tile_number: bytes[2].into(),
            flags: bytes[3].into(),
        };
        if sprite.x_position <= 0 || 168 < sprite.x_position {
            return Option::None;
        }
        if ly + 16 < sprite.y_position {
            return Option::None;
        }
        if ly + 16 >= sprite.y_position + u16::from(lcdc.sprite_size) {
            return Option::None;
        }
        Some(sprite)
    }
    fn tile_address(&self, ly: u16, scy: u16) -> Address {
        let base_address =
            Sprite::BASE_ADDRESS + self.tile_number.to_unsigned_u16().wrapping_mul(16);
        let offset = 2 * ((ly + scy) % HEIGHT_TILE);
        base_address + offset
    }
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

#[derive(Debug, Clone, Copy)]
struct LcdControl {
    lcd_enable: bool,
    window_tile_map_select: TileMapSelect,
    window_enable: bool,
    tile_data_select: TileDataSelect,
    bg_tile_map_select: TileMapSelect,
    sprite_size: SpriteSize,
    sprite_enable: bool,
    bg_win_enable: bool,
}

impl From<u8> for LcdControl {
    fn from(v: u8) -> Self {
        Self {
            lcd_enable: (v & 0b_1000_0000) == 0b_1000_0000,
            window_tile_map_select: if (v & 0b_0100_0000) == 0b_0100_0000 {
                TileMapSelect::Method9C00
            } else {
                TileMapSelect::Method9800
            },
            window_enable: (v & 0b_0010_0000) == 0b_0010_0000,
            tile_data_select: if (v & 0b_0001_0000) == 0b_0001_0000 {
                TileDataSelect::Method8000
            } else {
                TileDataSelect::Method8800
            },
            bg_tile_map_select: if (v & 0b_0000_1000) == 0b_0000_1000 {
                TileMapSelect::Method9C00
            } else {
                TileMapSelect::Method9800
            },
            sprite_size: if (v & 0b_0000_0100) == 0b_0000_0100 {
                SpriteSize::Tall
            } else {
                SpriteSize::Normal
            },
            sprite_enable: (v & 0b_0000_0010) == 0b_0000_0010,
            bg_win_enable: (v & 0b_0000_0001) == 0b_0000_0001,
        }
    }
}

impl From<LcdControl> for u8 {
    fn from(lcdc: LcdControl) -> Self {
        let mut v;
        if lcdc.lcd_enable {
            v = 0b_1000_0000;
        } else {
            v = 0b_0000_0000;
        }
        if lcdc.window_tile_map_select == TileMapSelect::Method9C00 {
            v |= 0b_0100_0000;
        }
        if lcdc.window_enable {
            v |= 0b_0010_0000;
        }
        if lcdc.tile_data_select == TileDataSelect::Method8000 {
            v |= 0b_0001_0000;
        }
        if lcdc.bg_tile_map_select == TileMapSelect::Method9C00 {
            v |= 0b_0000_1000;
        }
        if lcdc.sprite_size == SpriteSize::Tall {
            v |= 0b_0000_0100;
        }
        if lcdc.sprite_enable {
            v |= 0b_0000_0010;
        }
        if lcdc.bg_win_enable {
            v |= 0b_0000_0001;
        }
        v
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
    lcdc: LcdControl,
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

    bus: Weak<RefCell<dyn Bus>>,
}

impl PPU {
    pub fn new(lcd: Box<dyn LCD>, bus: Weak<RefCell<dyn Bus>>) -> Self {
        Self {
            bus,
            lcd,
            clock: 0,
            clock_next_target: SCANLINE_CYCLE,
            frame_buffer: [[WHITE; 160]; 144],
            oam: [0; 4 * 40],
            vram: [0; 8 * 1024],
            lcdc: LcdControl::from(0),
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

    pub fn print_vram(&self) {
        println!("{:?}", self.vram);
    }

    pub fn tick(&mut self, cycle: u8) {
        self.clock += cycle as u64;
        if self.clock_next_target <= self.clock {
            self.clock_next_target += SCANLINE_CYCLE;
            self.scan_line(self.ly);
            self.ly += 1;
            if self.ly == HEIGHT_LCD {
                // V-Blank 割り込み
                let value = self.bus.upgrade().unwrap().borrow().read(0xFF0F) | 0b_0000_0001;
                self.bus.upgrade().unwrap().borrow().write(0xFF0F, value);
            }
            if self.ly >= (HEIGHT_LCD + HEIGHT_LCD_MARGIN) {
                debug_log!("LCD REFRESH!!!");
                self.lcd.draw(&self.frame_buffer);
                self.ly = 0;
            }
        }
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
        if self.ly >= HEIGHT_LCD {
            return;
        }

        // スキャンラインごとのLCDにpushしたピクセル数(0 - 160)
        let mut rx = 0u16;
        loop {
            if rx >= WIDTH_LCD - 1 {
                break;
            }
            // mode 2: OAM Scan
            let sprite_buffer = Sprite::oam_scan(&self.oam, self.ly, self.lcdc);
            debug_log!("sprite_buffer: {:?}", sprite_buffer.len());

            // mode 3: Drawing
            for sprite in &sprite_buffer {
                if sprite.x_position <= rx + 8 {
                    let address = sprite.tile_address(self.ly, self.scy);
                    let low = self.read(address);
                    let high = self.read(address + 1);
                    let tile_line = TileLine { low, high };
                    for color in tile_line {
                        self.fifo_sprite.push_back(Pixel {
                            color,
                            palette: self.obp0,
                            background_priority: (sprite.flags >> 7) == 0b1,
                        });
                    }
                }
            }

            let tile_number = self.fetch_bg_tile_number(ly, rx);
            let tile_data = self.fetch_bg_tile_data(tile_number, ly);
            if self.fifo_background.is_empty() {
                assert_eq!(self.fifo_background.len(), 0);
                self.push_bg_fifo(tile_data);
            }

            if !self.fifo_background.is_empty() {
                // Push Pixel to LCD
                let mut discarded = self.scx % 8;
                while self.fifo_background.len() > 0 {
                    let bg_pixel = self.fifo_background.pop_front().unwrap();
                    let sp_pixel = self.fifo_sprite.pop_front();
                    let pixel = match sp_pixel {
                        Some(sp_pixel) => {
                            if sp_pixel.color == Color::White {
                                bg_pixel
                            } else if sp_pixel.background_priority && bg_pixel.color != Color::White
                            {
                                bg_pixel
                            } else {
                                sp_pixel
                            }
                        }
                        None => bg_pixel,
                    };
                    if discarded > 0 {
                        discarded -= 1;
                        continue;
                    }
                    self.frame_buffer[ly as usize][rx as usize] = pixel.color.to_rgba();
                    rx += 1;
                }
            }
            // mode 0: H-Blank
        }
    }

    fn fetch_bg_tile_number(&self, ly: u16, rx: u16) -> u8 {
        self.read(tile_number_address(
            self.lcdc.bg_tile_map_select.into(),
            ly,
            rx,
            self.scx,
            self.scy,
        ))
    }
    fn fetch_bg_tile_data(&self, tile_number: u8, ly: u16) -> TileLine {
        let address = tile_number_to_address(tile_number, self.lcdc.tile_data_select, ly, self.scy);
        let low = self.read(address);
        let high = self.read(address + 1);
        TileLine { low, high }
    }
    fn push_bg_fifo(&mut self, tile_line: TileLine) {
        // 8画素分のピクセルデータを fifo にいれ、pushしたピクセル数を返す
        for color in tile_line {
            self.fifo_background.push_back(Pixel {
                color,
                palette: self.obp0,
                background_priority: false,
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
                    0xFF40 => self.lcdc.into(),
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
                debug_log!("Write OAM: {:X?}, Data: {}", address, data);
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
                    0xFF40 => self.lcdc = LcdControl::from(data),
                    0xFF41 => self.stat = data,
                    0xFF42 => self.scy = data as u16,
                    0xFF43 => self.scx = data as u16,
                    0xFF44 => self.ly = data as u16,
                    0xFF45 => self.lyc = data,
                    0xFF46 => {
                        debug_log!("Write FF46: 0x{:04X?}", data);
                        // OAM DMA 転送
                        // 転送元: XX00 - XX9F の4バイトx40個を転送。XXは00-F1
                        // 転送元: FE00 - FE9F
                        let src_start = (data as u16) << 8;
                        debug_log!("src_start: 0x{:04X?}", src_start);
                        let src_end = src_start | 0x009F;
                        for a in (src_start..=src_end).step_by(0x1) {
                            let data = self.bus.upgrade().unwrap().borrow().read(a);
                            debug_log!("Write OAM: {:04X?}, Data: {}", a, data);
                            self.oam[(a - src_start) as usize] = data;
                        }
                    }
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
