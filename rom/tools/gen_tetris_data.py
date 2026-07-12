#!/usr/bin/env python3
"""Generates the tile-graphics and piece-rotation data fragments used by
tetris.asm.tmpl, and assembles the final ROM.

Usage: gen_tetris_data.py <template.asm> <output.gb>
"""
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))
from genfont import FONT, glyph_bytes  # noqa: E402
import gbasm  # noqa: E402

TILE_ORDER = ["BLANK", "BORDER", "LOCKED", "ACTIVE"] + list("0123456789") + list("ACEGILMNOPRSTV") + ["SPACE"]

SPECIAL_TILES = {
    "BLANK": [0x00] * 8,
    "BORDER": [0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55],
    "LOCKED": [0xFF] * 8,
    "ACTIVE": [0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF],
}

CHAR_TO_NAME = {c: c for c in "0123456789ACEGILMNOPRSTV"}
CHAR_TO_NAME[" "] = "SPACE"


def tile_index(name):
    return TILE_ORDER.index(name)


def char_index(ch):
    return tile_index(CHAR_TO_NAME[ch.upper()])


def gen_tile_data_lines():
    lines = []
    for name in TILE_ORDER:
        if name in SPECIAL_TILES:
            rows = SPECIAL_TILES[name]
        else:
            rows = glyph_bytes(FONT[name if name != "SPACE" else " "])
        lines.append(f"; tile {tile_index(name)}: {name}")
        for b in rows:
            lines.append(f"DB ${b:02X},${b:02X}")
    return "\n".join(lines)


def gen_piece_table_lines():
    pieces = {
        "I": [(0, 1), (1, 1), (2, 1), (3, 1)],
        "O": [(1, 0), (2, 0), (1, 1), (2, 1)],
        "T": [(1, 0), (0, 1), (1, 1), (2, 1)],
        "S": [(1, 0), (2, 0), (0, 1), (1, 1)],
        "Z": [(0, 0), (1, 0), (1, 1), (2, 1)],
        "J": [(0, 0), (0, 1), (1, 1), (2, 1)],
        "L": [(2, 0), (0, 1), (1, 1), (2, 1)],
    }

    def rotate_cw(cells):
        return [(3 - y, x) for (x, y) in cells]

    order = ["I", "O", "T", "S", "Z", "J", "L"]
    lines = []
    for name in order:
        cells = pieces[name]
        states = [cells]
        cur = cells
        for _ in range(3):
            cur = rotate_cw(cur)
            states.append(cur)
        for r, cs in enumerate(states):
            vals = []
            for (x, y) in cs:
                vals.append(str(x))
                vals.append(str(y))
            lines.append(f"DB {','.join(vals)}  ; {name} rot{r}")
    return "\n".join(lines)


def gen_tile_constants():
    lines = []
    for name in TILE_ORDER:
        if name.isdigit():
            const = f"TILE_DIGIT{name}"
        elif len(name) == 1 and name.isalpha():
            const = f"TILE_LETTER_{name}"
        else:
            const = f"TILE_{name}"
        lines.append(f"{const} EQU {tile_index(name)}")
    return "\n".join(lines)


def gen_text_bytes(s):
    """Returns a comma-separated DB list of tile indices for a string."""
    return ",".join(str(char_index(c)) for c in s)


def main():
    if len(sys.argv) != 3:
        print("usage: gen_tetris_data.py <template.asm> <output.gb>")
        sys.exit(1)
    template_path, out_path = sys.argv[1], sys.argv[2]
    with open(template_path) as f:
        text = f.read()

    text = text.replace("@@TILE_CONSTANTS@@", gen_tile_constants())
    text = text.replace("@@TILE_DATA@@", gen_tile_data_lines())
    text = text.replace("@@PIECE_TABLE@@", gen_piece_table_lines())
    text = text.replace("@@STR_TETRIS@@", gen_text_bytes("TETRIS"))
    text = text.replace("@@STR_SCORE@@", gen_text_bytes("SCORE"))
    text = text.replace("@@STR_LINES@@", gen_text_bytes("LINES"))
    text = text.replace("@@STR_GAME@@", gen_text_bytes("GAME"))
    text = text.replace("@@STR_OVER@@", gen_text_bytes("OVER"))
    text = text.replace("@@STR_PRESS@@", gen_text_bytes("PRESS"))
    text = text.replace("@@STR_START@@", gen_text_bytes("START"))

    final_path = os.path.join(os.path.dirname(out_path), "_tetris_final.asm")
    with open(final_path, "w") as f:
        f.write(text)

    asm = gbasm.Assembler()
    asm.load(text)
    rom = asm.assemble()
    with open(out_path, "wb") as f:
        f.write(rom)
    print(f"assembled {len(rom)} bytes -> {out_path}")


if __name__ == "__main__":
    main()
