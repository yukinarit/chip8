use std::collections::VecDeque;
use std::default::Default;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use core::{Chip8, Screen};
use log::*;
use rustbox::{
    Color::{self, White},
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
    #[structopt(short = "f", long = "fps", default_value = "10")]
    fps: i32,
    #[structopt(short = "c", long = "cycle-per-frame", default_value = "10")]
    cycle_per_frame: i32,
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

    fn run(&mut self, mut chip8: Chip8, rx: Rx, opts: Option) -> Result<(), ()> {
        let mut rb = RustBox::init(Default::default()).unwrap();

        let timeout = Duration::from_millis(1);
        let frame = Duration::from_millis((1000 / opts.fps) as u64);
        loop {
            let now = Instant::now();

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

            // Run Chip8 Instructions.
            for _ in 0..opts.cycle_per_frame {
                chip8.cycle();
            }

            // Poll draw event.
            for cmd in rx.try_iter() {
                match cmd {
                    Cmd::Draw((x, y, data)) => {
                        self.draw(&mut rb, wrap(x, y, &bitarray(&data)));
                    }
                    Cmd::Clear => {
                        rb.clear();
                    }
                }
            }
            rb.present();
            if let Some(remaining) = frame.checked_sub(now.elapsed()) {
                sleep(remaining);
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

    Console::new().run(chip8, rx, opts)
}

fn main() -> Result<(), ()> {
    env_logger::init();
    let opts = Option::from_args();
    run(opts)
}
