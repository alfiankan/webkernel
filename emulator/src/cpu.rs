use crate::mmu::Mmu;

const FLAG_Z: u8 = 0b1000_0000;
const FLAG_N: u8 = 0b0100_0000;
const FLAG_H: u8 = 0b0010_0000;
const FLAG_C: u8 = 0b0001_0000;

#[derive(Default)]
pub struct Cpu {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
    pub ime_scheduled: bool,
    pub halted: bool,
    pub halt_bug: bool,
    pub stopped: bool,
}

impl Cpu {
    pub fn new() -> Self {
        let mut c = Cpu::default();
        // Post-bootrom DMG register state
        c.a = 0x01;
        c.f = 0xB0;
        c.b = 0x00;
        c.c = 0x13;
        c.d = 0x00;
        c.e = 0xD8;
        c.h = 0x01;
        c.l = 0x4D;
        c.sp = 0xFFFE;
        c.pc = 0x0100;
        c.ime = false;
        c
    }

    fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16 & 0xF0)
    }
    fn set_af(&mut self, v: u16) {
        self.a = (v >> 8) as u8;
        self.f = (v & 0xF0) as u8;
    }
    fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }
    fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = v as u8;
    }
    fn de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }
    fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = v as u8;
    }
    fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }

    fn flag(&self, mask: u8) -> bool {
        self.f & mask != 0
    }
    fn set_flag(&mut self, mask: u8, on: bool) {
        if on {
            self.f |= mask;
        } else {
            self.f &= !mask;
        }
    }

    fn imm8(&mut self, mmu: &mut Mmu) -> u8 {
        let v = mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }
    fn imm16(&mut self, mmu: &mut Mmu) -> u16 {
        let lo = self.imm8(mmu) as u16;
        let hi = self.imm8(mmu) as u16;
        (hi << 8) | lo
    }

    fn get_r8(&mut self, mmu: &mut Mmu, idx: u8) -> u8 {
        match idx {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => mmu.read(self.hl()),
            7 => self.a,
            _ => unreachable!(),
        }
    }
    fn set_r8(&mut self, mmu: &mut Mmu, idx: u8, v: u8) {
        match idx {
            0 => self.b = v,
            1 => self.c = v,
            2 => self.d = v,
            3 => self.e = v,
            4 => self.h = v,
            5 => self.l = v,
            6 => mmu.write(self.hl(), v),
            7 => self.a = v,
            _ => unreachable!(),
        }
    }

    fn get_rp(&self, p: u8) -> u16 {
        match p {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.sp,
            _ => unreachable!(),
        }
    }
    fn set_rp(&mut self, p: u8, v: u16) {
        match p {
            0 => self.set_bc(v),
            1 => self.set_de(v),
            2 => self.set_hl(v),
            3 => self.sp = v,
            _ => unreachable!(),
        }
    }
    fn get_rp2(&self, p: u8) -> u16 {
        match p {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.af(),
            _ => unreachable!(),
        }
    }
    fn set_rp2(&mut self, p: u8, v: u16) {
        match p {
            0 => self.set_bc(v),
            1 => self.set_de(v),
            2 => self.set_hl(v),
            3 => self.set_af(v),
            _ => unreachable!(),
        }
    }

    fn cond(&self, y: u8) -> bool {
        match y & 3 {
            0 => !self.flag(FLAG_Z),
            1 => self.flag(FLAG_Z),
            2 => !self.flag(FLAG_C),
            3 => self.flag(FLAG_C),
            _ => unreachable!(),
        }
    }

    fn push(&mut self, mmu: &mut Mmu, v: u16) {
        self.sp = self.sp.wrapping_sub(1);
        mmu.write(self.sp, (v >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        mmu.write(self.sp, v as u8);
    }
    fn pop(&mut self, mmu: &mut Mmu) -> u16 {
        let lo = mmu.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let hi = mmu.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (hi << 8) | lo
    }

    fn alu(&mut self, op: u8, val: u8) {
        let a = self.a;
        match op {
            0 => {
                // ADD
                let (r, c) = a.overflowing_add(val);
                let h = (a & 0xF) + (val & 0xF) > 0xF;
                self.a = r;
                self.set_flag(FLAG_Z, r == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, h);
                self.set_flag(FLAG_C, c);
            }
            1 => {
                // ADC
                let carry = if self.flag(FLAG_C) { 1u16 } else { 0 };
                let sum = a as u16 + val as u16 + carry;
                let h = (a & 0xF) + (val & 0xF) + carry as u8 > 0xF;
                self.a = sum as u8;
                self.set_flag(FLAG_Z, self.a == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, h);
                self.set_flag(FLAG_C, sum > 0xFF);
            }
            2 => {
                // SUB
                let (r, c) = a.overflowing_sub(val);
                let h = (a & 0xF) < (val & 0xF);
                self.a = r;
                self.set_flag(FLAG_Z, r == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, h);
                self.set_flag(FLAG_C, c);
            }
            3 => {
                // SBC
                let carry = if self.flag(FLAG_C) { 1i16 } else { 0 };
                let diff = a as i16 - val as i16 - carry;
                let h = (a as i16 & 0xF) - (val as i16 & 0xF) - carry < 0;
                self.a = diff as u8;
                self.set_flag(FLAG_Z, self.a == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, h);
                self.set_flag(FLAG_C, diff < 0);
            }
            4 => {
                // AND
                self.a &= val;
                self.set_flag(FLAG_Z, self.a == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                self.set_flag(FLAG_C, false);
            }
            5 => {
                // XOR
                self.a ^= val;
                self.set_flag(FLAG_Z, self.a == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            6 => {
                // OR
                self.a |= val;
                self.set_flag(FLAG_Z, self.a == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            7 => {
                // CP
                let (r, c) = a.overflowing_sub(val);
                let h = (a & 0xF) < (val & 0xF);
                self.set_flag(FLAG_Z, r == 0);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, h);
                self.set_flag(FLAG_C, c);
            }
            _ => unreachable!(),
        }
    }

    fn inc8(&mut self, v: u8) -> u8 {
        let r = v.wrapping_add(1);
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, (v & 0xF) == 0xF);
        r
    }
    fn dec8(&mut self, v: u8) -> u8 {
        let r = v.wrapping_sub(1);
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, true);
        self.set_flag(FLAG_H, (v & 0xF) == 0);
        r
    }

    fn rlc(&mut self, v: u8) -> u8 {
        let c = v & 0x80 != 0;
        let r = v.rotate_left(1);
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, c);
        r
    }
    fn rrc(&mut self, v: u8) -> u8 {
        let c = v & 0x01 != 0;
        let r = v.rotate_right(1);
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, c);
        r
    }
    fn rl(&mut self, v: u8) -> u8 {
        let old_c = if self.flag(FLAG_C) { 1 } else { 0 };
        let c = v & 0x80 != 0;
        let r = (v << 1) | old_c;
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, c);
        r
    }
    fn rr(&mut self, v: u8) -> u8 {
        let old_c = if self.flag(FLAG_C) { 0x80 } else { 0 };
        let c = v & 0x01 != 0;
        let r = (v >> 1) | old_c;
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, c);
        r
    }
    fn sla(&mut self, v: u8) -> u8 {
        let c = v & 0x80 != 0;
        let r = v << 1;
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, c);
        r
    }
    fn sra(&mut self, v: u8) -> u8 {
        let c = v & 0x01 != 0;
        let r = (v >> 1) | (v & 0x80);
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, c);
        r
    }
    fn swap(&mut self, v: u8) -> u8 {
        let r = (v << 4) | (v >> 4);
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, false);
        r
    }
    fn srl(&mut self, v: u8) -> u8 {
        let c = v & 0x01 != 0;
        let r = v >> 1;
        self.set_flag(FLAG_Z, r == 0);
        self.set_flag(FLAG_N, false);
        self.set_flag(FLAG_H, false);
        self.set_flag(FLAG_C, c);
        r
    }

    /// Executes one instruction (or services HALT/interrupt) and returns elapsed T-cycles.
    pub fn step(&mut self, mmu: &mut Mmu) -> u32 {
        // EI's effect takes hold before the instruction following EI executes,
        // so a HALT immediately after EI must see the updated IME when
        // deciding whether the halt-bug applies.
        if self.ime_scheduled {
            self.ime_scheduled = false;
            self.ime = true;
        }

        if self.halted {
            if mmu.if_flag & mmu.ie_flag & 0x1F != 0 {
                self.halted = false;
            } else {
                return 4;
            }
        }

        let cycles = if !self.halted {
            let op = self.imm8(mmu);
            if self.halt_bug {
                self.halt_bug = false;
                self.pc = self.pc.wrapping_sub(1);
            }
            self.execute(mmu, op)
        } else {
            4
        };

        let isr_cycles = self.handle_interrupts(mmu);

        cycles + isr_cycles
    }

    fn handle_interrupts(&mut self, mmu: &mut Mmu) -> u32 {
        let pending = mmu.if_flag & mmu.ie_flag & 0x1F;
        if pending == 0 {
            return 0;
        }
        if !self.ime {
            return 0;
        }
        self.ime = false;
        self.halted = false;
        let bit = pending.trailing_zeros();
        mmu.if_flag &= !(1 << bit);
        let vector = 0x40 + (bit as u16) * 8;
        self.push(mmu, self.pc);
        self.pc = vector;
        20
    }

    fn execute(&mut self, mmu: &mut Mmu, op: u8) -> u32 {
        let x = op >> 6;
        let y = (op >> 3) & 7;
        let z = op & 7;
        let p = y >> 1;
        let q = y & 1;

        match x {
            0 => match z {
                0 => match y {
                    0 => 4, // NOP
                    1 => {
                        // LD (nn), SP
                        let addr = self.imm16(mmu);
                        mmu.write(addr, self.sp as u8);
                        mmu.write(addr.wrapping_add(1), (self.sp >> 8) as u8);
                        20
                    }
                    2 => {
                        // STOP
                        self.imm8(mmu);
                        self.stopped = true;
                        4
                    }
                    3 => {
                        // JR d
                        let d = self.imm8(mmu) as i8;
                        self.pc = self.pc.wrapping_add(d as i16 as u16);
                        12
                    }
                    4..=7 => {
                        let d = self.imm8(mmu) as i8;
                        if self.cond(y - 4) {
                            self.pc = self.pc.wrapping_add(d as i16 as u16);
                            12
                        } else {
                            8
                        }
                    }
                    _ => unreachable!(),
                },
                1 => {
                    if q == 0 {
                        let nn = self.imm16(mmu);
                        self.set_rp(p, nn);
                        12
                    } else {
                        let hl = self.hl();
                        let rp = self.get_rp(p);
                        let (r, c) = hl.overflowing_add(rp);
                        let h = (hl & 0xFFF) + (rp & 0xFFF) > 0xFFF;
                        self.set_hl(r);
                        self.set_flag(FLAG_N, false);
                        self.set_flag(FLAG_H, h);
                        self.set_flag(FLAG_C, c);
                        8
                    }
                }
                2 => {
                    if q == 0 {
                        match p {
                            0 => mmu.write(self.bc(), self.a),
                            1 => mmu.write(self.de(), self.a),
                            2 => {
                                let hl = self.hl();
                                mmu.write(hl, self.a);
                                self.set_hl(hl.wrapping_add(1));
                            }
                            3 => {
                                let hl = self.hl();
                                mmu.write(hl, self.a);
                                self.set_hl(hl.wrapping_sub(1));
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        match p {
                            0 => self.a = mmu.read(self.bc()),
                            1 => self.a = mmu.read(self.de()),
                            2 => {
                                let hl = self.hl();
                                self.a = mmu.read(hl);
                                self.set_hl(hl.wrapping_add(1));
                            }
                            3 => {
                                let hl = self.hl();
                                self.a = mmu.read(hl);
                                self.set_hl(hl.wrapping_sub(1));
                            }
                            _ => unreachable!(),
                        }
                    }
                    8
                }
                3 => {
                    let rp = self.get_rp(p);
                    if q == 0 {
                        self.set_rp(p, rp.wrapping_add(1));
                    } else {
                        self.set_rp(p, rp.wrapping_sub(1));
                    }
                    8
                }
                4 => {
                    let v = self.get_r8(mmu, y);
                    let r = self.inc8(v);
                    self.set_r8(mmu, y, r);
                    if y == 6 {
                        12
                    } else {
                        4
                    }
                }
                5 => {
                    let v = self.get_r8(mmu, y);
                    let r = self.dec8(v);
                    self.set_r8(mmu, y, r);
                    if y == 6 {
                        12
                    } else {
                        4
                    }
                }
                6 => {
                    let n = self.imm8(mmu);
                    self.set_r8(mmu, y, n);
                    if y == 6 {
                        12
                    } else {
                        8
                    }
                }
                7 => {
                    match y {
                        0 => {
                            // RLCA
                            let r = self.rlc(self.a);
                            self.a = r;
                            self.set_flag(FLAG_Z, false);
                        }
                        1 => {
                            let r = self.rrc(self.a);
                            self.a = r;
                            self.set_flag(FLAG_Z, false);
                        }
                        2 => {
                            let r = self.rl(self.a);
                            self.a = r;
                            self.set_flag(FLAG_Z, false);
                        }
                        3 => {
                            let r = self.rr(self.a);
                            self.a = r;
                            self.set_flag(FLAG_Z, false);
                        }
                        4 => {
                            // DAA
                            let mut a = self.a;
                            let mut adjust = 0u8;
                            let mut carry = self.flag(FLAG_C);
                            if self.flag(FLAG_H) || (!self.flag(FLAG_N) && (a & 0xF) > 9) {
                                adjust |= 0x06;
                            }
                            if self.flag(FLAG_C) || (!self.flag(FLAG_N) && a > 0x99) {
                                adjust |= 0x60;
                                carry = true;
                            }
                            if self.flag(FLAG_N) {
                                a = a.wrapping_sub(adjust);
                            } else {
                                a = a.wrapping_add(adjust);
                            }
                            self.a = a;
                            self.set_flag(FLAG_Z, a == 0);
                            self.set_flag(FLAG_H, false);
                            self.set_flag(FLAG_C, carry);
                        }
                        5 => {
                            // CPL
                            self.a = !self.a;
                            self.set_flag(FLAG_N, true);
                            self.set_flag(FLAG_H, true);
                        }
                        6 => {
                            // SCF
                            self.set_flag(FLAG_N, false);
                            self.set_flag(FLAG_H, false);
                            self.set_flag(FLAG_C, true);
                        }
                        7 => {
                            // CCF
                            self.set_flag(FLAG_N, false);
                            self.set_flag(FLAG_H, false);
                            let c = self.flag(FLAG_C);
                            self.set_flag(FLAG_C, !c);
                        }
                        _ => unreachable!(),
                    }
                    4
                }
                _ => unreachable!(),
            },
            1 => {
                if z == 6 && y == 6 {
                    // HALT
                    if !self.ime && (mmu.if_flag & mmu.ie_flag & 0x1F != 0) {
                        self.halt_bug = true;
                    } else {
                        self.halted = true;
                    }
                    4
                } else {
                    let v = self.get_r8(mmu, z);
                    self.set_r8(mmu, y, v);
                    if y == 6 || z == 6 {
                        8
                    } else {
                        4
                    }
                }
            }
            2 => {
                let v = self.get_r8(mmu, z);
                self.alu(y, v);
                if z == 6 {
                    8
                } else {
                    4
                }
            }
            3 => self.execute_x3(mmu, op, y, z, p, q),
            _ => unreachable!(),
        }
    }

    fn execute_x3(&mut self, mmu: &mut Mmu, op: u8, y: u8, z: u8, p: u8, q: u8) -> u32 {
        match op {
            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                if self.cond(y) {
                    self.pc = self.pop(mmu);
                    20
                } else {
                    8
                }
            }
            0xC9 => {
                self.pc = self.pop(mmu);
                16
            }
            0xD9 => {
                self.pc = self.pop(mmu);
                self.ime = true;
                16
            }
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                let v = self.pop(mmu);
                self.set_rp2(p, v);
                12
            }
            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                let v = self.get_rp2(p);
                self.push(mmu, v);
                16
            }
            0xC2 | 0xCA | 0xD2 | 0xDA => {
                let nn = self.imm16(mmu);
                if self.cond(y) {
                    self.pc = nn;
                    16
                } else {
                    12
                }
            }
            0xC3 => {
                self.pc = self.imm16(mmu);
                16
            }
            0xE9 => {
                self.pc = self.hl();
                4
            }
            0xC4 | 0xCC | 0xD4 | 0xDC => {
                let nn = self.imm16(mmu);
                if self.cond(y) {
                    self.push(mmu, self.pc);
                    self.pc = nn;
                    24
                } else {
                    12
                }
            }
            0xCD => {
                let nn = self.imm16(mmu);
                self.push(mmu, self.pc);
                self.pc = nn;
                24
            }
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                let n = self.imm8(mmu);
                self.alu(y, n);
                8
            }
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                self.push(mmu, self.pc);
                self.pc = (y as u16) * 8;
                16
            }
            0xCB => self.execute_cb(mmu),
            0xE0 => {
                let n = self.imm8(mmu);
                mmu.write(0xFF00 + n as u16, self.a);
                12
            }
            0xF0 => {
                let n = self.imm8(mmu);
                self.a = mmu.read(0xFF00 + n as u16);
                12
            }
            0xE2 => {
                mmu.write(0xFF00 + self.c as u16, self.a);
                8
            }
            0xF2 => {
                self.a = mmu.read(0xFF00 + self.c as u16);
                8
            }
            0xEA => {
                let nn = self.imm16(mmu);
                mmu.write(nn, self.a);
                16
            }
            0xFA => {
                let nn = self.imm16(mmu);
                self.a = mmu.read(nn);
                16
            }
            0xE8 => {
                let d = self.imm8(mmu) as i8 as i16 as u16;
                let sp = self.sp;
                let r = sp.wrapping_add(d);
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (sp & 0xF) + (d & 0xF) > 0xF);
                self.set_flag(FLAG_C, (sp & 0xFF) + (d & 0xFF) > 0xFF);
                self.sp = r;
                16
            }
            0xF8 => {
                let d = self.imm8(mmu) as i8 as i16 as u16;
                let sp = self.sp;
                let r = sp.wrapping_add(d);
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (sp & 0xF) + (d & 0xF) > 0xF);
                self.set_flag(FLAG_C, (sp & 0xFF) + (d & 0xFF) > 0xFF);
                self.set_hl(r);
                12
            }
            0xF9 => {
                self.sp = self.hl();
                8
            }
            0xF3 => {
                self.ime = false;
                self.ime_scheduled = false;
                4
            }
            0xFB => {
                self.ime_scheduled = true;
                4
            }
            _ => {
                // Illegal opcode (D3,DB,DD,E3,E4,EB,EC,ED,F4,FC,FD): treat as NOP to stay resilient.
                let _ = (z, q);
                4
            }
        }
    }

    fn execute_cb(&mut self, mmu: &mut Mmu) -> u32 {
        let op = self.imm8(mmu);
        let x = op >> 6;
        let y = (op >> 3) & 7;
        let z = op & 7;
        let v = self.get_r8(mmu, z);
        let cycles_base = if z == 6 { 16 } else { 8 };
        match x {
            0 => {
                let r = match y {
                    0 => self.rlc(v),
                    1 => self.rrc(v),
                    2 => self.rl(v),
                    3 => self.rr(v),
                    4 => self.sla(v),
                    5 => self.sra(v),
                    6 => self.swap(v),
                    7 => self.srl(v),
                    _ => unreachable!(),
                };
                self.set_r8(mmu, z, r);
                cycles_base
            }
            1 => {
                // BIT y, r[z]
                let bit = v & (1 << y) == 0;
                self.set_flag(FLAG_Z, bit);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                if z == 6 {
                    12
                } else {
                    8
                }
            }
            2 => {
                // RES y, r[z]
                let r = v & !(1 << y);
                self.set_r8(mmu, z, r);
                cycles_base
            }
            3 => {
                // SET y, r[z]
                let r = v | (1 << y);
                self.set_r8(mmu, z, r);
                cycles_base
            }
            _ => unreachable!(),
        }
    }
}
