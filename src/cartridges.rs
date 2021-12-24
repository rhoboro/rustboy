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
    cartridge_type: u8,
    // 0148
    rom_size: u8,
    // 0149
    ram_size: u8,
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

enum NewLicenseeCode {
    None = 0x00,
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
        let header: CartridgeHeader =
            unsafe { std::ptr::read(buf[0x100..0x14F].as_ptr() as *const _) };
        Self { header }
    }
}
