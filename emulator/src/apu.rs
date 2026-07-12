const SAMPLE_RATE: u32 = 44100;
const CPU_CLOCK: u32 = 4_194_304;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

struct Square {
    enabled: bool,
    dac_on: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_timer: u8,
    shadow_freq: u16,
    sweep_enabled: bool,
    duty: u8,
    duty_pos: u8,
    length: u16,
    length_enabled: bool,
    freq: u16,
    timer: i32,
    vol_start: u8,
    vol_current: u8,
    env_add: bool,
    env_period: u8,
    env_timer: u8,
    has_sweep: bool,
}

impl Square {
    fn new(has_sweep: bool) -> Self {
        Square {
            enabled: false,
            dac_on: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_timer: 0,
            shadow_freq: 0,
            sweep_enabled: false,
            duty: 2,
            duty_pos: 0,
            length: 0,
            length_enabled: false,
            freq: 0,
            timer: 0,
            vol_start: 0,
            vol_current: 0,
            env_add: false,
            env_period: 0,
            env_timer: 0,
            has_sweep,
        }
    }

    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length == 0 {
            self.length = 64;
        }
        self.timer = ((2048 - self.freq as i32) * 4).max(1);
        self.vol_current = self.vol_start;
        self.env_timer = self.env_period;
        self.shadow_freq = self.freq;
        self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
        self.sweep_enabled = self.sweep_period != 0 || self.sweep_shift != 0;
        if self.has_sweep && self.sweep_shift != 0 {
            self.sweep_calc();
        }
    }

    fn sweep_calc(&mut self) -> u16 {
        let delta = self.shadow_freq >> self.sweep_shift;
        let new_freq = if self.sweep_negate {
            self.shadow_freq.wrapping_sub(delta)
        } else {
            self.shadow_freq.wrapping_add(delta)
        };
        if new_freq > 2047 {
            self.enabled = false;
        }
        new_freq
    }

    fn step_sweep(&mut self) {
        if !self.has_sweep {
            return;
        }
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }
        if self.sweep_timer == 0 {
            self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
            if self.sweep_enabled && self.sweep_period > 0 {
                let new_freq = self.sweep_calc();
                if new_freq <= 2047 && self.sweep_shift != 0 {
                    self.freq = new_freq;
                    self.shadow_freq = new_freq;
                    self.sweep_calc();
                }
            }
        }
    }

    fn step_length(&mut self) {
        if self.length_enabled && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                self.enabled = false;
            }
        }
    }

    fn step_envelope(&mut self) {
        if self.env_period == 0 {
            return;
        }
        if self.env_timer > 0 {
            self.env_timer -= 1;
        }
        if self.env_timer == 0 {
            self.env_timer = self.env_period;
            if self.env_add && self.vol_current < 15 {
                self.vol_current += 1;
            } else if !self.env_add && self.vol_current > 0 {
                self.vol_current -= 1;
            }
        }
    }

    fn step(&mut self, cycles: i32) {
        self.timer -= cycles;
        while self.timer <= 0 {
            self.timer += ((2048 - self.freq as i32) * 4).max(1);
            self.duty_pos = (self.duty_pos + 1) % 8;
        }
    }

    fn output(&self) -> f32 {
        if !self.enabled || !self.dac_on {
            return 0.0;
        }
        let bit = DUTY_TABLE[self.duty as usize][self.duty_pos as usize];
        if bit == 1 {
            self.vol_current as f32 / 15.0
        } else {
            0.0
        }
    }
}

struct Wave {
    enabled: bool,
    dac_on: bool,
    length: u16,
    length_enabled: bool,
    freq: u16,
    timer: i32,
    volume_shift: u8,
    ram: [u8; 16],
    pos: u8,
}

impl Wave {
    fn new() -> Self {
        Wave {
            enabled: false,
            dac_on: false,
            length: 0,
            length_enabled: false,
            freq: 0,
            timer: 0,
            volume_shift: 0,
            ram: [0; 16],
            pos: 0,
        }
    }

    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length == 0 {
            self.length = 256;
        }
        self.timer = ((2048 - self.freq as i32) * 2).max(1);
        self.pos = 0;
    }

    fn step_length(&mut self) {
        if self.length_enabled && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                self.enabled = false;
            }
        }
    }

    fn step(&mut self, cycles: i32) {
        self.timer -= cycles;
        while self.timer <= 0 {
            self.timer += ((2048 - self.freq as i32) * 2).max(1);
            self.pos = (self.pos + 1) % 32;
        }
    }

    fn output(&self) -> f32 {
        if !self.enabled || !self.dac_on || self.volume_shift == 0 {
            return 0.0;
        }
        let byte = self.ram[(self.pos / 2) as usize];
        let sample = if self.pos % 2 == 0 { byte >> 4 } else { byte & 0x0F };
        let shifted = sample >> (self.volume_shift - 1);
        shifted as f32 / 15.0
    }
}

struct Noise {
    enabled: bool,
    dac_on: bool,
    length: u16,
    length_enabled: bool,
    vol_start: u8,
    vol_current: u8,
    env_add: bool,
    env_period: u8,
    env_timer: u8,
    shift: u8,
    width_mode: bool,
    divisor_code: u8,
    lfsr: u16,
    timer: i32,
}

const DIVISORS: [i32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

impl Noise {
    fn new() -> Self {
        Noise {
            enabled: false,
            dac_on: false,
            length: 0,
            length_enabled: false,
            vol_start: 0,
            vol_current: 0,
            env_add: false,
            env_period: 0,
            env_timer: 0,
            shift: 0,
            width_mode: false,
            divisor_code: 0,
            lfsr: 0x7FFF,
            timer: 0,
        }
    }

    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length == 0 {
            self.length = 64;
        }
        self.vol_current = self.vol_start;
        self.env_timer = self.env_period;
        self.lfsr = 0x7FFF;
        self.timer = DIVISORS[self.divisor_code as usize] << self.shift;
    }

    fn step_length(&mut self) {
        if self.length_enabled && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                self.enabled = false;
            }
        }
    }

    fn step_envelope(&mut self) {
        if self.env_period == 0 {
            return;
        }
        if self.env_timer > 0 {
            self.env_timer -= 1;
        }
        if self.env_timer == 0 {
            self.env_timer = self.env_period;
            if self.env_add && self.vol_current < 15 {
                self.vol_current += 1;
            } else if !self.env_add && self.vol_current > 0 {
                self.vol_current -= 1;
            }
        }
    }

    fn step(&mut self, cycles: i32) {
        self.timer -= cycles;
        while self.timer <= 0 {
            self.timer += (DIVISORS[self.divisor_code as usize] << self.shift).max(1);
            let bit = (self.lfsr & 1) ^ ((self.lfsr >> 1) & 1);
            self.lfsr = (self.lfsr >> 1) | (bit << 14);
            if self.width_mode {
                self.lfsr = (self.lfsr & !0x40) | (bit << 6);
            }
        }
    }

    fn output(&self) -> f32 {
        if !self.enabled || !self.dac_on {
            return 0.0;
        }
        if self.lfsr & 1 == 0 {
            self.vol_current as f32 / 15.0
        } else {
            0.0
        }
    }
}

pub struct Apu {
    pub enabled: bool,
    ch1: Square,
    ch2: Square,
    ch3: Wave,
    ch4: Noise,
    nr50: u8,
    nr51: u8,
    frame_seq: u8,
    frame_cycle_acc: i32,
    sample_acc: i32,
    pub buffer: Vec<f32>,
}

impl Apu {
    pub fn new() -> Self {
        Apu {
            enabled: true,
            ch1: Square::new(true),
            ch2: Square::new(false),
            ch3: Wave::new(),
            ch4: Noise::new(),
            nr50: 0x77,
            nr51: 0xF3,
            frame_seq: 0,
            frame_cycle_acc: 0,
            sample_acc: 0,
            buffer: Vec::with_capacity(4096),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => 0x80 | (self.ch1.sweep_period << 4) | (if self.ch1.sweep_negate {0x08} else {0}) | self.ch1.sweep_shift,
            0xFF11 => (self.ch1.duty << 6) | 0x3F,
            0xFF12 => (self.ch1.vol_start << 4) | (if self.ch1.env_add {0x08} else {0}) | self.ch1.env_period,
            0xFF13 => 0xFF,
            0xFF14 => 0xBF | (if self.ch1.length_enabled {0x40} else {0}),
            0xFF16 => (self.ch2.duty << 6) | 0x3F,
            0xFF17 => (self.ch2.vol_start << 4) | (if self.ch2.env_add {0x08} else {0}) | self.ch2.env_period,
            0xFF19 => 0xBF | (if self.ch2.length_enabled {0x40} else {0}),
            0xFF1A => if self.ch3.dac_on {0xFF} else {0x7F},
            0xFF1C => 0x9F | (self.ch3.volume_shift << 5),
            0xFF1E => 0xBF | (if self.ch3.length_enabled {0x40} else {0}),
            0xFF21 => (self.ch4.vol_start << 4) | (if self.ch4.env_add {0x08} else {0}) | self.ch4.env_period,
            0xFF22 => (self.ch4.shift << 4) | (if self.ch4.width_mode {0x08} else {0}) | self.ch4.divisor_code,
            0xFF23 => 0xBF | (if self.ch4.length_enabled {0x40} else {0}),
            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => {
                let mut v = if self.enabled {0x80} else {0};
                v |= if self.ch1.enabled {1} else {0};
                v |= if self.ch2.enabled {2} else {0};
                v |= if self.ch3.enabled {4} else {0};
                v |= if self.ch4.enabled {8} else {0};
                v | 0x70
            }
            0xFF30..=0xFF3F => self.ch3.ram[(addr - 0xFF30) as usize],
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        if !self.enabled && addr != 0xFF26 && !(0xFF30..=0xFF3F).contains(&addr) {
            return;
        }
        match addr {
            0xFF10 => {
                self.ch1.sweep_period = (val >> 4) & 0x07;
                self.ch1.sweep_negate = val & 0x08 != 0;
                self.ch1.sweep_shift = val & 0x07;
            }
            0xFF11 => {
                self.ch1.duty = val >> 6;
                self.ch1.length = 64 - (val & 0x3F) as u16;
            }
            0xFF12 => {
                self.ch1.vol_start = val >> 4;
                self.ch1.env_add = val & 0x08 != 0;
                self.ch1.env_period = val & 0x07;
                self.ch1.dac_on = val & 0xF8 != 0;
                if !self.ch1.dac_on {
                    self.ch1.enabled = false;
                }
            }
            0xFF13 => self.ch1.freq = (self.ch1.freq & 0x700) | val as u16,
            0xFF14 => {
                self.ch1.freq = (self.ch1.freq & 0xFF) | (((val & 0x07) as u16) << 8);
                self.ch1.length_enabled = val & 0x40 != 0;
                if val & 0x80 != 0 {
                    self.ch1.trigger();
                }
            }
            0xFF16 => {
                self.ch2.duty = val >> 6;
                self.ch2.length = 64 - (val & 0x3F) as u16;
            }
            0xFF17 => {
                self.ch2.vol_start = val >> 4;
                self.ch2.env_add = val & 0x08 != 0;
                self.ch2.env_period = val & 0x07;
                self.ch2.dac_on = val & 0xF8 != 0;
                if !self.ch2.dac_on {
                    self.ch2.enabled = false;
                }
            }
            0xFF18 => self.ch2.freq = (self.ch2.freq & 0x700) | val as u16,
            0xFF19 => {
                self.ch2.freq = (self.ch2.freq & 0xFF) | (((val & 0x07) as u16) << 8);
                self.ch2.length_enabled = val & 0x40 != 0;
                if val & 0x80 != 0 {
                    self.ch2.trigger();
                }
            }
            0xFF1A => {
                self.ch3.dac_on = val & 0x80 != 0;
                if !self.ch3.dac_on {
                    self.ch3.enabled = false;
                }
            }
            0xFF1B => self.ch3.length = 256 - val as u16,
            0xFF1C => self.ch3.volume_shift = (val >> 5) & 0x03,
            0xFF1D => self.ch3.freq = (self.ch3.freq & 0x700) | val as u16,
            0xFF1E => {
                self.ch3.freq = (self.ch3.freq & 0xFF) | (((val & 0x07) as u16) << 8);
                self.ch3.length_enabled = val & 0x40 != 0;
                if val & 0x80 != 0 {
                    self.ch3.trigger();
                }
            }
            0xFF20 => self.ch4.length = 64 - (val & 0x3F) as u16,
            0xFF21 => {
                self.ch4.vol_start = val >> 4;
                self.ch4.env_add = val & 0x08 != 0;
                self.ch4.env_period = val & 0x07;
                self.ch4.dac_on = val & 0xF8 != 0;
                if !self.ch4.dac_on {
                    self.ch4.enabled = false;
                }
            }
            0xFF22 => {
                self.ch4.shift = val >> 4;
                self.ch4.width_mode = val & 0x08 != 0;
                self.ch4.divisor_code = val & 0x07;
            }
            0xFF23 => {
                self.ch4.length_enabled = val & 0x40 != 0;
                if val & 0x80 != 0 {
                    self.ch4.trigger();
                }
            }
            0xFF24 => self.nr50 = val,
            0xFF25 => self.nr51 = val,
            0xFF26 => {
                self.enabled = val & 0x80 != 0;
            }
            0xFF30..=0xFF3F => self.ch3.ram[(addr - 0xFF30) as usize] = val,
            _ => {}
        }
    }

    pub fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }
        self.ch1.step(cycles as i32);
        self.ch2.step(cycles as i32);
        self.ch3.step(cycles as i32);
        self.ch4.step(cycles as i32);

        self.frame_cycle_acc += cycles as i32;
        while self.frame_cycle_acc >= 8192 {
            self.frame_cycle_acc -= 8192;
            match self.frame_seq {
                0 => {
                    self.ch1.step_length();
                    self.ch2.step_length();
                    self.ch3.step_length();
                    self.ch4.step_length();
                }
                2 => {
                    self.ch1.step_length();
                    self.ch2.step_length();
                    self.ch3.step_length();
                    self.ch4.step_length();
                    self.ch1.step_sweep();
                }
                4 => {
                    self.ch1.step_length();
                    self.ch2.step_length();
                    self.ch3.step_length();
                    self.ch4.step_length();
                }
                6 => {
                    self.ch1.step_length();
                    self.ch2.step_length();
                    self.ch3.step_length();
                    self.ch4.step_length();
                    self.ch1.step_sweep();
                }
                7 => {
                    self.ch1.step_envelope();
                    self.ch2.step_envelope();
                    self.ch4.step_envelope();
                }
                _ => {}
            }
            self.frame_seq = (self.frame_seq + 1) % 8;
        }

        self.sample_acc += cycles as i32;
        let period = (CPU_CLOCK / SAMPLE_RATE) as i32;
        while self.sample_acc >= period {
            self.sample_acc -= period;
            self.push_sample();
        }
    }

    fn push_sample(&mut self) {
        let c1 = self.ch1.output();
        let c2 = self.ch2.output();
        let c3 = self.ch3.output();
        let c4 = self.ch4.output();

        let mut left = 0.0;
        let mut right = 0.0;
        if self.nr51 & 0x01 != 0 {
            right += c1;
        }
        if self.nr51 & 0x02 != 0 {
            right += c2;
        }
        if self.nr51 & 0x04 != 0 {
            right += c3;
        }
        if self.nr51 & 0x08 != 0 {
            right += c4;
        }
        if self.nr51 & 0x10 != 0 {
            left += c1;
        }
        if self.nr51 & 0x20 != 0 {
            left += c2;
        }
        if self.nr51 & 0x40 != 0 {
            left += c3;
        }
        if self.nr51 & 0x80 != 0 {
            left += c4;
        }

        let left_vol = ((self.nr50 & 0x07) + 1) as f32 / 8.0;
        let right_vol = (((self.nr50 >> 4) & 0x07) + 1) as f32 / 8.0;

        self.buffer.push((left / 4.0) * left_vol);
        self.buffer.push((right / 4.0) * right_vol);
    }

    pub fn take_buffer(&mut self) -> Vec<f32> {
        core::mem::take(&mut self.buffer)
    }
}
