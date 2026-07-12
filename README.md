# Pixel Pocket — a Web Game Boy Emulator

A Game Boy (DMG) emulator engine written in Rust, compiled to WebAssembly,
paired with a Game Boy–styled web UI and an original falling-block puzzle
cartridge ("TETRIS", written from scratch — not Nintendo's ROM).

## Structure

```
emulator/       Rust Game Boy core (CPU, PPU, APU, timer, joypad, MBC1/3/5)
                compiled to a dependency-free WASM module (no wasm-bindgen —
                plain `extern "C"` exports + exported linear memory).
rom/            The Tetris-style cartridge, plus the tools used to build it:
  tools/gbasm.py        a small two-pass SM83 assembler (no external toolchain)
  tools/genfont.py       generates the 8x8 tile font used for on-screen text
  tools/gen_tetris_data.py  fills tetris.asm.tmpl's data tables and assembles it
  src/tetris.asm.tmpl    the game's source (hand-written SM83 assembly)
  tests/                 small assembly test ROMs used by the Rust test suite
web/            The browser front-end: index.html/style.css/main.js, plus the
                pre-built wasm module (pkg/) and cartridge (roms/tetris.gb).
```

## Building the emulator core

```sh
cd emulator
./build.sh          # builds gbcore.wasm and copies it into web/pkg/
```

Requires the `wasm32-unknown-unknown` Rust target:
`rustup target add wasm32-unknown-unknown`.

## Rebuilding the Tetris cartridge

```sh
cd rom/tools
python3 gen_tetris_data.py ../src/tetris.asm.tmpl ../tetris.gb
cp ../tetris.gb ../../web/roms/tetris.gb
```

## Running

Serve the `web/` directory with any static file server and open it in a
browser:

```sh
cd web
python3 -m http.server 8080
# open http://localhost:8080
```

Controls: arrow keys move, X = A (rotate), Z = B, Enter = Start,
Shift = Select — or use the on-screen buttons (mouse or touch). A "Load .gb
cartridge" file picker is included, so any other Game Boy ROM can be dropped
in too.

## Testing

The emulator core has a Rust test suite that boots real assembled ROMs
headlessly and asserts on emulated CPU/PPU/WRAM state — including full
Tetris gameplay (spawning, movement, rotation, gravity, line-clearing, and
scoring):

```sh
cd emulator
cargo test --lib
```
