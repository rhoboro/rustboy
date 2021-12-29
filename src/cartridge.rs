use crate::Address;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Read;

// バンク1つのサイズは16KB
const BANK_SIZE: usize = 16 * 1024;

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

pub struct Cartridge {
    header: CartridgeHeader,

    // Memory Bank Controller
    mbc: Box<dyn Mbc>,

    // ROMデータ
    // ROMデータサイズは 16KB * バンク数N
    // 最初の 16KB が00バンク
    // 以降は 16KB ごとにバンクNとなる
    // メモリマップの0000-3FFFがバンク0に接続される
    // メモリマップの4000-7FFFがバンク1-Nのいずれかに接続される
    // バンクの切り替えはMBCが行う
    // rom_banks: Vec<Vec<u8>>,

    // ROMデータに含まれるバンク数
    num_of_banks: usize,

    // RAMデータ
    // データのセーブなどに利用
    // RamSize から算出
    ram_data: Vec<u8>,
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(
            f,
            "{:?}, num_of_banks: {}, current_bank_num: {}",
            self.header,
            self.num_of_banks,
            self.mbc.current_bank()
        )
    }
}

impl Cartridge {
    pub fn new(filename: &str) -> Self {
        let mut f = File::open(filename).expect("Rom file does not found");
        let mut buf = Vec::new();
        let rom_size = f.read_to_end(&mut buf).unwrap();
        assert_eq!(rom_size % (BANK_SIZE), 0);

        // header checksum
        Self::validate_checksum(&buf).expect("Rom file checksum failed");

        let header: CartridgeHeader =
            unsafe { std::ptr::read(buf[0x100..0x14F].as_ptr() as *const _) };

        // TODO: header.rom_size から算出
        let num_of_banks = rom_size / (BANK_SIZE);
        let rom_banks = buf.chunks(BANK_SIZE).map(|c| c.to_vec()).collect();
        let mbc = Self::create_mbc(&header.cartridge_type, rom_banks);
        Self {
            header,
            mbc,
            num_of_banks,
            ram_data: Vec::new(),
        }
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

    fn create_mbc(mbc_type: &CartridgeType, banks: Vec<Vec<u8>>) -> Box<dyn Mbc> {
        match mbc_type {
            CartridgeType::RomOnly => Box::new(RomOnly::new(banks)),
            CartridgeType::Mbc1 => Box::new(Mbc1::new(banks)),
            _ => Box::new(Mbc1::new(banks)),
        }
    }

    pub fn switch_bank(&mut self, num: usize) -> Result<(), &str> {
        self.mbc.switch_bank(num)
    }
    pub fn current_bank(&self) -> usize {
        self.mbc.current_bank()
    }
    pub fn read_rom(&self, address: Address) -> Result<u8, &str> {
        self.mbc.read_rom(address)
    }
    pub fn write_rom(&mut self, address: Address, data: u8) -> Result<(), &str> {
        self.mbc.write_rom(address, data)
    }
    pub fn read_ram(&self, address: Address) -> Result<u8, &str> {
        todo!()
    }
    pub fn write_ram(&mut self, address: Address, data: u8) -> Result<(), &str> {
        todo!()
    }
}

trait Mbc {
    fn new(banks: Vec<Vec<u8>>) -> Self
    where
        Self: Sized;
    fn switch_bank(&mut self, num: usize) -> Result<(), &str>;
    fn current_bank(&self) -> usize;
    fn read_rom(&self, address: Address) -> Result<u8, &str>;
    // ROM だけど MBC 制御レジスタへの書き込みにも利用される
    fn write_rom(&mut self, address: Address, data: u8) -> Result<(), &str>;
}

struct RomOnly {
    rom_banks: Vec<Vec<u8>>,
    current_bank: usize,
}

impl Mbc for RomOnly {
    fn new(banks: Vec<Vec<u8>>) -> Self {
        Self {
            rom_banks: banks,
            current_bank: 1,
        }
    }
    fn switch_bank(&mut self, _num: usize) -> Result<(), &str> {
        unimplemented!();
    }
    fn current_bank(&self) -> usize {
        self.current_bank
    }
    fn read_rom(&self, address: Address) -> Result<u8, &str> {
        match address {
            0x0000..=0x3FFF => {
                // バンク0から読み込み
                Ok(self.rom_banks[0][address as usize])
            }
            0x4000..=0x7FFF => Ok(self.rom_banks[1][(address - 0x4000) as usize]),
            _ => Err("Rom Read Error"),
        }
    }
    fn write_rom(&mut self, address: Address, data: u8) -> Result<(), &str> {
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
    rom_banks: Vec<Vec<u8>>,
    current_bank: usize,
}

impl Mbc for Mbc1 {
    fn new(banks: Vec<Vec<u8>>) -> Self {
        Self {
            rom_banks: banks,
            current_bank: 1,
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
    fn read_rom(&self, address: Address) -> Result<u8, &str> {
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
    fn write_rom(&mut self, address: Address, data: u8) -> Result<(), &str> {
        match address {
            0x0000..=0x7FFF => {
                // バンク0に書き込み
                self.rom_banks[0][address as usize] = data;
                Ok(())
            }
            0xA000..=0xBFFF => {
                // バンク1-Nに書き込み
                self.rom_banks[self.current_bank][(address - 0x4000) as usize] = data;
                Ok(())
            }
            _ => Err("Rom Write Error"),
        }
    }
}
