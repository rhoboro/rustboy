use super::{Mbc, RamBank, RamSize, RomBank, BANK_SIZE_RAM};
use crate::Address;

pub struct RomOnly {
    rom_banks: Vec<RomBank>,
    #[allow(dead_code)]
    ram_banks: Vec<RamBank>,
    current_bank: usize,
}

impl RomOnly {
    pub fn new(banks: Vec<RomBank>, ram_size: &RamSize) -> Self {
        Self {
            rom_banks: banks,
            ram_banks: vec![[0; BANK_SIZE_RAM]; ram_size.num_of_banks()],
            current_bank: 1,
        }
    }
}

impl Mbc for RomOnly {
    fn current_bank(&self) -> usize {
        self.current_bank
    }
    fn read(&self, address: Address) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                // バンク0から読み込み
                self.rom_banks[0][address as usize]
            }
            0x4000..=0x7FFF => self.rom_banks[1][(address - 0x4000) as usize],
            _ => unimplemented!(),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        match address {
            0x0000..=0x3FFF => {
                self.rom_banks[0][address as usize] = data;
            }
            0x4000..=0x7FFF => {
                self.rom_banks[1][(address - 0x4000) as usize] = data;
            }
            _ => unimplemented!(),
        }
    }
}
