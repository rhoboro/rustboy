use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Weak;

use crate::arithmetic::ArithmeticUtil;
use crate::cpu::CPU;
use crate::io::{Bus, IO};
use crate::Address;

#[derive(Debug, Clone, Copy, PartialEq)]
enum TimerStatus {
    // 1
    RUNNING,
    // 0
    STOPPED,
}

#[derive(Debug, Clone, Copy)]
enum Clock {
    // 00
    // 4096Hz
    Hz4096,
    // 01
    // 4096Hz * 64
    Hz262144,
    // 10
    // 4096Hz * 16
    Hz65536,
    // 11
    // 4096Hz * 4
    Hz16384,
}

impl Clock {
    // 分周
    fn divide(&self) -> u32 {
        // CPU は 4.194304 MHz
        match self {
            Clock::Hz4096 => CPU::CLOCK / 4096,
            Clock::Hz16384 => CPU::CLOCK / 16384,
            Clock::Hz65536 => CPU::CLOCK / 65536,
            Clock::Hz262144 => CPU::CLOCK / 262144,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TAC {
    // Bit 7-3 未使用
    // Bit 2 タイマー停止状態
    status: TimerStatus,
    // Bit 1-0
    clock: Clock,
}

impl From<u8> for TAC {
    fn from(v: u8) -> Self {
        let status = match v & 0b_0000_0100 {
            0b100 => TimerStatus::RUNNING,
            0b000 => TimerStatus::STOPPED,
            _ => unreachable!(),
        };
        let clock = match v & 0b_0000_0011 {
            0b00 => Clock::Hz4096,
            0b01 => Clock::Hz262144,
            0b10 => Clock::Hz65536,
            0b11 => Clock::Hz16384,
            _ => unreachable!(),
        };
        Self { status, clock }
    }
}

impl From<TAC> for u8 {
    fn from(tac: TAC) -> Self {
        let status = match tac.status {
            TimerStatus::RUNNING => 0b100,
            TimerStatus::STOPPED => 0b000,
        };
        let clock = match tac.clock {
            Clock::Hz4096 => 0b00,
            Clock::Hz262144 => 0b01,
            Clock::Hz65536 => 0b10,
            Clock::Hz16384 => 0b11,
        };
        status | clock
    }
}

pub struct Timer {
    // 分周レジスタ
    // 16384Hz でインクリメントされる
    // このレジスタに何かが書き込まれた時は、00hにリセットされる
    // FF04
    div: u8,
    div_tmp: u32,

    // タイマーカウンタ
    // 周波数はtac.clockにより可変
    // オーバーフローしたら tma の値をセットして割り込みを入れる
    // FF05
    tima: u8,
    tima_tmp: u32,

    // タイマーモジュロ
    // tima がオーバーフローしたらこの値がセットされる
    // FF06
    tma: u8,

    // タイマー制御
    // FF07
    tac: TAC,

    bus: Weak<RefCell<dyn Bus>>,
}

impl Timer {
    // 分周レジスタの周波数
    const CLOCK_DIV: u32 = 16384;

    pub fn new(bus: Weak<RefCell<dyn Bus>>) -> Self {
        Self {
            bus,
            div: 0,
            div_tmp: 0,
            tima: 0,
            tima_tmp: 0,
            tma: 0,
            tac: TAC::from(0),
        }
    }
    pub fn tick(&mut self, cycle: u8) {
        self.increment_div(cycle);
        if self.tac.status == TimerStatus::RUNNING {
            self.increment_tima(cycle);
        }
    }
    fn increment_div(&mut self, cycle: u8) {
        self.div_tmp = self.div_tmp.wrapping_add(cycle as u32);
        if self.div_tmp >= (CPU::CLOCK / Timer::CLOCK_DIV) {
            // 16384Hz でインクリメントする
            self.div = self.div.wrapping_add(1);
            self.div_tmp = self.div_tmp - (CPU::CLOCK / Timer::CLOCK_DIV);
        }
    }
    fn increment_tima(&mut self, cycle: u8) {
        self.tima_tmp = self.tima_tmp.wrapping_add(cycle as u32);
        if self.tima_tmp >= self.tac.clock.divide() {
            if self.tima.calc_carry(1) {
                // タイマーを初期化して割り込み
                self.tima = self.tma;
                let value = self.bus.upgrade().unwrap().borrow().read(0xFF0F) | 0b00000100;
                self.bus.upgrade().unwrap().borrow().write(0xFF0F, value);
            } else {
                self.tima = self.tima.wrapping_add(1);
            }
            self.tima_tmp = self.tima_tmp - self.tac.clock.divide();
        }
    }
    pub fn print_timer(&self) {
        println!("{:?}", self);
    }
}

impl Debug for Timer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Timer: {{ div: {}, div_tmp: {}, tima: {}, tima_tmp: {}, tma: {}, tac: 0b{:08b} }}",
            self.div,
            self.div_tmp,
            self.tima,
            self.tima_tmp,
            self.tma,
            u8::from(self.tac)
        )
    }
}

impl IO for Timer {
    fn read(&self, address: Address) -> u8 {
        match address {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma as u8,
            0xFF07 => u8::from(self.tac),
            _ => unreachable!(),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        match address {
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = data,
            0xFF06 => self.tma = data,
            0xFF07 => self.tac = TAC::from(data),
            _ => unreachable!(),
        }
    }
}
