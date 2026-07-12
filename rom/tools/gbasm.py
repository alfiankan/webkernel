#!/usr/bin/env python3
"""Minimal two-pass assembler for the Game Boy (SM83) instruction set.

Supports a practical subset of RGBDS-like syntax: ORG, DB, DW, DS,
labels ("name:"), decimal/$hex/0xhex numbers, string literals in DB,
and the full SM83 mnemonic set. Good enough to hand-assemble small
homebrew ROMs without any external toolchain.
"""
import re
import sys

R8 = {"B": 0, "C": 1, "D": 2, "E": 3, "H": 4, "L": 5, "(HL)": 6, "A": 7}
RP = {"BC": 0, "DE": 1, "HL": 2, "SP": 3}
RP2 = {"BC": 0, "DE": 1, "HL": 2, "AF": 3}
CC = {"NZ": 0, "Z": 1, "NC": 2, "C": 3}
ALU = {"ADD": 0, "ADC": 1, "SUB": 2, "SBC": 3, "AND": 4, "XOR": 5, "OR": 6, "CP": 7}
ROT = {"RLC": 0, "RRC": 1, "RL": 2, "RR": 3, "SLA": 4, "SRA": 5, "SWAP": 6, "SRL": 7}


class AsmError(Exception):
    pass


def parse_number(tok, labels=None, line=0):
    tok = tok.strip()
    neg = False
    if tok.startswith("-"):
        neg = True
        tok = tok[1:].strip()
    if tok.startswith("$"):
        v = int(tok[1:], 16)
    elif tok.lower().startswith("0x"):
        v = int(tok[2:], 16)
    elif tok.startswith("%"):
        v = int(tok[1:], 2)
    elif re.fullmatch(r"[0-9]+", tok):
        v = int(tok)
    elif labels is not None:
        if tok not in labels:
            raise AsmError(f"line {line}: unknown label '{tok}'")
        v = labels[tok]
    else:
        v = 0  # placeholder during size-only pass
    return -v if neg else v


def is_number_token(tok):
    tok = tok.strip()
    if tok.startswith("-"):
        tok = tok[1:]
    return bool(re.fullmatch(r"(\$[0-9A-Fa-f]+|0[xX][0-9A-Fa-f]+|%[01]+|[0-9]+)", tok))


class Assembler:
    def __init__(self):
        self.labels = {}
        self.lines = []  # (lineno, org_addr, mnemonic, operands, raw)

    def load(self, text):
        for lineno, raw in enumerate(text.splitlines(), 1):
            line = raw.split(";", 1)[0].strip()
            if not line:
                continue
            self.lines.append((lineno, line))

    def assemble(self):
        # Pass 1: compute label addresses and instruction sizes.
        addr = 0
        entries = []
        for lineno, line in self.lines:
            eq = re.match(r"^([A-Za-z_][A-Za-z0-9_]*)\s+EQU\s+(.+)$", line, re.IGNORECASE)
            if eq:
                self.labels[eq.group(1)] = parse_number(eq.group(2), self.labels, lineno)
                continue
            label = None
            m = re.match(r"^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$", line)
            if m:
                label = m.group(1)
                line = m.group(2).strip()
                self.labels[label] = addr
                if not line:
                    continue
            parts = self.split_instr(line)
            mnem = parts[0].upper()
            operands = parts[1:]
            if mnem == "ORG":
                addr = parse_number(operands[0])
                entries.append((lineno, "ORG", operands, addr))
                continue
            size = self.size_of(mnem, operands, lineno)
            entries.append((lineno, mnem, operands, addr))
            addr += size
        self.entries = entries

        # Pass 2: emit bytes into a sparse dict, then flatten.
        out = {}
        addr = 0
        for lineno, mnem, operands, at_addr in entries:
            if mnem == "ORG":
                addr = at_addr
                continue
            bytes_out = self.encode(mnem, operands, addr, lineno)
            for b in bytes_out:
                out[addr] = b & 0xFF
                addr += 1

        max_addr = max(out.keys(), default=-1)
        rom = bytearray(max_addr + 1)
        for a, b in out.items():
            rom[a] = b
        return rom

    def split_instr(self, line):
        # mnemonic followed by comma separated operands (respecting parens)
        m = re.match(r"^(\S+)\s*(.*)$", line)
        mnem = m.group(1)
        rest = m.group(2).strip()
        if not rest:
            return [mnem]
        operands = [o.strip() for o in self.split_commas(rest)]
        return [mnem] + operands

    def split_commas(self, s):
        out = []
        depth = 0
        cur = ""
        in_str = False
        for ch in s:
            if ch == '"':
                in_str = not in_str
            if ch == "," and depth == 0 and not in_str:
                out.append(cur)
                cur = ""
                continue
            if ch == "(" and not in_str:
                depth += 1
            if ch == ")" and not in_str:
                depth -= 1
            cur += ch
        if cur.strip():
            out.append(cur)
        return out

    # ---- sizing (pass 1) ----
    def size_of(self, mnem, ops, lineno):
        if mnem in ("DB",):
            n = 0
            for o in ops:
                o = o.strip()
                if o.startswith('"') and o.endswith('"'):
                    n += len(o[1:-1])
                else:
                    n += 1
            return n
        if mnem == "DW":
            return 2 * len(ops)
        if mnem == "DS":
            return parse_number(ops[0])
        try:
            b = self.encode(mnem, ops, 0, lineno, size_only=True)
            return len(b)
        except AsmError:
            raise
        except Exception as e:
            raise AsmError(f"line {lineno}: cannot size '{mnem} {','.join(ops)}': {e}")

    # ---- encoding (pass 1 sizing + pass 2 emitting) ----
    def encode(self, mnem, ops, addr, lineno, size_only=False):
        L = self.labels if not size_only else None

        def num(tok):
            return parse_number(tok, L, lineno)

        def is_r8(tok):
            return tok.upper() in R8

        def r8(tok):
            return R8[tok.upper()]

        if mnem == "DB":
            out = []
            for o in ops:
                o = o.strip()
                if o.startswith('"') and o.endswith('"'):
                    out.extend(ord(ch) for ch in o[1:-1])
                else:
                    out.append(num(o) & 0xFF)
            return out
        if mnem == "DW":
            out = []
            for o in ops:
                v = num(o)
                out.extend([v & 0xFF, (v >> 8) & 0xFF])
            return out
        if mnem == "DS":
            n = num(ops[0])
            fill = num(ops[1]) if len(ops) > 1 else 0
            return [fill] * n

        if mnem == "NOP":
            return [0x00]
        if mnem == "STOP":
            return [0x10, 0x00]
        if mnem == "HALT":
            return [0x76]
        if mnem == "DI":
            return [0xF3]
        if mnem == "EI":
            return [0xFB]
        if mnem == "RET":
            if ops:
                return [0xC0 | (CC[ops[0].upper()] << 3)]
            return [0xC9]
        if mnem == "RETI":
            return [0xD9]
        if mnem == "RLCA":
            return [0x07]
        if mnem == "RRCA":
            return [0x0F]
        if mnem == "RLA":
            return [0x17]
        if mnem == "RRA":
            return [0x1F]
        if mnem == "DAA":
            return [0x27]
        if mnem == "CPL":
            return [0x2F]
        if mnem == "SCF":
            return [0x37]
        if mnem == "CCF":
            return [0x3F]

        if mnem == "JR":
            if len(ops) == 1:
                target = num(ops[0])
                rel = target - (addr + 2)
                return [0x18, rel & 0xFF]
            else:
                cc = CC[ops[0].upper()]
                target = num(ops[1])
                rel = target - (addr + 2)
                return [0x20 | (cc << 3), rel & 0xFF]

        if mnem == "JP":
            if len(ops) == 1 and ops[0].upper() in ("(HL)", "HL"):
                return [0xE9]
            if len(ops) == 1:
                v = num(ops[0])
                return [0xC3, v & 0xFF, (v >> 8) & 0xFF]
            cc = CC[ops[0].upper()]
            v = num(ops[1])
            return [0xC2 | (cc << 3), v & 0xFF, (v >> 8) & 0xFF]

        if mnem == "CALL":
            if len(ops) == 1:
                v = num(ops[0])
                return [0xCD, v & 0xFF, (v >> 8) & 0xFF]
            cc = CC[ops[0].upper()]
            v = num(ops[1])
            return [0xC4 | (cc << 3), v & 0xFF, (v >> 8) & 0xFF]

        if mnem == "RST":
            v = num(ops[0])
            return [0xC7 | v]

        if mnem == "PUSH":
            return [0xC5 | (RP2[ops[0].upper()] << 4)]
        if mnem == "POP":
            return [0xC1 | (RP2[ops[0].upper()] << 4)]

        if mnem == "INC":
            o = ops[0].upper()
            if o in RP:
                return [0x03 | (RP[o] << 4)]
            return [0x04 | (r8(o) << 3)]
        if mnem == "DEC":
            o = ops[0].upper()
            if o in RP:
                return [0x0B | (RP[o] << 4)]
            return [0x05 | (r8(o) << 3)]

        if mnem in ROT:
            o = ops[0].upper()
            return [0xCB, (ROT[mnem] << 3) | r8(o)]
        if mnem == "BIT":
            bit = num(ops[0])
            o = ops[1].upper()
            return [0xCB, 0x40 | (bit << 3) | r8(o)]
        if mnem == "RES":
            bit = num(ops[0])
            o = ops[1].upper()
            return [0xCB, 0x80 | (bit << 3) | r8(o)]
        if mnem == "SET":
            bit = num(ops[0])
            o = ops[1].upper()
            return [0xCB, 0xC0 | (bit << 3) | r8(o)]

        if mnem == "ADD" and ops[0].upper() == "HL":
            return [0x09 | (RP[ops[1].upper()] << 4)]
        if mnem == "ADD" and ops[0].upper() == "SP":
            v = num(ops[1])
            return [0xE8, v & 0xFF]

        if mnem in ALU:
            op = ALU[mnem]
            src = ops[-1].upper()
            if len(ops) == 2 and ops[0].upper() != "A":
                raise AsmError(f"line {lineno}: {mnem} dest must be A")
            if is_r8(src):
                return [0x80 | (op << 3) | r8(src)]
            v = num(src)
            return [0xC6 | (op << 3), v & 0xFF]

        if mnem == "LD":
            return self.encode_ld(ops, addr, lineno, num, is_r8, r8)

        if mnem == "LDH":
            dst, src = ops[0].strip(), ops[1].strip()
            if dst.upper() == "A":
                # LDH A,(n)
                inner = src[1:-1].strip()
                if inner.upper() == "C":
                    return [0xF2]
                v = num(inner)
                return [0xF0, v & 0xFF]
            else:
                inner = dst[1:-1].strip()
                if inner.upper() == "C":
                    return [0xE2]
                v = num(inner)
                return [0xE0, v & 0xFF]

        raise AsmError(f"line {lineno}: unknown mnemonic '{mnem}'")

    def encode_ld(self, ops, addr, lineno, num, is_r8, r8):
        dst, src = ops[0].strip(), ops[1].strip()
        du, su = dst.upper(), src.upper()

        # LD (nn), SP
        if du.startswith("(") and su == "SP":
            v = num(dst[1:-1].strip())
            return [0x08, v & 0xFF, (v >> 8) & 0xFF]

        # LD SP, HL
        if du == "SP" and su == "HL":
            return [0xF9]

        # LD HL, SP+d
        if du == "HL" and su.startswith("SP"):
            m = re.match(r"SP\s*\+\s*(.+)", su)
            d = num(m.group(1))
            return [0xF8, d & 0xFF]

        # 16-bit immediate load: LD rp, nn  (BC/DE/HL/SP <- imm16 or label)
        if du in RP:
            v = num(src)
            return [0x01 | (RP[du] << 4), v & 0xFF, (v >> 8) & 0xFF]

        # (BC)/(DE)/(HL+)/(HL-) forms with A
        if du == "(BC)" and su == "A":
            return [0x02]
        if du == "(DE)" and su == "A":
            return [0x12]
        if su == "(BC)" and du == "A":
            return [0x0A]
        if su == "(DE)" and du == "A":
            return [0x1A]
        if du in ("(HL+)", "(HLI)") and su == "A":
            return [0x22]
        if du in ("(HL-)", "(HLD)") and su == "A":
            return [0x32]
        if su in ("(HL+)", "(HLI)") and du == "A":
            return [0x2A]
        if su in ("(HL-)", "(HLD)") and du == "A":
            return [0x3A]

        # LD (nn), A / LD A,(nn)
        if du == "A" and su.startswith("(") and su.endswith(")"):
            inner = su[1:-1].strip()
            if inner in R8 or inner in ("(HL)",):
                pass
            elif inner == "C":
                return [0xF2]
            elif not (inner in RP):
                v = num(src[1:-1].strip())
                return [0xFA, v & 0xFF, (v >> 8) & 0xFF]
        if su == "A" and du.startswith("(") and du.endswith(")"):
            inner = du[1:-1].strip()
            if inner == "C":
                return [0xE2]
            if inner not in RP and inner not in ("HL",):
                v = num(dst[1:-1].strip())
                return [0xEA, v & 0xFF, (v >> 8) & 0xFF]

        # 8-bit reg/(HL) <-> reg/(HL)
        if is_r8(dst) and is_r8(src):
            return [0x40 | (r8(du) << 3) | r8(su)]

        # 8-bit immediate: LD r, n
        if is_r8(dst):
            v = num(src)
            return [0x06 | (r8(du) << 3), v & 0xFF]

        raise AsmError(f"line {lineno}: cannot encode LD {dst},{src}")


def assemble_file(path):
    with open(path) as f:
        text = f.read()
    asm = Assembler()
    asm.load(text)
    rom = asm.assemble()
    return rom, asm.labels


def main():
    if len(sys.argv) != 3:
        print("usage: gbasm.py <input.asm> <output.gb>")
        sys.exit(1)
    rom, labels = assemble_file(sys.argv[1])
    with open(sys.argv[2], "wb") as f:
        f.write(rom)
    print(f"assembled {len(rom)} bytes -> {sys.argv[2]}")


if __name__ == "__main__":
    main()
