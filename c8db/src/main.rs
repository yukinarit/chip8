use std::io::Read;

use core::{Chip8, Error};

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut chip8 = Chip8::new();
    let file = std::fs::File::open("helloworld.rom")?;
    chip8.ram.load(file)?;
    chip8.run();

    Ok(())
}
