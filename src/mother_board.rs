use crate::cartridges::Cartridge;
use crate::cpu::{Bus, CPU};
use crate::lcd::Lcd;
use crate::sound::Sound;
use crate::timer::Timer;

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
    let _ = mb.run();
    Ok(())
}

#[derive(Debug)]
pub struct MotherBoard {
    cpu: CPU,
    // lcd
    // joypad
}

impl MotherBoard {
    pub fn new(config: &Config) -> Self {
        let cartridge = Cartridge::new(&config.romfile);
        let bus = Box::new(DataBus { cartridge });
        // TODO: 各種IOはMotherBoardが保持してCPUからは参照にしたい
        let timer = Box::new(Timer {});
        let sound = Box::new(Sound {});
        let lcd = Box::new(Lcd {});
        let cpu = CPU::new(bus, timer, sound, lcd);
        Self { cpu }
    }

    fn run(&mut self) -> Result<(), &str> {
        self.cpu.reset();
        loop {
            self.cpu.tick();
            break;
        }
        Ok(())
    }
}

struct DataBus {
    cartridge: Cartridge,
}

impl Bus for DataBus {
    // メモリから1バイト読み込む
    fn read(&self, address: Address) -> u8 {
        // https://w.atwiki.jp/gbspec/pages/13.html
        match address {
            0x0000..=0x7FFF => {
                // 0x0000 - 0x3FFF: 16KB ROM バンク0
                // 0x4000 - 0x7FFF: 16KB ROM バンク1 から N
                self.cartridge.read(address)
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
            _ => unreachable!(),
        }
    }

    // メモリに1バイト書き込む
    fn write(&mut self, address: Address, data: u8) {
        match address {
            0x0000..=0x7FFF => {
                // 0x0000 - 0x3FFF: 16KB ROM バンク0
                // 0x4000 - 0x7FFF: 16KB ROM バンク1 から N
                self.cartridge.write(address, data);
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
            _ => unreachable!(),
        }
    }
}
