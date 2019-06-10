use std::convert::From;
use std::io::Read;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::*;
use rand::prelude::*;

#[derive(Debug)]
pub struct Error(pub String);

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error(e.to_string())
    }
}

/// The Chip8 emulator.
pub struct Chip8 {
    pub cpu: Cpu,
    pub ram: Ram,
    pub dsp: Box<Display>,
    pub inp: mpsc::Receiver<Key>,
}

impl Chip8 {
    pub fn new(dsp: Box<Display>, inp: mpsc::Receiver<Key>) -> Self {
        Chip8 {
            cpu: Cpu::new(),
            ram: Ram::new(),
            dsp,
            inp,
        }
    }

    /// Run chip8 emulator.
    pub fn run(&mut self) {
        self.cpu.run(&mut self.ram, &mut self.dsp, &mut self.inp)
    }

    /// One tick of CPU.
    pub fn tick(&mut self) {
        self.cpu.tick(&mut self.ram, &mut self.dsp, &mut self.inp)
    }
}

/// Trait to draw information to display device.
pub trait Display {
    fn draw(&self, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()>;
    fn clear(&self);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key(pub u8);

impl std::convert::From<char> for Key {
    fn from(c: char) -> Key {
        match c {
            '1' => Key(0x1),
            '2' => Key(0x2),
            '3' => Key(0x3),
            'q' => Key(0x4),
            'w' => Key(0x5),
            'e' => Key(0x6),
            'a' => Key(0x7),
            's' => Key(0x8),
            'd' => Key(0x9),
            'z' => Key(0xA),
            'x' => Key(0x0),
            'c' => Key(0xB),
            '4' => Key(0xC),
            'r' => Key(0xD),
            'f' => Key(0xE),
            'v' => Key(0xF),
            _ => Key(0x99),
        }
    }
}

#[derive(Debug)]
pub struct Cpu {
    /// 8bit general purpose Registers.
    v: [u8; 16],
    /// Index register.
    i: u16,
    /// Stack,
    stack: [u16; 16],
    /// Stack pointer.
    sp: u16,
    /// Program counter.
    pub pc: u16,
    /// Delay timer.
    pub dt: DelayTimer,
    /// Key being entered.
    key: Option<Key>,
}

/// 60Hz Delay timer using thread.
#[derive(Debug)]
pub struct DelayTimer {
    v: Arc<AtomicU8>,
    th: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Display for DelayTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.v.load(Ordering::SeqCst))
    }
}

impl DelayTimer {
    pub fn new() -> DelayTimer {
        DelayTimer {
            v: Arc::new(AtomicU8::new(0)),
            th: None,
        }
    }

    pub fn start(&mut self) {
        let tick = Duration::from_millis((1000 / 60) as u64);

        let v = Arc::clone(&self.v);
        let th = std::thread::spawn(move || loop {
            let now = Instant::now();

            // Increment counter.
            loop {
                let curr = v.load(Ordering::SeqCst);
                if curr > 0 {
                    if curr == v.compare_and_swap(curr, curr - 1, Ordering::SeqCst) {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Adjust to 60Hz.
            if let Some(remaining) = tick.checked_sub(now.elapsed()) {
                sleep(remaining);
            }
        });
        self.th = Some(th);
    }

    pub fn get(&self) -> u8 {
        self.v.load(Ordering::SeqCst)
    }

    pub fn set(&mut self, val: u8) {
        self.v.store(val, Ordering::SeqCst);
    }
}

/// Return value to determine the next `pc`.
pub enum Res {
    /// Increase `pc` by 2.
    Next,
    /// Increase `pc` by 4.
    Skip,
    /// Set `pc` the value.
    Jump(u16),
}

use self::Res::{Jump, Next, Skip};

fn addr(n1: u8, n2: u8, n3: u8) -> u16 {
    ((n1 as u16) << 8) + ((n2 as u16) << 4) + n3 as u16
}

fn fontaddr(n: u8) -> u16 {
    n as u16 * 5
}

fn var(x1: u8, x2: u8) -> u8 {
    ((x1 as u8) << 4) + x2 as u8
}

fn idx(x: u8) -> usize {
    x as usize
}

impl Cpu {
    fn new() -> Self {
        let mut dt = DelayTimer::new();
        dt.start();
        Cpu {
            v: [0; 16],
            i: 0,
            stack: [0; 16],
            sp: 0,
            pc: 0x200,
            dt,
            key: None,
        }
    }

    /// Send `draw` instruction to display.
    fn draw(&self, dsp: &mut Box<Display>, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()> {
            dsp.draw(x, y, data)
    }

    /// Send `clear` instruction to display.
    fn clear(&self, dsp: &mut Box<Display>) -> Result<(), ()> {
        dsp.clear();
        Ok(())
    }

    pub fn run(
        &mut self,
        ram: &mut Ram,
        dsp: &mut Box<Display>,
        inp: &mut mpsc::Receiver<Key>,
    ) {
        loop {
            if self.pc >= 0xFFF || (self.pc + 1) >= 0xFFF {
                break;
            }
            self.tick(ram, dsp, inp);
        }
    }

    /// One tick of CPU.
    pub fn tick(
        &mut self,
        ram: &mut Ram,
        io: &mut Box<Display>,
        inp: &mut mpsc::Receiver<Key>,
    ) {
        let pc = self.pc as usize;
        let o1: u8 = ram.buf[pc] >> 4;
        let o2: u8 = ram.buf[pc] & 0xf;
        let o3: u8 = ram.buf[pc + 1] >> 4;
        let o4: u8 = ram.buf[pc + 1] & 0xf;
        let res = match (o1, o2, o3, o4) {
            (0x0, 0x0, 0xE, 0x0) => {
                trace!("00E0 - CLS");
                self.clear(io).unwrap();
                Next
            }
            (0x0, 0x0, 0xE, 0xE) => {
                trace!("00EE - RET");
                let pc = self.stack[self.sp as usize - 1];
                self.sp -= 1;
                Jump(pc + 2)
            }
            (0x0, n1, n2, n3) => {
                let nnn = addr(n1, n2, n3);
                trace!("0nnn - SYS {}", nnn);
                Jump(nnn)
            }
            (0x1, n1, n2, n3) => {
                let nnn = addr(n1, n2, n3);
                trace!("1nnn - JP {}", nnn);
                Jump(nnn)
            }
            (0x2, n1, n2, n3) => {
                let nnn = addr(n1, n2, n3);
                trace!("2nnn - CALL {}", nnn);
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                Jump(nnn)
            }
            (0x3, x, k1, k2) => {
                let kk = var(k1, k2);
                let vx = self.v[idx(x)];
                trace!("SE V{}({}) K({})", x, vx, kk);
                if vx == kk {
                    Skip
                } else {
                    Next
                }
            }
            (0x4, x, k1, k2) => {
                let kk = var(k1, k2);
                trace!("SNE Vx({}) K({})", x, kk);
                if self.v[idx(x)] != kk {
                    Skip
                } else {
                    Next
                }
            }
            (0x5, x, y, 0x0) => {
                trace!("SE Vx({}), Vy({})", x, y);
                if self.v[idx(x)] == self.v[idx(y)] {
                    Skip
                } else {
                    Next
                }
            }
            (0x6, x, k1, k2) => {
                let kk = var(k1, k2);
                trace!("6xkk - LD V{}={}", x, kk);
                self.v[idx(x)] = kk;
                Next
            }
            (0x7, x, k1, k2) => {
                let x = idx(x);
                let kk = var(k1, k2);
                trace!("7xkk - ADD V{} {}", x, kk);
                self.v[x] = self.v[x].overflowing_add(kk).0;
                Next
            }
            (0x8, x, y, 0x0) => {
                trace!("8xy0 - LD V{} V{}", x, y);
                self.v[idx(x)] = self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x1) => {
                trace!("8xy1 - OR V{} V{}", x, y);
                self.v[idx(x)] |= self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x2) => {
                trace!("8xy2 - AND V{} V{}", x, y);
                self.v[idx(x)] &= self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x3) => {
                trace!("8xy3 - XOR V{} V{}", x, y);
                self.v[idx(x)] ^= self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x4) => {
                trace!("8xy4 - ADD V{} V{}", x, y);
                let xy = self.v[idx(x)] as u16 + self.v[idx(y)] as u16;
                if xy > 0xff {
                    self.v[0xf] = 1;
                } else {
                    self.v[0xf] = 0;
                }
                self.v[idx(x)] = (xy & 0xff) as u8;
                Next
            }
            (0x8, x, y, 0x5) => {
                let vx = self.v[idx(x)];
                let vy = self.v[idx(y)];
                trace!("8xy5 - SUB V{}={} V{}={}", x, vx, y, vy);
                let (val, overflow) = vx.overflowing_sub(vy);
                if !overflow {
                    self.v[0xf] = 1;
                } else {
                    self.v[0xf] = 0;
                }
                self.v[idx(x)] = val;
                Next
            }
            (0x8, x, y, 0x6) => {
                trace!("8xy6 - SHR V{} V{}", x, y);
                self.v[0xf] = self.v[idx(x)] & 0x1;
                self.v[idx(x)] /= 2;
                Next
            }
            (0x8, x, y, 0x7) => {
                let vx = self.v[idx(x)];
                let vy = self.v[idx(y)];
                trace!("8xy7 - SUBN V{}={} V{}={}", x, vx, y, vy);
                let (val, overflow) = vy.overflowing_sub(vx);

                if !overflow {
                    self.v[0xf] = 1;
                } else {
                    self.v[0xf] = 0;
                }
                self.v[idx(x)] = val;
                Next
            }
            (0x8, x, y, 0xE) => {
                trace!("8xyE - SHL V{} V{}", x, y);
                self.v[0xf] = self.v[idx(x)] >> 7;
                self.v[idx(x)] = self.v[idx(x)].overflowing_mul(2).0;
                Next
            }
            (0x9, x, y, 0x0) => {
                trace!("SNE V{}, V{}", x, y);
                if self.v[idx(x)] != self.v[idx(y)] {
                    Skip
                } else {
                    Next
                }
            }
            (0xA, n1, n2, n3) => {
                self.i = addr(n1, n2, n3);
                trace!("Annn - LD I, {}", self.i);
                Next
            }
            (0xB, n1, n2, n3) => {
                let i = addr(n1, n2, n3) + self.v[0] as u16;
                trace!("Bnnn - JP V0, {:x}", i);
                Jump(i)
            }
            (0xC, x, k1, k2) => {
                let rnd: u8 = random();
                let kk = var(k1, k2);
                trace!("Cxkk - RND V{} {}", x, kk);
                self.v[idx(x)] = rnd & kk;
                Next
            }
            (0xD, x, y, n) => {
                let vx = self.v[idx(x)];
                let vy = self.v[idx(y)];
                let since = self.i as usize;
                let until = since + idx(n);
                let bytes = (&ram.buf[since..until]).to_vec();
                trace!(
                    "Dxyn - DRW V{}={}, V{}={}, nibble={}, bytes={:?}",
                    x,
                    vx,
                    y,
                    vy,
                    n,
                    bytes
                );
                self.v[0xf] = self.draw(io, vx, vy, bytes).unwrap();
                Next
            }
            (0xE, x, 0x9, 0xE) => {
                trace!("Ex9E - SKP V{}={}", x, self.v[idx(x)]);
                if let Some(key) = self.key(inp) {
                    if key.0 == self.v[idx(x)] {
                        self.key = None;
                        Skip
                    } else {
                        Next
                    }
                } else {
                    Next
                }
            }
            (0xE, x, 0xA, 0x1) => {
                trace!("ExA1 - SKNP V{}={}", x, self.v[idx(x)]);
                if let Some(key) = self.key(inp) {
                    if key.0 == self.v[idx(x)] {
                        self.key = None;
                        Next
                    } else {
                        Skip
                    }
                } else {
                    Skip
                }
            }
            (0xF, x, 0x0, 0x7) => {
                trace!("Fx07 - LD Vx, DT");
                self.v[idx(x)] = self.dt.get();
                Next
            }
            (0xF, x, 0x0, 0xA) => {
                trace!("Fx0A - LD Vx, K");
                let mut pressed = false;
                if let Some(c) = self.key(inp) {
                    debug!("Got {:?}", c);
                    self.v[idx(x)] = c.0;
                    pressed = true;
                }

                if pressed {
                    Next
                } else {
                    Jump(self.pc)
                }
            }
            (0xF, x, 0x1, 0x5) => {
                trace!("Fx15 - LD DT, Vx");
                self.dt.set(self.v[idx(x)]);
                Next
            }
            (0xF, x, 0x1, 0x8) => {
                trace!("Fx18 - LD ST, Vx");
                Next
            }
            (0xF, x, 0x1, 0xE) => {
                trace!("ADD I, Vx");
                self.i += self.v[idx(x)] as u16;
                Next
            }
            (0xF, x, 0x2, 0x9) => {
                let vx = self.v[idx(x)];
                trace!("Fx29 - LD F, Vx={}", vx);
                self.i = fontaddr(vx);
                Next
            }
            (0xF, x, 0x3, 0x3) => {
                trace!("Fx33 - LD B, Vx");
                let i = self.i as usize;
                let vx = self.v[idx(x)];
                ram.buf[i] = (vx / 100) as u8 % 10;
                ram.buf[i + 1] = (vx / 10) as u8 % 10;
                ram.buf[i + 2] = vx % 10;
                Next
            }
            (0xF, x, 0x5, 0x5) => {
                trace!("Fx55 - LD [I], V{}", x);
                for n in 0..x + 1 {
                    ram.buf[self.i as usize + idx(n)] = self.v[idx(n)];
                }
                Next
            }
            (0xF, x, 0x6, 0x5) => {
                trace!("Fx65 - LD V{}, I={}", x, self.i);
                for n in 0..x + 1 {
                    self.v[idx(n)] = ram.buf[self.i as usize + idx(n)];
                }
                Next
            }
            _ => {
                panic!("N/A {:x}{:x}{:x}{:x}", o1, o2, o3, o4);
                Next
            }
        };

        // Determine the next `pc`.
        match res {
            Next => {
                self.pc += 2;
            }
            Skip => {
                self.pc += 4;
            }
            Jump(loc) => {
                self.pc = loc;
            }
        }
        self.dump();
    }

    fn key(&mut self, inp: &mut mpsc::Receiver<Key>) -> Option<Key> {
        inp.try_recv().ok().or(self.key).map(|k| {
            debug!("receiving key {:?}", k);
            self.key = Some(k);
            k
        })
    }

    pub fn dump(&self) {
        trace!(
            " v{:?} i={}({:x}) stack={:?} sp={} pc={}({:x}) dt={}",
            self.v,
            self.i,
            self.i,
            self.stack,
            self.sp,
            self.pc,
            self.pc,
            self.dt
        );
    }
}

/// Chip-8 RAM.
pub struct Ram {
    /// Chip-8 has 0xFFFF (4096) bytes of RAM.
    buf: [u8; 0xFFF],
}

impl Ram {
    fn new() -> Self {
        Ram {
            buf: [0; 0xFFF],
        }
    }

    /// Load Chip8 ROM into memory.
    pub fn load<S: Read>(&mut self, mut stream: S) -> Result<(), Error> {
        self.load_fontset();
        loop {
            if stream.read(&mut self.buf[0x200..])? == 0 {
                break;
            }
        }

        Ok(())
    }

    fn load_fontset(&mut self) {
        let fontset = vec![
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];
        &self.buf[..fontset.len()].copy_from_slice(&fontset);
    }
}
