use std::fmt;

const INSTRUCTIONS_START: u16 = 0x200;

#[derive(Copy,Clone,PartialEq)]
enum Pixel {
    On,
    Off,
}

impl Pixel {
    fn flip(&self) -> Pixel {
        if *self == Pixel::On {
            Pixel::Off
        } else {
            Pixel::On
        }
    }
}

pub struct Chip8 {
    // 4k of RAM
    ram: [u8; 4096],

    stack: Vec<u16>,

    pixels: [[Pixel; 32]; 64],

    // registers
    v: [u8; 16],  // gen purpose
    i: u16,       // index/address
    pc: u16,      // program counter

    // state of keys
    //keys: [bool; 16],

    //delay_timer: u8,
    //sound_timer: u8,

    draw_flag: bool,
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
            draw_flag: true,
            stack: Vec::new(),
            pixels: [[Pixel::Off; 32]; 64],
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
            0xd => self.draw_sprite(instr),
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

    fn show(&self) {
        for y in 0..32 {
            for x in 0..64 {
                print!("{}", self.pixels[x][y]);
            }
            println!("");
        }
    }

    // Opcodes
    fn draw_sprite(&mut self, instr: u16) {
        let instr = instr as usize;

        let x_reg = (instr & 0x0f00) >> 8;
        let y_reg = (instr & 0x00f0) >> 4;

        let n = instr & 0x000f;

        let x_start = self.v[x_reg] as usize;
        let y_start = self.v[y_reg] as usize;

        let mem_start = self.i as usize;

        let mut collision = false;

        for i in 0..n {
            let mem_location = mem_start + i;
            let byte = self.ram[mem_location];
            for j in (0..8).rev() {
                let x = x_start + j;
                let y = y_start + i;
                let needs_flip = byte & (1 << j) > 0;
                let pixel = self.pixels[x][y];
                if needs_flip {
                    if self.pixels[x][y] == Pixel::On {
                        collision = true;
                    }
                    self.pixels[x][y] = pixel.flip();
                }
            }
        }

        self.v[0xf] = if collision {1} else {0};

        self.pc = self.pc + 2;
    }

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

impl fmt::Display for Pixel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = match *self {
            Pixel::On => "*",
            Pixel::Off => " ",
        };
        write!(f, "{}", output)
    }
}

pub fn run(rom: Vec<u8>) {
    let mut chip8 = Chip8::initialize(rom);
    loop {
        chip8.emulate_cycle();

        if chip8.draw_flag {
            chip8.show();
        }
    }
}
