extern crate chip8;

use chip8::Chip8;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut emulator = Chip8::new();
    
    emulator.initialize();
    emulator.load_rom(&args[1]);

    // loop {
    //     emulator.emulate_cycle();
    // }
}
