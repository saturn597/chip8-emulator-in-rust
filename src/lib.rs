use std::fmt;

const INSTRUCTIONS_START: u16 = 0x200;

pub struct Chip8 {
    // 4k of RAM
    ram: [u8; 4096],

    stack: Vec<u16>,

    // registers
    v: [u8; 16],  // gen purpose
    i: u16,       // index/address
    pc: u16,      // program counter

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
            let location = i + (INSTRUCTIONS_START as usize);
            ram[location] = rom[i];
        }
        Chip8 {
            ram,
            //draw_flag: true,
            stack: Vec::new(),
            v: [0; 16],
            i: 0,
            //sp: 0,
            pc: INSTRUCTIONS_START,
            //keys: [false; 16],
            //delay_timer: 0,
            //sound_timer: 0,
        }
    }

    pub fn emulate_cycle(&mut self) {
        let instr = self.fetch();
        println!("Instruction: {}", instr);
        match (instr & 0xf000) >> 12 {
            0x2 => self.jump_subroutine(instr),
            0x6 => self.set_register(instr),
            0xa => self.set_index(instr),
            _ => panic!("unrecognized instruction!"),
        }
    }

    fn fetch(&self) -> u16 {
        self.fetch_at(self.pc)
    }

    fn fetch_at(&self, addr: u16) -> u16 {
        let addr = addr as usize;
        let first_byte = self.ram[addr] as u16;
        let second_byte = self.ram[addr + 1] as u16;
        first_byte << 8 | second_byte
    }

    // Opcodes
    fn jump_subroutine(&mut self, instr: u16) {
        self.stack.push(self.pc);
        self.pc = instr & 0x0fff;

        println!("jumped to subroutine at {}", self.pc);
    }

    fn set_index(&mut self, instr: u16) {
        // set the "I" register (index/address register)
        let value = instr & 0x0fff;
        self.i = value;

        println!("set I to {}", self.i);

        self.pc = self.pc + 2;
    }

    fn set_register(&mut self, instr: u16) {
        // set a general purpose register (one of the "V's")
        let reg = (instr & 0x0f00) >> 8;
        let reg = reg as usize;
        let value = (instr & 0x00ff) as u8;

        self.v[reg] = value;

        println!("Set V{} to {}", reg, self.v[reg]);

        self.pc = self.pc + 2;
    }
}

impl fmt::Display for Chip8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "At {}, instruction {}", self.pc, self.ram[self.pc as usize])
    }
}

pub fn run(rom: Vec<u8>) {
    let mut chip8 = Chip8::initialize(rom);
    loop {
        chip8.emulate_cycle();
    }
}
