use crate::io::IO;
use crate::Address;
use core::fmt::Debug;
use std::convert::Into;
use std::default::Default;
use std::fmt::Formatter;

// &, + 演算子をサポートする型にデフォルト実装を自動で付与したい
trait ArithmeticUtil<RHS = Self> {
    // TODO: 加算しつつ結果を返したほうが良さそう
    fn calc_half_carry(&self, rhs: RHS) -> bool;
    fn calc_carry(&self, rhs: RHS) -> bool;
    fn calc_half_borrow(&self, rhs: RHS) -> bool;
    fn calc_borrow(&self, rhs: RHS) -> bool;
}

// TODO: 正しいか自信ないのでテスト書く
impl ArithmeticUtil<u8> for u8 {
    fn calc_half_carry(&self, rhs: u8) -> bool {
        ((self & 0x0F) + (rhs & 0x0F)) & 0x10 == 0x10
    }
    fn calc_carry(&self, rhs: u8) -> bool {
        ((*self as u16 & 0x00FF) + (rhs as u16 & 0x00FF)) & 0x0100 == 0x0100
    }
    fn calc_half_borrow(&self, rhs: u8) -> bool {
        (*self & 0x0F) < (rhs & 0x0F)
    }
    fn calc_borrow(&self, rhs: u8) -> bool {
        *self < rhs
    }
}
// TODO: 正しいか自信ないのでテスト書く
impl ArithmeticUtil<u16> for u16 {
    fn calc_half_carry(&self, rhs: u16) -> bool {
        ((self & 0x000F) + (rhs & 0x000F)) & 0x0010 == 0x0010
    }
    fn calc_carry(&self, rhs: u16) -> bool {
        ((*self as u32 & 0x0000FFFF) + (rhs as u32 & 0x0000FFFF)) & 0x00010000 == 0x00010000
    }
    fn calc_half_borrow(&self, rhs: u16) -> bool {
        (*self & 0x00FF) < (rhs & 0x00FF)
    }
    fn calc_borrow(&self, rhs: u16) -> bool {
        *self < rhs
    }
}

// アドレスバスは16bit
// データバスは8bit
pub trait Bus {
    fn read(&self, _address: Address) -> u8;
    fn write(&mut self, _address: Address, _data: u8);
}

impl Debug for dyn Bus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Default, Copy, Clone, Debug)]
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
}

impl From<u8> for Flags {
    fn from(v: u8) -> Self {
        Self {
            z: (v & 0b10000000) == 0b10000000,
            n: (v & 0b01000000) == 0b01000000,
            h: (v & 0b00100000) == 0b00100000,
            c: (v & 0b00010000) == 0b00010000,
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
    sp: Address,
    // プログラムカウンタ
    pc: Address,
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
    fn af(&self) -> u16 {
        let f: u8 = self.f.into();
        ((self.a as u16) << 8) | f as u16
    }
    fn set_af(&mut self, v: u16) {
        self.a = ((v & 0xFF00) >> 8) as u8;
        self.f = Flags::from((v & 0x00FF) as u8);
    }
    fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }
    fn set_bc(&mut self, v: u16) {
        self.b = ((v & 0xFF00) >> 8) as u8;
        self.c = (v & 0x00FF) as u8;
    }
    fn de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }
    fn set_de(&mut self, v: u16) {
        self.d = ((v & 0xFF00) >> 8) as u8;
        self.e = (v & 0x00FF) as u8;
    }
    fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    fn set_hl(&mut self, v: u16) {
        self.h = ((v & 0xFF00) >> 8) as u8;
        self.l = (v & 0x00FF) as u8;
    }
}

#[derive(Default, Copy, Clone, Debug)]
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

impl From<u8> for InterruptEnables {
    fn from(v: u8) -> Self {
        Self {
            joypad: (v & 0b0010000) == 0b0010000,
            serial: (v & 0b0001000) == 0b0001000,
            timer: (v & 0b0000100) == 0b0000100,
            lcd_stat: (v & 0b0000010) == 0b0000010,
            v_blank: (v & 0b0000001) == 0b0000001,
        }
    }
}

impl Into<u8> for InterruptEnables {
    fn into(self) -> u8 {
        let mut v = 0b00000000;
        if self.joypad {
            v |= 0b00010000;
        }
        if self.serial {
            v |= 0b000001000;
        }
        if self.timer {
            v |= 0b000000100;
        }
        if self.lcd_stat {
            v |= 0b000000010;
        }
        if self.v_blank {
            v |= 0b000000001;
        }
        v
    }
}

#[derive(Default, Copy, Clone, Debug)]
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

impl From<u8> for InterruptFlags {
    fn from(v: u8) -> Self {
        Self {
            joypad: (v & 0b0010000) == 0b0010000,
            serial: (v & 0b0001000) == 0b0001000,
            timer: (v & 0b0000100) == 0b0000100,
            lcd_stat: (v & 0b0000010) == 0b0000010,
            v_blank: (v & 0b0000001) == 0b0000001,
        }
    }
}

impl Into<u8> for InterruptFlags {
    fn into(self) -> u8 {
        let mut v = 0b00000000;
        if self.joypad {
            v |= 0b00010000;
        }
        if self.serial {
            v |= 0b000001000;
        }
        if self.timer {
            v |= 0b000000100;
        }
        if self.lcd_stat {
            v |= 0b000000010;
        }
        if self.v_blank {
            v |= 0b000000001;
        }
        v
    }
}

#[derive(Debug)]
pub struct CPU {
    registers: Registers,
    bus: Box<dyn Bus>,
    // Interrupt Master Enable Flag
    ime: bool,

    // 0xFE00 - 0xFE9F スプライト属性テーブル (Object Attribute Memory)
    // oam: Box<dyn IO>,

    // 以下はIOレジスタ
    // 0xFF00 コントロールパッド情報/機種タイプ
    p1: u8,
    // 0xFF01 シリアル通信送受信データ
    sb: u8,
    // 0xFF02 シリアル通信制御
    sc: u8,
    // 0xFF04 ディバイダーレジスタ
    div: u8,
    // 0xFF05 - 0xFF07
    timer: Box<dyn IO>,

    // 0xFF0F 割り込みフラグ
    ifg: InterruptFlags,

    // 0xFF10 - FF3F
    sound: Box<dyn IO>,

    // 0xFF46 DMA(Direct Memory Access)
    dma: u8,

    // 0xFF40 - 0xFF4B
    lcd: Box<dyn IO>,

    // 0xFF80 - 0xFFFE はSPが指すスタック領域
    stack: [u8; 0xFFFE - 0xFF80 + 1],

    // 0xFFFF 割り込みマスク
    ie: InterruptEnables,
}

impl CPU {
    pub fn new(
        bus: Box<dyn Bus>,
        timer: Box<dyn IO>,
        sound: Box<dyn IO>,
        lcd: Box<dyn IO>,
    ) -> Self {
        Self {
            bus,
            lcd,
            timer,
            sound,
            registers: Registers::new(),
            ime: false,
            p1: 0,
            sb: 0,
            sc: 0,

            div: 0,
            ifg: InterruptFlags::default(),
            dma: 0,
            stack: [0; 127],
            ie: InterruptEnables::default(),
        }
    }
    pub fn tick(&mut self) -> Result<u8, &str> {
        // fetch
        let opcode = self.fetch();
        // decode & execute
        if opcode == 0xCB {
            // CBの場合は16bit命令になる
            let opcode = self.fetch();
            self.execute_cb(opcode);
        } else {
            self.execute(opcode);
        }
        println!("{:?}", self.registers);
        Ok(opcode)
    }
    // PCの位置から1バイト読み取り、PCをインクリメントする
    fn fetch(&mut self) -> u8 {
        let byte = self.read(self.registers.pc);
        // opcode実行前にインクリメントしておく
        self.registers.pc += 0x01;
        byte
    }
    // https://gbdev.io/gb-opcodes/optables/
    fn execute(&mut self, opcode: u8) {
        match opcode {
            0x00 => self.nop_0x00(),
            0x01 => self.ld_bc_d16_0x01(),
            0x02 => self.ld_bc_a_0x02(),
            // 0x03 => self.inc_bc_0x03(),
            0x04 => self.inc_b_0x04(),
            0x05 => self.dec_b_0x05(),
            0x06 => self.ld_b_d8_0x06(),
            // 0x07 => self.rlca_0x07(),
            0x08 => self.ld_a16_sp_0x08(),
            // 0x09 => self.add_hl_bc_0x09(),
            0x0A => self.ld_a_bc_0x0a(),
            // 0x0B => self.dec_bc_0x0b(),
            0x0C => self.inc_c_0x0c(),
            0x0D => self.dec_c_0x0d(),
            0x0E => self.ld_c_d8_0x0e(),
            // 0x0F => self.rrca_0x0f(),
            // 0x10 => self.stop_d8_0x10(),
            0x11 => self.ld_de_d16_0x11(),
            0x12 => self.ld_de_a_0x12(),
            // 0x13 => self.inc_de_0x13(),
            0x14 => self.inc_d_0x14(),
            0x15 => self.dec_d_0x15(),
            0x16 => self.ld_d_d8_0x16(),
            // 0x17 => self.rla_0x17(),
            // 0x18 => self.jr_r8_0x18(),
            // 0x19 => self.add_hl_de_0x19(),
            0x1A => self.ld_a_de_0x1a(),
            // 0x1B => self.dec_de_0x1b(),
            0x1C => self.inc_e_0x1c(),
            0x1D => self.dec_e_0x1d(),
            0x1E => self.ld_e_d8_0x1e(),
            // 0x1F => self.rra_0x1f(),
            0x20 => self.jr_nz_r8_0x20(),
            0x21 => self.ld_hl_d16_0x21(),
            0x22 => self.ld_hl_a_0x22(),
            // 0x23 => self.inc_hl_0x23(),
            0x24 => self.inc_h_0x24(),
            0x25 => self.dec_h_0x25(),
            0x26 => self.ld_h_d8_0x26(),
            // 0x27 => self.daa_0x27(),
            0x28 => self.jr_z_r8_0x28(),
            // 0x29 => self.add_hl_hl_0x29(),
            0x2A => self.ld_a_hl_0x2a(),
            // 0x2B => self.dec_hl_0x2b(),
            0x2C => self.inc_l_0x2c(),
            0x2D => self.dec_l_0x2d(),
            0x2E => self.ld_l_d8_0x2e(),
            // 0x2F => self.cpl_0x2f(),
            0x30 => self.jr_nc_r8_0x30(),
            0x31 => self.ld_sp_d16_0x31(),
            0x32 => self.ld_hl_a_0x32(),
            // 0x33 => self.inc_sp_0x33(),
            0x34 => self.inc_hl_0x34(),
            0x35 => self.dec_hl_0x35(),
            0x36 => self.ld_hl_d8_0x36(),
            // 0x37 => self.scf_0x37(),
            0x38 => self.jr_c_r8_0x38(),
            // 0x39 => self.add_hl_sp_0x39(),
            0x3A => self.ld_a_hl_0x3a(),
            // 0x3B => self.dec_sp_0x3b(),
            0x3C => self.inc_a_0x3c(),
            0x3D => self.dec_a_0x3d(),
            0x3E => self.ld_a_d8_0x3e(),
            // 0x3F => self.ccf_0x3f(),
            0x40 => self.ld_b_b_0x40(),
            0x41 => self.ld_b_c_0x41(),
            0x42 => self.ld_b_d_0x42(),
            0x43 => self.ld_b_e_0x43(),
            0x44 => self.ld_b_h_0x44(),
            0x45 => self.ld_b_l_0x45(),
            0x46 => self.ld_b_hl_0x46(),
            0x47 => self.ld_b_a_0x47(),
            0x48 => self.ld_c_b_0x48(),
            0x49 => self.ld_c_c_0x49(),
            0x4A => self.ld_c_d_0x4a(),
            0x4B => self.ld_c_e_0x4b(),
            0x4C => self.ld_c_h_0x4c(),
            0x4D => self.ld_c_l_0x4d(),
            0x4E => self.ld_c_hl_0x4e(),
            0x4F => self.ld_c_a_0x4f(),
            0x50 => self.ld_d_b_0x50(),
            0x51 => self.ld_d_c_0x51(),
            0x52 => self.ld_d_d_0x52(),
            0x53 => self.ld_d_e_0x53(),
            0x54 => self.ld_d_h_0x54(),
            0x55 => self.ld_d_l_0x55(),
            0x56 => self.ld_d_hl_0x56(),
            0x57 => self.ld_d_a_0x57(),
            0x58 => self.ld_e_b_0x58(),
            0x59 => self.ld_e_c_0x59(),
            0x5A => self.ld_e_d_0x5a(),
            0x5B => self.ld_e_e_0x5b(),
            0x5C => self.ld_e_h_0x5c(),
            0x5D => self.ld_e_l_0x5d(),
            0x5E => self.ld_e_hl_0x5e(),
            0x5F => self.ld_e_a_0x5f(),
            0x60 => self.ld_h_b_0x60(),
            0x61 => self.ld_h_c_0x61(),
            0x62 => self.ld_h_d_0x62(),
            0x63 => self.ld_h_e_0x63(),
            0x64 => self.ld_h_h_0x64(),
            0x65 => self.ld_h_l_0x65(),
            0x66 => self.ld_h_hl_0x66(),
            0x67 => self.ld_h_a_0x67(),
            0x68 => self.ld_l_b_0x68(),
            0x69 => self.ld_l_c_0x69(),
            0x6A => self.ld_l_d_0x6a(),
            0x6B => self.ld_l_e_0x6b(),
            0x6C => self.ld_l_h_0x6c(),
            0x6D => self.ld_l_l_0x6d(),
            0x6E => self.ld_l_hl_0x6e(),
            0x6F => self.ld_l_a_0x6f(),
            0x70 => self.ld_hl_b_0x70(),
            0x71 => self.ld_hl_c_0x71(),
            0x72 => self.ld_hl_d_0x72(),
            0x73 => self.ld_hl_e_0x73(),
            0x74 => self.ld_hl_h_0x74(),
            0x75 => self.ld_hl_l_0x75(),
            // 0x76 => self.halt_0x76(),
            0x77 => self.ld_hl_a_0x77(),
            0x78 => self.ld_a_b_0x78(),
            0x79 => self.ld_a_c_0x79(),
            0x7A => self.ld_a_d_0x7a(),
            0x7B => self.ld_a_e_0x7b(),
            0x7C => self.ld_a_h_0x7c(),
            0x7D => self.ld_a_l_0x7d(),
            0x7E => self.ld_a_hl_0x7e(),
            0x7F => self.ld_a_a_0x7f(),
            0x80 => self.add_a_b_0x80(),
            0x81 => self.add_a_c_0x81(),
            0x82 => self.add_a_d_0x82(),
            0x83 => self.add_a_e_0x83(),
            0x84 => self.add_a_h_0x84(),
            0x85 => self.add_a_l_0x85(),
            0x86 => self.add_a_hl_0x86(),
            0x87 => self.add_a_a_0x87(),
            0x88 => self.adc_a_b_0x88(),
            0x89 => self.adc_a_c_0x89(),
            0x8A => self.adc_a_d_0x8a(),
            0x8B => self.adc_a_e_0x8b(),
            0x8C => self.adc_a_h_0x8c(),
            0x8D => self.adc_a_l_0x8d(),
            0x8E => self.adc_a_hl_0x8e(),
            0x8F => self.adc_a_a_0x8f(),
            0x90 => self.sub_b_0x90(),
            0x91 => self.sub_c_0x91(),
            0x92 => self.sub_d_0x92(),
            0x93 => self.sub_e_0x93(),
            0x94 => self.sub_h_0x94(),
            0x95 => self.sub_l_0x95(),
            0x96 => self.sub_hl_0x96(),
            0x97 => self.sub_a_0x97(),
            0x98 => self.sbc_a_b_0x98(),
            0x99 => self.sbc_a_c_0x99(),
            0x9A => self.sbc_a_d_0x9a(),
            0x9B => self.sbc_a_e_0x9b(),
            0x9C => self.sbc_a_h_0x9c(),
            0x9D => self.sbc_a_l_0x9d(),
            0x9E => self.sbc_a_hl_0x9e(),
            0x9F => self.sbc_a_a_0x9f(),
            0xA0 => self.and_b_0xa0(),
            0xA1 => self.and_c_0xa1(),
            0xA2 => self.and_d_0xa2(),
            0xA3 => self.and_e_0xa3(),
            0xA4 => self.and_h_0xa4(),
            0xA5 => self.and_l_0xa5(),
            0xA6 => self.and_hl_0xa6(),
            0xA7 => self.and_a_0xa7(),
            0xA8 => self.xor_b_0xa8(),
            0xA9 => self.xor_c_0xa9(),
            0xAA => self.xor_d_0xaa(),
            0xAB => self.xor_e_0xab(),
            0xAC => self.xor_h_0xac(),
            0xAD => self.xor_l_0xad(),
            0xAE => self.xor_hl_0xae(),
            0xAF => self.xor_a_0xaf(),
            0xB0 => self.or_b_0xb0(),
            0xB1 => self.or_c_0xb1(),
            0xB2 => self.or_d_0xb2(),
            0xB3 => self.or_e_0xb3(),
            0xB4 => self.or_h_0xb4(),
            0xB5 => self.or_l_0xb5(),
            0xB6 => self.or_hl_0xb6(),
            0xB7 => self.or_a_0xb7(),
            0xB8 => self.cp_b_0xb8(),
            0xB9 => self.cp_c_0xb9(),
            0xBA => self.cp_d_0xba(),
            0xBB => self.cp_e_0xbb(),
            0xBC => self.cp_h_0xbc(),
            0xBD => self.cp_l_0xbd(),
            0xBE => self.cp_hl_0xbe(),
            0xBF => self.cp_a_0xbf(),
            // 0xC0 => self.ret_nz_0xc0(),
            0xC1 => self.pop_bc_0xc1(),
            // 0xC2 => self.jp_nz_a16_0xc2(),
            0xC3 => self.jp_a16_0xc3(),
            // 0xC4 => self.call_nz_a16_0xc4(),
            0xC5 => self.push_bc_0xc5(),
            0xC6 => self.add_a_d8_0xc6(),
            // 0xC7 => self.rst_00h_0xc7(),
            // 0xC8 => self.ret_z_0xc8(),
            // 0xC9 => self.ret_0xc9(),
            // 0xCA => self.jp_z_a16_0xca(),
            // 0xCB => self.prefix_0xcb(),
            // 0xCC => self.call_z_a16_0xcc(),
            // 0xCD => self.call_a16_0xcd(),
            0xCE => self.adc_a_d8_0xce(),
            // 0xCF => self.rst_08h_0xcf(),
            // 0xD0 => self.ret_nc_0xd0(),
            0xD1 => self.pop_de_0xd1(),
            // 0xD2 => self.jp_nc_a16_0xd2(),
            // 0xD3 => self.illegal_d3_0xd3(),
            // 0xD4 => self.call_nc_a16_0xd4(),
            0xD5 => self.push_de_0xd5(),
            0xD6 => self.sub_d8_0xd6(),
            // 0xD7 => self.rst_10h_0xd7(),
            // 0xD8 => self.ret_c_0xd8(),
            // 0xD9 => self.reti_0xd9(),
            // 0xDA => self.jp_c_a16_0xda(),
            // 0xDB => self.illegal_db_0xdb(),
            // 0xDC => self.call_c_a16_0xdc(),
            // 0xDD => self.illegal_dd_0xdd(),
            // 0xDE => self.sbc_a_d8_0xde(),
            // 0xDF => self.rst_18h_0xdf(),
            0xE0 => self.ldh_a8_a_0xe0(),
            0xE1 => self.pop_hl_0xe1(),
            0xE2 => self.ld_c_a_0xe2(),
            // 0xE3 => self.illegal_e3_0xe3(),
            // 0xE4 => self.illegal_e4_0xe4(),
            0xE5 => self.push_hl_0xe5(),
            0xE6 => self.and_d8_0xe6(),
            // 0xE7 => self.rst_20h_0xe7(),
            // 0xE8 => self.add_sp_r8_0xe8(),
            // 0xE9 => self.jp_hl_0xe9(),
            0xEA => self.ld_a16_a_0xea(),
            // 0xEB => self.illegal_eb_0xeb(),
            // 0xEC => self.illegal_ec_0xec(),
            // 0xED => self.illegal_ed_0xed(),
            0xEE => self.xor_d8_0xee(),
            // 0xEF => self.rst_28h_0xef(),
            0xF0 => self.ldh_a_a8_0xf0(),
            0xF1 => self.pop_af_0xf1(),
            0xF2 => self.ld_a_c_0xf2(),
            0xF3 => self.di_0xf3(),
            // 0xF4 => self.illegal_f4_0xf4(),
            0xF5 => self.push_af_0xf5(),
            0xF6 => self.or_d8_0xf6(),
            // 0xF7 => self.rst_30h_0xf7(),
            0xF8 => self.ld_hl_sp_r8_0xf8(),
            0xF9 => self.ld_sp_hl_0xf9(),
            0xFA => self.ld_a_a16_0xfa(),
            // 0xFB => self.ei_0xfb(),
            // 0xFC => self.illegal_fc_0xfc(),
            // 0xFD => self.illegal_fd_0xfd(),
            0xFE => self.cp_d8_0xfe(),
            // 0xFF => self.rst_38h_0xff(),
            _ => println!("not implemented opcode: 0x{:x?}", opcode),
        }
    }
    fn execute_cb(&mut self, opcode: u8) {
        match opcode {
            // 0x00 => self.rlc_b_0xcb00(),
            // 0x01 => self.rlc_c_0xcb01(),
            // 0x02 => self.rlc_d_0xcb02(),
            // 0x03 => self.rlc_e_0xcb03(),
            // 0x04 => self.rlc_h_0xcb04(),
            // 0x05 => self.rlc_l_0xcb05(),
            // 0x06 => self.rlc_hl_0xcb06(),
            // 0x07 => self.rlc_a_0xcb07(),
            // 0x08 => self.rrc_b_0xcb08(),
            // 0x09 => self.rrc_c_0xcb09(),
            // 0x0A => self.rrc_d_0xcb0a(),
            // 0x0B => self.rrc_e_0xcb0b(),
            // 0x0C => self.rrc_h_0xcb0c(),
            // 0x0D => self.rrc_l_0xcb0d(),
            // 0x0E => self.rrc_hl_0xcb0e(),
            // 0x0F => self.rrc_a_0xcb0f(),
            // 0x10 => self.rl_b_0xcb10(),
            // 0x11 => self.rl_c_0xcb11(),
            // 0x12 => self.rl_d_0xcb12(),
            // 0x13 => self.rl_e_0xcb13(),
            // 0x14 => self.rl_h_0xcb14(),
            // 0x15 => self.rl_l_0xcb15(),
            // 0x16 => self.rl_hl_0xcb16(),
            // 0x17 => self.rl_a_0xcb17(),
            // 0x18 => self.rr_b_0xcb18(),
            // 0x19 => self.rr_c_0xcb19(),
            // 0x1A => self.rr_d_0xcb1a(),
            // 0x1B => self.rr_e_0xcb1b(),
            // 0x1C => self.rr_h_0xcb1c(),
            // 0x1D => self.rr_l_0xcb1d(),
            // 0x1E => self.rr_hl_0xcb1e(),
            // 0x1F => self.rr_a_0xcb1f(),
            // 0x20 => self.sla_b_0xcb20(),
            // 0x21 => self.sla_c_0xcb21(),
            // 0x22 => self.sla_d_0xcb22(),
            // 0x23 => self.sla_e_0xcb23(),
            // 0x24 => self.sla_h_0xcb24(),
            // 0x25 => self.sla_l_0xcb25(),
            // 0x26 => self.sla_hl_0xcb26(),
            // 0x27 => self.sla_a_0xcb27(),
            // 0x28 => self.sra_b_0xcb28(),
            // 0x29 => self.sra_c_0xcb29(),
            // 0x2A => self.sra_d_0xcb2a(),
            // 0x2B => self.sra_e_0xcb2b(),
            // 0x2C => self.sra_h_0xcb2c(),
            // 0x2D => self.sra_l_0xcb2d(),
            // 0x2E => self.sra_hl_0xcb2e(),
            // 0x2F => self.sra_a_0xcb2f(),
            // 0x30 => self.swap_b_0xcb30(),
            // 0x31 => self.swap_c_0xcb31(),
            // 0x32 => self.swap_d_0xcb32(),
            // 0x33 => self.swap_e_0xcb33(),
            // 0x34 => self.swap_h_0xcb34(),
            // 0x35 => self.swap_l_0xcb35(),
            // 0x36 => self.swap_hl_0xcb36(),
            // 0x37 => self.swap_a_0xcb37(),
            // 0x38 => self.srl_b_0xcb38(),
            // 0x39 => self.srl_c_0xcb39(),
            // 0x3A => self.srl_d_0xcb3a(),
            // 0x3B => self.srl_e_0xcb3b(),
            // 0x3C => self.srl_h_0xcb3c(),
            // 0x3D => self.srl_l_0xcb3d(),
            // 0x3E => self.srl_hl_0xcb3e(),
            // 0x3F => self.srl_a_0xcb3f(),
            // 0x40 => self.bit_0_b_0xcb40(),
            // 0x41 => self.bit_0_c_0xcb41(),
            // 0x42 => self.bit_0_d_0xcb42(),
            // 0x43 => self.bit_0_e_0xcb43(),
            // 0x44 => self.bit_0_h_0xcb44(),
            // 0x45 => self.bit_0_l_0xcb45(),
            // 0x46 => self.bit_0_hl_0xcb46(),
            // 0x47 => self.bit_0_a_0xcb47(),
            // 0x48 => self.bit_1_b_0xcb48(),
            // 0x49 => self.bit_1_c_0xcb49(),
            // 0x4A => self.bit_1_d_0xcb4a(),
            // 0x4B => self.bit_1_e_0xcb4b(),
            // 0x4C => self.bit_1_h_0xcb4c(),
            // 0x4D => self.bit_1_l_0xcb4d(),
            // 0x4E => self.bit_1_hl_0xcb4e(),
            // 0x4F => self.bit_1_a_0xcb4f(),
            // 0x50 => self.bit_2_b_0xcb50(),
            // 0x51 => self.bit_2_c_0xcb51(),
            // 0x52 => self.bit_2_d_0xcb52(),
            // 0x53 => self.bit_2_e_0xcb53(),
            // 0x54 => self.bit_2_h_0xcb54(),
            // 0x55 => self.bit_2_l_0xcb55(),
            // 0x56 => self.bit_2_hl_0xcb56(),
            // 0x57 => self.bit_2_a_0xcb57(),
            // 0x58 => self.bit_3_b_0xcb58(),
            // 0x59 => self.bit_3_c_0xcb59(),
            // 0x5A => self.bit_3_d_0xcb5a(),
            // 0x5B => self.bit_3_e_0xcb5b(),
            // 0x5C => self.bit_3_h_0xcb5c(),
            // 0x5D => self.bit_3_l_0xcb5d(),
            // 0x5E => self.bit_3_hl_0xcb5e(),
            // 0x5F => self.bit_3_a_0xcb5f(),
            // 0x60 => self.bit_4_b_0xcb60(),
            // 0x61 => self.bit_4_c_0xcb61(),
            // 0x62 => self.bit_4_d_0xcb62(),
            // 0x63 => self.bit_4_e_0xcb63(),
            // 0x64 => self.bit_4_h_0xcb64(),
            // 0x65 => self.bit_4_l_0xcb65(),
            // 0x66 => self.bit_4_hl_0xcb66(),
            // 0x67 => self.bit_4_a_0xcb67(),
            // 0x68 => self.bit_5_b_0xcb68(),
            // 0x69 => self.bit_5_c_0xcb69(),
            // 0x6A => self.bit_5_d_0xcb6a(),
            // 0x6B => self.bit_5_e_0xcb6b(),
            // 0x6C => self.bit_5_h_0xcb6c(),
            // 0x6D => self.bit_5_l_0xcb6d(),
            // 0x6E => self.bit_5_hl_0xcb6e(),
            // 0x6F => self.bit_5_a_0xcb6f(),
            // 0x70 => self.bit_6_b_0xcb70(),
            // 0x71 => self.bit_6_c_0xcb71(),
            // 0x72 => self.bit_6_d_0xcb72(),
            // 0x73 => self.bit_6_e_0xcb73(),
            // 0x74 => self.bit_6_h_0xcb74(),
            // 0x75 => self.bit_6_l_0xcb75(),
            // 0x76 => self.bit_6_hl_0xcb76(),
            // 0x77 => self.bit_6_a_0xcb77(),
            // 0x78 => self.bit_7_b_0xcb78(),
            // 0x79 => self.bit_7_c_0xcb79(),
            // 0x7A => self.bit_7_d_0xcb7a(),
            // 0x7B => self.bit_7_e_0xcb7b(),
            // 0x7C => self.bit_7_h_0xcb7c(),
            // 0x7D => self.bit_7_l_0xcb7d(),
            // 0x7E => self.bit_7_hl_0xcb7e(),
            // 0x7F => self.bit_7_a_0xcb7f(),
            // 0x80 => self.res_0_b_0xcb80(),
            // 0x81 => self.res_0_c_0xcb81(),
            // 0x82 => self.res_0_d_0xcb82(),
            // 0x83 => self.res_0_e_0xcb83(),
            // 0x84 => self.res_0_h_0xcb84(),
            // 0x85 => self.res_0_l_0xcb85(),
            // 0x86 => self.res_0_hl_0xcb86(),
            // 0x87 => self.res_0_a_0xcb87(),
            // 0x88 => self.res_1_b_0xcb88(),
            // 0x89 => self.res_1_c_0xcb89(),
            // 0x8A => self.res_1_d_0xcb8a(),
            // 0x8B => self.res_1_e_0xcb8b(),
            // 0x8C => self.res_1_h_0xcb8c(),
            // 0x8D => self.res_1_l_0xcb8d(),
            // 0x8E => self.res_1_hl_0xcb8e(),
            // 0x8F => self.res_1_a_0xcb8f(),
            // 0x90 => self.res_2_b_0xcb90(),
            // 0x91 => self.res_2_c_0xcb91(),
            // 0x92 => self.res_2_d_0xcb92(),
            // 0x93 => self.res_2_e_0xcb93(),
            // 0x94 => self.res_2_h_0xcb94(),
            // 0x95 => self.res_2_l_0xcb95(),
            // 0x96 => self.res_2_hl_0xcb96(),
            // 0x97 => self.res_2_a_0xcb97(),
            // 0x98 => self.res_3_b_0xcb98(),
            // 0x99 => self.res_3_c_0xcb99(),
            // 0x9A => self.res_3_d_0xcb9a(),
            // 0x9B => self.res_3_e_0xcb9b(),
            // 0x9C => self.res_3_h_0xcb9c(),
            // 0x9D => self.res_3_l_0xcb9d(),
            // 0x9E => self.res_3_hl_0xcb9e(),
            // 0x9F => self.res_3_a_0xcb9f(),
            // 0xA0 => self.res_4_b_0xcba0(),
            // 0xA1 => self.res_4_c_0xcba1(),
            // 0xA2 => self.res_4_d_0xcba2(),
            // 0xA3 => self.res_4_e_0xcba3(),
            // 0xA4 => self.res_4_h_0xcba4(),
            // 0xA5 => self.res_4_l_0xcba5(),
            // 0xA6 => self.res_4_hl_0xcba6(),
            // 0xA7 => self.res_4_a_0xcba7(),
            // 0xA8 => self.res_5_b_0xcba8(),
            // 0xA9 => self.res_5_c_0xcba9(),
            // 0xAA => self.res_5_d_0xcbaa(),
            // 0xAB => self.res_5_e_0xcbab(),
            // 0xAC => self.res_5_h_0xcbac(),
            // 0xAD => self.res_5_l_0xcbad(),
            // 0xAE => self.res_5_hl_0xcbae(),
            // 0xAF => self.res_5_a_0xcbaf(),
            // 0xB0 => self.res_6_b_0xcbb0(),
            // 0xB1 => self.res_6_c_0xcbb1(),
            // 0xB2 => self.res_6_d_0xcbb2(),
            // 0xB3 => self.res_6_e_0xcbb3(),
            // 0xB4 => self.res_6_h_0xcbb4(),
            // 0xB5 => self.res_6_l_0xcbb5(),
            // 0xB6 => self.res_6_hl_0xcbb6(),
            // 0xB7 => self.res_6_a_0xcbb7(),
            // 0xB8 => self.res_7_b_0xcbb8(),
            // 0xB9 => self.res_7_c_0xcbb9(),
            // 0xBA => self.res_7_d_0xcbba(),
            // 0xBB => self.res_7_e_0xcbbb(),
            // 0xBC => self.res_7_h_0xcbbc(),
            // 0xBD => self.res_7_l_0xcbbd(),
            // 0xBE => self.res_7_hl_0xcbbe(),
            // 0xBF => self.res_7_a_0xcbbf(),
            // 0xC0 => self.set_0_b_0xcbc0(),
            // 0xC1 => self.set_0_c_0xcbc1(),
            // 0xC2 => self.set_0_d_0xcbc2(),
            // 0xC3 => self.set_0_e_0xcbc3(),
            // 0xC4 => self.set_0_h_0xcbc4(),
            // 0xC5 => self.set_0_l_0xcbc5(),
            // 0xC6 => self.set_0_hl_0xcbc6(),
            // 0xC7 => self.set_0_a_0xcbc7(),
            // 0xC8 => self.set_1_b_0xcbc8(),
            // 0xC9 => self.set_1_c_0xcbc9(),
            // 0xCA => self.set_1_d_0xcbca(),
            // 0xCB => self.set_1_e_0xcbcb(),
            // 0xCC => self.set_1_h_0xcbcc(),
            // 0xCD => self.set_1_l_0xcbcd(),
            // 0xCE => self.set_1_hl_0xcbce(),
            // 0xCF => self.set_1_a_0xcbcf(),
            // 0xD0 => self.set_2_b_0xcbd0(),
            // 0xD1 => self.set_2_c_0xcbd1(),
            // 0xD2 => self.set_2_d_0xcbd2(),
            // 0xD3 => self.set_2_e_0xcbd3(),
            // 0xD4 => self.set_2_h_0xcbd4(),
            // 0xD5 => self.set_2_l_0xcbd5(),
            // 0xD6 => self.set_2_hl_0xcbd6(),
            // 0xD7 => self.set_2_a_0xcbd7(),
            // 0xD8 => self.set_3_b_0xcbd8(),
            // 0xD9 => self.set_3_c_0xcbd9(),
            // 0xDA => self.set_3_d_0xcbda(),
            // 0xDB => self.set_3_e_0xcbdb(),
            // 0xDC => self.set_3_h_0xcbdc(),
            // 0xDD => self.set_3_l_0xcbdd(),
            // 0xDE => self.set_3_hl_0xcbde(),
            // 0xDF => self.set_3_a_0xcbdf(),
            // 0xE0 => self.set_4_b_0xcbe0(),
            // 0xE1 => self.set_4_c_0xcbe1(),
            // 0xE2 => self.set_4_d_0xcbe2(),
            // 0xE3 => self.set_4_e_0xcbe3(),
            // 0xE4 => self.set_4_h_0xcbe4(),
            // 0xE5 => self.set_4_l_0xcbe5(),
            // 0xE6 => self.set_4_hl_0xcbe6(),
            // 0xE7 => self.set_4_a_0xcbe7(),
            // 0xE8 => self.set_5_b_0xcbe8(),
            // 0xE9 => self.set_5_c_0xcbe9(),
            // 0xEA => self.set_5_d_0xcbea(),
            // 0xEB => self.set_5_e_0xcbeb(),
            // 0xEC => self.set_5_h_0xcbec(),
            // 0xED => self.set_5_l_0xcbed(),
            // 0xEE => self.set_5_hl_0xcbee(),
            // 0xEF => self.set_5_a_0xcbef(),
            // 0xF0 => self.set_6_b_0xcbf0(),
            // 0xF1 => self.set_6_c_0xcbf1(),
            // 0xF2 => self.set_6_d_0xcbf2(),
            // 0xF3 => self.set_6_e_0xcbf3(),
            // 0xF4 => self.set_6_h_0xcbf4(),
            // 0xF5 => self.set_6_l_0xcbf5(),
            // 0xF6 => self.set_6_hl_0xcbf6(),
            // 0xF7 => self.set_6_a_0xcbf7(),
            // 0xF8 => self.set_7_b_0xcbf8(),
            // 0xF9 => self.set_7_c_0xcbf9(),
            // 0xFA => self.set_7_d_0xcbfa(),
            // 0xFB => self.set_7_e_0xcbfb(),
            // 0xFC => self.set_7_h_0xcbfc(),
            // 0xFD => self.set_7_l_0xcbfd(),
            // 0xFE => self.set_7_hl_0xcbfe(),
            // 0xFF => self.set_7_a_0xcbff(),
            _ => println!("not implemented opcode: 0xcb{:x?}", opcode),
        }
    }
    fn read(&self, address: Address) -> u8 {
        match address {
            0xFE00..=0xFE9F => {
                // 0xFE00 - 0xFE9F: スプライト属性テーブル (OAM)
                todo!()
            }
            0xFEA0..=0xFEFF => {
                // 0xFEA0 - 0xFEFF: 未使用
                unimplemented!()
            }
            0xFF00..=0xFF7F => {
                // 0xFF00 - 0xFF7F: I/Oレジスタ
                match address {
                    0xFF00 => self.p1,
                    0xFF01 => self.sb,
                    0xFF02 => self.sc,
                    0xFF04 => self.div,
                    0xFF05..=0xFF07 => self.timer.read(address),
                    0xFF0F => self.ifg.into(),
                    0xFF10..=0xFF3F => self.sound.read(address),
                    0xFF46 => self.dma,
                    0xFF40..=0xFF4B => self.lcd.read(address),
                    _ => unreachable!(),
                }
            }
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                todo!()
            }
            0xFFFF => {
                // 0xFFFF - 0xFFFF: 割り込み有効レジスタ
                self.ie.into()
            }
            // 0x0000 - 0xFDFF は ROM/RAM へのアクセス
            _ => self.bus.read(address),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        match address {
            0xFE00..=0xFE9F => {
                // 0xFE00 - 0xFE9F: スプライト属性テーブル (OAM)
                todo!()
            }
            0xFEA0..=0xFEFF => {
                // 0xFEA0 - 0xFEFF: 未使用
                unreachable!()
            }
            0xFF00..=0xFF7F => {
                // 0xFF00 - 0xFF7F: I/Oレジスタ
                match address {
                    0xFF00 => self.p1 = data,
                    0xFF01 => self.sb = data,
                    0xFF02 => self.sc = data,
                    0xFF04 => self.div = data,
                    0xFF05..=0xFF07 => self.timer.write(address, data),
                    0xFF0F => self.ifg = InterruptFlags::from(data),
                    0xFF10..=0xFF3F => self.sound.write(address, data),
                    0xFF46 => self.dma = data,
                    0xFF40..=0xFF4B => self.lcd.write(address, data),
                    _ => unreachable!(),
                }
            }
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                todo!()
            }
            0xFFFF => {
                // 0xFFFF - 0xFFFF: 割り込み有効レジスタ
                self.ie = InterruptEnables::from(data);
            }
            // 0x0000 - 0xFDFF は ROM/RAM へのアクセス
            _ => self.bus.write(address, data),
        }
    }
    pub fn reset(&mut self) {
        println!("Reset");
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
        self.write(0xFF48, 0xFF); // OBP0
        self.write(0xFF49, 0xFF); // OBP1
        self.write(0xFF4A, 0x00); // WY
        self.write(0xFF4B, 0x00); // WX
        self.write(0xFFFF, 0x00); // IE

        self.registers.reset();
    }

    // 以下は opcode と対応
    // bytes: 1 cycles: [4]
    fn nop_0x00(&mut self) {
        println!("NOP");
    }
    // bytes: 3 cycles: [12]
    fn ld_bc_d16_0x01(&mut self) {
        println!("LD BC, d16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let d16 = h << 8 | l;
        self.registers.set_bc(d16);
    }
    // bytes: 1 cycles: [8]
    fn ld_bc_a_0x02(&mut self) {
        println!("LD (BC), A");
        self.write(self.registers.bc(), self.registers.a);
    }
    // bytes: 1 cycles: [8]
    fn inc_bc_0x03(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_b_0x04(&mut self) {
        println!("INC B");
        self.registers.f.h = self.registers.b.calc_half_carry(1);
        self.registers.b += 1;
        self.registers.f.n = false;
        self.registers.f.z = self.registers.b == 0;
    }
    // bytes: 1 cycles: [4]
    fn dec_b_0x05(&mut self) {
        println!("DEC B");
        self.registers.f.h = self.registers.b.calc_half_borrow(1);
        self.registers.b -= 1;
        self.registers.f.n = true;
        self.registers.f.z = self.registers.b == 0;
    }
    // bytes: 2 cycles: [8]
    fn ld_b_d8_0x06(&mut self) {
        println!("LD B, d8");
        let d8 = self.fetch();
        self.registers.b = d8;
    }
    // bytes: 1 cycles: [4]
    fn rlca_0x07(&mut self) {}
    // bytes: 3 cycles: [20]
    fn ld_a16_sp_0x08(&mut self) {
        println!("LD (a16), SP");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        self.write(a16, self.read(self.registers.sp));
    }
    // bytes: 1 cycles: [8]
    fn add_hl_bc_0x09(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_bc_0x0a(&mut self) {
        println!("LD A, (BC)");
        self.registers.a = self.read(self.registers.bc());
    }
    // bytes: 1 cycles: [8]
    fn dec_bc_0x0b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_c_0x0c(&mut self) {
        println!("INC C");
        self.registers.f.h = self.registers.c.calc_half_carry(1);
        self.registers.c += 1;
        self.registers.f.n = false;
        self.registers.f.z = self.registers.c == 0;
    }
    // bytes: 1 cycles: [4]
    fn dec_c_0x0d(&mut self) {
        println!("DEC C");
        self.registers.f.h = self.registers.c.calc_half_borrow(1);
        self.registers.c -= 1;
        self.registers.f.n = true;
        self.registers.f.z = self.registers.c == 0;
    }
    // bytes: 2 cycles: [8]
    fn ld_c_d8_0x0e(&mut self) {
        println!("ld C, d8");
        let d8 = self.fetch();
        self.registers.c = d8;
    }
    // bytes: 1 cycles: [4]
    fn rrca_0x0f(&mut self) {}
    // bytes: 2 cycles: [4]
    fn stop_d8_0x10(&mut self) {}
    // bytes: 3 cycles: [12]
    fn ld_de_d16_0x11(&mut self) {
        println!("ld DE, d16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let d16 = h << 8 | l;
        self.registers.set_de(d16);
    }
    // bytes: 1 cycles: [8]
    fn ld_de_a_0x12(&mut self) {
        println!("LD (DE), A");
        self.write(self.registers.de(), self.registers.a);
    }
    // bytes: 1 cycles: [8]
    fn inc_de_0x13(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_d_0x14(&mut self) {
        println!("INC D");
        self.registers.f.h = self.registers.d.calc_half_carry(1);
        self.registers.d += 1;
        self.registers.f.n = false;
        self.registers.f.z = self.registers.d == 0;
    }
    // bytes: 1 cycles: [4]
    fn dec_d_0x15(&mut self) {
        println!("DEC D");
        self.registers.f.h = self.registers.d.calc_half_borrow(1);
        self.registers.d -= 1;
        self.registers.f.n = true;
        self.registers.f.z = self.registers.d == 0;
    }
    // bytes: 2 cycles: [8]
    fn ld_d_d8_0x16(&mut self) {
        println!("ld D, d8");
        let d8 = self.fetch();
        self.registers.d = d8;
    }
    // bytes: 1 cycles: [4]
    fn rla_0x17(&mut self) {}
    // bytes: 2 cycles: [12]
    fn jr_r8_0x18(&mut self) {}
    // bytes: 1 cycles: [8]
    fn add_hl_de_0x19(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_de_0x1a(&mut self) {
        println!("LD A, (DE)");
        self.registers.a = self.read(self.registers.de());
    }
    // bytes: 1 cycles: [8]
    fn dec_de_0x1b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_e_0x1c(&mut self) {
        println!("INC E");
        self.registers.f.h = self.registers.e.calc_half_carry(1);
        self.registers.e += 1;
        self.registers.f.n = false;
        self.registers.f.z = self.registers.e == 0;
    }
    // bytes: 1 cycles: [4]
    fn dec_e_0x1d(&mut self) {
        println!("DEC E");
        self.registers.f.h = self.registers.e.calc_half_borrow(1);
        self.registers.e -= 1;
        self.registers.f.n = true;
        self.registers.f.z = self.registers.e == 0;
    }
    // bytes: 2 cycles: [8]
    fn ld_e_d8_0x1e(&mut self) {
        println!("ld E, d8");
        let d8 = self.fetch();
        self.registers.e = d8;
    }
    // bytes: 1 cycles: [4]
    fn rra_0x1f(&mut self) {}
    // bytes: 2 cycles: [12, 8]
    fn jr_nz_r8_0x20(&mut self) {
        println!("JR NZ, r8");
        let r8: i8 = self.fetch() as i8;
        if !self.registers.f.z {
            self.registers.pc = self.registers.pc.wrapping_add(r8 as u16);
        }
    }
    // bytes: 3 cycles: [12]
    fn ld_hl_d16_0x21(&mut self) {
        println!("ld HL, d16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let d16 = h << 8 | l;
        self.registers.set_hl(d16);
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_a_0x22(&mut self) {
        println!("LD (HL+), A");
        self.write(self.registers.hl(), self.registers.a);
        self.registers.a = self.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl() + 1);
    }
    // bytes: 1 cycles: [8]
    fn inc_hl_0x23(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_h_0x24(&mut self) {
        println!("INC H");
        self.registers.f.h = self.registers.h.calc_half_carry(1);
        self.registers.h += 1;
        self.registers.f.n = false;
        self.registers.f.z = self.registers.h == 0;
    }
    // bytes: 1 cycles: [4]
    fn dec_h_0x25(&mut self) {
        println!("DEC H");
        self.registers.f.h = self.registers.h.calc_half_borrow(1);
        self.registers.h -= 1;
        self.registers.f.n = true;
        self.registers.f.z = self.registers.h == 0;
    }
    // bytes: 2 cycles: [8]
    fn ld_h_d8_0x26(&mut self) {
        println!("ld H, d8");
        let d8 = self.fetch();
        self.registers.h = d8;
    }
    // bytes: 1 cycles: [4]
    fn daa_0x27(&mut self) {}
    // bytes: 2 cycles: [12, 8]
    fn jr_z_r8_0x28(&mut self) {
        println!("JR Z, r8");
        let r8: i8 = self.fetch() as i8;
        if self.registers.f.z {
            self.registers.pc = self.registers.pc.wrapping_add(r8 as u16);
        }
    }
    // bytes: 1 cycles: [8]
    fn add_hl_hl_0x29(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_hl_0x2a(&mut self) {
        println!("LD A, (HL+)");
        self.registers.a = self.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl() + 1);
    }
    // bytes: 1 cycles: [8]
    fn dec_hl_0x2b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_l_0x2c(&mut self) {
        println!("INC L");
        self.registers.f.h = self.registers.l.calc_half_carry(1);
        self.registers.l += 1;
        self.registers.f.n = false;
        self.registers.f.z = self.registers.l == 0;
    }
    // bytes: 1 cycles: [4]
    fn dec_l_0x2d(&mut self) {
        println!("DEC L");
        self.registers.f.h = self.registers.l.calc_half_borrow(1);
        self.registers.l -= 1;
        self.registers.f.n = true;
        self.registers.f.z = self.registers.l == 0;
    }
    // bytes: 2 cycles: [8]
    fn ld_l_d8_0x2e(&mut self) {
        println!("ld L, d8");
        let d8 = self.fetch();
        self.registers.l = d8;
    }
    // bytes: 1 cycles: [4]
    fn cpl_0x2f(&mut self) {}
    // bytes: 2 cycles: [12, 8]
    fn jr_nc_r8_0x30(&mut self) {
        println!("JR NC, r8");
        let r8: i8 = self.fetch() as i8;
        if !self.registers.f.c {
            self.registers.pc = self.registers.pc.wrapping_add(r8 as u16);
        }
    }
    // bytes: 3 cycles: [12]
    fn ld_sp_d16_0x31(&mut self) {
        println!("ld SP, d16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let d16 = h << 8 | l;
        self.registers.sp = d16;
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_a_0x32(&mut self) {
        println!("LD (HL-), A");
        self.write(self.registers.hl(), self.registers.a);
        self.registers.a = self.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl() - 1);
    }
    // bytes: 1 cycles: [8]
    fn inc_sp_0x33(&mut self) {}
    // bytes: 1 cycles: [12]
    fn inc_hl_0x34(&mut self) {
        println!("INC (HL)");
        let hl = self.read(self.registers.hl());
        self.registers.f.h = hl.calc_half_carry(1);
        self.write(self.registers.hl(), hl + 1);
        self.registers.f.n = false;
        self.registers.f.z = (hl + 1) == 0;
    }
    // bytes: 1 cycles: [12]
    fn dec_hl_0x35(&mut self) {
        println!("DEC (HL)");
        let hl = self.read(self.registers.hl());
        self.registers.f.h = hl.calc_half_borrow(1);
        self.write(self.registers.hl(), hl - 1);
        self.registers.f.n = true;
        self.registers.f.z = (hl - 1) == 0;
    }
    // bytes: 2 cycles: [12]
    fn ld_hl_d8_0x36(&mut self) {
        println!("LD (HL), n");
        let d8 = self.fetch();
        self.write(self.registers.hl(), d8);
    }
    // bytes: 1 cycles: [4]
    fn scf_0x37(&mut self) {}
    // bytes: 2 cycles: [12, 8]
    fn jr_c_r8_0x38(&mut self) {
        println!("JR C, r8");
        let r8: i8 = self.fetch() as i8;
        if self.registers.f.c {
            self.registers.pc = self.registers.pc.wrapping_add(r8 as u16);
        }
    }
    // bytes: 1 cycles: [8]
    fn add_hl_sp_0x39(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_hl_0x3a(&mut self) {
        println!("LD A, (HL-)");
        self.registers.a = self.read(self.registers.hl());
        self.registers.set_hl(self.registers.hl() - 1);
    }
    // bytes: 1 cycles: [8]
    fn dec_sp_0x3b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_a_0x3c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_a_0x3d(&mut self) {
        println!("DEC A");
        self.registers.f.h = self.registers.a.calc_half_borrow(1);
        self.registers.a -= 1;
        self.registers.f.n = true;
        self.registers.f.z = self.registers.a == 0;
    }
    // bytes: 2 cycles: [8]
    fn ld_a_d8_0x3e(&mut self) {
        println!("LD A, d8");
        let d8: u16 = self.fetch().into();
        self.registers.a = self.read(d8);
    }
    // bytes: 1 cycles: [4]
    fn ccf_0x3f(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_b_0x40(&mut self) {
        println!("LD B, B");
        self.registers.b = self.registers.b;
    }
    // bytes: 1 cycles: [4]
    fn ld_b_c_0x41(&mut self) {
        println!("LD B, C");
        self.registers.b = self.registers.c;
    }
    // bytes: 1 cycles: [4]
    fn ld_b_d_0x42(&mut self) {
        println!("LD B, D");
        self.registers.b = self.registers.d;
    }
    // bytes: 1 cycles: [4]
    fn ld_b_e_0x43(&mut self) {
        println!("LD B, E");
        self.registers.b = self.registers.e;
    }
    // bytes: 1 cycles: [4]
    fn ld_b_h_0x44(&mut self) {
        println!("LD B, H");
        self.registers.b = self.registers.h;
    }
    // bytes: 1 cycles: [4]
    fn ld_b_l_0x45(&mut self) {
        println!("LD B, L");
        self.registers.b = self.registers.l;
    }
    // bytes: 1 cycles: [8]
    fn ld_b_hl_0x46(&mut self) {
        println!("LD B, (HL)");
        self.registers.b = self.read(self.registers.hl());
    }
    // bytes: 1 cycles: [4]
    fn ld_b_a_0x47(&mut self) {
        println!("LD B, A");
        self.registers.b = self.registers.a;
    }
    // bytes: 1 cycles: [4]
    fn ld_c_b_0x48(&mut self) {
        println!("LD C, B");
        self.registers.c = self.registers.b;
    }
    // bytes: 1 cycles: [4]
    fn ld_c_c_0x49(&mut self) {
        println!("LD C, C");
        self.registers.c = self.registers.c;
    }
    // bytes: 1 cycles: [4]
    fn ld_c_d_0x4a(&mut self) {
        println!("LD C, D");
        self.registers.c = self.registers.d;
    }
    // bytes: 1 cycles: [4]
    fn ld_c_e_0x4b(&mut self) {
        println!("LD C, E");
        self.registers.c = self.registers.e;
    }
    // bytes: 1 cycles: [4]
    fn ld_c_h_0x4c(&mut self) {
        println!("LD C, H");
        self.registers.c = self.registers.h;
    }
    // bytes: 1 cycles: [4]
    fn ld_c_l_0x4d(&mut self) {
        println!("LD C, B");
        self.registers.c = self.registers.l;
    }
    // bytes: 1 cycles: [8]
    fn ld_c_hl_0x4e(&mut self) {
        println!("LD C, (HL)");
        self.registers.c = self.read(self.registers.hl());
    }
    // bytes: 1 cycles: [4]
    fn ld_c_a_0x4f(&mut self) {
        println!("LD C, A");
        self.registers.c = self.registers.a;
    }
    // bytes: 1 cycles: [4]
    fn ld_d_b_0x50(&mut self) {
        println!("LD D, B");
        self.registers.d = self.registers.b;
    }
    // bytes: 1 cycles: [4]
    fn ld_d_c_0x51(&mut self) {
        println!("LD D, C");
        self.registers.d = self.registers.c;
    }
    // bytes: 1 cycles: [4]
    fn ld_d_d_0x52(&mut self) {
        println!("LD D, D");
        self.registers.d = self.registers.d;
    }
    // bytes: 1 cycles: [4]
    fn ld_d_e_0x53(&mut self) {
        println!("LD D, E");
        self.registers.d = self.registers.e;
    }
    // bytes: 1 cycles: [4]
    fn ld_d_h_0x54(&mut self) {
        println!("LD D, H");
        self.registers.d = self.registers.h;
    }
    // bytes: 1 cycles: [4]
    fn ld_d_l_0x55(&mut self) {
        println!("LD D, L");
        self.registers.d = self.registers.l;
    }
    // bytes: 1 cycles: [8]
    fn ld_d_hl_0x56(&mut self) {
        println!("LD D, (HL)");
        self.registers.d = self.read(self.registers.hl());
    }
    // bytes: 1 cycles: [4]
    fn ld_d_a_0x57(&mut self) {
        println!("LD D, A");
        self.registers.d = self.registers.a;
    }
    // bytes: 1 cycles: [4]
    fn ld_e_b_0x58(&mut self) {
        println!("LD E, B");
        self.registers.e = self.registers.b;
    }
    // bytes: 1 cycles: [4]
    fn ld_e_c_0x59(&mut self) {
        println!("LD E, C");
        self.registers.e = self.registers.c;
    }
    // bytes: 1 cycles: [4]
    fn ld_e_d_0x5a(&mut self) {
        println!("LD E, D");
        self.registers.e = self.registers.d;
    }
    // bytes: 1 cycles: [4]
    fn ld_e_e_0x5b(&mut self) {
        println!("LD E, E");
        self.registers.e = self.registers.e;
    }
    // bytes: 1 cycles: [4]
    fn ld_e_h_0x5c(&mut self) {
        println!("LD E, H");
        self.registers.e = self.registers.h;
    }
    // bytes: 1 cycles: [4]
    fn ld_e_l_0x5d(&mut self) {
        println!("LD E, L");
        self.registers.e = self.registers.l;
    }
    // bytes: 1 cycles: [8]
    fn ld_e_hl_0x5e(&mut self) {
        println!("LD E, (HL)");
        self.registers.e = self.read(self.registers.hl());
    }
    // bytes: 1 cycles: [4]
    fn ld_e_a_0x5f(&mut self) {
        println!("LD E, A");
        self.registers.e = self.registers.a;
    }
    // bytes: 1 cycles: [4]
    fn ld_h_b_0x60(&mut self) {
        println!("LD H, B");
        self.registers.h = self.registers.b;
    }
    // bytes: 1 cycles: [4]
    fn ld_h_c_0x61(&mut self) {
        println!("LD H, C");
        self.registers.h = self.registers.c;
    }
    // bytes: 1 cycles: [4]
    fn ld_h_d_0x62(&mut self) {
        println!("LD H, D");
        self.registers.h = self.registers.d;
    }
    // bytes: 1 cycles: [4]
    fn ld_h_e_0x63(&mut self) {
        println!("LD H, E");
        self.registers.h = self.registers.e;
    }
    // bytes: 1 cycles: [4]
    fn ld_h_h_0x64(&mut self) {
        println!("LD H, H");
        self.registers.h = self.registers.h;
    }
    // bytes: 1 cycles: [4]
    fn ld_h_l_0x65(&mut self) {
        println!("LD H, L");
        self.registers.h = self.registers.l;
    }
    // bytes: 1 cycles: [8]
    fn ld_h_hl_0x66(&mut self) {
        println!("LD H, (HL)");
        self.registers.h = self.read(self.registers.hl());
    }
    // bytes: 1 cycles: [4]
    fn ld_h_a_0x67(&mut self) {
        println!("LD H, A");
        self.registers.h = self.registers.a;
    }
    // bytes: 1 cycles: [4]
    fn ld_l_b_0x68(&mut self) {
        println!("LD L, B");
        self.registers.l = self.registers.b;
    }
    // bytes: 1 cycles: [4]
    fn ld_l_c_0x69(&mut self) {
        println!("LD L, C");
        self.registers.l = self.registers.c;
    }
    // bytes: 1 cycles: [4]
    fn ld_l_d_0x6a(&mut self) {
        println!("LD L, D");
        self.registers.l = self.registers.d;
    }
    // bytes: 1 cycles: [4]
    fn ld_l_e_0x6b(&mut self) {
        println!("LD L, E");
        self.registers.l = self.registers.e;
    }
    // bytes: 1 cycles: [4]
    fn ld_l_h_0x6c(&mut self) {
        println!("LD L, H");
        self.registers.l = self.registers.h;
    }
    // bytes: 1 cycles: [4]
    fn ld_l_l_0x6d(&mut self) {
        println!("LD L, L");
        self.registers.l = self.registers.l;
    }
    // bytes: 1 cycles: [8]
    fn ld_l_hl_0x6e(&mut self) {
        println!("LD L, (HL)");
        self.registers.l = self.read(self.registers.hl());
    }
    // bytes: 1 cycles: [4]
    fn ld_l_a_0x6f(&mut self) {
        println!("LD L, A");
        self.registers.l = self.registers.a;
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_b_0x70(&mut self) {
        println!("LD (HL), B");
        self.write(self.registers.hl(), self.registers.b);
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_c_0x71(&mut self) {
        println!("LD (HL), C");
        self.write(self.registers.hl(), self.registers.c);
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_d_0x72(&mut self) {
        println!("LD (HL), D");
        self.write(self.registers.hl(), self.registers.d);
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_e_0x73(&mut self) {
        println!("LD (HL), E");
        self.write(self.registers.hl(), self.registers.e);
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_h_0x74(&mut self) {
        println!("LD (HL), H");
        self.write(self.registers.hl(), self.registers.h);
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_l_0x75(&mut self) {
        println!("LD (HL), L");
        self.write(self.registers.hl(), self.registers.l);
    }
    // bytes: 1 cycles: [4]
    fn halt_0x76(&mut self) {
        println!("HALT");
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_a_0x77(&mut self) {
        println!("LD (HL), A");
        self.write(self.registers.hl(), self.registers.a);
    }
    // bytes: 1 cycles: [4]
    fn ld_a_b_0x78(&mut self) {
        println!("LD A, B");
        self.registers.a = self.registers.b;
    }
    // bytes: 1 cycles: [4]
    fn ld_a_c_0x79(&mut self) {
        println!("LD A, C");
        self.registers.a = self.registers.c;
    }
    // bytes: 1 cycles: [4]
    fn ld_a_d_0x7a(&mut self) {
        println!("LD A, D");
        self.registers.a = self.registers.d;
    }
    // bytes: 1 cycles: [4]
    fn ld_a_e_0x7b(&mut self) {
        println!("LD A, E");
        self.registers.a = self.registers.e;
    }
    // bytes: 1 cycles: [4]
    fn ld_a_h_0x7c(&mut self) {
        println!("LD A, H");
        self.registers.a = self.registers.h;
    }
    // bytes: 1 cycles: [4]
    fn ld_a_l_0x7d(&mut self) {
        println!("LD A, L");
        self.registers.a = self.registers.l;
    }
    // bytes: 1 cycles: [8]
    fn ld_a_hl_0x7e(&mut self) {
        println!("LD A, (HL)");
        self.registers.a = self.read(self.registers.hl());
    }
    // bytes: 1 cycles: [4]
    fn ld_a_a_0x7f(&mut self) {
        println!("LD A, A");
        self.registers.a = self.registers.a;
    }
    // bytes: 1 cycles: [4]
    fn add_a_b_0x80(&mut self) {
        println!("ADD A, B");
        self.registers.f.h = self.registers.a.calc_half_carry(self.registers.b);
        self.registers.f.c = self.registers.a.calc_carry(self.registers.b);
        self.registers.a += self.registers.b;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn add_a_c_0x81(&mut self) {
        println!("ADD A, C");
        self.registers.f.h = self.registers.a.calc_half_carry(self.registers.c);
        self.registers.f.c = self.registers.a.calc_carry(self.registers.c);
        self.registers.a += self.registers.c;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn add_a_d_0x82(&mut self) {
        println!("ADD A, D");
        self.registers.f.h = self.registers.a.calc_half_carry(self.registers.d);
        self.registers.f.c = self.registers.a.calc_carry(self.registers.d);
        self.registers.a += self.registers.d;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn add_a_e_0x83(&mut self) {
        println!("ADD A, E");
        self.registers.f.h = self.registers.a.calc_half_carry(self.registers.e);
        self.registers.f.c = self.registers.a.calc_carry(self.registers.e);
        self.registers.a += self.registers.e;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn add_a_h_0x84(&mut self) {
        println!("ADD A, H");
        self.registers.f.h = self.registers.a.calc_half_carry(self.registers.h);
        self.registers.f.c = self.registers.a.calc_carry(self.registers.h);
        self.registers.a += self.registers.h;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn add_a_l_0x85(&mut self) {
        println!("ADD A, L");
        self.registers.f.h = self.registers.a.calc_half_carry(self.registers.l);
        self.registers.f.c = self.registers.a.calc_carry(self.registers.l);
        self.registers.a += self.registers.l;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [8]
    fn add_a_hl_0x86(&mut self) {
        println!("ADD A, (HL)");
        let hl = self.read(self.registers.hl());
        self.registers.f.h = self.registers.a.calc_half_carry(hl);
        self.registers.f.c = self.registers.a.calc_carry(hl);
        self.registers.a += hl;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn add_a_a_0x87(&mut self) {
        println!("ADD A, A");
        self.registers.f.h = self.registers.a.calc_half_carry(self.registers.a);
        self.registers.f.c = self.registers.a.calc_carry(self.registers.a);
        self.registers.a += self.registers.a;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn adc_a_b_0x88(&mut self) {
        println!("ADC A, B");
        let rhs: u8 = self.registers.b + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn adc_a_c_0x89(&mut self) {
        println!("ADC A, C");
        let rhs: u8 = self.registers.c + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn adc_a_d_0x8a(&mut self) {
        println!("ADC A, D");
        let rhs: u8 = self.registers.d + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn adc_a_e_0x8b(&mut self) {
        println!("ADC A, E");
        let rhs: u8 = self.registers.e + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn adc_a_h_0x8c(&mut self) {
        println!("ADC A, H");
        let rhs: u8 = self.registers.h + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn adc_a_l_0x8d(&mut self) {
        println!("ADC A, L");
        let rhs: u8 = self.registers.l + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [8]
    fn adc_a_hl_0x8e(&mut self) {
        println!("ADC A, (HL)");
        let rhs: u8 = self.read(self.registers.hl()) + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn adc_a_a_0x8f(&mut self) {
        println!("ADC A, A");
        let rhs: u8 = self.registers.a + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sub_b_0x90(&mut self) {
        println!("SUB B");
        self.registers.f.h = self.registers.a.calc_half_borrow(self.registers.b);
        self.registers.f.c = self.registers.a.calc_borrow(self.registers.b);
        self.registers.a -= self.registers.b;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sub_c_0x91(&mut self) {
        println!("SUB C");
        self.registers.f.h = self.registers.a.calc_half_borrow(self.registers.c);
        self.registers.f.c = self.registers.a.calc_borrow(self.registers.c);
        self.registers.a -= self.registers.c;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sub_d_0x92(&mut self) {
        println!("SUB D");
        self.registers.f.h = self.registers.a.calc_half_borrow(self.registers.d);
        self.registers.f.c = self.registers.a.calc_borrow(self.registers.d);
        self.registers.a -= self.registers.d;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sub_e_0x93(&mut self) {
        println!("SUB E");
        self.registers.f.h = self.registers.a.calc_half_borrow(self.registers.e);
        self.registers.f.c = self.registers.a.calc_borrow(self.registers.e);
        self.registers.a -= self.registers.e;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sub_h_0x94(&mut self) {
        println!("SUB H");
        self.registers.f.h = self.registers.a.calc_half_borrow(self.registers.h);
        self.registers.f.c = self.registers.a.calc_borrow(self.registers.h);
        self.registers.a -= self.registers.h;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sub_l_0x95(&mut self) {
        println!("SUB L");
        self.registers.f.h = self.registers.a.calc_half_borrow(self.registers.l);
        self.registers.f.c = self.registers.a.calc_borrow(self.registers.l);
        self.registers.a -= self.registers.l;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [8]
    fn sub_hl_0x96(&mut self) {
        println!("SUB (HL)");
        let hl = self.read(self.registers.hl());
        self.registers.f.h = self.registers.a.calc_half_borrow(hl);
        self.registers.f.c = self.registers.a.calc_borrow(hl);
        self.registers.a -= hl;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sub_a_0x97(&mut self) {
        println!("SUB A");
        self.registers.f.h = self.registers.a.calc_half_borrow(self.registers.a);
        self.registers.f.c = self.registers.a.calc_borrow(self.registers.a);
        self.registers.a -= self.registers.a;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sbc_a_b_0x98(&mut self) {
        println!("SUB A, B");
        let rhs: u8 = self.registers.b + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sbc_a_c_0x99(&mut self) {
        println!("SUB A, C");
        let rhs: u8 = self.registers.c + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sbc_a_d_0x9a(&mut self) {
        println!("SUB A, D");
        let rhs: u8 = self.registers.d + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sbc_a_e_0x9b(&mut self) {
        println!("SUB A, E");
        let rhs: u8 = self.registers.e + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sbc_a_h_0x9c(&mut self) {
        println!("SUB A, H");
        let rhs: u8 = self.registers.h + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sbc_a_l_0x9d(&mut self) {
        println!("SUB A, L");
        let rhs: u8 = self.registers.l + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [8]
    fn sbc_a_hl_0x9e(&mut self) {
        println!("SUB A, (HL)");
        let rhs: u8 = self.read(self.registers.hl()) + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn sbc_a_a_0x9f(&mut self) {
        println!("SUB A, A");
        let rhs: u8 = self.registers.a + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a -= rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [4]
    fn and_b_0xa0(&mut self) {
        println!("AND B");
        self.registers.a &= self.registers.b;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn and_c_0xa1(&mut self) {
        println!("AND C");
        self.registers.a &= self.registers.c;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn and_d_0xa2(&mut self) {
        println!("AND D");
        self.registers.a &= self.registers.d;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn and_e_0xa3(&mut self) {
        println!("AND E");
        self.registers.a &= self.registers.e;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn and_h_0xa4(&mut self) {
        println!("AND H");
        self.registers.a &= self.registers.h;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn and_l_0xa5(&mut self) {
        println!("AND L");
        self.registers.a &= self.registers.l;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [8]
    fn and_hl_0xa6(&mut self) {
        println!("AND (HL)");
        self.registers.a &= self.read(self.registers.hl());
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn and_a_0xa7(&mut self) {
        println!("AND A");
        self.registers.a &= self.registers.a;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn xor_b_0xa8(&mut self) {
        println!("XOR B");
        self.registers.a ^= self.registers.b;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn xor_c_0xa9(&mut self) {
        println!("XOR C");
        self.registers.a ^= self.registers.c;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn xor_d_0xaa(&mut self) {
        println!("XOR D");
        self.registers.a ^= self.registers.d;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn xor_e_0xab(&mut self) {
        println!("XOR E");
        self.registers.a ^= self.registers.e;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn xor_h_0xac(&mut self) {
        println!("XOR H");
        self.registers.a ^= self.registers.h;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn xor_l_0xad(&mut self) {
        println!("XOR L");
        self.registers.a ^= self.registers.l;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [8]
    fn xor_hl_0xae(&mut self) {
        println!("XOR (HL)");
        self.registers.a ^= self.read(self.registers.hl());
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn xor_a_0xaf(&mut self) {
        println!("XOR A");
        self.registers.a ^= self.registers.a;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn or_b_0xb0(&mut self) {
        println!("OR B");
        self.registers.a |= self.registers.b;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn or_c_0xb1(&mut self) {
        println!("OR C");
        self.registers.a |= self.registers.c;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn or_d_0xb2(&mut self) {
        println!("OR D");
        self.registers.a |= self.registers.d;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn or_e_0xb3(&mut self) {
        println!("OR E");
        self.registers.a |= self.registers.e;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn or_h_0xb4(&mut self) {
        println!("OR H");
        self.registers.a |= self.registers.h;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn or_l_0xb5(&mut self) {
        println!("OR L");
        self.registers.a |= self.registers.l;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [8]
    fn or_hl_0xb6(&mut self) {
        println!("OR (HL)");
        self.registers.a |= self.read(self.registers.hl());
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn or_a_0xb7(&mut self) {
        println!("OR A");
        self.registers.a |= self.registers.a;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [4]
    fn cp_b_0xb8(&mut self) {
        println!("CP B");
        let rhs = self.registers.b;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [4]
    fn cp_c_0xb9(&mut self) {
        println!("CP C");
        let rhs = self.registers.c;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [4]
    fn cp_d_0xba(&mut self) {
        println!("CP D");
        let rhs = self.registers.d;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [4]
    fn cp_e_0xbb(&mut self) {
        println!("CP E");
        let rhs = self.registers.e;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [4]
    fn cp_h_0xbc(&mut self) {
        println!("CP H");
        let rhs = self.registers.h;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [4]
    fn cp_l_0xbd(&mut self) {
        println!("CP L");
        let rhs = self.registers.l;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [8]
    fn cp_hl_0xbe(&mut self) {
        println!("CP (HL)");
        let rhs = self.read(self.registers.hl());
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [4]
    fn cp_a_0xbf(&mut self) {
        println!("CP A");
        let rhs = self.registers.a;
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [20, 8]
    fn ret_nz_0xc0(&mut self) {}
    // bytes: 1 cycles: [12]
    fn pop_bc_0xc1(&mut self) {
        println!("POP BC");
        self.registers.b = self.read(self.registers.sp + 1);
        self.registers.c = self.read(self.registers.sp);
        self.registers.sp += 2;
    }
    // bytes: 3 cycles: [16, 12]
    fn jp_nz_a16_0xc2(&mut self) {
        println!("JP NZ, a16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        if !self.registers.f.z {
            self.registers.pc = a16;
        }
    }
    // bytes: 3 cycles: [16]
    fn jp_a16_0xc3(&mut self) {
        println!("JP a16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        self.registers.pc = a16;
    }
    // bytes: 3 cycles: [24, 12]
    fn call_nz_a16_0xc4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_bc_0xc5(&mut self) {
        println!("PUSH BC");
        self.write(self.registers.sp - 1, self.registers.b);
        self.write(self.registers.sp - 2, self.registers.c);
        self.registers.sp -= 2;
    }
    // bytes: 2 cycles: [8]
    fn add_a_d8_0xc6(&mut self) {
        println!("ADD A, d8");
        let d8: u8 = self.fetch();
        self.registers.f.h = self.registers.a.calc_half_carry(d8);
        self.registers.f.c = self.registers.a.calc_carry(d8);
        self.registers.a += d8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [16]
    fn rst_00h_0xc7(&mut self) {}
    // bytes: 1 cycles: [20, 8]
    fn ret_z_0xc8(&mut self) {}
    // bytes: 1 cycles: [16]
    fn ret_0xc9(&mut self) {}
    // bytes: 3 cycles: [16, 12]
    fn jp_z_a16_0xca(&mut self) {
        println!("JP Z, a16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        if self.registers.f.z {
            self.registers.pc = a16;
        }
    }
    // bytes: 1 cycles: [4]
    fn prefix_0xcb(&mut self) {}
    // bytes: 3 cycles: [24, 12]
    fn call_z_a16_0xcc(&mut self) {}
    // bytes: 3 cycles: [24]
    fn call_a16_0xcd(&mut self) {}
    // bytes: 2 cycles: [8]
    fn adc_a_d8_0xce(&mut self) {
        println!("ADC A, d8");
        let rhs: u8 = self.fetch() + self.registers.f.c as u8;
        self.registers.f.h = self.registers.a.calc_half_carry(rhs);
        self.registers.f.c = self.registers.a.calc_carry(rhs);
        self.registers.a += rhs;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [16]
    fn rst_08h_0xcf(&mut self) {}
    // bytes: 1 cycles: [20, 8]
    fn ret_nc_0xd0(&mut self) {}
    // bytes: 1 cycles: [12]
    fn pop_de_0xd1(&mut self) {
        println!("POP DE");
        self.registers.d = self.read(self.registers.sp + 1);
        self.registers.e = self.read(self.registers.sp);
        self.registers.sp += 2;
    }
    // bytes: 3 cycles: [16, 12]
    fn jp_nc_a16_0xd2(&mut self) {
        println!("JP NC, a16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        if !self.registers.f.c {
            self.registers.pc = a16;
        }
    }
    // bytes: 1 cycles: [4]
    fn illegal_d3_0xd3(&mut self) {}
    // bytes: 3 cycles: [24, 12]
    fn call_nc_a16_0xd4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_de_0xd5(&mut self) {
        println!("PUSH DE");
        self.write(self.registers.sp - 1, self.registers.d);
        self.write(self.registers.sp - 2, self.registers.e);
        self.registers.sp -= 2;
    }
    // bytes: 2 cycles: [8]
    fn sub_d8_0xd6(&mut self) {
        println!("SUB d8");
        let d8 = self.fetch();
        self.registers.f.h = self.registers.a.calc_half_borrow(d8);
        self.registers.f.c = self.registers.a.calc_borrow(d8);
        self.registers.a -= d8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
    }
    // bytes: 1 cycles: [16]
    fn rst_10h_0xd7(&mut self) {}
    // bytes: 1 cycles: [20, 8]
    fn ret_c_0xd8(&mut self) {}
    // bytes: 1 cycles: [16]
    fn reti_0xd9(&mut self) {}
    // bytes: 3 cycles: [16, 12]
    fn jp_c_a16_0xda(&mut self) {
        println!("JP C, a16");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        if self.registers.f.c {
            self.registers.pc = a16;
        }
    }
    // bytes: 1 cycles: [4]
    fn illegal_db_0xdb(&mut self) {}
    // bytes: 3 cycles: [24, 12]
    fn call_c_a16_0xdc(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_dd_0xdd(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sbc_a_d8_0xde(&mut self) {}
    // bytes: 1 cycles: [16]
    fn rst_18h_0xdf(&mut self) {}
    // bytes: 2 cycles: [12]
    fn ldh_a8_a_0xe0(&mut self) {
        println!("LD (a8), A");
        let a8: u16 = self.fetch().into();
        self.write(0xFF00 + a8, self.registers.a);
    }
    // bytes: 1 cycles: [12]
    fn pop_hl_0xe1(&mut self) {
        println!("POP HL");
        self.registers.h = self.read(self.registers.sp + 1);
        self.registers.l = self.read(self.registers.sp);
        self.registers.sp += 2;
    }
    // bytes: 1 cycles: [8]
    fn ld_c_a_0xe2(&mut self) {
        println!("LD (C), A");
        self.write(0xFF00 + self.registers.c as u16, self.registers.a);
    }
    // bytes: 1 cycles: [4]
    fn illegal_e3_0xe3(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_e4_0xe4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_hl_0xe5(&mut self) {
        println!("PUSH HL");
        self.write(self.registers.sp - 1, self.registers.h);
        self.write(self.registers.sp - 2, self.registers.l);
        self.registers.sp -= 2;
    }
    // bytes: 2 cycles: [8]
    fn and_d8_0xe6(&mut self) {
        println!("AND d8");
        self.registers.a &= self.fetch();
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [16]
    fn rst_20h_0xe7(&mut self) {}
    // bytes: 2 cycles: [16]
    fn add_sp_r8_0xe8(&mut self) {}
    // bytes: 1 cycles: [4]
    fn jp_hl_0xe9(&mut self) {}
    // bytes: 3 cycles: [16]
    fn ld_a16_a_0xea(&mut self) {
        println!("LD (a16), A");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        self.write(a16, self.registers.a);
    }
    // bytes: 1 cycles: [4]
    fn illegal_eb_0xeb(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_ec_0xec(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_ed_0xed(&mut self) {}
    // bytes: 2 cycles: [8]
    fn xor_d8_0xee(&mut self) {
        println!("XOR d8");
        self.registers.a ^= self.fetch();
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [16]
    fn rst_28h_0xef(&mut self) {}
    // bytes: 2 cycles: [12]
    fn ldh_a_a8_0xf0(&mut self) {
        println!("LDH A, (a8)");
        let a8: u16 = self.fetch().into();
        self.registers.a = self.read(a8 + 0xFF00);
    }
    // bytes: 1 cycles: [12]
    fn pop_af_0xf1(&mut self) {
        println!("POP AF");
        self.registers.a = self.read(self.registers.sp + 1);
        self.registers.f = Flags::from(self.read(self.registers.sp));
        self.registers.sp += 2;
    }
    // bytes: 1 cycles: [8]
    fn ld_a_c_0xf2(&mut self) {
        println!("LD A, (C)");
        self.registers.a = self.read(0xFF00 + self.registers.c as u16);
    }
    // bytes: 1 cycles: [4]
    fn di_0xf3(&mut self) {
        println!("DI");
        self.ime = false;
    }
    // bytes: 1 cycles: [4]
    fn illegal_f4_0xf4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_af_0xf5(&mut self) {
        println!("PUSH AF");
        self.write(self.registers.sp - 1, self.registers.a);
        self.write(self.registers.sp - 2, self.registers.f.into());
        self.registers.sp -= 2;
    }
    // bytes: 2 cycles: [8]
    fn or_d8_0xf6(&mut self) {
        println!("OR d8");
        self.registers.a |= self.fetch();
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
    // bytes: 1 cycles: [16]
    fn rst_30h_0xf7(&mut self) {}
    // bytes: 2 cycles: [12]
    fn ld_hl_sp_r8_0xf8(&mut self) {
        println!("LD HL, SP+r8");
        let r8: i8 = self.fetch() as i8;
        self.registers
            .set_hl(self.registers.sp.wrapping_add(r8 as u16));
        self.registers.f.z = false;
        self.registers.f.n = false;
        self.registers.f.h = self.registers.sp.calc_half_carry(r8 as u16);
        self.registers.f.c = self.registers.sp.calc_carry(r8 as u16);
    }
    // bytes: 1 cycles: [8]
    fn ld_sp_hl_0xf9(&mut self) {
        println!("LD SP, HL");
        self.registers.sp = self.registers.hl();
    }
    // bytes: 3 cycles: [16]
    fn ld_a_a16_0xfa(&mut self) {
        println!("LD A, (a16)");
        let l: u16 = self.fetch().into();
        let h: u16 = self.fetch().into();
        let a16 = h << 8 | l;
        self.registers.a = self.read(a16);
    }
    // bytes: 1 cycles: [4]
    fn ei_0xfb(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_fc_0xfc(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_fd_0xfd(&mut self) {}
    // bytes: 2 cycles: [8]
    fn cp_d8_0xfe(&mut self) {
        println!("CP d8");
        let rhs = self.fetch();
        self.registers.f.h = self.registers.a.calc_half_borrow(rhs);
        self.registers.f.c = self.registers.a.calc_borrow(rhs);
        self.registers.a = (self.registers.a > rhs) as u8;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = true;
    }
    // bytes: 1 cycles: [16]
    fn rst_38h_0xff(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_b_0xcb00(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_c_0xcb01(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_d_0xcb02(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_e_0xcb03(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_h_0xcb04(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_l_0xcb05(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rlc_hl_0xcb06(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_a_0xcb07(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_b_0xcb08(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_c_0xcb09(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_d_0xcb0a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_e_0xcb0b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_h_0xcb0c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_l_0xcb0d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rrc_hl_0xcb0e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_a_0xcb0f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_b_0xcb10(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_c_0xcb11(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_d_0xcb12(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_e_0xcb13(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_h_0xcb14(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_l_0xcb15(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rl_hl_0xcb16(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_a_0xcb17(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_b_0xcb18(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_c_0xcb19(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_d_0xcb1a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_e_0xcb1b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_h_0xcb1c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_l_0xcb1d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rr_hl_0xcb1e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_a_0xcb1f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_b_0xcb20(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_c_0xcb21(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_d_0xcb22(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_e_0xcb23(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_h_0xcb24(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_l_0xcb25(&mut self) {}
    // bytes: 2 cycles: [16]
    fn sla_hl_0xcb26(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_a_0xcb27(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_b_0xcb28(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_c_0xcb29(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_d_0xcb2a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_e_0xcb2b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_h_0xcb2c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_l_0xcb2d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn sra_hl_0xcb2e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_a_0xcb2f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_b_0xcb30(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_c_0xcb31(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_d_0xcb32(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_e_0xcb33(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_h_0xcb34(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_l_0xcb35(&mut self) {}
    // bytes: 2 cycles: [16]
    fn swap_hl_0xcb36(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_a_0xcb37(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_b_0xcb38(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_c_0xcb39(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_d_0xcb3a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_e_0xcb3b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_h_0xcb3c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_l_0xcb3d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn srl_hl_0xcb3e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_a_0xcb3f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_b_0xcb40(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_c_0xcb41(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_d_0xcb42(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_e_0xcb43(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_h_0xcb44(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_l_0xcb45(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_0_hl_0xcb46(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_a_0xcb47(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_b_0xcb48(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_c_0xcb49(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_d_0xcb4a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_e_0xcb4b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_h_0xcb4c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_l_0xcb4d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_1_hl_0xcb4e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_a_0xcb4f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_b_0xcb50(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_c_0xcb51(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_d_0xcb52(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_e_0xcb53(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_h_0xcb54(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_l_0xcb55(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_2_hl_0xcb56(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_a_0xcb57(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_b_0xcb58(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_c_0xcb59(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_d_0xcb5a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_e_0xcb5b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_h_0xcb5c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_l_0xcb5d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_3_hl_0xcb5e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_a_0xcb5f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_b_0xcb60(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_c_0xcb61(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_d_0xcb62(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_e_0xcb63(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_h_0xcb64(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_l_0xcb65(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_4_hl_0xcb66(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_a_0xcb67(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_b_0xcb68(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_c_0xcb69(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_d_0xcb6a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_e_0xcb6b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_h_0xcb6c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_l_0xcb6d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_5_hl_0xcb6e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_a_0xcb6f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_b_0xcb70(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_c_0xcb71(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_d_0xcb72(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_e_0xcb73(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_h_0xcb74(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_l_0xcb75(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_6_hl_0xcb76(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_a_0xcb77(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_b_0xcb78(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_c_0xcb79(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_d_0xcb7a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_e_0xcb7b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_h_0xcb7c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_l_0xcb7d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_7_hl_0xcb7e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_a_0xcb7f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_b_0xcb80(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_c_0xcb81(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_d_0xcb82(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_e_0xcb83(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_h_0xcb84(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_l_0xcb85(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_0_hl_0xcb86(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_a_0xcb87(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_b_0xcb88(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_c_0xcb89(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_d_0xcb8a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_e_0xcb8b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_h_0xcb8c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_l_0xcb8d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_1_hl_0xcb8e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_a_0xcb8f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_b_0xcb90(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_c_0xcb91(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_d_0xcb92(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_e_0xcb93(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_h_0xcb94(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_l_0xcb95(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_2_hl_0xcb96(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_a_0xcb97(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_b_0xcb98(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_c_0xcb99(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_d_0xcb9a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_e_0xcb9b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_h_0xcb9c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_l_0xcb9d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_3_hl_0xcb9e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_a_0xcb9f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_b_0xcba0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_c_0xcba1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_d_0xcba2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_e_0xcba3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_h_0xcba4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_l_0xcba5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_4_hl_0xcba6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_a_0xcba7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_b_0xcba8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_c_0xcba9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_d_0xcbaa(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_e_0xcbab(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_h_0xcbac(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_l_0xcbad(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_5_hl_0xcbae(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_a_0xcbaf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_b_0xcbb0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_c_0xcbb1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_d_0xcbb2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_e_0xcbb3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_h_0xcbb4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_l_0xcbb5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_6_hl_0xcbb6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_a_0xcbb7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_b_0xcbb8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_c_0xcbb9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_d_0xcbba(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_e_0xcbbb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_h_0xcbbc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_l_0xcbbd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_7_hl_0xcbbe(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_a_0xcbbf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_b_0xcbc0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_c_0xcbc1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_d_0xcbc2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_e_0xcbc3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_h_0xcbc4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_l_0xcbc5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_0_hl_0xcbc6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_a_0xcbc7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_b_0xcbc8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_c_0xcbc9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_d_0xcbca(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_e_0xcbcb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_h_0xcbcc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_l_0xcbcd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_1_hl_0xcbce(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_a_0xcbcf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_b_0xcbd0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_c_0xcbd1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_d_0xcbd2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_e_0xcbd3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_h_0xcbd4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_l_0xcbd5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_2_hl_0xcbd6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_a_0xcbd7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_b_0xcbd8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_c_0xcbd9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_d_0xcbda(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_e_0xcbdb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_h_0xcbdc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_l_0xcbdd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_3_hl_0xcbde(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_a_0xcbdf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_b_0xcbe0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_c_0xcbe1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_d_0xcbe2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_e_0xcbe3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_h_0xcbe4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_l_0xcbe5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_4_hl_0xcbe6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_a_0xcbe7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_b_0xcbe8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_c_0xcbe9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_d_0xcbea(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_e_0xcbeb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_h_0xcbec(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_l_0xcbed(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_5_hl_0xcbee(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_a_0xcbef(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_b_0xcbf0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_c_0xcbf1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_d_0xcbf2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_e_0xcbf3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_h_0xcbf4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_l_0xcbf5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_6_hl_0xcbf6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_a_0xcbf7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_b_0xcbf8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_c_0xcbf9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_d_0xcbfa(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_e_0xcbfb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_h_0xcbfc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_l_0xcbfd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_7_hl_0xcbfe(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_a_0xcbff(&mut self) {}
}
