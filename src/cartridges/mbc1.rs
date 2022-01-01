use super::{Mbc, RamBank, RamSize, RomBank, BANK_SIZE_RAM};
use crate::Address;

pub struct Mbc1 {
    rom_banks: Vec<RomBank>,
    ram_banks: Vec<RamBank>,
    current_bank: usize,
    bank_mode: BankMode,
    ram_mode: RamMode,
}

enum BankMode {
    // ROMバンクモードではRAMバンクは 0x00 のみを使用することができる
    Rom,
    // RAMバンクモードではROMバンクは 0x00-0x1F のみ使用することができる
    Ram,
}

enum RamMode {
    Disable,
    Enable,
}

impl Mbc1 {
    pub fn new(banks: Vec<RomBank>, ram_size: &RamSize) -> Self {
        Self {
            rom_banks: banks,
            ram_banks: vec![[0; BANK_SIZE_RAM]; ram_size.num_of_banks()],
            current_bank: 1,
            bank_mode: BankMode::Rom,
            ram_mode: RamMode::Disable,
        }
    }
}

impl Mbc for Mbc1 {
    fn current_bank(&self) -> usize {
        self.current_bank
    }
    fn read(&self, address: Address) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                // バンク0から読み込み
                self.rom_banks[0][address as usize]
            }
            0x4000..=0x7FFF => {
                // バンク1-Nから読み込み
                self.rom_banks[self.current_bank][(address - 0x4000) as usize]
            }
            _ => unreachable!(),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        // TODO: bit演算。自信ないので後で確認
        match address {
            0x0000..=0x1FFF => {
                // 外部RAMの有効/無効切替
                match data & 0x0F {
                    0x0A => {
                        self.ram_mode = RamMode::Enable;
                    }
                    _ => {
                        self.ram_mode = RamMode::Disable;
                    }
                }
            }
            0x2000..=0x3FFF => {
                // ROM バンク番号 (書き込み専用)
                // ROM バンクの下位5bit
                let mask = data & 0x1F;
                println!("current bank: {}", self.current_bank);
                self.current_bank = (self.current_bank & 0b1100000) | (mask as usize & 0x7F);
                println!("new current bank: {}", self.current_bank);
            }
            0x4000..=0x5FFF => {
                // RAM バンク番号または、 ROM バンク番号の上位ビット (書き込み専用)
                match self.bank_mode {
                    BankMode::Rom => match data & 0x3 {
                        0x00 => {
                            self.current_bank = self.current_bank & 0b0011111;
                        }
                        0x01 => {
                            self.current_bank = (self.current_bank & 0b1011111) | 0b0100000;
                        }
                        0x10 => {
                            self.current_bank = (self.current_bank & 0b0111111) | 0b1000000;
                        }
                        0x11 => {
                            self.current_bank = self.current_bank | 0b1100000;
                        }
                        _ => unimplemented!(),
                    },
                    BankMode::Ram => {
                        // Ramバンクを切り替える
                        todo!()
                    }
                }
            }
            0x6000..=0x7FFF => match data & 0x1 {
                0 => {
                    self.bank_mode = BankMode::Rom;
                }
                1 => {
                    self.bank_mode = BankMode::Ram;
                }
                _ => unreachable!(),
            },
            _ => unimplemented!(),
        }
    }
}
