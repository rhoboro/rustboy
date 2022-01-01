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
        // バンクの切り替え
        println!("{}", self.read(0x4500));
        println!("{}", self.read(0x4501));
        println!("{}", self.read(0x4502));
        self.write(0x2000, 0x01);
        println!("{}", self.read(0x4500));
        println!("{}", self.read(0x4501));
        println!("{}", self.read(0x4502));
        self.write(0x2000, 0x02);
        println!("{}", self.read(0x4500));
        println!("{}", self.read(0x4501));
        println!("{}", self.read(0x4502));
        self.write(0x2000, 0x03);
        println!("{}", self.read(0x4500));
        println!("{}", self.read(0x4501));
        println!("{}", self.read(0x4502));
        self.write(0x2000, 0x01);
        println!("{}", self.read(0x4500));
        println!("{}", self.read(0x4501));
        println!("{}", self.read(0x4502));
    }
    fn read(&self, address: Address) -> u8 {
        self.bus.read(address)
    }
    fn write(&mut self, address: Address, data: u8) {
        self.bus.write(address, data)
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        self.registers.reset()
    }
}
