use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Weak;

use crate::arithmetic::ArithmeticUtil;
use crate::io::{Bus, IO};
use crate::Address;

#[derive(Debug, Clone, Copy)]
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
        let status = if (v & 0b_0000_0100) == 0 {
            TimerStatus::STOPPED
        } else {
            TimerStatus::RUNNING
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

    // タイマーカウンタ
    // オーバーフローしたら tma の値をセットして割り込みを入れる
    // FF05
    tima: u8,

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
    pub fn new(bus: Weak<RefCell<dyn Bus>>) -> Self {
        Self {
            bus,
            div: 0,
            tima: 0,
            tma: 0,
            tac: TAC::from(0),
        }
    }
    pub fn tick(&mut self, cycle: u8) {
        self.increment_div(cycle);
        self.increment_tima(cycle);
    }
    fn increment_div(&mut self, cycle: u8) {
        // 16384Hz でインクリメントする
        self.div = self.div.wrapping_add(cycle.wrapping_mul(4))
    }
    fn increment_tima(&mut self, cycle: u8) {
        let timer_cycle = match self.tac.clock {
            Clock::Hz4096 => cycle,
            Clock::Hz16384 => cycle.wrapping_mul(4),
            Clock::Hz65536 => cycle.wrapping_mul(16),
            Clock::Hz262144 => cycle.wrapping_mul(64),
        };
        if self.tima.calc_carry(timer_cycle) {
            self.tima = self.tma;
            // 割り込み
            let value = self.bus.upgrade().unwrap().borrow().read(0xFF0F) & 0b00000100;
            self.bus.upgrade().unwrap().borrow().write(0xFF0F, value);
        } else {
            self.tima = self.tima.wrapping_add(1);
        }
    }
}

impl Debug for Timer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timer")
    }
}

impl IO for Timer {
    fn read(&self, address: Address) -> u8 {
        debug_log!("Read Timer: {:X?}", address);
        match address {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma as u8,
            0xFF07 => u8::from(self.tac),
            _ => unreachable!(),
        }
    }
    fn write(&mut self, address: Address, data: u8) {
        debug_log!("Write Timer: {:X?}, Data: {}", address, data);
        match address {
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = data,
            0xFF06 => self.tma = data,
            0xFF07 => self.tac = TAC::from(data),
            _ => unreachable!(),
        }
    }
}
