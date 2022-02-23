use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Read;

use header::{CartridgeHeader, CartridgeType, RamSize};
use mbc1::Mbc1;
use rom_only::RomOnly;

use crate::Address;

mod header;
mod mbc1;
mod rom_only;

// ROMバンク1つのサイズは16KB
pub const BANK_SIZE_ROM: usize = 16 * 1024;
// RAMバンク1つのサイズは8KB
pub const BANK_SIZE_RAM: usize = 8 * 1024;

pub type RomBank = [u8; BANK_SIZE_ROM];
pub type RamBank = [u8; BANK_SIZE_RAM];

pub struct Cartridge {
    header: CartridgeHeader,

    // Memory Bank Controller
    mbc: Box<dyn Mbc>,
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(
            f,
            "{:?}, num_of_banks: {}, current_bank_num: {}",
            self.header,
            self.header.rom_size.num_of_banks(),
            self.mbc.current_rom_bank()
        )
    }
}

impl Cartridge {
    pub fn new(filename: &str) -> Self {
        let mut f = File::open(filename).expect("Rom file does not found");
        let mut buf = Vec::new();
        let rom_size = f.read_to_end(&mut buf).unwrap();
        assert_eq!(rom_size % (BANK_SIZE_ROM), 0);

        // header checksum
        Self::validate_checksum(&buf).expect("Rom file checksum failed");

        let header: CartridgeHeader =
            unsafe { std::ptr::read(buf[0x100..0x14F].as_ptr() as *const _) };

        let rom_banks = buf
            .chunks(BANK_SIZE_ROM)
            .map(|c| c.try_into().unwrap())
            .collect();
        let mbc = Self::create_mbc(&header.cartridge_type, &header.ram_size, rom_banks);
        Self { header, mbc }
    }

    fn validate_checksum(buf: &Vec<u8>) -> Result<i16, &str> {
        // https://gbdev.io/pandocs/The_Cartridge_Header.html#014d---header-checksum
        let mut x: i16 = 0;
        for m in 0x134..=0x14C {
            x = (x - buf[m] as i16 - 1) & 0x00FF;
        }
        if x == buf[0x14D] as i16 {
            Ok(x)
        } else {
            Err("Broken Data")
        }
    }

    fn create_mbc(
        mbc_type: &CartridgeType,
        ram_size: &RamSize,
        banks: Vec<RomBank>,
    ) -> Box<dyn Mbc> {
        match mbc_type {
            CartridgeType::RomOnly => Box::new(RomOnly::new(banks, ram_size)),
            CartridgeType::Mbc1 => Box::new(Mbc1::new(banks, ram_size)),
            _ => todo!(),
        }
    }

    pub fn read(&self, address: Address) -> u8 {
        self.mbc.read(address)
    }

    pub fn write(&mut self, address: Address, data: u8) {
        self.mbc.write(address, data)
    }
}

pub trait Mbc {
    // デバッグ用
    fn current_rom_bank(&self) -> usize;
    fn current_ram_bank(&self) -> usize;
    // ROM/RAMの読み込み
    fn read(&self, address: Address) -> u8;
    // ROM/RAMの書き込み（ROM内の一部がMBC制御レジスタへの書き込みにも利用される）
    fn write(&mut self, address: Address, data: u8);
}
