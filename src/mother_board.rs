use crate::cartridge::Cartridge;

use crate::Address;

/// 引数から構築される設定値群
pub struct Config {
    pub romfile: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &str> {
        if args.len() < 2 {
            return Err("Several arguments are missing.");
        }
        let romfile = args[1].clone();
        Ok(Config { romfile })
    }
}

/// エントリポイント
pub fn run(config: Config) -> Result<(), &'static str> {
    let mut mb = MotherBoard::new(&config);
    mb.run();
    Ok(())
}

#[derive(Debug)]
pub struct MotherBoard {
    cartridge: Cartridge,
    // cpu
    // lcd
    // joypad
}

impl MotherBoard {
    pub fn new(config: &Config) -> Self {
        let cartridge = Cartridge::new(&config.romfile);
        Self { cartridge }
    }

    fn run(&mut self) -> Result<(), &str> {
        println!("{:?}", self.cartridge);
        println!("{:?}", self.read(0x4364));
        self.cartridge.switch_bank(2);
        println!("{:?}", self.read(0x4364));
        self.cartridge.switch_bank(1);
        println!("{:?}", self.read(0x4364));
        Ok(())
    }

    // メモリから1バイト読み込む
    fn read(&self, address: Address) -> Result<u8, &str> {
        // https://w.atwiki.jp/gbspec/pages/13.html
        match address {
            0x0000..=0x7FFF => {
                // 0x0000 - 0x3FFF: 16KB ROM バンク0
                // 0x4000 - 0x7FFF: 16KB ROM バンク1 から N
                self.cartridge.read_rom(address)
            }
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                todo!()
            }
            0xA000..=0xBFFF => {
                // 0xA000 - 0xBFFF: 8KB カートリッジ RAM バンク0 から N
                todo!()
            }
            0xC000..=0xDFFF => {
                // 0xC000 - 0xCFFF: 4KB 作業 RAM(メインメモリ)
                // 0xD000 - 0xDFFF: 4KB 作業 RAM(メインメモリ)
                todo!()
            }
            0xE000..=0xFDFF => {
                // 0xE000 - 0xFDFF: 0xC000 - 0xDDFF と同じ内容
                todo!()
            }
            // 以降はシステム領域（WR信号は外部に出力されずCPU内部で処理される）
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
                todo!()
            }
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                todo!()
            }
            0xFFFF => {
                // 0xFFFF - 0xFFFF: 割り込み有効レジスタ
                todo!()
            }
            _ => Err("Memory Read Error"),
        }
    }

    // メモリに1バイト書き込む
    fn write(&mut self, address: Address, data: u8) -> Result<(), &str> {
        match address {
            0x0000..=0x7FFF => {
                // 0x0000 - 0x3FFF: 16KB ROM バンク0
                // 0x4000 - 0x7FFF: 16KB ROM バンク1 から N
                self.cartridge.write_rom(address, data)
            }
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                todo!()
            }
            0xA000..=0xBFFF => {
                // 0xA000 - 0xBFFF: 8KB カートリッジ RAM バンク0 から N
                todo!()
            }
            0xC000..=0xDFFF => {
                // 0xC000 - 0xCFFF: 4KB 作業 RAM(メインメモリ)
                // 0xD000 - 0xDFFF: 4KB 作業 RAM(メインメモリ)
                todo!()
            }
            0xE000..=0xFDFF => {
                // 0xE000 - 0xFDFF: 0xC000 - 0xDDFF と同じ内容
                todo!()
            }
            // 以降はシステム領域（WR信号は外部に出力されずCPU内部で処理される）
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
                todo!()
            }
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                todo!()
            }
            0xFFFF => {
                // 0xFFFF - 0xFFFF: 割り込み有効レジスタ
                todo!()
            }
            _ => Err("Memory Write Error"),
        }
    }
}
