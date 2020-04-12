use ncurses;
use rand::Rng;
use std::char;
use std::collections::HashMap;
use std::fmt;
use std::thread;
use std::time;

const KEYBOARD_MAP: [(char, usize); 16] = [
    ('1', 1),
    ('2', 2),
    ('3', 3),
    ('q', 4),
    ('w', 5),
    ('e', 6),
    ('a', 7),
    ('s', 8),
    ('d', 9),
    ('x', 0),
    ('z', 0xa),
    ('c', 0xb),
    ('4', 0xc),
    ('r', 0xd),
    ('f', 0xe),
    ('v', 0xf),
];

const INSTRUCTIONS_START: u16 = 0x200;
const SCREEN_WIDTH: usize = 64;
const SCREEN_HEIGHT: usize = 32;

const FONT: [u8; 80] = [
  0xf0, 0x90, 0x90, 0x90, 0xf0, // 0
  0x20, 0x60, 0x20, 0x20, 0x70, // 1
  0xf0, 0x10, 0xf0, 0x80, 0xf0, // 2
  0xf0, 0x10, 0xf0, 0x10, 0xf0, // 3
  0x90, 0x90, 0xf0, 0x10, 0x10, // 4
  0xf0, 0x80, 0xf0, 0x10, 0xf0, // 5
  0xf0, 0x80, 0xf0, 0x90, 0xf0, // 6
  0xf0, 0x10, 0x20, 0x40, 0x40, // 7
  0xf0, 0x90, 0xf0, 0x90, 0xf0, // 8
  0xf0, 0x90, 0xf0, 0x10, 0xf0, // 9
  0xf0, 0x90, 0xf0, 0x90, 0x90, // a
  0xe0, 0x90, 0xe0, 0x90, 0xe0, // b
  0xf0, 0x80, 0x80, 0x80, 0xf0, // c
  0xe0, 0x90, 0x90, 0x90, 0xe0, // d
  0xf0, 0x80, 0xf0, 0x80, 0xf0, // e
  0xf0, 0x80, 0xf0, 0x80, 0x80  // f
];
const FONT_START: usize = 0x50;

#[derive(Copy,Clone,PartialEq)]
enum Pixel {
    On,
    Off,
}


#[derive(Copy,Clone,PartialEq)]
enum Key {
    Up,
    Down,
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

pub struct Timer {
    start_count: u8,
    start_instant: time::Instant,
}

impl Timer {
    fn initialize() -> Timer {
        Timer {
            start_count: 0,
            start_instant: time::Instant::now(),
        }
    }

    fn start(&mut self, count: u8) {
        self.start_count = count;
        self.start_instant = time::Instant::now();
    }

    fn get_value(&self) -> u8 {
        let now = time::Instant::now();
        let millis = now.duration_since(self.start_instant).as_millis() as f32;
        let ticks = millis * 60. / 1000.;
        self.start_count.saturating_sub(ticks.floor() as u8)
    }
}

pub struct Chip8 {
    // 4k of RAM
    ram: [u8; 4096],

    stack: Vec<u16>,

    pixels: [[Pixel; SCREEN_HEIGHT]; SCREEN_WIDTH],

    // registers
    v: [u8; 16],  // gen purpose
    i: u16,       // index/address
    pc: u16,      // program counter

    // state of keys
    keys: [Key; 16],

    delay_timer: Timer,
    sound_timer: u8,  // TODO: need to implement this so it counts down

    draw_queue: Vec<(u8, u8, Pixel)>,

}

impl Chip8 {
    fn initialize(rom: Vec<u8>) -> Chip8 {
        let mut ram = [0; 4096];
        // TODO: verify rom length < ram length - 0x200
        for i in 0..rom.len() {
            let location = i + (INSTRUCTIONS_START as usize);
            ram[location] = rom[i];
        }

		for i in 0..FONT.len() {
            // TODO: generalize this - maybe an array_to_ram method?
			let location = i + (FONT_START as usize);	
			ram[location] = FONT[i]; 
		}

        Chip8 {
            ram,
            stack: Vec::new(),
            pixels: [[Pixel::Off; SCREEN_HEIGHT]; SCREEN_WIDTH],
            v: [0; 16],
            i: 0,
            //sp: 0,
            pc: INSTRUCTIONS_START,
            keys: [Key::Up; 16],
            
            delay_timer: Timer::initialize(),
            sound_timer: 0,
            
            draw_queue: Vec::new(),
        }
    }

    pub fn emulate_cycle(&mut self) {
        let instr = self.fetch();
        //println!("Instruction: {}", instr);
        match (instr & 0xf000) >> 12 {
            0x0 => {
                match instr & 0x0fff {
                    0x0e0 => self.clear_screen(instr),
                    0x0ee => self.ret(instr),
                    _ => panic!("RCA 1802 program? Instr: {}", instr),
                }
            },
            0x1 => self.jump(instr),
            0x2 => self.jump_subroutine(instr),
            0x3 => self.skip_if_equal(instr),
            0x4 => self.skip_if_unequal(instr),
            0x6 => self.set_register(instr),
            0x7 => self.add_const_to_v(instr),
            0x8 => {
                match instr & 0x00f {
                    0x0 => self.reg_set(instr),
                    0x2 => self.reg_and(instr),
                    0x4 => self.reg_add(instr),
                    0x5 => self.reg_subtract(instr),
                    0x6 => self.shift_right(instr),
                    _ => panic!("unrecognized instruction/leading 8: {}", instr),
                }
            },
            0x9 => {
                match instr & 0x000f {
                    0 => self.skip_if_regs_unequal(instr),
                    _ => panic!("unrecognized instruction/leading 9: {}", instr),
                }
            },
            0xa => self.set_index(instr),
            0xc => self.rand(instr),
            0xd => self.draw_sprite(instr),
            0xe => {
                match instr & 0x00ff {
                    0x9e => self.skip_if_key(instr),
                    0xa1 => self.skip_if_not_key(instr),
                    _ => panic!("unrecognized instruction/leading e: {}", instr),
                }
            },
            0xf => {
                match instr & 0x00ff {
                    0x07 => self.get_delay_timer(instr),
                    0x15 => self.set_delay_timer(instr),
                    0x18 => self.set_sound_timer(instr),
                    0x1e => self.add_reg_to_i(instr),
                    0x29 => self.set_char_location(instr),
                    0x33 => self.set_bcd(instr),
                    0x65 => self.reg_load(instr),
                    _ => panic!("unrecognized instruction/leading f: {}", instr),
                }
            }
            _ => panic!("unrecognized instruction: {}", instr),
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
    fn add_const_to_v(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;
        let n = (instr & 0x00ff) as u8;

        self.v[reg] = self.v[reg].wrapping_add(n);
        //println!("V{} == {}", reg, self.v[reg]);
        self.pc = self.pc + 2;
    }

    fn add_reg_to_i(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;

        // I is actually a 12 bit value, so overflow if > 4095
        self.i = self.i + (self.v[reg] as u16);
        if self.i > 4095 {
            self.v[0xf] = 1;
        } else {
            self.v[0xf] = 0;
        }
        self.i = self.i % 4096;

        self.pc = self.pc + 2;
    }

    fn clear_screen(&mut self, _instr: u16) {
        // TODO: should add all pixels to self.draw_queue
        self.pixels = [[Pixel::Off; SCREEN_HEIGHT]; SCREEN_WIDTH];
        self.pc = self.pc + 2;
    }

    fn draw_sprite(&mut self, instr: u16) {
        let instr = instr as usize;

        let x_reg = (instr & 0x0f00) >> 8;
        let y_reg = (instr & 0x00f0) >> 4;

        let n = instr & 0x000f;

        let x_start = self.v[x_reg] as usize;
        let y_start = self.v[y_reg] as usize;
        //println!("x: {}, y: {}", x_start, y_start);
        //println!("n: {}", n);

        let mem_start = self.i as usize;

        let mut collision = false;

        for i in 0..n {
            let mem_location = mem_start + i;
            let byte = self.ram[mem_location];
            let y = y_start + i;
            if y >= SCREEN_HEIGHT {
                continue;
            }
            for j in 0..8 {
                let x = x_start + j;
                if x >= SCREEN_WIDTH {
                    continue;
                }
                let needs_flip = byte & (1 << (7-j)) > 0;
                let pixel = self.pixels[x][y];
                if needs_flip {
                    if self.pixels[x][y] == Pixel::On {
                        collision = true;
                    }
                    self.pixels[x][y] = pixel.flip();
                    self.draw_queue.push((x as u8, y as u8, self.pixels[x][y]));
                }
            }
        }

        self.v[0xf] = if collision {1} else {0};

        self.pc = self.pc + 2;
    }

    fn get_delay_timer(&mut self, instr: u16) {
        let reg = (instr & 0x0f00) >> 8;
        let reg = reg as usize;

        self.v[reg] = self.delay_timer.get_value();

        //println!("Got delay_timer: {}", self.v[reg]);
        self.pc = self.pc + 2;
    }

    fn jump(&mut self, instr: u16) {
        self.pc = instr & 0x0fff;
    }

    fn jump_subroutine(&mut self, instr: u16) {
        self.stack.push(self.pc);
        self.pc = instr & 0x0fff;

        //println!("jumped to subroutine at {}", self.pc);
    }

    fn rand(&mut self, instr: u16) {
        let reg = (instr & 0x0f00) >> 8;
        let reg = reg as usize;

        let random = rand::thread_rng().gen_range(0, 255) as u8;
        let val = (instr & 0x00ff) as u8;
        
        self.v[reg] = val & random;

        self.pc = self.pc + 2;
    }

    fn reg_get_for_math(&mut self, instr: u16) -> (usize, usize) {
        (
            ((instr & 0x0f00) >> 8) as usize,
            ((instr & 0x00f0) >> 4) as usize,
        )
    }

    fn reg_add(&mut self, instr: u16) {
        let (reg1, reg2) = self.reg_get_for_math(instr);

        let val1 = self.v[reg1];
        let val2 = self.v[reg2];

        let (sum, overflow) = val1.overflowing_add(val2);

        self.v[0xf] = if overflow {1} else {0};

        //println!("V{} was {} and V{} was {}", reg1, self.v[reg1], reg2, self.v[reg2]);
        //println!("result should be {}", sum);
        //println!("VF is {}", self.v[0xf]);

        self.v[reg1] = sum;

        //println!("result is: {}", self.v[reg1]);

        self.pc = self.pc + 2;
    }

    fn reg_and(&mut self, instr: u16) {
        let (reg1, reg2) = self.reg_get_for_math(instr);

        let result = self.v[reg1] & self.v[reg2];

        //println!("V{} was {} and V{} was {}", reg1, self.v[reg1], reg2, self.v[reg2]);
        //println!("result should be {}", result);

        self.v[reg1] = result as u8;

        //println!("result is: {}", self.v[reg1]);

        self.pc = self.pc + 2;
    }

    fn reg_load(&mut self, instr: u16) {
        let count = ((instr & 0x0f00) >> 8) + 1;
        //println!("count: {}", count);
        //println!("contents of &I: {} {} {}", self.ram[self.i as usize], self.ram[self.i as usize + 1], self.ram[self.i as usize + 2]);
        for reg in 0..count {
            let mem_location = (self.i + reg) as usize;
            self.v[reg as usize] = self.ram[mem_location];
            //println!("Stored {} in V{}", self.v[reg as usize], reg);
        }

        self.pc = self.pc + 2;
    }

    fn reg_set(&mut self, instr: u16) {
        let (reg1, reg2) = self.reg_get_for_math(instr);
        self.v[reg1] = self.v[reg2];

        self.pc = self.pc + 2;
    }

    fn reg_subtract(&mut self, instr: u16) {
        let (reg1, reg2) = self.reg_get_for_math(instr);

        let val1 = self.v[reg1];
        let val2 = self.v[reg2];

        let (sum, overflow) = val1.overflowing_sub(val2);

        self.v[0xf] = if overflow {0} else {1};

        //panic!("subtracting {} from {} with result {}, overflow {}", val2, val1, sum, overflow);

        //println!("V{} was {} and V{} was {}", reg1, self.v[reg1], reg2, self.v[reg2]);
        //println!("result should be {}", sum);
        //println!("VF is {}", self.v[0xf]);

        self.v[reg1] = sum;

        //println!("result is: {}", self.v[reg1]);

        self.pc = self.pc + 2;
    }

    fn ret(&mut self, _instr: u16) {
        let addr = self.stack.pop().unwrap_or_else(|| {
            panic!("Error popping stack");
        });

        self.pc = addr + 2;
        //println!("returned from subroutine to {}", self.pc);
    }

    fn set_bcd(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;
        let val = self.v[reg];

        let hundreds = val / 100;
        let tens = (val - 100 * hundreds) / 10;
        let ones = val - 100 * hundreds - 10 * tens;
        //println!("val: {}; hundreds: {}, tens: {}, ones: {}", val, hundreds, tens, ones);

        let start = self.i as usize;
        self.ram[start] = hundreds;
        self.ram[start + 1] = tens;
        self.ram[start + 2] = ones;

        self.pc = self.pc + 2;
    }

    fn set_char_location(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;
        let ch = self.v[reg] as usize;
        self.i = (FONT_START + ch * 5) as u16;

        self.pc = self.pc + 2;
    }

    fn set_delay_timer(&mut self, instr: u16) {
        let reg = (instr & 0x0f00) >> 8;
        let reg = reg as usize;

        self.delay_timer.start(self.v[reg]);

        //println!("set delay_timer to {} based on register {}", self.v[reg], reg);
        
        self.pc = self.pc + 2;
    }

    fn set_index(&mut self, instr: u16) {
        // set the "I" register (index/address register)
        let value = instr & 0x0fff;
        self.i = value;

        //println!("set I to {}", self.i);

        self.pc = self.pc + 2;
    }

    fn set_register(&mut self, instr: u16) {
        // set a general purpose register (one of the "V's")
        let reg = (instr & 0x0f00) >> 8;
        let reg = reg as usize;
        let value = (instr & 0x00ff) as u8;

        self.v[reg] = value;

        //println!("Set V{} to {}", reg, self.v[reg]);

        self.pc = self.pc + 2;
    }

    fn set_sound_timer(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;
        self.sound_timer = self.v[reg];
        //println!("setting sound_timer to {}", self.sound_timer);

        self.pc = self.pc + 2;
    }

    fn shift_right(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;
        let val = self.v[reg];

        self.v[0xf] = 1 & val;
        self.v[reg] = val >> 1;

        self.pc = self.pc + 2;
    }

    fn skip_if_equal(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;
        let n = (instr & 0x00ff) as u8;

        let incr = if self.v[reg] == n {4} else {2};
        //println!("Incrementing by {}", incr);
        self.pc = self.pc + incr;
    }

    fn skip_if_regs_unequal(&mut self, instr: u16) {
       let (reg1, reg2) = self.reg_get_for_math(instr); 
       let incr = if self.v[reg1] != self.v[reg2] {4} else {2};
       self.pc = self.pc + incr;
    }

    fn skip_if_unequal(&mut self, instr: u16) {
        let reg = ((instr & 0x0f00) >> 8) as usize;
        let n = (instr & 0x00ff) as u8;
        let incr = if self.v[reg] == n {2} else {4};
        self.pc = self.pc + incr;
    }

    fn skip_if_key(&mut self, instr: u16) {
        let reg = (instr & 0x0f00) >> 8;
        let reg = reg as usize;

        let incr = match self.test_key(self.v[reg]) {
            Key::Up => 2,
            Key::Down => 4,
        };

        self.pc = self.pc + incr;
    }

    fn skip_if_not_key(&mut self, instr: u16) {
        let reg = (instr & 0x0f00) >> 8;
        let reg = reg as usize;

        let incr = match self.test_key(self.v[reg]) {
            Key::Up => 4,
            Key::Down => 2,
        };
        self.pc = self.pc + incr;
    }

    fn test_key(&mut self, key_index: u8) -> Key {
        // This isn't right - in the Chip8, keys don't get "reset" when read. However, ncurses
        // doesn't detect "key up" events, so this seems like a good place to set they key back to
        // up.
        let key_index = key_index as usize;
        let key = self.keys[key_index].clone();
        self.keys[key_index] = Key::Up;
        key
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
    let keyboard: HashMap<char, usize> = KEYBOARD_MAP.iter().cloned().collect();

    ncurses::initscr();
    ncurses::raw();
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    ncurses::nodelay(ncurses::stdscr(), true);
    ncurses::noecho();

    let mut chip8 = Chip8::initialize(rom);
    loop {
        let ch = char::from_u32(ncurses::getch() as u32);

        if let Some(k) = ch {
            if let Some(key) = keyboard.get(&k) {
                chip8.keys[*key] = Key::Down;
            }
        }

        chip8.emulate_cycle();


        for item in chip8.draw_queue.iter() {
            let (x, y, pixel) = item;

            let ch = match pixel {
                Pixel::On => '#',
                Pixel::Off => ' ',
            };
            ncurses::mvaddch(*y as i32, *x as i32, ch as ncurses::chtype);
        }
        ncurses::refresh();
        chip8.draw_queue.clear();


        let duration = time::Duration::from_millis(2);
        thread::sleep(duration);  // TODO: fix the timing
    }
}
