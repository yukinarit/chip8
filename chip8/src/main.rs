use std::collections::VecDeque;
use std::default::Default;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use core::{Chip8, Screen};
use log::*;
use rustbox::{
    Color::{self, Black, White},
    Key, RustBox, Style, RB_BOLD,
};
use structopt::StructOpt;

type Tx = mpsc::Sender<Cmd>;

type Rx = mpsc::Receiver<Cmd>;

static PIXEL: char = ' ';

#[derive(Debug, StructOpt)]
#[structopt(name = "chip8", about = "chip8 program options.")]
struct Option {
    rom: PathBuf,
}

enum Cmd {
    Draw((u8, u8, Vec<u8>)),
    Clear,
}

struct Adaptor {
    tx: Tx,
}

impl Adaptor {
    fn new(tx: Tx) -> Adaptor {
        Adaptor { tx: tx }
    }
}

impl Screen for Adaptor {
    fn draw(&self, x: u8, y: u8, data: Vec<u8>) -> Result<(), ()> {
        self.tx
            .send(Cmd::Draw((x, y, data)))
            .map_err(|e| error!("{:?}", e))
    }

    fn clear(&self) {
        self.tx.send(Cmd::Clear).unwrap()
    }
}

fn bitarray(data: &Vec<u8>) -> Vec<u8> {
    let mut s = Vec::new();
    for byte in data {
        for n in 0..8 {
            if byte & (1 << (8 - n - 1)) > 0 {
                s.push(1);
            } else {
                s.push(0);
            }
        }
    }
    s
}

fn wrap(x_: u8, y_: u8, data: &Vec<u8>) -> VecDeque<Cell> {
    let mut cells = VecDeque::new();
    let mut x = x_;
    let mut y = y_;
    for byte in data.chunks(8) {
        for b in byte {
            //if (x + 1) > 63 {
            //    x = x_;
            //    y += 1;
            //} else {
            //    x += 1;
            //}
            if *b == 1 {
                let cell = Cell::new(x, y, RB_BOLD, White, White, PIXEL);
                cells.push_back(cell);
            }
            x += 1;
        }
        x = x_;
        y += 1;
    }

    cells
}

struct Cell {
    x: u8,
    y: u8,
    st: Style,
    bg: Color,
    fg: Color,
    ch: char,
}

impl Cell {
    fn new(x: u8, y: u8, st: Style, bg: Color, fg: Color, ch: char) -> Self {
        Cell {
            x: x,
            y: y,
            st: st,
            bg: bg,
            fg: fg,
            ch: ch,
        }
    }
}

struct Console;

impl Console {
    fn new() -> Self {
        Console {}
    }

    fn run(&mut self, mut chip8: Chip8, rx: Rx) -> Result<(), ()> {
        let mut rb = RustBox::init(Default::default()).unwrap();

        rb.present();
        let timeout = Duration::from_millis(1);
        loop {
            // Poll UI event.
            match rb.peek_event(timeout.clone(), false) {
                Ok(rustbox::Event::KeyEvent(key)) => match key {
                    Key::Char('q') => {
                        break;
                    }
                    _ => {}
                },
                Err(e) => error!("{}", e),
                _ => {}
            }

            // Poll draw event.
            chip8.cycle();
            if let Ok(cmd) = rx.recv_timeout(timeout.clone()) {
                match cmd {
                    Cmd::Draw((x, y, data)) => {
                        self.draw(&mut rb, wrap(x, y, &bitarray(&data)));
                        rb.present();
                    }
                    Cmd::Clear => {
                        rb.clear();
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&self, rbox: &mut RustBox, mut cells: VecDeque<Cell>) {
        loop {
            match cells.pop_front() {
                Some(c) => {
                    rbox.print_char(c.x as usize, c.y as usize, c.st, c.fg, c.bg, c.ch);
                }
                None => break,
            }
        }
    }
}

fn run(opts: Option) -> Result<(), ()> {
    let (tx, rx) = mpsc::channel();
    let adaptor = Adaptor::new(tx);

    let mut chip8 = Chip8::new();
    let rom = &opts.rom.canonicalize().unwrap();
    let file = std::fs::File::open(&rom.to_str().unwrap()).unwrap();
    chip8.ram.load(file).unwrap();
    chip8.screen = Some(Box::new(adaptor));

    Console::new().run(chip8, rx)
}

fn main() -> Result<(), ()> {
    env_logger::init();
    let opts = Option::from_args();
    run(opts)
}
