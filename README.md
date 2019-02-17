Chip8 written in Rust
=====================

Crates
------

* core: Chip8 emulator core library. Does't have UI.
* c8db: Chip8 line debugger. Just like GDB.
* chip8: [Termbox](https://github.com/nsf/termbox) based Chip8 emulator for terminal.


Requirements
------------

Rust >= 1.31

Build
-----

```
cargo build
```

Usage
-----

* Run chip8
	```
	cd chip8
	cargo run ./roms/INVADERS
	```

* See trace log
	```
	RUST_LOG=trace cargo run <ROM>
	```

* Change FPS(Frame per second)
	```
	cargo run <ROM> -f 30
	```

* Change FPS and Cycle per frame
	```
	cargo run <ROM> -f 30 -c 20
	```

* Run c8db
	```
	cd c8db
	cargo run -p c8db <ROM>
	```
