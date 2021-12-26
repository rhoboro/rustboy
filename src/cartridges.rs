use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Read;

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
    KByte32 = 0x00,
    KByte64 = 0x01,
    KByte128 = 0x02,
    KByte256 = 0x03,
    KByte512 = 0x04,
    MByte1 = 0x05,
    MByte2 = 0x06,
    MByte4 = 0x07,
    MByte8 = 0x08,
    MByte1_1 = 0x52,
    MByte1_2 = 0x53,
    MByte1_5 = 0x54,
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
    // ROMデータサイズは 16KB * バンク数N
    // 最初の 16KB が00バンク
    // 以降は 16KB ごとにバンクNとなる
    // N はヘッダーの RomSize から算出
    header: CartridgeHeader,

    // ROMデータ
    rom_data: Vec<u8>,

    // RAMデータ
    // データのセーブなどに利用
    // RamSize から算出
    ram_data: Vec<u8>,
}

impl Debug for Cartridge {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // rom_data は表示しない
        write!(f, "{:?}", self.header)
    }
}

impl Cartridge {
    pub fn new(filename: &str) -> Self {
        let mut f = File::open(filename).expect("Rom file does not found");
        let mut buf = Vec::new();
        let rom_size = f.read_to_end(&mut buf).unwrap();
        assert_eq!(rom_size % (16 * 1024), 0);

        // header checksum
        Self::validate_checksum(&buf).expect("Rom file checksum failed");

        let header: CartridgeHeader =
            unsafe { std::ptr::read(buf[0x100..0x14F].as_ptr() as *const _) };
        Self {
            header,
            rom_data: buf,
            ram_data: Vec::new(),
        }
    }

    fn validate_checksum(buf: &Vec<u8>) -> Result<i16, String> {
        // https://gbdev.io/pandocs/The_Cartridge_Header.html#014d---header-checksum
        let mut x: i16 = 0;
        for m in 0x134..=0x14C {
            x = (x - buf[m] as i16 - 1) & 0x00FF;
        }
        if x == buf[0x14D] as i16 {
            Ok(x)
        } else {
            Err("Failed".to_string())
        }
    }

    pub fn load(&self, memory_map: &mut [u8; 0xFFFF]) {
        match self.header.cartridge_type {
            CartridgeType::RomOnly => {
                // ROM バンク なし
                // カートリッジが32KB以下ならそのままマップ
                for i in 0x0000..0x7FFF {
                    memory_map[i] = self.rom_data[i]
                }
            }
            CartridgeType::Mbc1 => {
                // ROM バンク 00
                // カートリッジの最初の16KB
                for i in 0x0000..0x3FFF {
                    memory_map[i] = self.rom_data[i]
                }
                // ROM バンク 01-7F(20, 40, 60を除く125個まで)
                let bank_num = 0x01;
                let offset = 0x4000 * bank_num;
                for i in 0x4000..0x7FFF {
                    memory_map[i] = self.rom_data[i + offset];
                }
            }
            _ => {
                todo!()
            }
        }
    }
}

trait MBC {
    fn new() -> Self;
}

struct RomOnly {}

impl MBC for RomOnly {
    fn new() -> RomOnly {
        Self {}
    }
}
