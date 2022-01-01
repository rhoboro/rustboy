#[derive(Debug)]
pub struct CartridgeHeader {
    // 0100-0103
    pub entry_point: [u8; 4],
    // 0104-0133
    pub nintendo_logo: [u8; 48],
    // 0134-0143
    pub title: [u8; 16],

    // title に含まれる
    // // 013F-0142
    // manufacturer_code: [u8; 4],
    // // 0143
    // cgb_flag: u8,

    // 0144-0145
    pub new_licensee_code: [u8; 2],
    // 0146
    pub sgb_flag: u8,
    // 0147
    pub cartridge_type: CartridgeType,
    // 0148
    pub rom_size: RomSize,
    // 0149
    pub ram_size: RamSize,
    // 014A
    pub destination_code: u8,
    // 014B
    pub old_licensee_code: u8,
    // 014C
    pub mask_rom_version_number: u8,
    // 014D
    pub header_checksum: u8,
    // 014E-014F
    pub global_checksum: [u8; 2],
}

#[derive(Debug, PartialEq, Hash)]
#[allow(dead_code)]
pub enum CartridgeType {
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
pub enum RomSize {
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
    pub fn num_of_banks(&self) -> usize {
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
pub enum RamSize {
    NoRam = 0x00,
    UnUsed = 0x01,
    KB8 = 0x02,
    KB32 = 0x03,
    KB128 = 0x04,
    KB64 = 0x05,
}

impl RamSize {
    pub fn num_of_banks(&self) -> usize {
        match self {
            RamSize::NoRam | RamSize::UnUsed => 0,
            RamSize::KB8 => 1,
            RamSize::KB32 => 4,
            RamSize::KB64 => 8,
            RamSize::KB128 => 16,
        }
    }
}
