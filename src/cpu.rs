use crate::Address;
use core::fmt::Debug;
use std::convert::Into;
use std::default::Default;
use std::fmt::Formatter;

// Bus は Trait にしてDIしたい
// アドレスバスは16bit
// データバスは8bit
pub trait Bus {
    fn read(&self, _address: Address) -> u8 {
        todo!()
    }
    fn write(&mut self, _address: Address, _data: u8) {
        todo!()
    }
}

impl Debug for dyn Bus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Default, Debug)]
struct Flags {
    // 外部クレートを使う場合は bitflags が良さそう
    // 7bit: Zero flag
    // 結果が0のときにセットする
    // jump で利用される
    z: bool,
    // 6bit: Subtraction flag(BCD)
    // BCD Number は 0x00 - 0x99 のこと
    // 1つ前の命令が減算のときにセットする
    // DAA命令でのみ利用される
    n: bool,
    // 5bit: Half Carry flag(BCD)
    // 下位4bitに対する Carry flag
    // DAA命令でのみ利用される
    h: bool,
    // 4bit: Carry flag
    // 8bit加算で0xFF、16bit加算で0xFFFFを超えたとき、減算で0未満のときにセットされる
    // jump といくつかの命令(ADC, SBC, RL, RLAなど)で利用される
    c: bool,
    // 下記は使わない
    unused3: Option<bool>,
    unused2: Option<bool>,
    unused1: Option<bool>,
    unused0: Option<bool>,
}

impl From<u8> for Flags {
    fn from(v: u8) -> Self {
        Self {
            z: ((v & 0b10000000) >> 7) == 0b1,
            n: ((v & 0b01000000) >> 6) == 0b1,
            h: ((v & 0b00100000) >> 5) == 0b1,
            c: ((v & 0b00010000) >> 4) == 0b1,
            unused3: Option::None,
            unused2: Option::None,
            unused1: Option::None,
            unused0: Option::None,
        }
    }
}

impl Into<u8> for Flags {
    fn into(self) -> u8 {
        let mut v;
        if self.z {
            v = 0b10000000;
        } else {
            v = 0b00000000;
        }
        if self.n {
            v |= 0b010000000;
        }
        if self.h {
            v |= 0b001000000;
        }
        if self.c {
            v |= 0b000100000;
        }
        v
    }
}

#[derive(Debug)]
struct Registers {
    // https://w.atwiki.jp/gbspec/pages/34.html
    // 8ビットレジスタは AF、BC、DE、HL の組み合わせで
    // 16 ビットのペアレジスタとしても扱う
    // アキュームレータ
    a: u8,
    // フラグ
    f: Flags,
    // 汎用
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    // スタックポインタ
    sp: u16,
    // プログラムカウンタ
    pc: u16,
}

impl Registers {
    fn new() -> Self {
        Self {
            a: 0x01,
            f: Flags::from(0xB0),
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFF,
            pc: 0x0100,
        }
    }
    fn reset(&mut self) {
        // https://w.atwiki.jp/gbspec/pages/26.html
        self.a = 0x01;
        self.f = Flags::from(0xB0);
        self.b = 0x00;
        self.c = 0x13;
        self.d = 0x00;
        self.e = 0xD8;
        self.h = 0x01;
        self.l = 0x4D;
        self.sp = 0xFFFF;
        self.pc = 0x0100;
    }
}

#[derive(Default, Debug)]
struct InterruptEnables {
    // https://gbdev.io/pandocs/Interrupts.html
    // Bit 4: Joypad   Interrupt Enable (INT $60)  (1=Enable)
    joypad: bool,
    // Bit 3: Serial   Interrupt Enable (INT $58)  (1=Enable)
    serial: bool,
    // Bit 2: Timer    Interrupt Enable (INT $50)  (1=Enable)
    timer: bool,
    // Bit 1: LCD STAT Interrupt Enable (INT $48)  (1=Enable)
    lcd_stat: bool,
    // Bit 0: VBlank   Interrupt Enable (INT $40)  (1=Enable)
    v_blank: bool,
}

#[derive(Default, Debug)]
struct InterruptFlags {
    // https://gbdev.io/pandocs/Interrupts.html
    // Bit 4: Joypad   Interrupt Request (INT $60)  (1=Request)
    joypad: bool,
    // Bit 3: Serial   Interrupt Request (INT $58)  (1=Request)
    serial: bool,
    // Bit 2: Timer    Interrupt Request (INT $50)  (1=Request)
    timer: bool,
    // Bit 1: LCD STAT Interrupt Request (INT $48)  (1=Request)
    lcd_stat: bool,
    // Bit 0: VBlank   Interrupt Request (INT $40)  (1=Request)
    v_blank: bool,
}

#[derive(Debug)]
pub struct CPU {
    registers: Registers,
    // Interrupt Master Enable Flag
    ime: bool,
    // メモリマップの 0xFFFF に対応
    ie: InterruptEnables,
    // メモリマップの 0xFF0F に対応
    ifg: InterruptFlags,
    bus: Box<dyn Bus>,
}

impl CPU {
    pub fn new(bus: Box<dyn Bus>) -> Self {
        Self {
            bus,
            registers: Registers::new(),
            ime: false,
            ie: InterruptEnables::default(),
            ifg: InterruptFlags::default(),
        }
    }
    pub fn tick(&mut self) {
        self.reset();

        println!("{}", self.read(0xFF00)); // P1
        println!("{}", self.read(0xFF01)); // SB
        println!("{}", self.read(0xFF02)); // SC
        println!("{}", self.read(0xFF04)); // DIV
        println!("{}", self.read(0xFF05)); // TIMA
        println!("{}", self.read(0xFF06)); // TMA
        println!("{}", self.read(0xFF07)); // TAC
        println!("{}", self.read(0xFF0F)); // IF
        println!("{}", self.read(0xFF10)); // NR10
        println!("{}", self.read(0xFF11)); // NR11
        println!("{}", self.read(0xFF12)); // NR12
        println!("{}", self.read(0xFF13)); // NR13
        println!("{}", self.read(0xFF14)); // NR14
        println!("{}", self.read(0xFF16)); // NR21
        println!("{}", self.read(0xFF17)); // NR22
        println!("{}", self.read(0xFF18)); // NR23
        println!("{}", self.read(0xFF19)); // NR24
        println!("{}", self.read(0xFF1A)); // NR30
        println!("{}", self.read(0xFF1B)); // NR31
        println!("{}", self.read(0xFF1C)); // NR32
        println!("{}", self.read(0xFF1D)); // NR33
        println!("{}", self.read(0xFF1E)); // NR34
        println!("{}", self.read(0xFF20)); // NR41
        println!("{}", self.read(0xFF21)); // NR42
        println!("{}", self.read(0xFF22)); // NR43
        println!("{}", self.read(0xFF23)); // NR44
        println!("{}", self.read(0xFF24)); // NR50
        println!("{}", self.read(0xFF25)); // NR51
        println!("{}", self.read(0xFF26)); // NR52
        println!("{}", self.read(0xFF40)); // LCDC
        println!("{}", self.read(0xFF41)); // STAT
        println!("{}", self.read(0xFF42)); // SCY
        println!("{}", self.read(0xFF43)); // SCX
        println!("{}", self.read(0xFF44)); // LY
        println!("{}", self.read(0xFF45)); // LYC
        println!("{}", self.read(0xFF46)); // DMA
        println!("{}", self.read(0xFF47)); // BGP
        println!("{}", self.read(0xFF48)); // OBP0
        println!("{}", self.read(0xFF49)); // OBP1
        println!("{}", self.read(0xFF4A)); // WY
        println!("{}", self.read(0xFF4B)); // WX
        println!("{}", self.read(0xFF4D)); // KEY1
        println!("{}", self.read(0xFF4F)); // VBK
        println!("{}", self.read(0xFF51)); // HDMA1
        println!("{}", self.read(0xFF52)); // HDMA2
        println!("{}", self.read(0xFF53)); // HDMA3
        println!("{}", self.read(0xFF54)); // HDMA4
        println!("{}", self.read(0xFF55)); // HDMA5
        println!("{}", self.read(0xFF56)); // RP
        println!("{}", self.read(0xFF68)); // BCPS
        println!("{}", self.read(0xFF69)); // BCPD
        println!("{}", self.read(0xFF6A)); // OCPS
        println!("{}", self.read(0xFF6B)); // OCPD
        println!("{}", self.read(0xFF70)); // SVBK
        println!("{}", self.read(0xFFFF)); // IE
    }
    fn read(&self, address: Address) -> u8 {
        self.bus.read(address)
    }
    fn write(&mut self, address: Address, data: u8) {
        self.bus.write(address, data)
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        self.write(0xFF00, 0xCF); // P1
        self.write(0xFF01, 0x00); // SB
        self.write(0xFF02, 0x7E); // SC
        self.write(0xFF04, 0x18); // DIV
        self.write(0xFF05, 0x00); // TIMA
        self.write(0xFF06, 0x00); // TMA
        self.write(0xFF07, 0xF8); // TAC
        self.write(0xFF0F, 0xE1); // IF
        self.write(0xFF10, 0x80); // NR10
        self.write(0xFF11, 0xBF); // NR11
        self.write(0xFF12, 0xF3); // NR12
        self.write(0xFF13, 0xFF); // NR13
        self.write(0xFF14, 0xBF); // NR14
        self.write(0xFF16, 0x3F); // NR21
        self.write(0xFF17, 0x00); // NR22
        self.write(0xFF18, 0xFF); // NR23
        self.write(0xFF19, 0xBF); // NR24
        self.write(0xFF1A, 0x7F); // NR30
        self.write(0xFF1B, 0xFF); // NR31
        self.write(0xFF1C, 0x9F); // NR32
        self.write(0xFF1D, 0xFF); // NR33
        self.write(0xFF1E, 0xBF); // NR34
        self.write(0xFF20, 0xFF); // NR41
        self.write(0xFF21, 0x00); // NR42
        self.write(0xFF22, 0x00); // NR43
        self.write(0xFF23, 0xBF); // NR44
        self.write(0xFF24, 0x77); // NR50
        self.write(0xFF25, 0xF3); // NR51
        self.write(0xFF26, 0xF1); // NR52
        self.write(0xFF40, 0x91); // LCDC
        self.write(0xFF41, 0x81); // STAT
        self.write(0xFF42, 0x00); // SCY
        self.write(0xFF43, 0x00); // SCX
        self.write(0xFF44, 0x91); // LY
        self.write(0xFF45, 0x00); // LYC
        self.write(0xFF46, 0xFF); // DMA
        self.write(0xFF47, 0xFC); // BGP
        // self.write(0xFF48,  ??7); // OBP0
        // self.write(0xFF49,  ??7); // OBP1
        self.write(0xFF4A, 0x00); // WY
        self.write(0xFF4B, 0x00); // WX
        self.write(0xFF4D, 0xFF); // KEY1
        self.write(0xFF4F, 0xFF); // VBK
        self.write(0xFF51, 0xFF); // HDMA1
        self.write(0xFF52, 0xFF); // HDMA2
        self.write(0xFF53, 0xFF); // HDMA3
        self.write(0xFF54, 0xFF); // HDMA4
        self.write(0xFF55, 0xFF); // HDMA5
        self.write(0xFF56, 0xFF); // RP
        self.write(0xFF68, 0xFF); // BCPS
        self.write(0xFF69, 0xFF); // BCPD
        self.write(0xFF6A, 0xFF); // OCPS
        self.write(0xFF6B, 0xFF); // OCPD
        self.write(0xFF70, 0xFF); // SVBK
        self.write(0xFFFF, 0x00); // IE

        self.registers.reset()
    }
}
