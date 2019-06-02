Chip8 in Rust [![Build Status](https://travis-ci.com/yukinarit/chip8.svg?branch=master)](https://travis-ci.com/yukinarit/chip8)
=============

![](demo.gif)


RustでChip8エミュレータを書いてみる。[README in English.](README.md)

Chip8とは?
----------

Chip8とは1970年代にゲーム用に設計された小さな仮想マシン。仕様は非常にシンプルなので、GithubやWeb上で多くのプログラミング言語での[実装](https://github.com/topics/chip8)を見つけることができる。仕様を見て一から実装するのはそんなに難しくないので、自分の好きな言語で実装してみるといいと思います。

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

* 括弧内の文字はChip8のキー配列
* ESCキーでプログラムを終了する


License
-------

Chip8, along with all its associated documentation, examples and tooling, are made available under the [MIT license](https://opensource.org/licenses/MIT). See LICENSE for additional details.
