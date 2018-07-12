extern crate rand;
extern crate sdl2;

use rand::prelude::*;
use std::fs::File;
use std::io::Read;

use sdl2::keyboard::Keycode;

/// Starting address for program ROMs.
const PROGRAM_ROM_START: usize = 0x200;
/// Starting address for the fontset.
const FONTSET_START: usize = 0x000;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
/// The true size of the display in memory (RGB24 pixel format). 3 times as big
/// as the emulated display because each pixel has to be represented by an RGB
/// triplet.
pub const DISPLAY_SIZE: usize = DISPLAY_HEIGHT * DISPLAY_WIDTH * 3;

#[cfg_attr(rustfmt, rustfmt_skip)]
const CHIP8_FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

/// Methods to extract parts of an opcode.
trait Opcode {
    fn x(&self) -> usize;
    fn y(&self) -> usize;
    fn n(&self) -> usize;
    fn kk(&self) -> u8;
    fn nnn(&self) -> usize;
}

impl Opcode for u16 {
    fn x(&self) -> usize { ((self & 0x0F00) >> 8) as usize }
    fn y(&self) -> usize { ((self & 0x00F0) >> 4) as usize }
    fn n(&self) -> usize { (self & 0x000F) as usize }
    fn kk(&self) -> u8 { (self & 0x00FF) as u8 }
    fn nnn(&self) -> usize { (self & 0x0FFF) as usize }
}

/// Main CHIP-8 CPU data structure.
pub struct CPU {
    pub opcode: u16, // current opcode
    pub memory: [u8; 4096],
    pub v_reg: [u8; 16], // registers
    pub i_addr: usize,   // u16, address register
    pub pc: usize,       // u16, program counter
    pub display: [u8; DISPLAY_SIZE],
    pub stack: [usize; 16], // u16
    pub sp: usize,          // u8, stack pointer
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub keypad: [u8; 16],
}

impl CPU {
    pub fn new() -> CPU {
        let mut cpu = CPU {
            opcode: 0,
            memory: [0; 4096],
            v_reg: [0; 16],
            i_addr: 0,
            pc: PROGRAM_ROM_START,
            display: [0; DISPLAY_SIZE],
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [0; 16],
        };
        // You shouldn't have to load the fontset in separately, assume it's
        // loaded in when the machine starts.
        cpu.load_fontset();
        cpu
    }

    /// Load fontset into memory.
    fn load_fontset(&mut self) {
        for (i, byte) in CHIP8_FONTSET.iter().enumerate() {
            self.memory[FONTSET_START + i] = *byte;
        };
    }

    /// Load a program ROM into memory.
    pub fn load_rom(&mut self, filename: &str) {
        let mut file = File::open(filename).unwrap();

        // Reads up to memory (4 KB) bytes
        file.read(&mut self.memory[(PROGRAM_ROM_START as usize)..])
            .unwrap();
    }

    /// Get the state of a pixel (On/Off).
    pub fn get_pixel(&self, pixel_index: usize) -> u8 {
        let triplet_index = pixel_index * 3;
        // We only check the first byte since they should all be equal
        let red = self.display[triplet_index];

        match red {
            255 => 1,
            0 => 0,
            _ => panic!("red value wasn't 255 or 0: {}", red),
        }
    }

    /// Set the state of a pixel (On/Off).
    pub fn set_pixel(&mut self, pixel_index: usize, state: u8) {
        let triplet_index = pixel_index * 3;

        let pixel_value = match state {
            1 => 255,
            0 => 0,
            _ => panic!("bad pixel state {}", state),
        };

        self.display[triplet_index + 0] = pixel_value;
        self.display[triplet_index + 1] = pixel_value;
        self.display[triplet_index + 2] = pixel_value;
    }

    /// Set the state of a pixel (On/Off) with XOR.
    pub fn xor_pixel(&mut self, pixel_index: usize, state: u8) {
        // Emulating XOR here, if pixels match turn off, else turn on
        if self.get_pixel(pixel_index) == state {
            self.set_pixel(pixel_index, 0);
        } else {
            self.set_pixel(pixel_index, 1);
        }
    }

    /// Maps an SDL2 Keycode to the hex digit it represents in CHIP-8.
    pub fn keycode_to_hex(&self, key: Keycode) -> Option<u8> {
        match key {
            // row 1
            Keycode::Num1 => Some(0x1),
            Keycode::Num2 => Some(0x2),
            Keycode::Num3 => Some(0x3),
            Keycode::Num4 => Some(0xC),
            // row 2
            Keycode::Q => Some(0x4),
            Keycode::W => Some(0x5),
            Keycode::E => Some(0x6),
            Keycode::R => Some(0xD),
            // row 3
            Keycode::A => Some(0x7),
            Keycode::S => Some(0x8),
            Keycode::D => Some(0x9),
            Keycode::F => Some(0xE),
            // row 4
            Keycode::Z => Some(0xA),
            Keycode::X => Some(0x0),
            Keycode::C => Some(0xB),
            Keycode::V => Some(0xF),
            // ignore any other key
            _ => None,
        }
    }

    /// Update the keypad to reflect a keypress.
    pub fn update_keypad(&mut self, key: sdl2::keyboard::Keycode, key_down: bool) {
        if let Some(hex) = self.keycode_to_hex(key) {
            self.keypad[hex as usize] = key_down as u8;
        }
    }

    /// Emulate a CPU cycle.
    pub fn emulate_cycle(&mut self) {
        self.fetch_opcode();
        // println!("{:X}", self.opcode);
        self.decode_opcode();
        self.update_timers();
    }

    fn fetch_opcode(&mut self) {
        let pc = self.pc as usize;

        // Bytes are cast into u16 so we can merge them next
        let byte1 = self.memory[pc] as u16;
        let byte2 = self.memory[pc + 1] as u16;

        // Merge the 2-byte instruction at the program counter
        self.opcode = (byte1 << 8) | byte2;
    }

    /// (00E0) Clear the display.
    fn opcode_cls(&mut self) {
        self.display = [0; DISPLAY_SIZE];
        self.pc += 2;
    }

    /// (00EE) Return from a subroutine.
    fn opcode_ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp];
        self.pc += 2;
    }

    /// (1nnn) Jump to location.
    fn opcode_jp(&mut self) {
        self.pc = self.opcode.nnn();
    }

    /// (2nnn) Call subroutine.
    fn opcode_call(&mut self) {
        self.stack[self.sp] = self.pc;
        self.sp += 1;
        self.pc = self.opcode.nnn();
    }

    /// (3xkk) Skip next instruction if Vx == kk.
    fn opcode_se_byte(&mut self) {
        if self.v_reg[self.opcode.x()] == self.opcode.kk() {
            self.pc += 2;
        }
        self.pc += 2;
    }

    /// (4xkk) Skip next instruction if Vx != kk.
    fn opcode_sne_byte(&mut self) {
        if self.v_reg[self.opcode.x()] != self.opcode.kk() {
            self.pc += 2;
        }
        self.pc += 2;
    }

    /// (5xy0) Skip next instruction if Vx == Vy.
    fn opcode_se_vx(&mut self) {
        if self.v_reg[self.opcode.x()] == self.v_reg[self.opcode.y()] {
            self.pc += 2;
        }
        self.pc += 2;
    }

    /// (6xkk) Set Vx to kk.
    fn opcode_ld_byte(&mut self) {
        self.v_reg[self.opcode.x()] = self.opcode.kk();
        self.pc += 2;
    }

    /// (7xkk) Add kk to Vx.
    fn opcode_add_byte(&mut self) {
        self.v_reg[self.opcode.x()] = self.v_reg[self.opcode.x()].wrapping_add(self.opcode.kk());
        self.pc += 2;
    }

    /// (8xy0) Set Vx to Vy.
    fn opcode_ld_vy(&mut self) {
        self.v_reg[self.opcode.x()] = self.v_reg[self.opcode.y()];
        self.pc += 2;
    }

    /// (8xy1) Bitwise OR.
    fn opcode_or(&mut self) {
        self.v_reg[self.opcode.x()] |= self.v_reg[self.opcode.y()];
        self.pc += 2;
    }

    /// (8xy2) Bitwise AND.
    fn opcode_and(&mut self) {
        self.v_reg[self.opcode.x()] &= self.v_reg[self.opcode.y()];
        self.pc += 2;
    }

    /// (8xy3) Bitwise XOR.
    fn opcode_xor(&mut self) {
        self.v_reg[self.opcode.x()] ^= self.v_reg[self.opcode.y()];
        self.pc += 2;
    }

    /// (8xy4) Add Vy to Vx, set VF to carry.
    fn opcode_add(&mut self) {
        let vx = self.v_reg[self.opcode.x()];
        let vy = self.v_reg[self.opcode.y()];

        let (result, overflow) = vx.overflowing_add(vy);

        // Set carry flag
        if overflow {
            self.v_reg[0xF] = 1;
        } else {
            self.v_reg[0xF] = 0;
        }

        self.v_reg[self.opcode.x()] = result;
        self.pc += 2;
    }

    /// (8xy5) Set Vx to Vx - Vy, set VF to carry.
    fn opcode_sub(&mut self) {
        let vx = self.v_reg[self.opcode.x()];
        let vy = self.v_reg[self.opcode.y()];

        let (result, overflow) = vx.overflowing_sub(vy);

        if overflow {
            self.v_reg[0xF] = 0;
        } else {
            self.v_reg[0xF] = 1;
        }

        self.v_reg[self.opcode.x()] = result;
        self.pc += 2;
    }

    /// (8xy6) Right shift.
    fn opcode_shr(&mut self) {
        let lsb = self.v_reg[self.opcode.x()] & 0x01;

        self.v_reg[0xF] = lsb;
        self.v_reg[self.opcode.x()] >>= 1;
        self.pc += 2;
    }

    /// (8xy7) Set Vx to Vy - Vx, set VF to carry
    fn opcode_subn(&mut self) {
        let vx = self.v_reg[self.opcode.x()];
        let vy = self.v_reg[self.opcode.y()];

        let (result, overflow) = vy.overflowing_sub(vx);

        if overflow {
            self.v_reg[0xF] = 0;
        } else {
            self.v_reg[0xF] = 1;
        }

        self.v_reg[self.opcode.x()] = result;
        self.pc += 2;
    }

    /// (8xyE) Left shift.
    fn opcode_shl(&mut self) {
        // 0x8 = 0b1000
        let msb = self.v_reg[self.opcode.x()] & 0x80;

        self.v_reg[0xF] = msb;
        self.v_reg[self.opcode.x()] <<= 1;
        self.pc += 2;
    }

    /// Skip next instruction if Vx != Vy
    fn opcode_sne(&mut self) {
        let vx = self.v_reg[self.opcode.x()];
        let vy = self.v_reg[self.opcode.y()];

        if vx != vy {
            self.pc += 2;
        }
        self.pc += 2;
    }

    /// Set address register to NNN
    fn opcode_ld(&mut self) {
        self.i_addr = self.opcode.nnn();
        self.pc += 2;
    }

    /// Jump to NNN + V0
    fn opcode_jp_v0(&mut self) {
        self.pc = self.opcode.nnn();
        self.pc += self.v_reg[0] as usize;
    }

    /// Generate random byte AND kk, store in Vx
    fn opcode_rnd(&mut self) {
        let mut rng = thread_rng();
        let random_num: u8 = rng.gen(); // Generates a random u8 number

        self.v_reg[self.opcode.x()] = random_num & self.opcode.kk();
        self.pc += 2;
    }

    /// (Dxyn) Draw an n-byte sprite at (Vx, Vy) from memory location I
    fn opcode_drw(&mut self) {
        let x = self.v_reg[self.opcode.x()] as usize;
        let y = self.v_reg[self.opcode.y()] as usize;
        let n = self.opcode.n(); // Sprite height

        // The pixel where we start drawing from
        let starting_pixel = x + (y * DISPLAY_WIDTH);

        // Set collision flag off, we'll turn it on if we get a collision
        // at any point while drawing.
        self.v_reg[0xF] = 0;

        // For each row in the sprite...
        for row_number in 0..n as usize {
            // The actual pixels of this row for the sprite
            let sprite_row: u8 = self.memory[self.i_addr + row_number];

            // For each pixel in the sprite row...
            for pixel_number in 0..8 as usize {
                // We use masking to go through each bit in the row
                let sprite_pixel = if (sprite_row & (0x80 >> pixel_number)) == 0 {
                    0
                } else {
                    1
                };

                // The pixel we are about to write to
                let mut target_pixel_index = starting_pixel.wrapping_add((row_number * DISPLAY_WIDTH) + pixel_number);

                // Handle vertical wrapping
                if target_pixel_index > 2047 {
                    target_pixel_index -= DISPLAY_WIDTH * 31;
                }

                // Handle overflow by wrapping to the start of the row
                if (starting_pixel + pixel_number) >= DISPLAY_WIDTH {
                    target_pixel_index -= DISPLAY_WIDTH;
                }

                // Check collision
                if self.get_pixel(target_pixel_index) == 1 {
                    self.v_reg[0xF] = 1;
                }

                // Set the pixel with XOR
                self.xor_pixel(target_pixel_index, sprite_pixel);
            }
        }
        self.pc += 2;
    }

    /// (Ex9E) Skip next instruction if key with value Vx pressed.
    fn opcode_skp(&mut self) {
        let vx = self.v_reg[self.opcode.x()];
        
        if self.keypad[vx as usize] == 1 {
            self.pc += 2;
        }
        self.pc += 2;
    }

    /// (ExA1) Skip next instruction if key with value Vx not pressed.
    fn opcode_sknp(&mut self) {
        let vx = self.v_reg[self.opcode.x()];
        
        if self.keypad[vx as usize] == 0 {
            self.pc += 2;
        }
        self.pc += 2;
    }

    /// (Fx07) Set Vx to DT.
    fn opcode_get_dt(&mut self) {
        self.v_reg[self.opcode.x()] = self.delay_timer;
        self.pc += 2;
    }

    /// (Fx0A) Wait for a key press, store key in Vx.
    fn opcode_waitkey(&mut self) {
        // TODO: Implement blocking
        self.pc += 2;
    }

    /// (Fx15) Set delay timer to Vx.
    fn opcode_set_dt(&mut self) {
        self.delay_timer = self.v_reg[self.opcode.x()];
        self.pc += 2;
    }

    /// (Fx18) Set sound timer to Vx.
    fn opcode_set_st(&mut self) {
        self.sound_timer = self.v_reg[self.opcode.x()];
        self.pc += 2;
    }

    /// (Fx1E) I = I + Vx.
    fn opcode_add_i(&mut self) {
        self.i_addr += self.v_reg[self.opcode.x()] as usize;
        self.pc += 2;
    }

    // (Fx29) I = location of sprite in memory for digit Vx
    fn opcode_set_sprite(&mut self) {
        // Hex digit we want the sprite addr for
        let vx = self.v_reg[self.opcode.x()];

        // Digit sprites are 5 bytes long starting at 0x0, so we multiply to
        // get the address.
        // 0 * 5 = 0. 1 * 5 = 5. 0xF * 5 = 75 etc.
        self.i_addr = FONTSET_START + ((vx * 5) as usize);
        self.pc += 2;
    }

    /// (Fx33) Store BCD representation of Vx in I, I+1, I+2
    fn opcode_bcd_vx(&mut self) {
        let vx = self.v_reg[self.opcode.x()];

        // Given the number 235:
        // 235 / 100 = 2
        // 235 - 200 = 35 / 10 = 3
        // 235 - 200 - 30 = 5
        let hundreds = vx / 100;
        let tens = (vx - (hundreds * 100)) / 10;
        let ones = vx - (hundreds * 100) - (tens * 10);

        self.memory[self.i_addr + 0] = hundreds;
        self.memory[self.i_addr + 1] = tens;
        self.memory[self.i_addr + 2] = ones;
        self.pc += 2;
    }

    /// (Fx55) Store [V0..Vx] at I.
    fn opcode_store_vx(&mut self) {
        let x = self.opcode.x();

        for i in 0..=x {
            self.memory[self.i_addr + i] = self.v_reg[i];
        }
        self.pc += 2;
    }

    /// (Fx65) Fill [V0..Vx] from I.
    fn opcode_read_vx(&mut self) {
        let x = self.opcode.x();

        for i in 0..=x {
            self.v_reg[i] = self.memory[self.i_addr + i];
        }
        self.pc += 2;
    }

    // ----- End of opcodes ----- //

    fn decode_opcode(&mut self) {
        match self.opcode & 0xF000 {
            0x0000 => match self.opcode & 0x00FF {
                0x00E0 => self.opcode_cls(),
                0x00EE => self.opcode_ret(),
                _ => panic!("unknown opcode {}", self.opcode),
            },

            0x1000 => self.opcode_jp(),
            0x2000 => self.opcode_call(),
            0x3000 => self.opcode_se_byte(),
            0x4000 => self.opcode_sne_byte(),
            0x5000 => self.opcode_se_vx(),
            0x6000 => self.opcode_ld_byte(),
            0x7000 => self.opcode_add_byte(),

            0x8000 => match self.opcode & 0x000F {
                0x0000 => self.opcode_ld_vy(),
                0x0001 => self.opcode_or(),
                0x0002 => self.opcode_and(),
                0x0003 => self.opcode_xor(),
                0x0004 => self.opcode_add(),
                0x0005 => self.opcode_sub(),
                0x0006 => self.opcode_shr(),
                0x0007 => self.opcode_subn(),
                0x000E => self.opcode_shl(),
                _ => panic!("unknown opcode {}", self.opcode),
            },

            0x9000 => self.opcode_sne(),
            0xA000 => self.opcode_ld(),
            0xB000 => self.opcode_jp_v0(),
            0xC000 => self.opcode_rnd(),
            0xD000 => self.opcode_drw(),

            0xE000 => match self.opcode & 0xF0FF {
                0xE09E => self.opcode_skp(),
                0xE0A1 => self.opcode_sknp(),
                _ => panic!("unknown opcode {}", self.opcode),
            },

            0xF000 => match self.opcode & 0xF0FF {
                0xF007 => self.opcode_get_dt(),
                0xF00A => self.opcode_waitkey(),
                0xF015 => self.opcode_set_dt(),
                0xF018 => self.opcode_set_st(),
                0xF01E => self.opcode_add_i(),
                0xF029 => self.opcode_set_sprite(),
                0xF033 => self.opcode_bcd_vx(),
                0xF055 => self.opcode_store_vx(),
                0xF065 => self.opcode_read_vx(),
                _ => panic!("unknown opcode {}", self.opcode),
            },

            _ => panic!("unknown opcode {}", self.opcode),
        }
    }

    fn update_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize() {
        let c = CPU::new();

        assert_eq!(c.pc, 0x200);

        // make sure fontset was loaded
        assert_eq!(c.memory[FONTSET_START + 0], 0xF0);
        assert_eq!(c.memory[FONTSET_START + 1], 0x90);
        assert_eq!(c.memory[FONTSET_START + 65], 0xE0);
        assert_eq!(c.memory[FONTSET_START + 79], 0x80);
    }

    #[test]
    fn fetch_opcode() {
        let mut c = CPU::new();

        c.memory[c.pc] = 0xD6;
        c.memory[c.pc + 1] = 0x3E;
        c.fetch_opcode();

        assert_eq!(c.opcode, 0xD63E)
    }

    #[test]
    fn load_rom() {
        let mut c = CPU::new();
        c.load_rom("PONG");

        // test first two bytes
        assert_eq!(c.memory[0x200], 0x6A);
        assert_eq!(c.memory[0x201], 0x02);

        // test some bytes in the middle
        assert_eq!(c.memory[0x200 + 0xE0], 0xD4);
        assert_eq!(c.memory[0x201 + 0xE0], 0x55);
    }

    // opcode tests

    #[test]
    fn opcode_cls() {
        let mut c = CPU::new();

        c.display[0] = 1;
        c.display[DISPLAY_SIZE - 1] = 1;
        c.opcode = 0x00E0;
        c.decode_opcode();

        assert_eq!(c.display[0], 0);
        assert_eq!(c.display[DISPLAY_SIZE - 1], 0);
    }

    #[test]
    fn opcode_ret() {
        let mut c = CPU::new();

        c.stack[0] = 21;
        c.sp = 1;
        c.opcode = 0x00EE;
        c.decode_opcode();

        assert_eq!(c.pc, c.stack[0] + 2);
        assert_eq!(c.sp, 0);
    }

    #[test]
    fn opcode_jp() {
        let mut c = CPU::new();

        c.opcode = 0x1666;
        c.decode_opcode();

        assert_eq!(c.pc, 0x666);
    }

    #[test]
    fn opcode_call() {
        let mut c = CPU::new();

        c.opcode = 0x2666;
        c.pc = 0x51;
        c.sp = 1;
        c.stack[0] = 0x21;
        c.decode_opcode();

        assert_eq!(c.sp, 2);
        assert_eq!(c.stack[1], 0x51);
        assert_eq!(c.pc, 0x666);
    }

    #[test]
    fn opcode_se_byte() {}

    #[test]
    fn opcode_sne_byte() {}

    #[test]
    fn opcode_se_vx() {}

    #[test]
    fn opcode_ld_byte() {}

    #[test]
    fn opcode_add_byte() {}

    #[test]
    fn opcode_ld_vy() {}

    #[test]
    fn opcode_or() {}

    #[test]
    fn opcode_and() {}

    #[test]
    fn opcode_xor() {}

    #[test]
    fn opcode_add() {}

    #[test]
    fn opcode_sub() {}

    #[test]
    fn opcode_shr() {}

    #[test]
    fn opcode_subn() {}

    #[test]
    fn opcode_shl() {}

    #[test]
    fn opcode_sne() {}

    #[test]
    fn opcode_ld() {}

    #[test]
    fn opcode_jp_v0() {}

    #[test]
    fn opcode_rnd() {}

    #[test]
    fn opcode_drw() {
        let mut c = CPU::new();
        
        // coordinates: (63, 0) i.e. upper right corner of screen
        c.v_reg[0] = 63;
        c.v_reg[1] = 0;

        // Put a 2x2 cube at 0x755 in memory
        c.i_addr = 0x755;
        c.memory[c.i_addr] = 0xC0;
        c.memory[c.i_addr + 1] = 0xC0;

        // Turn the last pixel on row 0 on so we can test that it's turned off
        c.set_pixel(DISPLAY_WIDTH - 1, 1);

        // Draw 2-byte sprite at V0 and V1 (set above)
        c.opcode = 0xD012;

        c.decode_opcode();

        assert_eq!(c.get_pixel(DISPLAY_WIDTH - 1), 0, "pixel wasn't zeroed");
        assert_eq!(c.get_pixel(DISPLAY_WIDTH * 2 - 1), 1);
        assert_eq!(c.v_reg[0xF], 1, "carry bit should be set by collision");
        // wrapping
        assert_eq!(c.get_pixel(0), 1, "sprite should wrap");
        assert_eq!(c.get_pixel(DISPLAY_WIDTH), 1, "sprite should wrap ");
    }

    #[test]
    fn opcode_skp() {
        let mut c = CPU::new();
        
        c.keypad[0xA] = 1; // A is pressed
        c.v_reg[0xC] = 0xA;
        c.opcode = 0xEC9E;

        let old_pc = c.pc;

        c.decode_opcode();

        assert_eq!(c.pc, old_pc + 4);
    }

    #[test]
    fn opcode_sknp() {
        let mut c = CPU::new();

        c.keypad[0xA] = 0; // A is not pressed
        c.v_reg[0xC] = 0xA;
        c.opcode = 0xECA1;

        let old_pc = c.pc;

        c.decode_opcode();

        assert_eq!(c.pc, old_pc + 4);
    }

    #[test]
    fn opcode_ld_set_dt() {}

    #[test]
    fn opcode_ld_k() {}

    #[test]
    fn opcode_ld_get_dt() {}

    #[test]
    fn opcode_set_st() {}

    #[test]
    fn opcode_add_i() {}

    #[test]
    fn opcode_set_sprite() {
        let mut c = CPU::new();

        c.v_reg[0xA] = 0xA;
        c.opcode = 0xFA29; // A = get the sprite for 0xA
        c.decode_opcode();

        // Check to see that memory[I] holds the sprite for 0xA
        assert_eq!(c.memory[c.i_addr + 0], 0xF0);
        assert_eq!(c.memory[c.i_addr + 1], 0x90);
        assert_eq!(c.memory[c.i_addr + 2], 0xF0);
        assert_eq!(c.memory[c.i_addr + 3], 0x90);
        assert_eq!(c.memory[c.i_addr + 4], 0x90);
    }

    #[test]
    fn opcode_bcd_vx() {
        let mut c = CPU::new();

        c.v_reg[0x2] = 235;

        c.opcode = 0xF233; // Store BCD of V[2]
        c.decode_opcode();

        assert_eq!(c.memory[c.i_addr + 0], 2);
        assert_eq!(c.memory[c.i_addr + 1], 3);
        assert_eq!(c.memory[c.i_addr + 2], 5);
    }

    #[test]
    fn opcode_store_vx() {
        let mut c = CPU::new();

        c.v_reg[0x0] = 0xAA;
        c.v_reg[0x1] = 0xAB;
        c.v_reg[0x2] = 0xBB;

        c.opcode = 0xF255; // Store V0-V2 in memory at I
        c.i_addr = 0x932;
        c.decode_opcode();

        assert_eq!(c.memory[c.i_addr + 0], 0xAA);
        assert_eq!(c.memory[c.i_addr + 1], 0xAB);
        assert_eq!(c.memory[c.i_addr + 2], 0xBB);
    }

    #[test]
    fn opcode_read_vx() {
        let mut c = CPU::new();

        c.i_addr = 0x944;
        c.memory[c.i_addr + 0] = 0xCC;
        c.memory[c.i_addr + 1] = 0xCD;
        c.memory[c.i_addr + 2] = 0xDD;

        c.opcode = 0xF265; // Read V0-V2 from memory to V2
        c.decode_opcode();

        assert_eq!(c.v_reg[0x0], 0xCC);
        assert_eq!(c.v_reg[0x1], 0xCD);
        assert_eq!(c.v_reg[0x2], 0xDD);
    }

    #[test]
    fn update_keypad() {
        let mut c = CPU::new();

        c.update_keypad(Keycode::A, true);
        c.update_keypad(Keycode::F, true);
        c.update_keypad(Keycode::F, false);

        assert_eq!(c.keypad[0x7], 1);
        assert_eq!(c.keypad[0xE], 0);
    }

}
