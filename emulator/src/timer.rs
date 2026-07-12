pub struct Timer {
    div: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    tima_acc: u32,
    pub irq: bool,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div: 0xAB00,
            tima: 0,
            tma: 0,
            tac: 0xF8,
            tima_acc: 0,
            irq: false,
        }
    }

    pub fn div(&self) -> u8 {
        (self.div >> 8) as u8
    }

    pub fn reset_div(&mut self) {
        self.div = 0;
    }

    fn period(&self) -> u32 {
        match self.tac & 0x03 {
            0 => 1024,
            1 => 16,
            2 => 64,
            3 => 256,
            _ => unreachable!(),
        }
    }

    pub fn step(&mut self, cycles: u32) {
        self.div = self.div.wrapping_add(cycles as u16);

        if self.tac & 0x04 == 0 {
            return;
        }
        self.tima_acc += cycles;
        let period = self.period();
        while self.tima_acc >= period {
            self.tima_acc -= period;
            let (r, overflow) = self.tima.overflowing_add(1);
            if overflow {
                self.tima = self.tma;
                self.irq = true;
            } else {
                self.tima = r;
            }
        }
    }
}
