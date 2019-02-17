use std::convert::From;
use std::default::Default;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, Instant};

use core::{Chip8, Display};
use log::*;
use rustbox::{
    Color::{self, Black, White},
    Key, RustBox, RB_BOLD,
};
use structopt::StructOpt;

static PIXEL: char = ' ';

const WIDTH: usize = 64;

const HEIGHT: usize = 32;

#[derive(Debug, StructOpt)]
#[structopt(name = "chip8", about = "chip8 program options.")]
struct Args {
    rom: PathBuf,
    #[structopt(short = "f", long = "fps", default_value = "60")]
    fps: i32,
    #[structopt(short = "c", long = "cycle-per-frame", default_value = "5")]
    cycle_per_frame: i32,
}

#[derive(Debug, Clone, Copy)]
enum Fill {
    Fill,
    Unfill,
}

impl std::convert::From<Fill> for Color {
    fn from(f: Fill) -> Color {
        match f {
            Fill::Fill => White,
            Fill::Unfill => Black,
        }
    }
}

impl std::convert::From<Fill> for u8 {
    fn from(f: Fill) -> u8 {
        match f {
            Fill::Fill => 1,
            Fill::Unfill => 0,
        }
    }
}

impl std::convert::From<u8> for Fill {
    fn from(f: u8) -> Fill {
        match f {
            1 => Fill::Fill,
            _ => Fill::Unfill,
        }
    }
}

struct DisplayAdaptor {
    console: Arc<Mutex<Console>>,
}

impl DisplayAdaptor {
    fn new(console: Arc<Mutex<Console>>) -> DisplayAdaptor {
        DisplayAdaptor { console: console }
    }
}

impl Display for DisplayAdaptor {
    fn draw(&self, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()> {
        self.console.lock().unwrap().draw(x, y, data)
    }

    fn clear(&self) {
        self.console.lock().unwrap().clear();
    }
}

fn bitarray(byte: u8) -> Vec<u8> {
    let mut s = Vec::new();
    for n in 0..8 {
        s.push((byte >> (7 - n)) & 0x1);
    }
    s
}

struct Console {
    rb: RustBox,
    keyboard: mpsc::Sender<core::Key>,
    /// Current screen buffer.
    curr: [[u8; HEIGHT]; WIDTH],
}

impl Console {
    fn new(rb: RustBox, keyboard: mpsc::Sender<core::Key>) -> Self {
        let console = Console {
            rb: rb,
            keyboard: keyboard,
            curr: [[0; HEIGHT]; WIDTH],
        };
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                console.draw_pixel(x, y, Fill::Unfill);
            }
        }
        console
    }

    fn peek_keyevent(&self) -> Option<()> {
        match self.rb.peek_event(Duration::from_millis(0), false) {
            Ok(rustbox::Event::KeyEvent(key)) => match key {
                Key::Esc => {
                    std::process::exit(0);
                }
                Key::Char(c) => {
                    let k = core::Key::from(c);
                    if k.0 != 0x99 {
                        debug!("sending key {:?}", c);
                        self.keyboard
                            .send(k)
                            .map_err(|e| error!("Keyboard error: {}", e))
                            .unwrap();
                    }
                }
                _ => {}
            },
            Ok(rustbox::Event::NoEvent) => {
                return None;
            }
            Err(e) => {
                error!("{}", e);
            }
            _ => {}
        };

        Some(())
    }

    fn draw(&mut self, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()> {
        let x = x as usize;
        let y = y as usize;
        let mut vf = 0;
        for (iy, b) in data.iter().enumerate() {
            let next = bitarray(*b);
            for (ix, nb) in next.iter().enumerate() {
                if x + ix >= WIDTH || y + iy >= HEIGHT {
                    continue;
                }
                let cb = self.curr[x + ix][y + iy];
                match (cb, nb) {
                    (0, 0) => {}
                    (0, 1) => {
                        self.draw_pixel(x + ix, y + iy, Fill::Fill);
                    }
                    (1, 0) => {
                        self.draw_pixel(x + ix, y + iy, Fill::Fill);
                    }
                    (1, 1) => {
                        vf = 1;
                        self.draw_pixel(x + ix, y + iy, Fill::Unfill);
                    }
                    _ => {
                        panic!("Illegal bit value: cb={}, nb={}", cb, nb);
                    }
                }
                self.curr[x + ix][y + iy] ^= nb;
            }
        }

        Ok(vf)
    }

    fn draw_pixel(&self, x: usize, y: usize, fill: Fill) {
        // debug!("Draw pixel {} {} {:?}", x, y, fill);
        self.rb.print_char(x, y, RB_BOLD, White, fill.into(), PIXEL);
    }

    fn flush(&mut self) {
        self.rb.present();
    }

    fn clear(&mut self) {
        self.rb.clear();
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                self.curr[x][y] = 0;
                self.draw_pixel(x, y, Fill::Unfill);
            }
        }
    }
}

fn emuloop(mut chip8: Chip8, console: Arc<Mutex<Console>>, opts: Args) -> Result<(), ()> {
    let frame = Duration::from_millis((1000 / opts.fps) as u64);
    loop {
        let now = Instant::now();

        // Run Chip8 Instructions.
        for _ in 0..opts.cycle_per_frame {
            chip8.cycle();
        }

        match console.lock() {
            Ok(mut c) => {
                loop {
                    if let None = c.peek_keyevent() {
                        break;
                    }
                }
                c.flush();
            }
            Err(e) => {
                error!("Unable to unlock Console: {}", e);
            }
        }

        if let Some(remaining) = frame.checked_sub(now.elapsed()) {
            sleep(remaining);
        }
    }
}

fn run(opts: Args) -> Result<(), ()> {
    let (itx, irx) = mpsc::channel();
    let rb = RustBox::init(Default::default()).unwrap();
    let console = Arc::new(Mutex::new(Console::new(rb, itx)));
    let adaptor = DisplayAdaptor::new(console.clone());

    let mut chip8 = Chip8::new();
    let rom = &opts.rom.canonicalize().unwrap();
    let file = std::fs::File::open(&rom.to_str().unwrap()).unwrap();
    chip8.ram.load(file).unwrap();
    chip8.dsp = Some(Box::new(adaptor));
    chip8.inp = Some(irx);
    emuloop(chip8, console, opts)
}

fn main() -> Result<(), ()> {
    log4rs::init_file("logger.yml", Default::default()).unwrap();
    let opts = Args::from_args();
    run(opts)
}
