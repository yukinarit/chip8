use std::io::{BufRead, Write};
use std::path::PathBuf;

use structopt::StructOpt;

use core::{Chip8, Error};

#[derive(Debug, StructOpt)]
#[structopt(name = "c8db", about = "c8db program options.")]
struct Option {
    rom: PathBuf,
}

fn prompt() {
    print!("> ");
    std::io::stdout().flush().unwrap();
}

fn main() -> Result<(), Error> {
    let opts = Option::from_args();
    env_logger::init();
    let mut chip8 = Chip8::new();
    let rom = &opts.rom.canonicalize().unwrap();
    let file = std::fs::File::open(&rom.to_str().unwrap()).unwrap();
    chip8.ram.load(file)?;

    let cpu = &mut chip8.cpu;
    let ram = &mut chip8.ram;
    let stdin = std::io::stdin();
    loop {
        prompt();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap();

        cpu.cycle(ram, &mut None);
        cpu.dump();
    }

    Ok(())
}
