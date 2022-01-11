use crate::cartridges::Cartridge;
use crate::cpu::{Bus, CPU};
use crate::io::IO;
use crate::lcd::PPU;
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
    // joypad
}

impl MotherBoard {
    pub fn new(config: &Config) -> Self {
        // TODO: 各種IOはMotherBoardが保持してCPUからは参照にしたい
        let cartridge = Box::new(Cartridge::new(&config.romfile));
        println!("{:?}", cartridge);
        let ppu = Box::new(PPU::new());
        let timer = Box::new(Timer {});
        let sound = Box::new(Sound {});
        let bus = Box::new(DataBus {
            cartridge,
            ppu,
            timer,
            sound,
            ram: [0; 4 * 1024 * 2],
            stack: [0; 127],
        });
        let cpu = CPU::new(bus);
        Self { cpu }
    }

    fn run(&mut self) -> Result<(), &str> {
        self.cpu.reset();
        loop {
            let opcode = self.cpu.tick().unwrap();
            if opcode == 0x76 {
                // HALT
                break;
            }
        }
        Ok(())
    }
}

struct DataBus {
    cartridge: Box<Cartridge>,
    ram: [u8; 4 * 1024 * 2],
    stack: [u8; 0xFFFE - 0xFF80 + 1],
    ppu: Box<dyn IO>,
    timer: Box<dyn IO>,
    sound: Box<dyn IO>,
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
                self.ppu.read(address)
            }
            0xA000..=0xBFFF => {
                // 0xA000 - 0xBFFF: 8KB カートリッジ RAM バンク0 から N
                todo!()
            }
            0xC000..=0xDFFF => {
                // 0xC000 - 0xCFFF: 4KB 作業 RAM(メインメモリ)
                // 0xD000 - 0xDFFF: 4KB 作業 RAM(メインメモリ)
                self.ram[(address - 0xC000) as usize]
            }
            0xE000..=0xFDFF => {
                // 0xE000 - 0xFDFF: 0xC000 - 0xDDFF と同じ内容
                self.read(address - 0x2000)
            }
            // 以降はシステム領域（WR信号は外部に出力されずCPU内部で処理される）
            // 0xFE00 - 0xFE9F: スプライト属性テーブル (OAM)
            0xFE00..=0xFE9F => self.ppu.read(address),
            // 以下はI/Oポート
            0xFF05..=0xFF07 => self.timer.read(address),
            0xFF10..=0xFF3F => self.sound.read(address),
            0xFF40..=0xFF4B => self.ppu.read(address),
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                self.stack[(address - 0xFF80) as usize]
            }
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
                self.ppu.write(address, data);
            }
            0xA000..=0xBFFF => {
                // 0xA000 - 0xBFFF: 8KB カートリッジ RAM バンク0 から N
                todo!()
            }
            0xC000..=0xDFFF => {
                // 0xC000 - 0xCFFF: 4KB 作業 RAM(メインメモリ)
                // 0xD000 - 0xDFFF: 4KB 作業 RAM(メインメモリ)
                self.ram[(address - 0xC000) as usize] = data;
            }
            0xE000..=0xFDFF => {
                // 0xE000 - 0xFDFF: 0xC000 - 0xDDFF と同じ内容
                self.write(address - 0x2000, data)
            }
            // 以降はシステム領域（WR信号は外部に出力されず本来はCPU内部で処理される）
            // 0xFE00 - 0xFE9F: スプライト属性テーブル (OAM)
            0xFE00..=0xFE9F => self.ppu.write(address, data),
            // 以下はI/Oポート
            0xFF05..=0xFF07 => self.timer.write(address, data),
            0xFF10..=0xFF3F => self.sound.write(address, data),
            0xFF40..=0xFF4B => self.ppu.write(address, data),
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                self.stack[(address - 0xFF80) as usize] = data;
            }
            _ => unreachable!(),
        }
    }
}
