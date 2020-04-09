use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please provide a filename");
        process::exit(1);
    }

    let rom = fs::read(&args[1]).unwrap_or_else(|err| {
        println!("Couldn't open file: {}", err);
        process::exit(1);
    });

    chip8::run(rom);
}

