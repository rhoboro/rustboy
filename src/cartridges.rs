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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Cartridge {
    header: CartridgeHeader,
}

impl Cartridge {
    pub fn new(filename: &str) -> Self {
        let mut f = File::open(filename).expect("Rom file does not found");
        let mut buf = Vec::new();
        // TODO: 必要な箇所だけ読み込む
        let _ = f.read_to_end(&mut buf);

        // header checksum
        Self::validate_checksum(&buf).expect("Rom file checksum failed");

        let header: CartridgeHeader =
            unsafe { std::ptr::read(buf[0x100..0x14F].as_ptr() as *const _) };
        Self { header }
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
}
