use crate::cpu::CPU;
use crate::interruption::Interruption;
use crate::mother_board::Stack;
use crate::ppu::PPU;
use std::io::{stdin, stdout, Write};
use std::process::exit;

macro_rules! debug_log {
    () => (
        let debug_mode = false;
        if debug_mode {
            print!("\n")
        });
    ($fmt:expr) => (
        let debug_mode = false;
        if debug_mode {
            print!(concat!($fmt, "\n"))
        });
    ($fmt:expr, $($arg:tt)*) => (
        let debug_mode = false;
        if debug_mode {
            print!(concat!($fmt, "\n"), $($arg)*)
        });
}

fn prompt(message: &String) -> String {
    print!("{}", message);
    stdout().flush().ok();
    let mut input = String::new();
    stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

pub struct BreakPoint {
    breakpoints: Vec<u16>,
    should_stop: bool,
    counts: Vec<u64>,
    counter: u64,
}

impl BreakPoint {
    pub fn new(points: &[u16]) -> Self {
        Self {
            breakpoints: points.to_vec(),
            counts: vec![],
            should_stop: false,
            counter: 0,
        }
    }
    pub fn breakpoint(
        &mut self,
        opcode: u16,
        cpu: &CPU,
        stack: &Stack,
        ppu: &PPU,
        int: &Interruption,
    ) {
        println!("COUNTS: {:}", self.counter);
        println!("OPCODE: 0x{:04X?}", opcode);
        cpu.print_registers();
        int.print_interrupt_flags();
        int.print_interrupt_enables();
        self.counter += 1;
        if !self.should_stop
            & !self.breakpoints.contains(&opcode)
            & !self.counts.contains(&self.counter)
        {
            return;
        }
        self.should_stop = false;

        loop {
            let input = prompt(&"Breakpoint >>> ".to_string());
            let commands: Vec<&str> = input.split(" ").collect();
            match commands[0] {
                "continue" | "c" => {
                    println!("Continue");
                    break;
                }
                "next" | "n" => {
                    println!("Next");
                    self.should_stop = true;
                    break;
                }
                "break" | "b" => {
                    if let Some(arg) = commands.get(1) {
                        let without_prefix = arg.trim_start_matches("0x");
                        let point = u16::from_str_radix(without_prefix, 16).unwrap();
                        self.breakpoints.push(point);
                        println!("Add breakpoint: {:04X?}", point);
                    }
                }
                "remove" | "r" => {
                    if let Some(arg) = commands.get(1) {
                        let without_prefix = arg.trim_start_matches("0x");
                        let point = u16::from_str_radix(without_prefix, 16).unwrap();
                        if self.breakpoints.contains(&point) {
                            let i = self.breakpoints.iter().position(|p| *p == point).unwrap();
                            self.breakpoints.remove(i);
                        }
                        println!("Remove breakpoint: {:04X?}", point);
                    }
                }
                "count" | "bc" => {
                    if let Some(arg) = commands.get(1) {
                        let without_prefix = arg.trim_start_matches("0x");
                        let point = u64::from_str_radix(without_prefix, 10).unwrap();
                        self.counts.push(point);
                        println!("Add breakpoint: {:}", point);
                    }
                }
                "print" | "p" => match commands.get(1) {
                    Some(&"reg") => cpu.print_registers(),
                    Some(&"ifg") => int.print_interrupt_flags(),
                    Some(&"ie") => int.print_interrupt_enables(),
                    Some(&"vram") => ppu.print_vram(),
                    Some(&"stack") => println!("{:?}", stack),
                    Some(&"count") => println!("{:?}", self.counter),
                    _ => println!("available: reg, stack, vram, count"),
                },
                "quit" | "q" => {
                    println!("Bye");
                    exit(0);
                }
                "" => {}
                _ => {
                    println!("Command not found");
                }
            }
        }
    }
}
