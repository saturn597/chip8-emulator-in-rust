use std::fmt;

const INSTRUCTIONS_START: usize = 0x200;

pub struct Chip8 {
    // 4k of RAM
    ram: [u8; 4096],

    //stack: [u16; 16],

    // registers
    //v: [u8; 16],
    //i: u16,
    //sp: u16,
    pc: u16,

    // state of keys
    //keys: [bool; 16],

    //delay_timer: u8,
    //sound_timer: u8,

    //draw_flag: bool,
}

impl Chip8 {
    fn initialize(rom: Vec<u8>) -> Chip8 {
        let mut ram = [0; 4096];
        // TODO: verify rom length < ram length - 0x200
        for i in 0..rom.len() {
            ram[i + INSTRUCTIONS_START] = rom[i];
        }
        Chip8 {
            ram,
            //draw_flag: true,
            //stack: [0; 16],
            //v: [0; 16],
            //i: 0,
            //sp: 0,
            pc: INSTRUCTIONS_START as u16,
            //keys: [false; 16],
            //delay_timer: 0,
            //sound_timer: 0,
        }
    }
}

impl fmt::Display for Chip8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "At {}, instruction {}", self.pc, self.ram[self.pc as usize])
    }
}

pub fn run(rom: Vec<u8>) {
    let chip8 = Chip8::initialize(rom);
    println!("{}", chip8);
}
