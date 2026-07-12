use crate::apu::Apu;
use crate::cartridge::Cartridge;
use crate::joypad::Joypad;
use crate::ppu::Ppu;
use crate::timer::Timer;

pub struct Mmu {
    pub cart: Cartridge,
    pub ppu: Ppu,
    pub timer: Timer,
    pub joypad: Joypad,
    pub apu: Apu,
    wram: [u8; 0x2000],
    hram: [u8; 0x7F],
    pub if_flag: u8,
    pub ie_flag: u8,
    serial_data: u8,
    serial_ctrl: u8,
    dma_active: bool,
}

impl Mmu {
    pub fn new(rom: Vec<u8>) -> Self {
        Mmu {
            cart: Cartridge::new(rom),
            ppu: Ppu::new(),
            timer: Timer::new(),
            joypad: Joypad::new(),
            apu: Apu::new(),
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            if_flag: 0xE1,
            ie_flag: 0,
            serial_data: 0,
            serial_ctrl: 0,
            dma_active: false,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cart.read(addr),
            0x8000..=0x9FFF => self.ppu.read_vram(addr),
            0xA000..=0xBFFF => self.cart.read(addr),
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize],
            0xFE00..=0xFE9F => self.ppu.read_oam(addr),
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00 => self.joypad.read(),
            0xFF01 => self.serial_data,
            0xFF02 => self.serial_ctrl,
            0xFF04 => self.timer.div(),
            0xFF05 => self.timer.tima,
            0xFF06 => self.timer.tma,
            0xFF07 => self.timer.tac | 0xF8,
            0xFF0F => self.if_flag | 0xE0,
            0xFF10..=0xFF3F => self.apu.read(addr),
            0xFF40 => self.ppu.lcdc,
            0xFF41 => self.ppu.stat | 0x80,
            0xFF42 => self.ppu.scy,
            0xFF43 => self.ppu.scx,
            0xFF44 => self.ppu.ly,
            0xFF45 => self.ppu.lyc,
            0xFF46 => 0xFF,
            0xFF47 => self.ppu.bgp,
            0xFF48 => self.ppu.obp0,
            0xFF49 => self.ppu.obp1,
            0xFF4A => self.ppu.wy,
            0xFF4B => self.ppu.wx,
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie_flag,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x7FFF => self.cart.write(addr, val),
            0x8000..=0x9FFF => self.ppu.write_vram(addr, val),
            0xA000..=0xBFFF => self.cart.write(addr, val),
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = val,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = val,
            0xFE00..=0xFE9F => self.ppu.write_oam(addr, val),
            0xFEA0..=0xFEFF => {}
            0xFF00 => self.joypad.write(val),
            0xFF01 => self.serial_data = val,
            0xFF02 => self.serial_ctrl = val,
            0xFF04 => self.timer.reset_div(),
            0xFF05 => self.timer.tima = val,
            0xFF06 => self.timer.tma = val,
            0xFF07 => self.timer.tac = val,
            0xFF0F => self.if_flag = val & 0x1F,
            0xFF10..=0xFF3F => self.apu.write(addr, val),
            0xFF40 => self.ppu.lcdc = val,
            0xFF41 => self.ppu.stat = (self.ppu.stat & 0x07) | (val & 0xF8),
            0xFF42 => self.ppu.scy = val,
            0xFF43 => self.ppu.scx = val,
            0xFF44 => {}
            0xFF45 => self.ppu.lyc = val,
            0xFF46 => self.do_dma(val),
            0xFF47 => self.ppu.bgp = val,
            0xFF48 => self.ppu.obp0 = val,
            0xFF49 => self.ppu.obp1 = val,
            0xFF4A => self.ppu.wy = val,
            0xFF4B => self.ppu.wx = val,
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = val,
            0xFFFF => self.ie_flag = val,
            _ => {}
        }
    }

    fn do_dma(&mut self, val: u8) {
        self.dma_active = true;
        let src = (val as u16) << 8;
        for i in 0..0xA0u16 {
            let b = self.read(src + i);
            self.ppu.write_oam(0xFE00 + i, b);
        }
        self.dma_active = false;
    }

    /// Advance all timing-sensitive subsystems by the given number of T-cycles
    /// and fold any newly-raised interrupt sources into IF.
    pub fn tick_components(&mut self, cycles: u32) {
        self.timer.step(cycles);
        if self.timer.irq {
            self.timer.irq = false;
            self.if_flag |= 0x04;
        }

        self.ppu.step(cycles);
        if self.ppu.vblank_irq {
            self.ppu.vblank_irq = false;
            self.if_flag |= 0x01;
        }
        if self.ppu.stat_irq {
            self.ppu.stat_irq = false;
            self.if_flag |= 0x02;
        }

        if self.joypad.irq {
            self.joypad.irq = false;
            self.if_flag |= 0x10;
        }

        self.apu.step(cycles);
    }
}
