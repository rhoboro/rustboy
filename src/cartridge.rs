use crate::Address;
use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Read;

// バンク1つのサイズは16KB
const BANK_SIZE_ROM: usize = 16 * 1024;
const BANK_SIZE_RAM: usize = 8 * 1024;
type RomBank = [u8; BANK_SIZE_ROM];
type RamBank = [u8; BANK_SIZE_RAM];

#[derive(Debug)]
struct CartridgeHeader {
    // 0100-0103
    entry_point: [u8; 4],
    // 0104-0133
    nintendo_logo: [u8; 48],
    // 0134-0143
    title: [u8; 16],

    // title に含まれる
    // // 013F-0142
    // manufacturer_code: [u8; 4],
    // // 0143
    // cgb_flag: u8,

    // 0144-0145
    new_licensee_code: [u8; 2],
    // 0146
    sgb_flag: u8,
    // 0147
    cartridge_type: CartridgeType,
    // 0148
    rom_size: RomSize,
    // 0149
    ram_size: RamSize,
    // 014A
    destination_code: u8,
    // 014B
    old_licensee_code: u8,
    // 014C
    mask_rom_version_number: u8,
    // 014D
    header_checksum: u8,
    // 014E-014F
    global_checksum: [u8; 2],
}

#[derive(Debug, PartialEq, Hash)]
#[allow(dead_code)]
enum CartridgeType {
    RomOnly = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
    Mbc2 = 0x05,
    Mbc2Battery = 0x06,
    RomRam = 0x08,
    RomRamBattery = 0x09,
    Mmm01 = 0x0B,
    Mmm01Ram = 0x0C,
    Mmm01RamBattery = 0x0D,
    Mbc3TimerBattery = 0x0F,
    Mbc3TimerRamBatter = 0x10,
    Mbc3 = 0x11,
    Mbc3Ram = 0x12,
    Mbc3RamBattery = 0x13,
    Mbc5 = 0x19,
    Mbc5Ram = 0x1A,
    Mbc5RamBattery = 0x1B,
    Mbc5Rumble = 0x1C,
    Mbc5RumbleRam = 0x1D,
    Mbc5RumbleRamBattery = 0x1E,
    Mbc6 = 0x20,
    Mbc7SensorRumbleRamZBattery = 0x22,
    PocketCamera = 0xFC,
    BandaiTama5 = 0xFD,
    HuC3 = 0xFE,
    HuC1RamBattery = 0xFF,
}

#[derive(Debug)]
#[allow(dead_code)]
enum RomSize {
    // TODO: バンク数Nも enum から取得できるようにしたい
    KBytes32 = 0x00,
    KBytes64 = 0x01,
    KBytes128 = 0x02,
    KBytes256 = 0x03,
    KBytes512 = 0x04,
    MBytes1 = 0x05,
    MBytes2 = 0x06,
    MBytes4 = 0x07,
    MBytes8 = 0x08,
    MBytes1_1 = 0x52,
    MBytes1_2 = 0x53,
    MBytes1_5 = 0x54,
}

impl RomSize {
    fn num_of_banks(&self) -> usize {
        match self {
            RomSize::KBytes32 => 0,
            RomSize::KBytes64 => 4,
            RomSize::KBytes128 => 8,
            RomSize::KBytes256 => 16,
            RomSize::KBytes512 => 32,
            RomSize::MBytes1 => 64,
            RomSize::MBytes2 => 128,
            RomSize::MBytes4 => 256,
            RomSize::MBytes8 => 512,
            RomSize::MBytes1_1 => 72,
            RomSize::MBytes1_2 => 80,
            RomSize::MBytes1_5 => 96,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum RamSize {
    NoRam = 0x00,
    UnUsed = 0x01,
    KB8 = 0x02,
    KB32 = 0x03,
    KB128 = 0x04,
    KB64 = 0x05,
}

impl RamSize {
    fn num_of_banks(&self) -> usize {
        match self {
            RamSize::NoRam | RamSize::UnUsed => 0,
            RamSize::KB8 => 1,
            RamSize::KB32 => 4,
            RamSize::KB64 => 8,
            RamSize::KB128 => 16,
        }
    }
}

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
            self.mbc.current_bank()
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

    pub fn switch_bank(&mut self, num: usize) -> Result<(), &str> {
        self.mbc.switch_bank(num)
    }
    pub fn current_bank(&self) -> usize {
        self.mbc.current_bank()
    }
    pub fn read(&self, address: Address) -> Result<u8, &str> {
        self.mbc.read(address)
    }
    pub fn write(&mut self, address: Address, data: u8) -> Result<(), &str> {
        self.mbc.write(address, data)
    }
}

trait Mbc {
    fn new(banks: Vec<RomBank>, ram_size: &RamSize) -> Self
    where
        Self: Sized;
    fn switch_bank(&mut self, num: usize) -> Result<(), &str>;
    fn current_bank(&self) -> usize;
    // ROM/RAMの読み込み
    fn read(&self, address: Address) -> Result<u8, &str>;
    // ROM/RAMの書き込み（ROM内の一部がMBC制御レジスタへの書き込みにも利用される）
    fn write(&mut self, address: Address, data: u8) -> Result<(), &str>;
}

struct RomOnly {
    rom_banks: Vec<RomBank>,
    ram_banks: Vec<RamBank>,
    current_bank: usize,
}

impl Mbc for RomOnly {
    fn new(banks: Vec<RomBank>, ram_size: &RamSize) -> Self {
        Self {
            rom_banks: banks,
            ram_banks: vec![[0; BANK_SIZE_RAM]; ram_size.num_of_banks()],
            current_bank: 1,
        }
    }
    fn switch_bank(&mut self, _num: usize) -> Result<(), &str> {
        unimplemented!();
    }
    fn current_bank(&self) -> usize {
        self.current_bank
    }
    fn read(&self, address: Address) -> Result<u8, &str> {
        match address {
            0x0000..=0x3FFF => {
                // バンク0から読み込み
                Ok(self.rom_banks[0][address as usize])
            }
            0x4000..=0x7FFF => Ok(self.rom_banks[1][(address - 0x4000) as usize]),
            _ => Err("Rom Read Error"),
        }
    }
    fn write(&mut self, address: Address, data: u8) -> Result<(), &str> {
        match address {
            0x0000..=0x3FFF => {
                self.rom_banks[0][address as usize] = data;
                Ok(())
            }
            0x4000..=0x7FFF => {
                self.rom_banks[1][(address - 0x4000) as usize] = data;
                Ok(())
            }
            _ => Err("Rom Write Error"),
        }
    }
}

struct Mbc1 {
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

impl Mbc for Mbc1 {
    fn new(banks: Vec<RomBank>, ram_size: &RamSize) -> Self {
        Self {
            rom_banks: banks,
            ram_banks: vec![[0; BANK_SIZE_RAM]; ram_size.num_of_banks()],
            current_bank: 1,
            bank_mode: BankMode::Rom,
            ram_mode: RamMode::Disable,
        }
    }
    fn switch_bank(&mut self, num: usize) -> Result<(), &str> {
        if num >= self.rom_banks.len() {
            Err("Out of index")
        } else {
            self.current_bank = num;
            Ok(())
        }
    }
    fn current_bank(&self) -> usize {
        self.current_bank
    }
    fn read(&self, address: Address) -> Result<u8, &str> {
        match address {
            0x0000..=0x3FFF => {
                // バンク0から読み込み
                Ok(self.rom_banks[0][address as usize])
            }
            0x4000..=0x7FFF => {
                // バンク1-Nから読み込み
                Ok(self.rom_banks[self.current_bank][(address - 0x4000) as usize])
            }
            _ => Err("Rom Read Error"),
        }
    }
    fn write(&mut self, address: Address, data: u8) -> Result<(), &str> {
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
                Ok(())
            }
            0x2000..=0x3FFF => {
                // ROM バンク番号 (書き込み専用)
                // ROM バンクの下位5bit
                todo!()
            }
            0x4000..=0x5FFF => {
                // RAM バンク番号または、 ROM バンク番号の上位ビット (書き込み専用)
                match self.bank_mode {
                    BankMode::Rom => {
                        // Romバンクの上位2bitを指定する
                        todo!()
                    }
                    BankMode::Ram => {
                        // Ramバンクを切り替える
                        todo!()
                    }
                }
            }
            0x6000..=0x7FFF => match data & 0x1 {
                0 => {
                    self.bank_mode = BankMode::Rom;
                    Ok(())
                }
                1 => {
                    self.bank_mode = BankMode::Ram;
                    Ok(())
                }
                _ => Err("Must be 0 or 1"),
            },
            _ => Err(""),
        }
    }
}
