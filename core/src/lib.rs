use std::convert::From;
use std::io::Read;

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
    pub screen: Option<Box<Screen>>,
}

impl Chip8 {
    pub fn new() -> Self {
        Chip8 {
            cpu: Cpu::new(),
            ram: Ram::new(),
            screen: None,
        }
    }

    pub fn run(&mut self) {
        self.cpu.run(&mut self.ram, &mut self.screen)
    }
}

pub trait Screen {
    fn draw(&self, x: u8, y: u8, data: Vec<u8>) -> Result<(), ()>;
    fn clear(&self);
}

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
        }
    }

    fn draw(&self, io: &mut Option<Box<Screen>>, x: u8, y: u8, data: Vec<u8>) -> Result<(), ()> {
        if let Some(screen) = io {
            screen.draw(x, y, data);
        }
        Ok(())
    }

    fn clear(&self, io: &mut Option<Box<Screen>>) -> Result<(), ()> {
        if let Some(screen) = io {
            screen.clear();
        }
        Ok(())
    }

    pub fn run(&mut self, ram: &mut Ram, io: &mut Option<Box<Screen>>) {
        loop {
            if self.pc >= 0xFFF || (self.pc + 1) >= 0xFFF {
                break;
            }
            match self.cycle(ram, io) {
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
        }
    }

    pub fn cycle(&mut self, ram: &mut Ram, io: &mut Option<Box<Screen>>) -> Res {
        let pc = self.pc as usize;
        let o1: u8 = ram.buf[pc] >> 4;
        let o2: u8 = ram.buf[pc] & 0xf;
        let o3: u8 = ram.buf[pc + 1] >> 4;
        let o4: u8 = ram.buf[pc + 1] & 0xf;
        match (o1, o2, o3, o4) {
            (0x0, 0x0, 0xE, 0x0) => {
                trace!("CLS");
                self.clear(io).unwrap();
                Next
            }
            (0x0, 0x0, 0xE, 0xE) => {
                trace!("RET");
                self.sp -= 1;
                Jump(self.stack[self.sp as usize])
            }
            (0x0, n1, n2, n3) => {
                let nnn = addr(n1, n2, n3);
                trace!("SYS {:x}", nnn);
                Jump(nnn)
            }
            (0x1, n1, n2, n3) => {
                let nnn = addr(n1, n2, n3);
                trace!("JP {:x}", nnn);
                Jump(nnn)
            }
            (0x2, n1, n2, n3) => {
                let nnn = addr(n1, n2, n3);
                trace!("CALL {:x}", nnn);
                //self.sp += 1;
                //self.stack[self.sp as usize] = self.pc;
                //self.pc = nnn;
                Next
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
                trace!("LD V{}={}", x, kk);
                self.v[idx(x)] = kk;
                Next
            }
            (0x7, x, k1, k2) => {
                let x = idx(x);
                let kk = var(k1, k2);
                trace!("ADD V{} {}", x, kk);
                self.v[x] = self.v[x].overflowing_add(kk).0;
                Next
            }
            (0x8, x, y, 0x0) => {
                trace!("LD V{} V{}", x, y);
                self.v[idx(x)] = self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x1) => {
                trace!("OR V{} V{}", x, y);
                self.v[idx(x)] |= self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x2) => {
                trace!("AND V{} V{}", x, y);
                self.v[idx(x)] &= self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x3) => {
                trace!("XOR V{} V{}", x, y);
                self.v[idx(x)] ^= self.v[idx(y)];
                Next
            }
            (0x8, x, y, 0x4) => {
                trace!("ADD V{} V{}", x, y);
                let a = self.v[idx(x)] as u16 + self.v[idx(y)] as u16;
                if a > 0xff {
                    self.v[0xF - 1] = 1;
                    self.v[idx(x)] = (a & 0xFF) as u8;
                } else {
                    self.v[idx(x)] = a as u8;
                }
                Next
            }
            (0x8, x, y, 0x5) => {
                trace!("SUB V{} V{}", x, y);
                if self.v[idx(x)] > self.v[idx(y)] {
                    self.v[0xF - 1] = 1;
                }
                Next
            }
            (0x8, x, y, 0x6) => {
                trace!("SHR V{} V{}", x, y);
                self.v[0xF - 1] = self.v[idx(x)] & 0x1;
                self.v[idx(x)] /= 2;
                Next
            }
            (0x8, x, y, 0x7) => {
                trace!("SUBN V{} V{}", x, y);
                if self.v[idx(x)] > self.v[idx(y)] {
                    self.v[0xF - 1] = 1;
                }
                self.v[idx(x)] = self.v[idx(y)] - self.v[idx(x)];
                Next
            }
            (0x8, x, y, 0xE) => {
                trace!("SHL V{} V{}", x, y);
                self.v[0xF - 1] = self.v[idx(x)] & 0x1;
                self.v[idx(x)] = self.v[idx(x)] << 1;
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
                let i = addr(n1, n2, n3);
                trace!("LD I, {}", i);
                self.i = i;
                Next
            }
            (0xB, n1, n2, n3) => {
                let i = addr(n1, n2, n3);
                trace!("JP V0, {:x}", i);
                Jump(self.i + self.v[0] as u16)
            }
            (0xC, x, k1, k2) => {
                let rnd: u8 = random();
                let kk = var(k1, k2);
                trace!("RND V{} {}", x, kk);
                self.v[x as usize] = rnd & kk;
                Next
            }
            (0xD, x, y, n) => {
                let vx = self.v[idx(x)];
                let vy = self.v[idx(y)];
                trace!("DRW V{}={}, V{}={}, nibble={}", x, vx, y, vy, n);
                self.draw(
                    io,
                    vx,
                    vy,
                    (&ram.buf[self.i as usize..(self.i as usize + idx(n))]).to_vec(),
                )
                .unwrap();
                Next
            }
            (0xE, x, 0x9, 0xE) => {
                trace!("SKP Vx");
                Next
            }
            (0xE, x, 0xA, 0x1) => {
                trace!("SKNP Vx");
                Next
            }
            (0xF, x, 0x0, 0x7) => {
                trace!("LD Vx, DT");
                Next
            }
            (0xF, x, 0x0, 0xA) => {
                trace!("LD Vx, K");
                Next
            }
            (0xF, x, 0x1, 0x5) => {
                trace!("LD DT, Vx");
                Next
            }
            (0xF, x, 0x1, 0x8) => {
                trace!("LD ST, Vx");
                Next
            }
            (0xF, x, 0x1, 0xE) => {
                trace!("ADD I, Vx");
                self.i += self.v[idx(x)] as u16;
                Next
            }
            (0xF, x, 0x2, 0x9) => {
                trace!("LD F, Vx");
                Next
            }
            (0xF, x, 0x3, 0x3) => {
                trace!("LD B, Vx");
                Next
            }
            (0xF, x, 0x5, 0x5) => {
                trace!("LD [I], V{}", x);
                for n in 0..x {
                    ram.buf[self.i as usize + idx(n)] = self.v[idx(n)];
                }
                Next
            }
            (0xF, x, 0x6, 0x5) => {
                trace!("LD V{}, [I]", x);
                for n in 0..x {
                    self.v[idx(n)] = ram.buf[self.i as usize + idx(n)];
                }
                Next
            }
            _ => {
                warn!("N/A {:x}{:x}{:x}{:x}", o1, o2, o3, o4);
                Next
            }
        }
    }

    pub fn dump(&self) {
        println!(
            "v{:?} i={} stack={:?} sp={} pc={}",
            self.v, self.i, self.stack, self.sp, self.pc
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
        loop {
            let size = stream.read(&mut self.buf[0x200..])?;
            if size == 0 {
                break;
            }
            self.pbyte += size;
        }

        Ok(())
    }

    fn load_fontset(&mut self) {}

    fn fetch(&self, addr: usize) -> &[u8] {
        &self.buf[addr..(addr + 2)]
    }
}
