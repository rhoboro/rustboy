use std::cell::RefCell;
use std::rc::Rc;

use crate::cartridges::Cartridge;
use crate::cpu::CPU;
use crate::debugger::BreakPoint;
use crate::interruption::Interruption;
use crate::io::{Bus, IO};
use crate::joypad::JoyPad;
use crate::lcd::{BrailleTerminal, Terminal};
use crate::ppu::PPU;
use crate::sound::Sound;
use crate::timer::Timer;
use crate::Address;

/// 引数から構築される設定値群
pub struct Config {
    pub rom_file: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &str> {
        if args.len() < 2 {
            return Err("Several arguments are missing.");
        }
        let rom_file = args[1].clone();
        Ok(Config { rom_file })
    }
}

/// エントリポイント
pub fn run(config: Config) -> Result<(), &'static str> {
    let mb = MotherBoard::new(&config);
    let _ = mb.borrow().run();
    Ok(())
}

// 0xFFFE - 0xFF80
pub type Stack = [u8; 128];

#[derive(Debug)]
pub struct MotherBoard {
    cpu: Option<RefCell<CPU>>,
    cartridge: RefCell<Cartridge>,
    ram: RefCell<[u8; 4 * 1024 * 2]>,
    stack: RefCell<Stack>,
    ppu: RefCell<Box<PPU>>,
    interruption: RefCell<Box<Interruption>>,
    timer: Option<RefCell<Timer>>,
    sound: RefCell<Box<dyn IO>>,
    joypad: RefCell<Box<dyn IO>>,
}

impl MotherBoard {
    pub fn new(config: &Config) -> Rc<RefCell<Self>> {
        let cartridge = RefCell::new(Cartridge::new(&config.rom_file));
        debug_log!("{:?}", cartridge);
        let ppu = RefCell::new(Box::new(PPU::new(Box::new(BrailleTerminal::new()))));
        let interruption = RefCell::new(Box::new(Interruption::new()));
        let sound = RefCell::new(Box::new(Sound {}));
        let joypad = RefCell::new(Box::new(JoyPad::new()));
        let mut mb = Rc::new(RefCell::new(Self {
            cartridge,
            ppu,
            sound,
            joypad,
            interruption,
            ram: RefCell::new([0; 4 * 1024 * 2]),
            stack: RefCell::new([0; 128]),
            timer: Option::None,
            cpu: Option::None,
        }));
        let timer = RefCell::new(Timer::new(Rc::<RefCell<MotherBoard>>::downgrade(&mb)));
        let cpu = RefCell::new(CPU::new(Rc::<RefCell<MotherBoard>>::downgrade(&mb)));
        mb.as_ref().borrow_mut().cpu = Option::Some(cpu);
        mb.as_ref().borrow_mut().timer = Option::Some(timer);
        mb
    }

    fn run(&self) -> Result<(), &str> {
        let mut bp = BreakPoint::new(&[]);
        let mut cpu = self.cpu.as_ref().unwrap().borrow_mut();
        cpu.reset();
        loop {
            let (opcode, cycle) = cpu.tick().unwrap();
            {
                self.ppu.borrow_mut().tick(cycle);
                self.timer.as_ref().unwrap().borrow_mut().tick(cycle);
            }
            bp.breakpoint(
                opcode,
                &cpu,
                &self.stack.borrow(),
                &self.ppu.borrow(),
                &self.interruption.borrow(),
                &self.timer.as_ref().unwrap().borrow(),
            );
        }
    }
}

impl Bus for MotherBoard {
    // メモリから1バイト読み込む
    fn read(&self, address: Address) -> u8 {
        // https://w.atwiki.jp/gbspec/pages/13.html
        match address {
            0x0000..=0x7FFF => {
                // 0x0000 - 0x3FFF: 16KB ROM バンク0
                // 0x4000 - 0x7FFF: 16KB ROM バンク1 から N
                self.cartridge.borrow().read(address)
            }
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                self.ppu.borrow().read(address)
            }
            0xA000..=0xBFFF => {
                // 0xA000 - 0xBFFF: 8KB カートリッジ RAM バンク0 から N
                self.cartridge.borrow().read(address)
            }
            0xC000..=0xDFFF => {
                // 0xC000 - 0xCFFF: 4KB 作業 RAM(メインメモリ)
                // 0xD000 - 0xDFFF: 4KB 作業 RAM(メインメモリ)
                self.ram.borrow()[(address - 0xC000) as usize]
            }
            0xE000..=0xFDFF => {
                // 0xE000 - 0xFDFF: 0xC000 - 0xDDFF と同じ内容
                self.read(address - 0x2000)
            }
            0xFF0F => {
                // 割り込みフラグ
                self.interruption.borrow().read(address)
            }
            0xFFFF => {
                // 割り込み有効
                self.interruption.borrow().read(address)
            }
            // 以降はシステム領域（WR信号は外部に出力されずCPU内部で処理される）
            // 0xFE00 - 0xFE9F: スプライト属性テーブル (OAM)
            0xFF00 => self.joypad.borrow().read(address),
            0xFE00..=0xFE9F => self.ppu.borrow().read(address),
            // 以下はI/Oポート
            0xFF05..=0xFF07 => self.timer.as_ref().unwrap().borrow().read(address),
            0xFF10..=0xFF3F => self.sound.borrow().read(address),
            0xFF40..=0xFF4B => self.ppu.borrow().read(address),
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                self.stack.borrow()[(address - 0xFF80) as usize]
            }
            _ => {
                unreachable!()
            }
        }
    }

    // メモリに1バイト書き込む
    fn write(&self, address: Address, data: u8) {
        match address {
            0x0000..=0x7FFF => {
                // 0x0000 - 0x3FFF: 16KB ROM バンク0
                // 0x4000 - 0x7FFF: 16KB ROM バンク1 から N
                self.cartridge.borrow_mut().write(address, data);
            }
            0x8000..=0x9FFF => {
                // 0x8000 - 0x9FFF: 8KB VRAM
                self.ppu.borrow_mut().write(address, data);
            }
            0xA000..=0xBFFF => {
                // 0xA000 - 0xBFFF: 8KB カートリッジ RAM バンク0 から N
                self.cartridge.borrow_mut().write(address, data);
            }
            0xC000..=0xDFFF => {
                // 0xC000 - 0xCFFF: 4KB 作業 RAM(メインメモリ)
                // 0xD000 - 0xDFFF: 4KB 作業 RAM(メインメモリ)
                self.ram.borrow_mut()[(address - 0xC000) as usize] = data;
            }
            0xE000..=0xFDFF => {
                // 0xE000 - 0xFDFF: 0xC000 - 0xDDFF と同じ内容
                self.write(address - 0x2000, data)
            }
            0xFF0F => {
                // 割り込みフラグ
                self.interruption.borrow_mut().write(address, data)
            }
            0xFFFF => {
                // 割り込み有効
                self.interruption.borrow_mut().write(address, data)
            }
            // 以降はシステム領域（WR信号は外部に出力されず本来はCPU内部で処理される）
            // 0xFE00 - 0xFE9F: スプライト属性テーブル (OAM)
            0xFE00..=0xFE9F => self.ppu.borrow_mut().write(address, data),
            // 以下はI/Oポート
            0xFF00 => self.joypad.borrow_mut().write(address, data),
            0xFF05..=0xFF07 => self
                .timer
                .as_ref()
                .unwrap()
                .borrow_mut()
                .write(address, data),
            0xFF10..=0xFF3F => self.sound.borrow_mut().write(address, data),
            0xFF40..=0xFF4B => self.ppu.borrow_mut().write(address, data),
            0xFF80..=0xFFFE => {
                // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
                self.stack.borrow_mut()[(address - 0xFF80) as usize] = data;
            }
            _ => unreachable!(),
        }
    }
}
