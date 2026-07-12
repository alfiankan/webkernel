// Table-driven SM83 (Game Boy CPU) disassembler, mirroring the opcode
// decomposition in emulator/src/cpu.rs (x/y/z/p/q over the opcode byte).
// `peek(addr)` must return the byte at a 16-bit address.
(() => {
  "use strict";

  const R8 = ["B", "C", "D", "E", "H", "L", "(HL)", "A"];
  const RP = ["BC", "DE", "HL", "SP"];
  const RP2 = ["BC", "DE", "HL", "AF"];
  const CC = ["NZ", "Z", "NC", "C"];
  const ALU = ["ADD A,", "ADC A,", "SUB ", "SBC A,", "AND ", "XOR ", "OR ", "CP "];
  const ROT = ["RLC ", "RRC ", "RL ", "RR ", "SLA ", "SRA ", "SWAP ", "SRL "];

  const hex8 = (v) => "$" + (v & 0xff).toString(16).toUpperCase().padStart(2, "0");
  const hex16 = (v) => "$" + (v & 0xffff).toString(16).toUpperCase().padStart(4, "0");

  function disassembleCB(peek, addr) {
    const op = peek((addr + 1) & 0xffff);
    const x = op >> 6;
    const y = (op >> 3) & 7;
    const z = op & 7;
    const reg = R8[z];
    let text;
    if (x === 0) text = `${ROT[y]}${reg}`;
    else if (x === 1) text = `BIT ${y},${reg}`;
    else if (x === 2) text = `RES ${y},${reg}`;
    else text = `SET ${y},${reg}`;
    return { text, len: 2 };
  }

  function disassemble(peek, addr) {
    const op = peek(addr);
    const x = op >> 6;
    const y = (op >> 3) & 7;
    const z = op & 7;
    const p = y >> 1;
    const q = y & 1;

    const imm8 = () => peek((addr + 1) & 0xffff);
    const imm16 = () => peek((addr + 1) & 0xffff) | (peek((addr + 2) & 0xffff) << 8);
    const rel = () => {
      const d = imm8();
      const signed = d & 0x80 ? d - 0x100 : d;
      return (addr + 2 + signed) & 0xffff;
    };

    if (x === 0) {
      switch (z) {
        case 0:
          if (y === 0) return { text: "NOP", len: 1 };
          if (y === 1) return { text: `LD (${hex16(imm16())}),SP`, len: 3 };
          if (y === 2) return { text: "STOP", len: 2 };
          if (y === 3) return { text: `JR ${hex16(rel())}`, len: 2 };
          return { text: `JR ${CC[y - 4]},${hex16(rel())}`, len: 2 };
        case 1:
          if (q === 0) return { text: `LD ${RP[p]},${hex16(imm16())}`, len: 3 };
          return { text: `ADD HL,${RP[p]}`, len: 1 };
        case 2: {
          const forms = ["(BC),A", "(DE),A", "(HL+),A", "(HL-),A"];
          const formsR = ["A,(BC)", "A,(DE)", "A,(HL+)", "A,(HL-)"];
          return { text: `LD ${q === 0 ? forms[p] : formsR[p]}`, len: 1 };
        }
        case 3:
          return { text: `${q === 0 ? "INC" : "DEC"} ${RP[p]}`, len: 1 };
        case 4:
          return { text: `INC ${R8[y]}`, len: 1 };
        case 5:
          return { text: `DEC ${R8[y]}`, len: 1 };
        case 6:
          return { text: `LD ${R8[y]},${hex8(imm8())}`, len: 2 };
        case 7:
          return {
            text: ["RLCA", "RRCA", "RLA", "RRA", "DAA", "CPL", "SCF", "CCF"][y],
            len: 1,
          };
      }
    }

    if (x === 1) {
      if (z === 6 && y === 6) return { text: "HALT", len: 1 };
      return { text: `LD ${R8[y]},${R8[z]}`, len: 1 };
    }

    if (x === 2) {
      return { text: `${ALU[y]}${R8[z]}`, len: 1 };
    }

    // x === 3
    switch (op) {
      case 0xc0: case 0xc8: case 0xd0: case 0xd8:
        return { text: `RET ${CC[y]}`, len: 1 };
      case 0xc9: return { text: "RET", len: 1 };
      case 0xd9: return { text: "RETI", len: 1 };
      case 0xc1: case 0xd1: case 0xe1: case 0xf1:
        return { text: `POP ${RP2[p]}`, len: 1 };
      case 0xc5: case 0xd5: case 0xe5: case 0xf5:
        return { text: `PUSH ${RP2[p]}`, len: 1 };
      case 0xc2: case 0xca: case 0xd2: case 0xda:
        return { text: `JP ${CC[y]},${hex16(imm16())}`, len: 3 };
      case 0xc3: return { text: `JP ${hex16(imm16())}`, len: 3 };
      case 0xe9: return { text: "JP HL", len: 1 };
      case 0xc4: case 0xcc: case 0xd4: case 0xdc:
        return { text: `CALL ${CC[y]},${hex16(imm16())}`, len: 3 };
      case 0xcd: return { text: `CALL ${hex16(imm16())}`, len: 3 };
      case 0xc6: case 0xce: case 0xd6: case 0xde:
      case 0xe6: case 0xee: case 0xf6: case 0xfe:
        return { text: `${ALU[y]}${hex8(imm8())}`, len: 2 };
      case 0xc7: case 0xcf: case 0xd7: case 0xdf:
      case 0xe7: case 0xef: case 0xf7: case 0xff:
        return { text: `RST ${hex8(y * 8)}`, len: 1 };
      case 0xcb: return disassembleCB(peek, addr);
      case 0xe0: return { text: `LDH (${hex8(imm8())}),A`, len: 2 };
      case 0xf0: return { text: `LDH A,(${hex8(imm8())})`, len: 2 };
      case 0xe2: return { text: "LD (C),A", len: 1 };
      case 0xf2: return { text: "LD A,(C)", len: 1 };
      case 0xea: return { text: `LD (${hex16(imm16())}),A`, len: 3 };
      case 0xfa: return { text: `LD A,(${hex16(imm16())})`, len: 3 };
      case 0xe8: {
        const d = imm8();
        const signed = d & 0x80 ? d - 0x100 : d;
        return { text: `ADD SP,${signed}`, len: 2 };
      }
      case 0xf8: {
        const d = imm8();
        const signed = d & 0x80 ? d - 0x100 : d;
        return { text: `LD HL,SP${signed >= 0 ? "+" : ""}${signed}`, len: 2 };
      }
      case 0xf9: return { text: "LD SP,HL", len: 1 };
      case 0xf3: return { text: "DI", len: 1 };
      case 0xfb: return { text: "EI", len: 1 };
      default:
        return { text: `DB ${hex8(op)} (illegal)`, len: 1 };
    }
  }

  window.GBDisasm = { disassemble };
})();
