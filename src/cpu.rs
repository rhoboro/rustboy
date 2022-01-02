use crate::io::IO;
use crate::Address;
use core::fmt::Debug;
use std::convert::Into;
use std::default::Default;
use std::fmt::Formatter;

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
            joypad: ((v & 0b0010000) >> 4) == 0b1,
            serial: ((v & 0b0001000) >> 3) == 0b1,
            timer: ((v & 0b0000100) >> 2) == 0b1,
            lcd_stat: ((v & 0b0000010) >> 1) == 0b1,
            v_blank: (v & 0b0000001) == 0b1,
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
            joypad: ((v & 0b0010000) >> 4) == 0b1,
            serial: ((v & 0b0001000) >> 3) == 0b1,
            timer: ((v & 0b0000100) >> 2) == 0b1,
            lcd_stat: ((v & 0b0000010) >> 1) == 0b1,
            v_blank: (v & 0b0000001) == 0b1,
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
        Ok(opcode)
    }
    fn fetch(&mut self) -> u8 {
        let opcode = self.read(self.registers.pc);
        // PCのインクリメントはopcode実行よりも前
        self.registers.pc += 0x01;
        opcode
    }
    fn execute(&mut self, opcode: u8) {
        match opcode {
            0x00 => self.nop_0x00(),
            0x01 => self.ld_bc_d16_0x01(),
            0x02 => self.ld_bc_a_0x02(),
            0x03 => self.inc_bc_0x03(),
            0x04 => self.inc_b_0x04(),
            0x05 => self.dec_b_0x05(),
            0x06 => self.ld_b_d8_0x06(),
            0x07 => self.rlca_0x07(),
            0x08 => self.ld_a16_sp_0x08(),
            0x09 => self.add_hl_bc_0x09(),
            0x0A => self.ld_a_bc_0x0a(),
            0x0B => self.dec_bc_0x0b(),
            0x0C => self.inc_c_0x0c(),
            0x0D => self.dec_c_0x0d(),
            0x0E => self.ld_c_d8_0x0e(),
            0x0F => self.rrca_0x0f(),
            0x10 => self.stop_d8_0x10(),
            0x11 => self.ld_de_d16_0x11(),
            0x12 => self.ld_de_a_0x12(),
            0x13 => self.inc_de_0x13(),
            0x14 => self.inc_d_0x14(),
            0x15 => self.dec_d_0x15(),
            0x16 => self.ld_d_d8_0x16(),
            0x17 => self.rla_0x17(),
            0x18 => self.jr_r8_0x18(),
            0x19 => self.add_hl_de_0x19(),
            0x1A => self.ld_a_de_0x1a(),
            0x1B => self.dec_de_0x1b(),
            0x1C => self.inc_e_0x1c(),
            0x1D => self.dec_e_0x1d(),
            0x1E => self.ld_e_d8_0x1e(),
            0x1F => self.rra_0x1f(),
            0x20 => self.jr_nz_r8_0x20(),
            0x21 => self.ld_hl_d16_0x21(),
            0x22 => self.ld_hl_a_0x22(),
            0x23 => self.inc_hl_0x23(),
            0x24 => self.inc_h_0x24(),
            0x25 => self.dec_h_0x25(),
            0x26 => self.ld_h_d8_0x26(),
            0x27 => self.daa_0x27(),
            0x28 => self.jr_z_r8_0x28(),
            0x29 => self.add_hl_hl_0x29(),
            0x2A => self.ld_a_hl_0x2a(),
            0x2B => self.dec_hl_0x2b(),
            0x2C => self.inc_l_0x2c(),
            0x2D => self.dec_l_0x2d(),
            0x2E => self.ld_l_d8_0x2e(),
            0x2F => self.cpl_0x2f(),
            0x30 => self.jr_nc_r8_0x30(),
            0x31 => self.ld_sp_d16_0x31(),
            0x32 => self.ld_hl_a_0x32(),
            0x33 => self.inc_sp_0x33(),
            0x34 => self.inc_hl_0x34(),
            0x35 => self.dec_hl_0x35(),
            0x36 => self.ld_hl_d8_0x36(),
            0x37 => self.scf_0x37(),
            0x38 => self.jr_c_r8_0x38(),
            0x39 => self.add_hl_sp_0x39(),
            0x3A => self.ld_a_hl_0x3a(),
            0x3B => self.dec_sp_0x3b(),
            0x3C => self.inc_a_0x3c(),
            0x3D => self.dec_a_0x3d(),
            0x3E => self.ld_a_d8_0x3e(),
            0x3F => self.ccf_0x3f(),
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
            0x76 => self.halt_0x76(),
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
            0xC0 => self.ret_nz_0xc0(),
            0xC1 => self.pop_bc_0xc1(),
            0xC2 => self.jp_nz_a16_0xc2(),
            0xC3 => self.jp_a16_0xc3(),
            0xC4 => self.call_nz_a16_0xc4(),
            0xC5 => self.push_bc_0xc5(),
            0xC6 => self.add_a_d8_0xc6(),
            0xC7 => self.rst_00h_0xc7(),
            0xC8 => self.ret_z_0xc8(),
            0xC9 => self.ret_0xc9(),
            0xCA => self.jp_z_a16_0xca(),
            0xCB => self.prefix_0xcb(),
            0xCC => self.call_z_a16_0xcc(),
            0xCD => self.call_a16_0xcd(),
            0xCE => self.adc_a_d8_0xce(),
            0xCF => self.rst_08h_0xcf(),
            0xD0 => self.ret_nc_0xd0(),
            0xD1 => self.pop_de_0xd1(),
            0xD2 => self.jp_nc_a16_0xd2(),
            0xD3 => self.illegal_d3_0xd3(),
            0xD4 => self.call_nc_a16_0xd4(),
            0xD5 => self.push_de_0xd5(),
            0xD6 => self.sub_d8_0xd6(),
            0xD7 => self.rst_10h_0xd7(),
            0xD8 => self.ret_c_0xd8(),
            0xD9 => self.reti_0xd9(),
            0xDA => self.jp_c_a16_0xda(),
            0xDB => self.illegal_db_0xdb(),
            0xDC => self.call_c_a16_0xdc(),
            0xDD => self.illegal_dd_0xdd(),
            0xDE => self.sbc_a_d8_0xde(),
            0xDF => self.rst_18h_0xdf(),
            0xE0 => self.ldh_a8_a_0xe0(),
            0xE1 => self.pop_hl_0xe1(),
            0xE2 => self.ld_c_a_0xe2(),
            0xE3 => self.illegal_e3_0xe3(),
            0xE4 => self.illegal_e4_0xe4(),
            0xE5 => self.push_hl_0xe5(),
            0xE6 => self.and_d8_0xe6(),
            0xE7 => self.rst_20h_0xe7(),
            0xE8 => self.add_sp_r8_0xe8(),
            0xE9 => self.jp_hl_0xe9(),
            0xEA => self.ld_a16_a_0xea(),
            0xEB => self.illegal_eb_0xeb(),
            0xEC => self.illegal_ec_0xec(),
            0xED => self.illegal_ed_0xed(),
            0xEE => self.xor_d8_0xee(),
            0xEF => self.rst_28h_0xef(),
            0xF0 => self.ldh_a_a8_0xf0(),
            0xF1 => self.pop_af_0xf1(),
            0xF2 => self.ld_a_c_0xf2(),
            0xF3 => self.di_0xf3(),
            0xF4 => self.illegal_f4_0xf4(),
            0xF5 => self.push_af_0xf5(),
            0xF6 => self.or_d8_0xf6(),
            0xF7 => self.rst_30h_0xf7(),
            0xF8 => self.ld_hl_sp_r8_0xf8(),
            0xF9 => self.ld_sp_hl_0xf9(),
            0xFA => self.ld_a_a16_0xfa(),
            0xFB => self.ei_0xfb(),
            0xFC => self.illegal_fc_0xfc(),
            0xFD => self.illegal_fd_0xfd(),
            0xFE => self.cp_d8_0xfe(),
            0xFF => self.rst_38h_0xff(),
        }
    }
    fn execute_cb(&mut self, opcode: u8) {
        match opcode {
            0x00 => self.rlc_b_cb_0x00(),
            0x01 => self.rlc_c_cb_0x01(),
            0x02 => self.rlc_d_cb_0x02(),
            0x03 => self.rlc_e_cb_0x03(),
            0x04 => self.rlc_h_cb_0x04(),
            0x05 => self.rlc_l_cb_0x05(),
            0x06 => self.rlc_hl_cb_0x06(),
            0x07 => self.rlc_a_cb_0x07(),
            0x08 => self.rrc_b_cb_0x08(),
            0x09 => self.rrc_c_cb_0x09(),
            0x0A => self.rrc_d_cb_0x0a(),
            0x0B => self.rrc_e_cb_0x0b(),
            0x0C => self.rrc_h_cb_0x0c(),
            0x0D => self.rrc_l_cb_0x0d(),
            0x0E => self.rrc_hl_cb_0x0e(),
            0x0F => self.rrc_a_cb_0x0f(),
            0x10 => self.rl_b_cb_0x10(),
            0x11 => self.rl_c_cb_0x11(),
            0x12 => self.rl_d_cb_0x12(),
            0x13 => self.rl_e_cb_0x13(),
            0x14 => self.rl_h_cb_0x14(),
            0x15 => self.rl_l_cb_0x15(),
            0x16 => self.rl_hl_cb_0x16(),
            0x17 => self.rl_a_cb_0x17(),
            0x18 => self.rr_b_cb_0x18(),
            0x19 => self.rr_c_cb_0x19(),
            0x1A => self.rr_d_cb_0x1a(),
            0x1B => self.rr_e_cb_0x1b(),
            0x1C => self.rr_h_cb_0x1c(),
            0x1D => self.rr_l_cb_0x1d(),
            0x1E => self.rr_hl_cb_0x1e(),
            0x1F => self.rr_a_cb_0x1f(),
            0x20 => self.sla_b_cb_0x20(),
            0x21 => self.sla_c_cb_0x21(),
            0x22 => self.sla_d_cb_0x22(),
            0x23 => self.sla_e_cb_0x23(),
            0x24 => self.sla_h_cb_0x24(),
            0x25 => self.sla_l_cb_0x25(),
            0x26 => self.sla_hl_cb_0x26(),
            0x27 => self.sla_a_cb_0x27(),
            0x28 => self.sra_b_cb_0x28(),
            0x29 => self.sra_c_cb_0x29(),
            0x2A => self.sra_d_cb_0x2a(),
            0x2B => self.sra_e_cb_0x2b(),
            0x2C => self.sra_h_cb_0x2c(),
            0x2D => self.sra_l_cb_0x2d(),
            0x2E => self.sra_hl_cb_0x2e(),
            0x2F => self.sra_a_cb_0x2f(),
            0x30 => self.swap_b_cb_0x30(),
            0x31 => self.swap_c_cb_0x31(),
            0x32 => self.swap_d_cb_0x32(),
            0x33 => self.swap_e_cb_0x33(),
            0x34 => self.swap_h_cb_0x34(),
            0x35 => self.swap_l_cb_0x35(),
            0x36 => self.swap_hl_cb_0x36(),
            0x37 => self.swap_a_cb_0x37(),
            0x38 => self.srl_b_cb_0x38(),
            0x39 => self.srl_c_cb_0x39(),
            0x3A => self.srl_d_cb_0x3a(),
            0x3B => self.srl_e_cb_0x3b(),
            0x3C => self.srl_h_cb_0x3c(),
            0x3D => self.srl_l_cb_0x3d(),
            0x3E => self.srl_hl_cb_0x3e(),
            0x3F => self.srl_a_cb_0x3f(),
            0x40 => self.bit_0_b_cb_0x40(),
            0x41 => self.bit_0_c_cb_0x41(),
            0x42 => self.bit_0_d_cb_0x42(),
            0x43 => self.bit_0_e_cb_0x43(),
            0x44 => self.bit_0_h_cb_0x44(),
            0x45 => self.bit_0_l_cb_0x45(),
            0x46 => self.bit_0_hl_cb_0x46(),
            0x47 => self.bit_0_a_cb_0x47(),
            0x48 => self.bit_1_b_cb_0x48(),
            0x49 => self.bit_1_c_cb_0x49(),
            0x4A => self.bit_1_d_cb_0x4a(),
            0x4B => self.bit_1_e_cb_0x4b(),
            0x4C => self.bit_1_h_cb_0x4c(),
            0x4D => self.bit_1_l_cb_0x4d(),
            0x4E => self.bit_1_hl_cb_0x4e(),
            0x4F => self.bit_1_a_cb_0x4f(),
            0x50 => self.bit_2_b_cb_0x50(),
            0x51 => self.bit_2_c_cb_0x51(),
            0x52 => self.bit_2_d_cb_0x52(),
            0x53 => self.bit_2_e_cb_0x53(),
            0x54 => self.bit_2_h_cb_0x54(),
            0x55 => self.bit_2_l_cb_0x55(),
            0x56 => self.bit_2_hl_cb_0x56(),
            0x57 => self.bit_2_a_cb_0x57(),
            0x58 => self.bit_3_b_cb_0x58(),
            0x59 => self.bit_3_c_cb_0x59(),
            0x5A => self.bit_3_d_cb_0x5a(),
            0x5B => self.bit_3_e_cb_0x5b(),
            0x5C => self.bit_3_h_cb_0x5c(),
            0x5D => self.bit_3_l_cb_0x5d(),
            0x5E => self.bit_3_hl_cb_0x5e(),
            0x5F => self.bit_3_a_cb_0x5f(),
            0x60 => self.bit_4_b_cb_0x60(),
            0x61 => self.bit_4_c_cb_0x61(),
            0x62 => self.bit_4_d_cb_0x62(),
            0x63 => self.bit_4_e_cb_0x63(),
            0x64 => self.bit_4_h_cb_0x64(),
            0x65 => self.bit_4_l_cb_0x65(),
            0x66 => self.bit_4_hl_cb_0x66(),
            0x67 => self.bit_4_a_cb_0x67(),
            0x68 => self.bit_5_b_cb_0x68(),
            0x69 => self.bit_5_c_cb_0x69(),
            0x6A => self.bit_5_d_cb_0x6a(),
            0x6B => self.bit_5_e_cb_0x6b(),
            0x6C => self.bit_5_h_cb_0x6c(),
            0x6D => self.bit_5_l_cb_0x6d(),
            0x6E => self.bit_5_hl_cb_0x6e(),
            0x6F => self.bit_5_a_cb_0x6f(),
            0x70 => self.bit_6_b_cb_0x70(),
            0x71 => self.bit_6_c_cb_0x71(),
            0x72 => self.bit_6_d_cb_0x72(),
            0x73 => self.bit_6_e_cb_0x73(),
            0x74 => self.bit_6_h_cb_0x74(),
            0x75 => self.bit_6_l_cb_0x75(),
            0x76 => self.bit_6_hl_cb_0x76(),
            0x77 => self.bit_6_a_cb_0x77(),
            0x78 => self.bit_7_b_cb_0x78(),
            0x79 => self.bit_7_c_cb_0x79(),
            0x7A => self.bit_7_d_cb_0x7a(),
            0x7B => self.bit_7_e_cb_0x7b(),
            0x7C => self.bit_7_h_cb_0x7c(),
            0x7D => self.bit_7_l_cb_0x7d(),
            0x7E => self.bit_7_hl_cb_0x7e(),
            0x7F => self.bit_7_a_cb_0x7f(),
            0x80 => self.res_0_b_cb_0x80(),
            0x81 => self.res_0_c_cb_0x81(),
            0x82 => self.res_0_d_cb_0x82(),
            0x83 => self.res_0_e_cb_0x83(),
            0x84 => self.res_0_h_cb_0x84(),
            0x85 => self.res_0_l_cb_0x85(),
            0x86 => self.res_0_hl_cb_0x86(),
            0x87 => self.res_0_a_cb_0x87(),
            0x88 => self.res_1_b_cb_0x88(),
            0x89 => self.res_1_c_cb_0x89(),
            0x8A => self.res_1_d_cb_0x8a(),
            0x8B => self.res_1_e_cb_0x8b(),
            0x8C => self.res_1_h_cb_0x8c(),
            0x8D => self.res_1_l_cb_0x8d(),
            0x8E => self.res_1_hl_cb_0x8e(),
            0x8F => self.res_1_a_cb_0x8f(),
            0x90 => self.res_2_b_cb_0x90(),
            0x91 => self.res_2_c_cb_0x91(),
            0x92 => self.res_2_d_cb_0x92(),
            0x93 => self.res_2_e_cb_0x93(),
            0x94 => self.res_2_h_cb_0x94(),
            0x95 => self.res_2_l_cb_0x95(),
            0x96 => self.res_2_hl_cb_0x96(),
            0x97 => self.res_2_a_cb_0x97(),
            0x98 => self.res_3_b_cb_0x98(),
            0x99 => self.res_3_c_cb_0x99(),
            0x9A => self.res_3_d_cb_0x9a(),
            0x9B => self.res_3_e_cb_0x9b(),
            0x9C => self.res_3_h_cb_0x9c(),
            0x9D => self.res_3_l_cb_0x9d(),
            0x9E => self.res_3_hl_cb_0x9e(),
            0x9F => self.res_3_a_cb_0x9f(),
            0xA0 => self.res_4_b_cb_0xa0(),
            0xA1 => self.res_4_c_cb_0xa1(),
            0xA2 => self.res_4_d_cb_0xa2(),
            0xA3 => self.res_4_e_cb_0xa3(),
            0xA4 => self.res_4_h_cb_0xa4(),
            0xA5 => self.res_4_l_cb_0xa5(),
            0xA6 => self.res_4_hl_cb_0xa6(),
            0xA7 => self.res_4_a_cb_0xa7(),
            0xA8 => self.res_5_b_cb_0xa8(),
            0xA9 => self.res_5_c_cb_0xa9(),
            0xAA => self.res_5_d_cb_0xaa(),
            0xAB => self.res_5_e_cb_0xab(),
            0xAC => self.res_5_h_cb_0xac(),
            0xAD => self.res_5_l_cb_0xad(),
            0xAE => self.res_5_hl_cb_0xae(),
            0xAF => self.res_5_a_cb_0xaf(),
            0xB0 => self.res_6_b_cb_0xb0(),
            0xB1 => self.res_6_c_cb_0xb1(),
            0xB2 => self.res_6_d_cb_0xb2(),
            0xB3 => self.res_6_e_cb_0xb3(),
            0xB4 => self.res_6_h_cb_0xb4(),
            0xB5 => self.res_6_l_cb_0xb5(),
            0xB6 => self.res_6_hl_cb_0xb6(),
            0xB7 => self.res_6_a_cb_0xb7(),
            0xB8 => self.res_7_b_cb_0xb8(),
            0xB9 => self.res_7_c_cb_0xb9(),
            0xBA => self.res_7_d_cb_0xba(),
            0xBB => self.res_7_e_cb_0xbb(),
            0xBC => self.res_7_h_cb_0xbc(),
            0xBD => self.res_7_l_cb_0xbd(),
            0xBE => self.res_7_hl_cb_0xbe(),
            0xBF => self.res_7_a_cb_0xbf(),
            0xC0 => self.set_0_b_cb_0xc0(),
            0xC1 => self.set_0_c_cb_0xc1(),
            0xC2 => self.set_0_d_cb_0xc2(),
            0xC3 => self.set_0_e_cb_0xc3(),
            0xC4 => self.set_0_h_cb_0xc4(),
            0xC5 => self.set_0_l_cb_0xc5(),
            0xC6 => self.set_0_hl_cb_0xc6(),
            0xC7 => self.set_0_a_cb_0xc7(),
            0xC8 => self.set_1_b_cb_0xc8(),
            0xC9 => self.set_1_c_cb_0xc9(),
            0xCA => self.set_1_d_cb_0xca(),
            0xCB => self.set_1_e_cb_0xcb(),
            0xCC => self.set_1_h_cb_0xcc(),
            0xCD => self.set_1_l_cb_0xcd(),
            0xCE => self.set_1_hl_cb_0xce(),
            0xCF => self.set_1_a_cb_0xcf(),
            0xD0 => self.set_2_b_cb_0xd0(),
            0xD1 => self.set_2_c_cb_0xd1(),
            0xD2 => self.set_2_d_cb_0xd2(),
            0xD3 => self.set_2_e_cb_0xd3(),
            0xD4 => self.set_2_h_cb_0xd4(),
            0xD5 => self.set_2_l_cb_0xd5(),
            0xD6 => self.set_2_hl_cb_0xd6(),
            0xD7 => self.set_2_a_cb_0xd7(),
            0xD8 => self.set_3_b_cb_0xd8(),
            0xD9 => self.set_3_c_cb_0xd9(),
            0xDA => self.set_3_d_cb_0xda(),
            0xDB => self.set_3_e_cb_0xdb(),
            0xDC => self.set_3_h_cb_0xdc(),
            0xDD => self.set_3_l_cb_0xdd(),
            0xDE => self.set_3_hl_cb_0xde(),
            0xDF => self.set_3_a_cb_0xdf(),
            0xE0 => self.set_4_b_cb_0xe0(),
            0xE1 => self.set_4_c_cb_0xe1(),
            0xE2 => self.set_4_d_cb_0xe2(),
            0xE3 => self.set_4_e_cb_0xe3(),
            0xE4 => self.set_4_h_cb_0xe4(),
            0xE5 => self.set_4_l_cb_0xe5(),
            0xE6 => self.set_4_hl_cb_0xe6(),
            0xE7 => self.set_4_a_cb_0xe7(),
            0xE8 => self.set_5_b_cb_0xe8(),
            0xE9 => self.set_5_c_cb_0xe9(),
            0xEA => self.set_5_d_cb_0xea(),
            0xEB => self.set_5_e_cb_0xeb(),
            0xEC => self.set_5_h_cb_0xec(),
            0xED => self.set_5_l_cb_0xed(),
            0xEE => self.set_5_hl_cb_0xee(),
            0xEF => self.set_5_a_cb_0xef(),
            0xF0 => self.set_6_b_cb_0xf0(),
            0xF1 => self.set_6_c_cb_0xf1(),
            0xF2 => self.set_6_d_cb_0xf2(),
            0xF3 => self.set_6_e_cb_0xf3(),
            0xF4 => self.set_6_h_cb_0xf4(),
            0xF5 => self.set_6_l_cb_0xf5(),
            0xF6 => self.set_6_hl_cb_0xf6(),
            0xF7 => self.set_6_a_cb_0xf7(),
            0xF8 => self.set_7_b_cb_0xf8(),
            0xF9 => self.set_7_c_cb_0xf9(),
            0xFA => self.set_7_d_cb_0xfa(),
            0xFB => self.set_7_e_cb_0xfb(),
            0xFC => self.set_7_h_cb_0xfc(),
            0xFD => self.set_7_l_cb_0xfd(),
            0xFE => self.set_7_hl_cb_0xfe(),
            0xFF => self.set_7_a_cb_0xff(),
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
    fn ld_bc_d16_0x01(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_bc_a_0x02(&mut self) {}
    // bytes: 1 cycles: [8]
    fn inc_bc_0x03(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_b_0x04(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_b_0x05(&mut self) {}
    // bytes: 2 cycles: [8]
    fn ld_b_d8_0x06(&mut self) {}
    // bytes: 1 cycles: [4]
    fn rlca_0x07(&mut self) {}
    // bytes: 3 cycles: [20]
    fn ld_a16_sp_0x08(&mut self) {}
    // bytes: 1 cycles: [8]
    fn add_hl_bc_0x09(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_bc_0x0a(&mut self) {}
    // bytes: 1 cycles: [8]
    fn dec_bc_0x0b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_c_0x0c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_c_0x0d(&mut self) {}
    // bytes: 2 cycles: [8]
    fn ld_c_d8_0x0e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn rrca_0x0f(&mut self) {}
    // bytes: 2 cycles: [4]
    fn stop_d8_0x10(&mut self) {}
    // bytes: 3 cycles: [12]
    fn ld_de_d16_0x11(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_de_a_0x12(&mut self) {}
    // bytes: 1 cycles: [8]
    fn inc_de_0x13(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_d_0x14(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_d_0x15(&mut self) {}
    // bytes: 2 cycles: [8]
    fn ld_d_d8_0x16(&mut self) {}
    // bytes: 1 cycles: [4]
    fn rla_0x17(&mut self) {}
    // bytes: 2 cycles: [12]
    fn jr_r8_0x18(&mut self) {}
    // bytes: 1 cycles: [8]
    fn add_hl_de_0x19(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_de_0x1a(&mut self) {}
    // bytes: 1 cycles: [8]
    fn dec_de_0x1b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_e_0x1c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_e_0x1d(&mut self) {}
    // bytes: 2 cycles: [8]
    fn ld_e_d8_0x1e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn rra_0x1f(&mut self) {}
    // bytes: 2 cycles: [12, 8]
    fn jr_nz_r8_0x20(&mut self) {}
    // bytes: 3 cycles: [12]
    fn ld_hl_d16_0x21(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_a_0x22(&mut self) {}
    // bytes: 1 cycles: [8]
    fn inc_hl_0x23(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_h_0x24(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_h_0x25(&mut self) {}
    // bytes: 2 cycles: [8]
    fn ld_h_d8_0x26(&mut self) {}
    // bytes: 1 cycles: [4]
    fn daa_0x27(&mut self) {}
    // bytes: 2 cycles: [12, 8]
    fn jr_z_r8_0x28(&mut self) {}
    // bytes: 1 cycles: [8]
    fn add_hl_hl_0x29(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_hl_0x2a(&mut self) {}
    // bytes: 1 cycles: [8]
    fn dec_hl_0x2b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_l_0x2c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_l_0x2d(&mut self) {}
    // bytes: 2 cycles: [8]
    fn ld_l_d8_0x2e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cpl_0x2f(&mut self) {}
    // bytes: 2 cycles: [12, 8]
    fn jr_nc_r8_0x30(&mut self) {}
    // bytes: 3 cycles: [12]
    fn ld_sp_d16_0x31(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_a_0x32(&mut self) {}
    // bytes: 1 cycles: [8]
    fn inc_sp_0x33(&mut self) {}
    // bytes: 1 cycles: [12]
    fn inc_hl_0x34(&mut self) {}
    // bytes: 1 cycles: [12]
    fn dec_hl_0x35(&mut self) {}
    // bytes: 2 cycles: [12]
    fn ld_hl_d8_0x36(&mut self) {}
    // bytes: 1 cycles: [4]
    fn scf_0x37(&mut self) {
        println!("SCF");
    }
    // bytes: 2 cycles: [12, 8]
    fn jr_c_r8_0x38(&mut self) {
        println!("JR C, r8");
        let r8: i16 = self.read(self.registers.pc).into();
        self.registers.pc += 1;
        println!(
            "pc: {:x?}, C: {:x?}, r8: {:x?}",
            self.registers.pc, self.registers.f.c, r8
        );
        if self.registers.f.c {
            self.registers.pc = self.registers.pc.wrapping_add(r8 as u16);
            println!("pc: {:x?}", self.registers.pc);
        }
    }
    // bytes: 1 cycles: [8]
    fn add_hl_sp_0x39(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_hl_0x3a(&mut self) {}
    // bytes: 1 cycles: [8]
    fn dec_sp_0x3b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn inc_a_0x3c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn dec_a_0x3d(&mut self) {}
    // bytes: 2 cycles: [8]
    fn ld_a_d8_0x3e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ccf_0x3f(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_b_0x40(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_c_0x41(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_d_0x42(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_e_0x43(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_h_0x44(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_l_0x45(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_b_hl_0x46(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_b_a_0x47(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_c_b_0x48(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_c_c_0x49(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_c_d_0x4a(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_c_e_0x4b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_c_h_0x4c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_c_l_0x4d(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_c_hl_0x4e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_c_a_0x4f(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_d_b_0x50(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_d_c_0x51(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_d_d_0x52(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_d_e_0x53(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_d_h_0x54(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_d_l_0x55(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_d_hl_0x56(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_d_a_0x57(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_e_b_0x58(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_e_c_0x59(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_e_d_0x5a(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_e_e_0x5b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_e_h_0x5c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_e_l_0x5d(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_e_hl_0x5e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_e_a_0x5f(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_h_b_0x60(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_h_c_0x61(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_h_d_0x62(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_h_e_0x63(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_h_h_0x64(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_h_l_0x65(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_h_hl_0x66(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_h_a_0x67(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_l_b_0x68(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_l_c_0x69(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_l_d_0x6a(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_l_e_0x6b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_l_h_0x6c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_l_l_0x6d(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_l_hl_0x6e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_l_a_0x6f(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_b_0x70(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_c_0x71(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_d_0x72(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_e_0x73(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_h_0x74(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_hl_l_0x75(&mut self) {}
    // bytes: 1 cycles: [4]
    fn halt_0x76(&mut self) {
        println!("HALT");
    }
    // bytes: 1 cycles: [8]
    fn ld_hl_a_0x77(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_a_b_0x78(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_a_c_0x79(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_a_d_0x7a(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_a_e_0x7b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_a_h_0x7c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_a_l_0x7d(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_hl_0x7e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ld_a_a_0x7f(&mut self) {}
    // bytes: 1 cycles: [4]
    fn add_a_b_0x80(&mut self) {}
    // bytes: 1 cycles: [4]
    fn add_a_c_0x81(&mut self) {}
    // bytes: 1 cycles: [4]
    fn add_a_d_0x82(&mut self) {}
    // bytes: 1 cycles: [4]
    fn add_a_e_0x83(&mut self) {}
    // bytes: 1 cycles: [4]
    fn add_a_h_0x84(&mut self) {}
    // bytes: 1 cycles: [4]
    fn add_a_l_0x85(&mut self) {}
    // bytes: 1 cycles: [8]
    fn add_a_hl_0x86(&mut self) {}
    // bytes: 1 cycles: [4]
    fn add_a_a_0x87(&mut self) {}
    // bytes: 1 cycles: [4]
    fn adc_a_b_0x88(&mut self) {}
    // bytes: 1 cycles: [4]
    fn adc_a_c_0x89(&mut self) {}
    // bytes: 1 cycles: [4]
    fn adc_a_d_0x8a(&mut self) {}
    // bytes: 1 cycles: [4]
    fn adc_a_e_0x8b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn adc_a_h_0x8c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn adc_a_l_0x8d(&mut self) {}
    // bytes: 1 cycles: [8]
    fn adc_a_hl_0x8e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn adc_a_a_0x8f(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sub_b_0x90(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sub_c_0x91(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sub_d_0x92(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sub_e_0x93(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sub_h_0x94(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sub_l_0x95(&mut self) {}
    // bytes: 1 cycles: [8]
    fn sub_hl_0x96(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sub_a_0x97(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sbc_a_b_0x98(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sbc_a_c_0x99(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sbc_a_d_0x9a(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sbc_a_e_0x9b(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sbc_a_h_0x9c(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sbc_a_l_0x9d(&mut self) {}
    // bytes: 1 cycles: [8]
    fn sbc_a_hl_0x9e(&mut self) {}
    // bytes: 1 cycles: [4]
    fn sbc_a_a_0x9f(&mut self) {}
    // bytes: 1 cycles: [4]
    fn and_b_0xa0(&mut self) {}
    // bytes: 1 cycles: [4]
    fn and_c_0xa1(&mut self) {}
    // bytes: 1 cycles: [4]
    fn and_d_0xa2(&mut self) {}
    // bytes: 1 cycles: [4]
    fn and_e_0xa3(&mut self) {}
    // bytes: 1 cycles: [4]
    fn and_h_0xa4(&mut self) {}
    // bytes: 1 cycles: [4]
    fn and_l_0xa5(&mut self) {}
    // bytes: 1 cycles: [8]
    fn and_hl_0xa6(&mut self) {}
    // bytes: 1 cycles: [4]
    fn and_a_0xa7(&mut self) {}
    // bytes: 1 cycles: [4]
    fn xor_b_0xa8(&mut self) {}
    // bytes: 1 cycles: [4]
    fn xor_c_0xa9(&mut self) {}
    // bytes: 1 cycles: [4]
    fn xor_d_0xaa(&mut self) {}
    // bytes: 1 cycles: [4]
    fn xor_e_0xab(&mut self) {}
    // bytes: 1 cycles: [4]
    fn xor_h_0xac(&mut self) {}
    // bytes: 1 cycles: [4]
    fn xor_l_0xad(&mut self) {}
    // bytes: 1 cycles: [8]
    fn xor_hl_0xae(&mut self) {}
    // bytes: 1 cycles: [4]
    fn xor_a_0xaf(&mut self) {}
    // bytes: 1 cycles: [4]
    fn or_b_0xb0(&mut self) {}
    // bytes: 1 cycles: [4]
    fn or_c_0xb1(&mut self) {}
    // bytes: 1 cycles: [4]
    fn or_d_0xb2(&mut self) {}
    // bytes: 1 cycles: [4]
    fn or_e_0xb3(&mut self) {}
    // bytes: 1 cycles: [4]
    fn or_h_0xb4(&mut self) {}
    // bytes: 1 cycles: [4]
    fn or_l_0xb5(&mut self) {}
    // bytes: 1 cycles: [8]
    fn or_hl_0xb6(&mut self) {}
    // bytes: 1 cycles: [4]
    fn or_a_0xb7(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cp_b_0xb8(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cp_c_0xb9(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cp_d_0xba(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cp_e_0xbb(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cp_h_0xbc(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cp_l_0xbd(&mut self) {}
    // bytes: 1 cycles: [8]
    fn cp_hl_0xbe(&mut self) {}
    // bytes: 1 cycles: [4]
    fn cp_a_0xbf(&mut self) {}
    // bytes: 1 cycles: [20, 8]
    fn ret_nz_0xc0(&mut self) {}
    // bytes: 1 cycles: [12]
    fn pop_bc_0xc1(&mut self) {}
    // bytes: 3 cycles: [16, 12]
    fn jp_nz_a16_0xc2(&mut self) {}
    // bytes: 3 cycles: [16]
    fn jp_a16_0xc3(&mut self) {
        let l: u16 = self.read(self.registers.pc).into();
        let h: u16 = self.read(self.registers.pc + 1).into();
        let a16 = h << 8 | l;
        println!("JP a16: {:x?}", a16);
        self.registers.pc = a16;
    }
    // bytes: 3 cycles: [24, 12]
    fn call_nz_a16_0xc4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_bc_0xc5(&mut self) {}
    // bytes: 2 cycles: [8]
    fn add_a_d8_0xc6(&mut self) {}
    // bytes: 1 cycles: [16]
    fn rst_00h_0xc7(&mut self) {}
    // bytes: 1 cycles: [20, 8]
    fn ret_z_0xc8(&mut self) {}
    // bytes: 1 cycles: [16]
    fn ret_0xc9(&mut self) {}
    // bytes: 3 cycles: [16, 12]
    fn jp_z_a16_0xca(&mut self) {}
    // bytes: 1 cycles: [4]
    fn prefix_0xcb(&mut self) {}
    // bytes: 3 cycles: [24, 12]
    fn call_z_a16_0xcc(&mut self) {}
    // bytes: 3 cycles: [24]
    fn call_a16_0xcd(&mut self) {}
    // bytes: 2 cycles: [8]
    fn adc_a_d8_0xce(&mut self) {}
    // bytes: 1 cycles: [16]
    fn rst_08h_0xcf(&mut self) {}
    // bytes: 1 cycles: [20, 8]
    fn ret_nc_0xd0(&mut self) {}
    // bytes: 1 cycles: [12]
    fn pop_de_0xd1(&mut self) {}
    // bytes: 3 cycles: [16, 12]
    fn jp_nc_a16_0xd2(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_d3_0xd3(&mut self) {}
    // bytes: 3 cycles: [24, 12]
    fn call_nc_a16_0xd4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_de_0xd5(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sub_d8_0xd6(&mut self) {}
    // bytes: 1 cycles: [16]
    fn rst_10h_0xd7(&mut self) {}
    // bytes: 1 cycles: [20, 8]
    fn ret_c_0xd8(&mut self) {}
    // bytes: 1 cycles: [16]
    fn reti_0xd9(&mut self) {}
    // bytes: 3 cycles: [16, 12]
    fn jp_c_a16_0xda(&mut self) {}
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
    fn ldh_a8_a_0xe0(&mut self) {}
    // bytes: 1 cycles: [12]
    fn pop_hl_0xe1(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_c_a_0xe2(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_e3_0xe3(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_e4_0xe4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_hl_0xe5(&mut self) {}
    // bytes: 2 cycles: [8]
    fn and_d8_0xe6(&mut self) {}
    // bytes: 1 cycles: [16]
    fn rst_20h_0xe7(&mut self) {}
    // bytes: 2 cycles: [16]
    fn add_sp_r8_0xe8(&mut self) {}
    // bytes: 1 cycles: [4]
    fn jp_hl_0xe9(&mut self) {}
    // bytes: 3 cycles: [16]
    fn ld_a16_a_0xea(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_eb_0xeb(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_ec_0xec(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_ed_0xed(&mut self) {}
    // bytes: 2 cycles: [8]
    fn xor_d8_0xee(&mut self) {}
    // bytes: 1 cycles: [16]
    fn rst_28h_0xef(&mut self) {}
    // bytes: 2 cycles: [12]
    fn ldh_a_a8_0xf0(&mut self) {
        println!("LDH A, a8");
        let a8 = self.read(self.registers.pc);
        self.registers.a = a8;
    }
    // bytes: 1 cycles: [12]
    fn pop_af_0xf1(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_a_c_0xf2(&mut self) {}
    // bytes: 1 cycles: [4]
    fn di_0xf3(&mut self) {
        println!("DI");
        self.ime = false;
    }
    // bytes: 1 cycles: [4]
    fn illegal_f4_0xf4(&mut self) {}
    // bytes: 1 cycles: [16]
    fn push_af_0xf5(&mut self) {}
    // bytes: 2 cycles: [8]
    fn or_d8_0xf6(&mut self) {}
    // bytes: 1 cycles: [16]
    fn rst_30h_0xf7(&mut self) {}
    // bytes: 2 cycles: [12]
    fn ld_hl_sp_r8_0xf8(&mut self) {}
    // bytes: 1 cycles: [8]
    fn ld_sp_hl_0xf9(&mut self) {}
    // bytes: 3 cycles: [16]
    fn ld_a_a16_0xfa(&mut self) {}
    // bytes: 1 cycles: [4]
    fn ei_0xfb(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_fc_0xfc(&mut self) {}
    // bytes: 1 cycles: [4]
    fn illegal_fd_0xfd(&mut self) {}
    // bytes: 2 cycles: [8]
    fn cp_d8_0xfe(&mut self) {
        println!("CP d8");
        let d8 = self.read(self.registers.pc);
        self.registers.pc += 1;
        let (_, borrow) = self.registers.a.overflowing_sub(d8);
        let (_, h_borrow) = (self.registers.a & 0x0F).overflowing_sub(d8 & 0x0F);
        self.registers.f.z = self.registers.a == d8;
        self.registers.f.n = true;
        self.registers.f.h = h_borrow;
        self.registers.f.c = borrow;
    }
    // bytes: 1 cycles: [16]
    fn rst_38h_0xff(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_b_cb_0x00(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_c_cb_0x01(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_d_cb_0x02(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_e_cb_0x03(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_h_cb_0x04(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_l_cb_0x05(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rlc_hl_cb_0x06(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rlc_a_cb_0x07(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_b_cb_0x08(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_c_cb_0x09(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_d_cb_0x0a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_e_cb_0x0b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_h_cb_0x0c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_l_cb_0x0d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rrc_hl_cb_0x0e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rrc_a_cb_0x0f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_b_cb_0x10(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_c_cb_0x11(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_d_cb_0x12(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_e_cb_0x13(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_h_cb_0x14(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_l_cb_0x15(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rl_hl_cb_0x16(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rl_a_cb_0x17(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_b_cb_0x18(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_c_cb_0x19(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_d_cb_0x1a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_e_cb_0x1b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_h_cb_0x1c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_l_cb_0x1d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn rr_hl_cb_0x1e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn rr_a_cb_0x1f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_b_cb_0x20(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_c_cb_0x21(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_d_cb_0x22(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_e_cb_0x23(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_h_cb_0x24(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_l_cb_0x25(&mut self) {}
    // bytes: 2 cycles: [16]
    fn sla_hl_cb_0x26(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sla_a_cb_0x27(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_b_cb_0x28(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_c_cb_0x29(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_d_cb_0x2a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_e_cb_0x2b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_h_cb_0x2c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_l_cb_0x2d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn sra_hl_cb_0x2e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn sra_a_cb_0x2f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_b_cb_0x30(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_c_cb_0x31(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_d_cb_0x32(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_e_cb_0x33(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_h_cb_0x34(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_l_cb_0x35(&mut self) {}
    // bytes: 2 cycles: [16]
    fn swap_hl_cb_0x36(&mut self) {}
    // bytes: 2 cycles: [8]
    fn swap_a_cb_0x37(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_b_cb_0x38(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_c_cb_0x39(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_d_cb_0x3a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_e_cb_0x3b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_h_cb_0x3c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_l_cb_0x3d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn srl_hl_cb_0x3e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn srl_a_cb_0x3f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_b_cb_0x40(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_c_cb_0x41(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_d_cb_0x42(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_e_cb_0x43(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_h_cb_0x44(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_l_cb_0x45(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_0_hl_cb_0x46(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_0_a_cb_0x47(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_b_cb_0x48(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_c_cb_0x49(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_d_cb_0x4a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_e_cb_0x4b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_h_cb_0x4c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_l_cb_0x4d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_1_hl_cb_0x4e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_1_a_cb_0x4f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_b_cb_0x50(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_c_cb_0x51(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_d_cb_0x52(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_e_cb_0x53(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_h_cb_0x54(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_l_cb_0x55(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_2_hl_cb_0x56(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_2_a_cb_0x57(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_b_cb_0x58(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_c_cb_0x59(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_d_cb_0x5a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_e_cb_0x5b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_h_cb_0x5c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_l_cb_0x5d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_3_hl_cb_0x5e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_3_a_cb_0x5f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_b_cb_0x60(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_c_cb_0x61(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_d_cb_0x62(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_e_cb_0x63(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_h_cb_0x64(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_l_cb_0x65(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_4_hl_cb_0x66(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_4_a_cb_0x67(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_b_cb_0x68(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_c_cb_0x69(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_d_cb_0x6a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_e_cb_0x6b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_h_cb_0x6c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_l_cb_0x6d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_5_hl_cb_0x6e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_5_a_cb_0x6f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_b_cb_0x70(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_c_cb_0x71(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_d_cb_0x72(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_e_cb_0x73(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_h_cb_0x74(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_l_cb_0x75(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_6_hl_cb_0x76(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_6_a_cb_0x77(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_b_cb_0x78(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_c_cb_0x79(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_d_cb_0x7a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_e_cb_0x7b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_h_cb_0x7c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_l_cb_0x7d(&mut self) {}
    // bytes: 2 cycles: [12]
    fn bit_7_hl_cb_0x7e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn bit_7_a_cb_0x7f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_b_cb_0x80(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_c_cb_0x81(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_d_cb_0x82(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_e_cb_0x83(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_h_cb_0x84(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_l_cb_0x85(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_0_hl_cb_0x86(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_0_a_cb_0x87(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_b_cb_0x88(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_c_cb_0x89(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_d_cb_0x8a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_e_cb_0x8b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_h_cb_0x8c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_l_cb_0x8d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_1_hl_cb_0x8e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_1_a_cb_0x8f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_b_cb_0x90(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_c_cb_0x91(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_d_cb_0x92(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_e_cb_0x93(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_h_cb_0x94(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_l_cb_0x95(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_2_hl_cb_0x96(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_2_a_cb_0x97(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_b_cb_0x98(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_c_cb_0x99(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_d_cb_0x9a(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_e_cb_0x9b(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_h_cb_0x9c(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_l_cb_0x9d(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_3_hl_cb_0x9e(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_3_a_cb_0x9f(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_b_cb_0xa0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_c_cb_0xa1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_d_cb_0xa2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_e_cb_0xa3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_h_cb_0xa4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_l_cb_0xa5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_4_hl_cb_0xa6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_4_a_cb_0xa7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_b_cb_0xa8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_c_cb_0xa9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_d_cb_0xaa(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_e_cb_0xab(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_h_cb_0xac(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_l_cb_0xad(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_5_hl_cb_0xae(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_5_a_cb_0xaf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_b_cb_0xb0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_c_cb_0xb1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_d_cb_0xb2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_e_cb_0xb3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_h_cb_0xb4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_l_cb_0xb5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_6_hl_cb_0xb6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_6_a_cb_0xb7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_b_cb_0xb8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_c_cb_0xb9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_d_cb_0xba(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_e_cb_0xbb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_h_cb_0xbc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_l_cb_0xbd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn res_7_hl_cb_0xbe(&mut self) {}
    // bytes: 2 cycles: [8]
    fn res_7_a_cb_0xbf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_b_cb_0xc0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_c_cb_0xc1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_d_cb_0xc2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_e_cb_0xc3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_h_cb_0xc4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_l_cb_0xc5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_0_hl_cb_0xc6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_0_a_cb_0xc7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_b_cb_0xc8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_c_cb_0xc9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_d_cb_0xca(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_e_cb_0xcb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_h_cb_0xcc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_l_cb_0xcd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_1_hl_cb_0xce(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_1_a_cb_0xcf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_b_cb_0xd0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_c_cb_0xd1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_d_cb_0xd2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_e_cb_0xd3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_h_cb_0xd4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_l_cb_0xd5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_2_hl_cb_0xd6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_2_a_cb_0xd7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_b_cb_0xd8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_c_cb_0xd9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_d_cb_0xda(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_e_cb_0xdb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_h_cb_0xdc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_l_cb_0xdd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_3_hl_cb_0xde(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_3_a_cb_0xdf(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_b_cb_0xe0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_c_cb_0xe1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_d_cb_0xe2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_e_cb_0xe3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_h_cb_0xe4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_l_cb_0xe5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_4_hl_cb_0xe6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_4_a_cb_0xe7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_b_cb_0xe8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_c_cb_0xe9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_d_cb_0xea(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_e_cb_0xeb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_h_cb_0xec(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_l_cb_0xed(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_5_hl_cb_0xee(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_5_a_cb_0xef(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_b_cb_0xf0(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_c_cb_0xf1(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_d_cb_0xf2(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_e_cb_0xf3(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_h_cb_0xf4(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_l_cb_0xf5(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_6_hl_cb_0xf6(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_6_a_cb_0xf7(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_b_cb_0xf8(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_c_cb_0xf9(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_d_cb_0xfa(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_e_cb_0xfb(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_h_cb_0xfc(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_l_cb_0xfd(&mut self) {}
    // bytes: 2 cycles: [16]
    fn set_7_hl_cb_0xfe(&mut self) {}
    // bytes: 2 cycles: [8]
    fn set_7_a_cb_0xff(&mut self) {}
}
