mod cartridges;

use std::env;

use cartridges::Cartridge;

fn main() {
    let args: Vec<String> = env::args().collect();
    let rom = &args[1];
    let cartridge = Cartridge::new(&rom);
    println!("{:?}", &cartridge);
}
