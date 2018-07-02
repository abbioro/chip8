use std::fs::File;
use std::io::Read;

const PROGRAM_ROM_START: u16 = 0x200; // Programs start at 0x200
const DISPLAY_SIZE: usize = 64 * 32;

#[allow(dead_code)]
pub struct Chip8 {
    opcode: u16, // current opcode
    memory: [u8; 4096],
    v_reg: [u8; 16], // registers
    i_addr: u16, // address register
    pc: u16, // program counter
    display: [u8; DISPLAY_SIZE],
    stack: [u16; 16],
    sp: u8, // stack pointer
    delay_timer: u8,
    sound_timer: u8,
    keypad: [u8; 16],
}

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

impl Chip8 {
    pub fn new() -> Chip8 {
        Chip8 {
            opcode: 0,
            memory: [0; 4096],
            v_reg: [0; 16],
            i_addr: 0,
            pc: 0,
            display: [0; DISPLAY_SIZE],
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [0; 16],            
        }
    }

    pub fn initialize(&mut self) {
        // Reset everything
        self.opcode = 0;
        self.memory = [0; 4096];
        self.v_reg = [0; 16];
        self.i_addr = 0;
        self.pc = PROGRAM_ROM_START;
        self.display = [0; DISPLAY_SIZE];
        self.stack = [0; 16];
        self.sp = 0;
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.keypad = [0; 16];

        // Load fontset into memory
        for i in 0..80 {
            self.memory[i] = CHIP8_FONTSET[i];
        }
    }

    /// Load a program ROM into memory
    pub fn load_rom(&mut self, filename: &str) {
        let mut file = File::open(filename).unwrap();

        // Reads up to memory (4 KB) bytes
        file.read(&mut self.memory[(PROGRAM_ROM_START as usize)..]).unwrap();
    }

    pub fn emulate_cycle(&mut self) {
        self.fetch_opcode();
        self.decode_opcode();
        self.update_timers();
    }

    fn fetch_opcode(&mut self) {
        let pc = self.pc as usize;

        // Bytes are cast into u16 so we can merge them next
        let byte1 = self.memory[pc] as u16;
        let byte2 = self.memory[pc + 1] as u16;

        // Merge the 2-byte instruction at the program counter
        self.opcode = byte1 << 8 | byte2
    }

    // Vx = Register X
    // kk = byte

    /// Clear the display.
    fn opcode_cls(&mut self) {
        self.display = [0; DISPLAY_SIZE];
    }

    /// Return from a subroutine.
    fn opcode_ret(&mut self) {
        // NOTE: Original spec says decrement after popping stack, not sure if
        // that matters
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    /// Jump to location.
    fn opcode_jp(&mut self) {
        self.pc = self.opcode & 0x0FFF;
    }

    /// Call subroutine.
    fn opcode_call(&mut self) {
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = self.opcode & 0x0FFF;
    }

    /// Skip next instruction if Vx == kk
    fn opcode_se_byte(&mut self) {
        if self.v_reg[(self.opcode & 0x0F00) as usize] == (self.opcode & 0x00FF) as u8 {
            self.pc += 2;
        }
    }

    /// Skip next instruction if Vx != kk
    fn opcode_sne_byte(&mut self) {
        if self.v_reg[(self.opcode & 0x0F00) as usize] != (self.opcode & 0x00FF) as u8 {
            self.pc += 2;
        }
    }

    /// Skip next instruction if Vx == Vy
    fn opcode_se_vx(&mut self) {
        if self.v_reg[(self.opcode & 0x0F00) as usize] == self.v_reg[(self.opcode & 0x00F0) as usize] {
            self.pc += 2;
        }
    }

    /// Set Vx to kk
    fn opcode_ld_byte(&mut self) {
        self.v_reg[(self.opcode & 0x0F00) as usize] = (self.opcode & 0x00FF) as u8;
    }

    /// Add kk to Vx
    fn opcode_add_byte(&mut self) {
        self.v_reg[(self.opcode & 0x0F00) as usize] += (self.opcode & 0x00FF) as u8;
    }

    /// Set Vx to Vy
    fn opcode_ld_vy(&mut self) {
        self.v_reg[(self.opcode & 0x0F00) as usize] = self.v_reg[(self.opcode & 0x00F0) as usize];
    }

    /// Bitwise OR on Vx and Vy, storing result in Vx
    fn opcode_or(&mut self) {
        self.v_reg[(self.opcode & 0x0F00) as usize] |= self.v_reg[(self.opcode & 0x00F0) as usize];
    }

    /// Bitwise AND on Vx and Vy, storing result in Vx
    fn opcode_and(&mut self) {
        self.v_reg[(self.opcode & 0x0F00) as usize] &= self.v_reg[(self.opcode & 0x00F0) as usize];
    }

    /// Bitwise XOR
    fn opcode_xor(&mut self) {
        self.v_reg[(self.opcode & 0x0F00) as usize] ^= self.v_reg[(self.opcode & 0x00F0) as usize];
    }

    /// Add Vy to Vx, set VF to carry
    fn opcode_add(&mut self) {
        let result: u16 = (self.v_reg[(self.opcode & 0x0F00) as usize] as u16) + (self.v_reg[(self.opcode & 0x00F0) as usize] as u16);
        
        if result > 255 {
            self.v_reg[0xF] = 1;
        } else {
            self.v_reg[0xF] = 0;
        }

        self.v_reg[(self.opcode & 0x0F00) as usize] = result as u8;
    }

    /// Subtract Vy from Vx, set VF
    fn opcode_sub(&mut self) {
        
    }

    /// Right shift
    fn opcode_shr(&mut self) {

    }

    fn opcode_subn(&mut self) {

    }

    fn opcode_shl(&mut self) {

    }

    fn opcode_sne(&mut self) {

    }

    // Set memory address
    fn opcode_ld(&mut self) {
        self.i_addr = self.opcode & 0x0FFF;
        self.pc += 2;
    }

    fn opcode_jp_v0(&mut self) {

    }

    fn opcode_rnd(&mut self) {

    }

    fn opcode_drw(&mut self) {

    }

    fn opcode_skp(&mut self) {

    }

    fn opcode_sknp(&mut self) {

    }

    fn opcode_ld_set_dt(&mut self) {

    }

    fn opcode_ld_k(&mut self) {

    }

    fn opcode_ld_get_dt(&mut self) {

    }

    fn opcode_set_st(&mut self) {

    }

    fn opcode_add_i(&mut self) {

    }

    fn opcode_set_sprite(&mut self) {

    }

    fn opcode_bcd_vx(&mut self) {

    }

    fn opcode_store_vx(&mut self) {

    }

    fn opcode_read_vx(&mut self) {

    }

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
            }

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
                0xF007 => self.opcode_ld_set_dt(),
                0xF00A => self.opcode_ld_k(),
                0xF015 => self.opcode_ld_get_dt(),
                0xF018 => self.opcode_set_st(),
                0xF01E => self.opcode_add_i(),
                0xF029 => self.opcode_set_sprite(),
                0xF033 => self.opcode_bcd_vx(),
                0xF055 => self.opcode_store_vx(),
                0xF065 => self.opcode_read_vx(),
                _ => panic!("unknown opcode {}", self.opcode),    
            }

            _ => panic!("unknown opcode {}", self.opcode),
        }
    }

    fn update_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            println!("BEEP!"); // TODO: replace with sound code
            self.sound_timer -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize() {
        let mut c = Chip8::new();

        c.initialize();
        assert_eq!(c.pc, 0x200);
    }

    #[test]
    fn fetch_opcode() {
        let mut c = Chip8::new();

        c.initialize();
        c.memory[c.pc as usize] = 0xD6;
        c.memory[(c.pc + 1) as usize] = 0x3E;
        c.fetch_opcode();

        assert_eq!(c.opcode, 0xD63E)
    }

    #[test]
    fn load_rom() {
        let mut c = Chip8::new();
        
        c.initialize();
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
        let mut c = Chip8::new();
        c.initialize();

        c.display[0] = 1;
        c.display[DISPLAY_SIZE - 1] = 1;
        c.opcode = 0x00E0;
        c.decode_opcode();
        
        assert_eq!(c.display[0], 0);
        assert_eq!(c.display[DISPLAY_SIZE - 1], 0);
    }

    #[test]
    fn opcode_ret() {
        let mut c = Chip8::new();
        c.initialize();

        c.stack[0] = 21;
        c.sp = 1;
        c.opcode = 0x00EE;
        c.decode_opcode();

        assert_eq!(c.pc, c.stack[0]);
        assert_eq!(c.sp, 0);
    }

    #[test]
    fn opcode_jp() {
        let mut c = Chip8::new();
        c.initialize();

        c.opcode = 0x1666;
        c.decode_opcode();

        assert_eq!(c.pc, 0x666);
    }

    #[test]
    fn opcode_call() {
        let mut c = Chip8::new();
        c.initialize();

        c.opcode = 0x2666;
        c.pc = 0x51;
        c.sp = 1;
        c.stack[0] = 0x21;
        c.decode_opcode();

        assert_eq!(c.sp, 2);
        assert_eq!(c.stack[1], 0x51);
        assert_eq!(c.pc, c.opcode & 0x0FFF);
    }

    #[test]
    fn opcode_se_byte() {
        
    }

    #[test]
    fn opcode_sne_byte() {
        
    }

    #[test]
    fn opcode_se_vx() {
        
    }

    #[test]
    fn opcode_ld_byte() {
        
    }

    #[test]
    fn opcode_add_byte() {
        
    }

    #[test]
    fn opcode_ld_vy() {
        
    }

    #[test]
    fn opcode_or() {
        
    }

    #[test]
    fn opcode_and() {
        
    }

    #[test]
    fn opcode_xor() {
        
    }

    #[test]
    fn opcode_add() {
        
    }

    #[test]
    fn opcode_sub() {
        
    }

    #[test]
    fn opcode_shr() {
        
    }

    #[test]
    fn opcode_subn() {
        
    }

    #[test]
    fn opcode_shl() {
        
    }

    #[test]
    fn opcode_sne() {
        
    }

    #[test]
    fn opcode_ld() {
        
    }

    #[test]
    fn opcode_jp_v0() {
        
    }

    #[test]
    fn opcode_rnd() {
        
    }

    #[test]
    fn opcode_drw() {
        
    }

    #[test]
    fn opcode_skp() {
        
    }

    #[test]
    fn opcode_sknp() {
        
    }

    #[test]
    fn opcode_ld_set_dt() {
        
    }

    #[test]
    fn opcode_ld_k() {
        
    }

    #[test]
    fn opcode_ld_get_dt() {
        
    }

    #[test]
    fn opcode_set_st() {
        
    }

    #[test]
    fn opcode_add_i() {
        
    }

    #[test]
    fn opcode_set_sprite() {
        
    }

    #[test]
    fn opcode_bcd_vx() {
        
    }

    #[test]
    fn opcode_store_vx() {
        
    }

    #[test]
    fn opcode_read_vx() {
        
    }


}
