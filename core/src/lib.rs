#![feature(try_from)]
use std::convert::{From, TryFrom};
use std::io::Read;
use std::sync::mpsc;

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
    pub dsp: Option<Box<Display>>,
    pub inp: Option<mpsc::Receiver<Key>>,
}

impl Chip8 {
    pub fn new() -> Self {
        Chip8 {
            cpu: Cpu::new(),
            ram: Ram::new(),
            dsp: None,
            inp: None,
        }
    }

    pub fn run(&mut self) {
        self.cpu.run(&mut self.ram, &mut self.dsp, &mut self.inp)
    }

    pub fn cycle(&mut self) {
        self.cpu.cycle(&mut self.ram, &mut self.dsp, &mut self.inp)
    }
}

pub trait Display {
    fn draw(&self, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()>;
    fn clear(&self);
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key(pub char);

#[derive(Debug)]
pub struct Cpu {
    /// 8bit general purpose Registers.
    v: [u8; 0xF],
    /// Index register.
    i: u16,
    /// Stack,
    stack: [u16; 0xF],
    /// Stack pointer.
    sp: u16,
    /// Program counter.
    pub pc: u16,
    /// Dilay timer.
    pub dt: u8,
}

pub enum Res {
    Next,
    Skip,
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
        Cpu {
            v: [0; 0xF],
            i: 0,
            stack: [0; 0xF],
            sp: 0,
            pc: 0x200,
            dt: 0,
        }
    }

    fn draw(&self, io: &mut Option<Box<Display>>, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()> {
        if let Some(dsp) = io {
            dsp.draw(x, y, data)
        } else {
            Err(())
        }
    }

    fn clear(&self, io: &mut Option<Box<Display>>) -> Result<(), ()> {
        if let Some(dsp) = io {
            dsp.clear();
        }
        Ok(())
    }

    pub fn run(
        &mut self,
        ram: &mut Ram,
        io: &mut Option<Box<Display>>,
        inp: &mut Option<mpsc::Receiver<Key>>,
    ) {
        loop {
            if self.pc >= 0xFFF || (self.pc + 1) >= 0xFFF {
                break;
            }
            self.cycle(ram, io, inp);
        }
    }

    pub fn cycle(
        &mut self,
        ram: &mut Ram,
        io: &mut Option<Box<Display>>,
        inp: &mut Option<mpsc::Receiver<Key>>,
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
                let pc = self.stack[self.sp as usize];
                self.sp -= 1;
                Jump(pc)
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
                self.sp += 1;
                self.stack[self.sp as usize] = self.pc;
                Jump(nnn)
            }
            (0x3, x, k1, k2) => {
                let kk = var(k1, k2);
                trace!("SE Vx({}) K({})", x, kk);
                if self.v[idx(x)] == kk {
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
                let a = self.v[idx(x)] as u16 + self.v[idx(y)] as u16;
                if a > 0xff {
                    self.v[0xF - 1] = 1;
                } else {
                    self.v[0xF - 1] = 0;
                }
                self.v[idx(x)] = (a & 0xFF) as u8;
                Next
            }
            (0x8, x, y, 0x5) => {
                trace!("8xy5 - SUB V{} V{}", x, y);
                if self.v[idx(x)] > self.v[idx(y)] {
                    self.v[0xF - 1] = 1;
                } else {
                    self.v[0xF - 1] = 0;
                }
                self.v[idx(x)] = self.v[idx(x)] - self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x6) => {
                trace!("8xy6 - SHR V{} V{}", x, y);
                self.v[0xF - 1] = self.v[idx(x)] & 0x1;
                self.v[idx(x)] /= 2;
                Next
            }
            (0x8, x, y, 0x7) => {
                trace!("8xy7 - SUBN V{} V{}", x, y);
                if self.v[idx(y)] > self.v[idx(x)] {
                    self.v[0xF - 1] = 1;
                } else {
                    self.v[0xF - 1] = 0;
                }
                self.v[idx(x)] = self.v[idx(y)] - self.v[idx(x)];
                Next
            }
            (0x8, x, y, 0xE) => {
                trace!("8xyE - SHL V{} V{}", x, y);
                self.v[0xF - 1] = self.v[idx(x)] & 128;
                self.v[idx(x)] *= 2;
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
                self.v[0xF - 1] = self.draw(io, vx, vy, bytes).unwrap();
                Next
            }
            (0xE, x, 0x9, 0xE) => {
                let cx = char::try_from(self.v[idx(x)] as u32).unwrap();
                trace!("Ex9E - SKP V{}={} k={}", x, self.v[idx(x)], cx);
                match inp.as_ref() {
                    Some(inp) => {
                        if let Some(_) = inp.try_iter().find(|c| c.0 == cx) {
                            Skip
                        } else {
                            Next
                        }
                    }
                    None => Next,
                }
            }
            (0xE, x, 0xA, 0x1) => {
                let cx = char::try_from(self.v[idx(x)] as u32).unwrap();
                trace!("ExA1 - SKNP V{}={} k={}", x, self.v[idx(x)], cx);
                match inp.as_ref() {
                    Some(inp) => {
                        if let Some(_) = inp.try_iter().find(|c| c.0 == cx) {
                            Next
                        } else {
                            Skip
                        }
                    }
                    None => Skip,
                }
            }
            (0xF, x, 0x0, 0x7) => {
                trace!("Fx07 - LD Vx, DT");
                self.v[idx(x)] = self.dt;
                Next
            }
            (0xF, x, 0x0, 0xA) => {
                trace!("Fx0A - LD Vx, K");
                let mut pressed = false;
                match inp.as_ref() {
                    Some(inp) => {
                        for c in inp.try_iter() {
                            self.v[idx(x)] = u32::from(c.0) as u8;
                            pressed = true;
                        }

                        if pressed {
                            Next
                        } else {
                            Jump(self.pc)
                        }
                    }
                    None => Jump(self.pc),
                }
            }
            (0xF, x, 0x1, 0x5) => {
                trace!("Fx15 - LD DT, Vx");
                self.dt = self.v[idx(x)];
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
                warn!("N/A {:x}{:x}{:x}{:x}", o1, o2, o3, o4);
                Next
            }
        };
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
        if self.dt > 0 {
            self.dt -= 1;
        }
        self.dump();
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

pub struct Ram {
    /// Chip-8 has 0xFFFF (4096) bytes of RAM.
    buf: [u8; 0xFFF],
    /// Byte size of Chip-8 program.
    pbyte: usize,
}

impl Ram {
    fn new() -> Self {
        Ram {
            buf: [0; 0xFFF],
            pbyte: 0,
        }
    }

    pub fn load<S: Read>(&mut self, mut stream: S) -> Result<(), Error> {
        self.load_fontset();
        loop {
            let size = stream.read(&mut self.buf[0x200..])?;
            if size == 0 {
                break;
            }
            self.pbyte += size;
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

    fn fetch(&self, addr: usize) -> &[u8] {
        &self.buf[addr..(addr + 2)]
    }
}
