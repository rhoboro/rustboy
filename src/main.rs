mod cartridges;

use std::env;

use cartridges::Cartridge;

fn main() {
    // https://w.atwiki.jp/gbspec/pages/13.html
    // 0x0000 - 0x3FFF: 16KB ROM バンク0
    // 0x4000 - 0x7FFF: 16KB ROM バンク1 から N
    // 0x8000 - 0x9FFF: 8KB VRAM
    // 0xA000 - 0xBFFF: 8KB カートリッジ RAM バンク0 から N
    // 0xC000 - 0xCFFF: 4KB 作業 RAM(メインメモリ)
    // 0xD000 - 0xDFFF: 4KB 作業 RAM(メインメモリ)
    // 0xE000 - 0xFDFF: 0xC000 - 0xDDFF と同じ内容
    //
    // 以降はシステム領域（WR信号は外部に出力されずCPU内部で処理される）
    // 0xFE00 - 0xFE9F: スプライト属性テーブル (OAM)
    // 0xFEA0 - 0xFEFF: 未使用
    // 0xFF00 - 0xFF7F: I/Oレジスタ
    // 0xFF80 - 0xFFFE: 上位RAM スタック用の領域
    // 0xFFFF - 0xFFFF: 割り込み有効レジスタ
    let mut memory_map: [u8; 0xFFFF] = [0; 0xFFFF];
    let args: Vec<String> = env::args().collect();
    let rom = &args[1];
    let cartridge = Cartridge::new(&rom);
    println!("{:?}", cartridge);
    cartridge.load(&mut memory_map);
}
