Chip8 in Rust [![Build Status](https://travis-ci.com/yukinarit/chip8.svg?branch=master)](https://travis-ci.com/yukinarit/chip8)
=============

![](demo.gif)


A simple implementation of Chip-8 emurator in Rust programming language.

What's Chip8?
-------------

Chip-8 is a small virtual machine designed for gaming in 1980s. Because of its simplicity, there are [implementations](https://github.com/topics/chip8) in many programming languages. I recommend you to try implementing your own chip8!

Requirements
------------

* Linux / macOS
* Rust >= 1.31

Usage
-----

```
$ cargo run ./roms/INVADERS
```

Keyboard layout

|      |      |      |      |
|------|------|------|------|
| 1    | 2    | 3    | 4(C) |
| Q(4) | W(5) | E(6) | R(D) |
| A(7) | S(8) | D(9) | F(E) |
| Z(A) | X(0) | C(B) | V(F) |

* Letters in parenthesis are Chip8 keys
* ESC is used to stop the program


License
-------

Chip8, along with all its associated documentation, examples and tooling, are made available under the [MIT license](https://opensource.org/licenses/MIT). See LICENSE for additional details.
